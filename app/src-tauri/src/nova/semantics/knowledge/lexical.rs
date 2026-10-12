use std::collections::HashSet;

const STOP: &[&str] = &[
    "a", "an", "the", "this", "that", "these", "those", "what", "which", "who", "whom", "whose",
    "when", "where", "why", "how", "is", "are", "was", "were", "be", "been", "being", "do", "does",
    "did", "can", "could", "would", "should", "will", "may", "might", "please", "tell", "me", "about",
    "of", "to", "for", "from", "in", "on", "at", "and", "or", "but", "with", "as", "by",
];

pub fn canonical(input: &str) -> String {
    let lowered = input.to_lowercase();
    let mut out = String::new();
    let mut last_space = true;
    for ch in lowered.chars() {
        if ch.is_alphanumeric() || matches!(ch, '\'' | '’' | '-') {
            out.push(ch);
            last_space = false;
        } else if !last_space {
            out.push(' ');
            last_space = true;
        }
    }
    out.trim().to_string()
}

pub(super) fn normalize_word(word: &str) -> String {
    let mut out = word.to_string();
    if out.len() > 5 && out.ends_with("ies") {
        out.truncate(out.len() - 3);
        out.push('y');
    } else if out.len() > 6 && out.ends_with("able") {
        // Keep lightweight morphology structural rather than vocabulary-specific:
        // execute/executed/executable should share evidence without a word map.
        out.truncate(out.len() - 4);
    } else if out.len() > 6 && out.ends_with("ible") {
        out.truncate(out.len() - 4);
    } else if out.len() > 5 && out.ends_with("ing") {
        out.truncate(out.len() - 3);
    } else if out.len() > 4 && out.ends_with("ed") {
        out.truncate(out.len() - 2);
    } else if out.len() > 4
        && out.ends_with('s')
        && !out.ends_with("ss")
        && !out.ends_with("us")
        && !out.ends_with("is")
    {
        out.pop();
    }
    out
}

pub fn words(input: &str) -> Vec<String> {
    canonical(input)
        .split_whitespace()
        .map(normalize_word)
        .filter(|w| w.len() >= 2 && !STOP.contains(&w.as_str()))
        .collect()
}

pub(super) fn phrase_present(text: &str, phrase: &str) -> bool {
    let text_words: Vec<String> = canonical(text)
        .split_whitespace()
        .map(normalize_word)
        .collect();
    let phrase_words: Vec<String> = canonical(phrase)
        .split_whitespace()
        .map(normalize_word)
        .collect();
    if phrase_words.is_empty() || phrase_words.len() > text_words.len() {
        return false;
    }
    text_words
        .windows(phrase_words.len())
        .any(|window| window == phrase_words.as_slice())
}

pub(super) fn overlap_score(left: &HashSet<String>, right: &HashSet<String>) -> f32 {
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }
    let overlap = left.intersection(right).count() as f32;
    if overlap == 0.0 {
        return 0.0;
    }
    let coverage = overlap / left.len().min(right.len()) as f32;
    overlap * 5.0 + coverage * 8.0
}
