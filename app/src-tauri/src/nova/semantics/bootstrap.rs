use serde_json::json;

use super::knowledge::canonical;
use super::types::{
    CanonicalCognitiveState, EnglishBootstrapObservation, EnglishClauseObservation,
    EnglishContextLink, EnglishPhraseObservation, SurfaceSentence, SurfaceToken,
};

pub const BASIC_ENGLISH_BOOTSTRAP_VERSION: &str = "7.0.0-alpha.9.3.1-basic-english-bootstrap";

const DETERMINERS: &[&str] = &["a", "an", "the", "this", "that", "my", "your", "our", "its"];
const COPULAR_LINKS: &[&str] = &["am", "is", "are", "was", "were", "be", "been", "being"];
const AUXILIARY_LINKS: &[&str] = &[
    "can", "could", "will", "would", "shall", "should", "may", "might", "must",
    "do", "does", "did", "have", "has", "had", "cannot", "can't", "cant",
    "won't", "wont", "don't", "dont", "doesn't", "doesnt", "didn't", "didnt",
];
const NEGATION_MARKERS: &[&str] = &["not", "cannot", "can't", "cant", "won't", "wont", "don't", "dont", "doesn't", "doesnt", "didn't", "didnt"];
const COORDINATORS: &[&str] = &["and", "or"];
const QUESTION_CUES: &[&str] = &["who", "what", "where", "why", "how", "when", "which"];
const REQUEST_STARTERS: &[&str] = &["please", "say", "tell", "ask", "answer"];
const FIRST_PERSON: &[&str] = &["i", "me", "my", "mine", "myself", "we", "us", "our", "ours", "ourselves"];
const SECOND_PERSON: &[&str] = &["you", "your", "yours", "yourself", "yourselves"];
const CONTEXT_REFERENCES: &[&str] = &["it", "this", "that"];

fn word_tokens(state: &CanonicalCognitiveState, sentence: &SurfaceSentence) -> Vec<SurfaceToken> {
    state
        .surface
        .tokens
        .iter()
        .filter(|token| {
            token.word
                && token.start >= sentence.start
                && token.end <= sentence.end
                && token.scope_id == sentence.scope_id
        })
        .cloned()
        .collect()
}

fn key(token: &SurfaceToken) -> String {
    canonical(&token.surface)
}

fn span(words: &[SurfaceToken], start: usize, end: usize) -> String {
    words
        .get(start..end)
        .unwrap_or(&[])
        .iter()
        .map(|token| token.surface.as_str())
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}

fn phrase(words: &[SurfaceToken], start: usize, end: usize, id: &str, scope_id: &str, role: &str) -> Option<EnglishPhraseObservation> {
    if start >= end || end > words.len() {
        return None;
    }
    let text = span(words, start, end);
    if text.is_empty() {
        return None;
    }
    let content = &words[start..end];
    let head = content
        .iter()
        .rev()
        .find(|token| {
            let token_key = key(token);
            !DETERMINERS.contains(&token_key.as_str())
                && !COPULAR_LINKS.contains(&token_key.as_str())
                && !AUXILIARY_LINKS.contains(&token_key.as_str())
                && !NEGATION_MARKERS.contains(&token_key.as_str())
                && !COORDINATORS.contains(&token_key.as_str())
        })
        .map(|token| token.surface.clone());
    let head_index = head.as_ref().and_then(|value| content.iter().rposition(|token| token.surface.as_str() == value.as_str()));
    let modifiers = head_index
        .map(|index| {
            content[..index]
                .iter()
                .filter(|token| !DETERMINERS.contains(&key(token).as_str()))
                .map(|token| token.surface.clone())
                .collect()
        })
        .unwrap_or_default();
    Some(EnglishPhraseObservation {
        id: id.to_string(),
        scope_id: scope_id.to_string(),
        text,
        start: content.first().map(|token| token.start).unwrap_or(0),
        end: content.last().map(|token| token.end).unwrap_or(0),
        structural_role: role.to_string(),
        head,
        modifiers,
    })
}

fn utterance_kind(words: &[SurfaceToken], terminal: Option<&str>) -> String {
    let keys = words.iter().map(key).collect::<Vec<_>>();
    let first = keys.first().map(String::as_str).unwrap_or("");
    let second = keys.get(1).map(String::as_str).unwrap_or("");
    let third = keys.get(2).map(String::as_str).unwrap_or("");
    // "Can you ..." is structurally ambiguous. Treat it as a clear request only
    // when the following word is part of the tiny request-glue set; otherwise
    // preserve it as a question and let learned cognition refine the intent.
    let polar_request = AUXILIARY_LINKS.contains(&first)
        && SECOND_PERSON.contains(&second)
        && REQUEST_STARTERS.contains(&third);
    let direct_request = REQUEST_STARTERS.contains(&first)
        && !COPULAR_LINKS.contains(&second);
    if polar_request || direct_request {
        "request".to_string()
    } else if terminal == Some("?") || QUESTION_CUES.contains(&first) || AUXILIARY_LINKS.contains(&first) {
        "question".to_string()
    } else {
        "statement".to_string()
    }
}

fn determiner_positions(keys: &[String], from: usize) -> Vec<usize> {
    keys.iter()
        .enumerate()
        .skip(from)
        .filter_map(|(index, token)| DETERMINERS.contains(&token.as_str()).then_some(index))
        .collect()
}

#[derive(Clone, Debug)]
struct ClauseSplit {
    left_start: usize,
    left_end: usize,
    link_start: usize,
    link_end: usize,
    right_start: usize,
    operator_text: Option<String>,
}

fn next_determiner(keys: &[String], from: usize) -> Option<usize> {
    determiner_positions(keys, from).into_iter().next()
}

fn split_non_inverted(words: &[SurfaceToken]) -> Option<ClauseSplit> {
    if words.len() < 3 {
        return None;
    }
    let keys = words.iter().map(key).collect::<Vec<_>>();

    if let Some(copula) = keys.iter().position(|token| COPULAR_LINKS.contains(&token.as_str())) {
        if copula == 0 || copula + 1 >= words.len() {
            return None;
        }
        let mut right_start = copula + 1;
        while right_start < keys.len() && NEGATION_MARKERS.contains(&keys[right_start].as_str()) {
            right_start += 1;
        }
        if right_start >= words.len() { return None; }
        return Some(ClauseSplit {
            left_start: 0,
            left_end: copula,
            link_start: copula,
            link_end: copula + 1,
            right_start,
            operator_text: None,
        });
    }

    if let Some(aux) = keys.iter().position(|token| AUXILIARY_LINKS.contains(&token.as_str())) {
        if aux == 0 || aux + 1 >= words.len() {
            return None;
        }
        let mut content_start = aux + 1;
        while content_start < keys.len() && NEGATION_MARKERS.contains(&keys[content_start].as_str()) {
            content_start += 1;
        }
        if content_start >= words.len() { return None; }
        let right_start = next_determiner(&keys, content_start + 1)
            .unwrap_or_else(|| (content_start + 1).min(words.len().saturating_sub(1)));
        if right_start <= content_start || right_start >= words.len() { return None; }
        return Some(ClauseSplit {
            left_start: 0,
            left_end: aux,
            link_start: content_start,
            link_end: right_start,
            right_start,
            operator_text: Some(words[aux].surface.clone()),
        });
    }

    if let Some(object_start) = next_determiner(&keys, 1) {
        if object_start >= 2 {
            let link_start = object_start - 1;
            if link_start > 0 {
                return Some(ClauseSplit {
                    left_start: 0,
                    left_end: link_start,
                    link_start,
                    link_end: object_start,
                    right_start: object_start,
                    operator_text: None,
                });
            }
        }
    }

    Some(ClauseSplit {
        left_start: 0,
        left_end: 1,
        link_start: 1,
        link_end: 2,
        right_start: 2,
        operator_text: None,
    })
}

fn split_inverted(words: &[SurfaceToken]) -> Option<ClauseSplit> {
    if words.len() < 4 {
        return None;
    }
    let keys = words.iter().map(key).collect::<Vec<_>>();
    if !AUXILIARY_LINKS.contains(&keys[0].as_str()) {
        return None;
    }

    let object_start = determiner_positions(&keys, 2)
        .into_iter()
        .find(|index| *index >= 4)
        .unwrap_or(words.len());
    let content_end = object_start.min(words.len());
    if content_end <= 3 { return None; }

    let left_start = 1usize;
    // Minimal school-entry substrate: a determiner-led left group receives one
    // head token by default; extra structure remains visible in the surface and
    // can be refined by learned cognition later.
    let left_end = if DETERMINERS.contains(&keys[left_start].as_str()) { 3 } else { 2 };
    if left_end >= content_end { return None; }
    let link_start = left_end;
    let link_end = content_end;
    if link_end <= link_start || object_start >= words.len() { return None; }

    Some(ClauseSplit {
        left_start,
        left_end,
        link_start,
        link_end,
        right_start: object_start,
        operator_text: Some(words[0].surface.clone()),
    })
}

fn parse_segment(
    words: &[SurfaceToken],
    sentence: &SurfaceSentence,
    clause_serial: usize,
    inherited_kind: &str,
    coordinator: Option<String>,
) -> Option<(EnglishClauseObservation, Vec<EnglishPhraseObservation>)> {
    if words.len() < 2 {
        return None;
    }
    let keys = words.iter().map(key).collect::<Vec<_>>();
    let inverted = AUXILIARY_LINKS.contains(&keys[0].as_str()) && inherited_kind != "statement";
    let split = if inverted { split_inverted(words)? } else { split_non_inverted(words)? };
    if split.right_start >= words.len() { return None; }

    let clause_id = format!("english_clause_{}_{}", sentence.id, clause_serial);
    let left = phrase(words, split.left_start, split.left_end, &format!("{}_left", clause_id), &sentence.scope_id, "left_group")?;
    let link = phrase(words, split.link_start, split.link_end, &format!("{}_link", clause_id), &sentence.scope_id, "link_group")?;
    let right = phrase(words, split.right_start, words.len(), &format!("{}_right", clause_id), &sentence.scope_id, "right_group")?;
    let negative = keys.iter().any(|token| NEGATION_MARKERS.contains(&token.as_str()));
    let question_cue = keys.iter().find(|token| QUESTION_CUES.contains(&token.as_str())).cloned();
    let clause = EnglishClauseObservation {
        id: clause_id,
        sentence_id: sentence.id.clone(),
        scope_id: sentence.scope_id.clone(),
        surface: sentence.text.clone(),
        utterance_kind: inherited_kind.to_string(),
        left_text: left.text.clone(),
        link_text: link.text.clone(),
        operator_text: split.operator_text,
        right_text: right.text.clone(),
        negative,
        coordinator,
        question_cue,
        inverted,
        confidence: if inverted { 0.72 } else { 0.78 },
    };
    Some((clause, vec![left, link, right]))
}

fn context_links(state: &CanonicalCognitiveState) -> Vec<EnglishContextLink> {
    let mut out = Vec::new();
    let previous = state.conversation_context.iter().rev().find(|turn| !turn.content.trim().is_empty());
    let Some(previous) = previous else { return out; };
    let root_words = state.surface.tokens.iter().filter(|token| token.word && token.scope_id == "scope_discourse").collect::<Vec<_>>();
    for (index, token) in root_words.iter().enumerate() {
        let token_key = key(token);
        if CONTEXT_REFERENCES.contains(&token_key.as_str()) {
            let next_key = root_words.get(index + 1).map(|next| key(next));
            if matches!(token_key.as_str(), "this" | "that")
                && next_key.as_deref().is_some_and(|next| {
                    !COPULAR_LINKS.contains(&next)
                        && !AUXILIARY_LINKS.contains(&next)
                        && !NEGATION_MARKERS.contains(&next)
                        && !matches!(next, "in" | "on" | "at" | "with" | "for" | "from" | "about" | "to" | "because" | "if" | "and" | "or")
                })
            {
                continue;
            }
            out.push(EnglishContextLink {
                surface: token.surface.clone(),
                start: token.start,
                end: token.end,
                referent_kind: "previous_utterance_candidate".to_string(),
                referent_id: Some(previous.id.clone()),
                referent_text: Some(previous.content.clone()),
                confidence: 0.56,
            });
        }
    }
    out
}

pub fn analyze(state: &CanonicalCognitiveState) -> EnglishBootstrapObservation {
    let mut phrases = Vec::new();
    let mut clauses = Vec::new();
    let mut question_cues = Vec::new();
    let mut request_cues = Vec::new();
    let mut negation_cues = Vec::new();
    let mut coordination_cues = Vec::new();
    let mut connection_cues = Vec::new();
    let mut glue_tokens = Vec::new();
    let mut utterance_kinds = Vec::new();

    for sentence in &state.surface.sentences {
        let words = word_tokens(state, sentence);
        if words.is_empty() {
            continue;
        }
        let kind = utterance_kind(&words, sentence.terminal.as_deref());
        utterance_kinds.push(kind.clone());
        let keys = words.iter().map(key).collect::<Vec<_>>();
        for (index, token_key) in keys.iter().enumerate() {
            let surface = words[index].surface.clone();
            if QUESTION_CUES.contains(&token_key.as_str()) && !question_cues.contains(&surface) { question_cues.push(surface.clone()); }
            if REQUEST_STARTERS.contains(&token_key.as_str()) && !request_cues.contains(&surface) { request_cues.push(surface.clone()); }
            if NEGATION_MARKERS.contains(&token_key.as_str()) && !negation_cues.contains(&surface) { negation_cues.push(surface.clone()); }
            if COORDINATORS.contains(&token_key.as_str()) && !coordination_cues.contains(&surface) { coordination_cues.push(surface.clone()); }
            if matches!(token_key.as_str(), "if" | "because") && !connection_cues.contains(&surface) { connection_cues.push(surface.clone()); }
            if DETERMINERS.contains(&token_key.as_str())
                || COPULAR_LINKS.contains(&token_key.as_str())
                || AUXILIARY_LINKS.contains(&token_key.as_str())
                || QUESTION_CUES.contains(&token_key.as_str())
                || REQUEST_STARTERS.contains(&token_key.as_str())
                || NEGATION_MARKERS.contains(&token_key.as_str())
                || COORDINATORS.contains(&token_key.as_str())
                || FIRST_PERSON.contains(&token_key.as_str())
                || SECOND_PERSON.contains(&token_key.as_str())
                || CONTEXT_REFERENCES.contains(&token_key.as_str())
                || matches!(token_key.as_str(), "if" | "because" | "yes" | "no" | "to" | "of" | "in" | "on" | "at" | "with" | "for" | "from" | "about")
            {
                if !glue_tokens.contains(&surface) { glue_tokens.push(surface); }
            }
        }

        let split_points = keys.iter().enumerate().filter_map(|(index, token)| COORDINATORS.contains(&token.as_str()).then_some((index, token.clone()))).collect::<Vec<_>>();
        if split_points.is_empty() {
            if let Some((clause, mut clause_phrases)) = parse_segment(&words, sentence, clauses.len() + 1, &kind, None) {
                clauses.push(clause);
                phrases.append(&mut clause_phrases);
            }
        } else {
            let mut start = 0usize;
            let mut inherited: Option<String> = None;
            for (index, coordinator) in split_points.into_iter().chain(std::iter::once((words.len(), String::new()))) {
                if index > start {
                    if let Some((clause, mut clause_phrases)) = parse_segment(&words[start..index], sentence, clauses.len() + 1, &kind, inherited.clone()) {
                        clauses.push(clause);
                        phrases.append(&mut clause_phrases);
                    }
                }
                inherited = (!coordinator.is_empty()).then_some(coordinator);
                start = index.saturating_add(1);
            }
        }
    }

    let utterance_kind = if utterance_kinds.iter().any(|kind| kind == "request") {
        "request"
    } else if utterance_kinds.iter().any(|kind| kind == "question") {
        "question"
    } else {
        "statement"
    };

    EnglishBootstrapObservation {
        version: BASIC_ENGLISH_BOOTSTRAP_VERSION.to_string(),
        utterance_kind: utterance_kind.to_string(),
        question_cues,
        request_cues,
        negation_cues,
        coordination_cues,
        connection_cues,
        glue_tokens,
        phrases,
        clauses,
        context_links: context_links(state),
        note: "Small Rust-owned English substrate: preserves structural language cues without teaching Nova school grammar concepts or mapping content words to hidden meanings.".to_string(),
        diagnostics: json!({
            "wholeUtterancePreserved": state.source_raw == state.surface.raw,
            "quotationCount": state.surface.quotes.len(),
            "sentenceCount": state.surface.sentences.len(),
            "tokenCount": state.surface.tokens.len()
        }),
    }
}

pub fn sentence_kind<'a>(state: &'a CanonicalCognitiveState, sentence_id: &str) -> Option<&'a str> {
    state.english_bootstrap.clauses.iter().find(|clause| clause.sentence_id == sentence_id).map(|clause| clause.utterance_kind.as_str())
}

pub fn clause_for_sentence<'a>(state: &'a CanonicalCognitiveState, sentence_id: &str) -> Option<&'a EnglishClauseObservation> {
    state.english_bootstrap.clauses.iter().find(|clause| clause.sentence_id == sentence_id)
}
