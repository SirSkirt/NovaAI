use serde::{Deserialize, Serialize};
use serde_json::json;

use super::knowledge::{canonical, KnowledgeEdge, KnowledgeEvidenceItem, KnowledgeSnapshot};
use super::types::Proposition;
use crate::nova::ids::{EvidenceId, FrameId, PropositionId};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompositionResult {
    pub proposition: Option<Proposition>,
    pub surface: Option<String>,
    #[serde(default)]
    pub used_claim_ids: Vec<String>,
    pub explanation: String,
}

fn sentence_case(text: &str) -> String {
    let trimmed = text.trim();
    let mut chars = trimmed.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

fn finish_sentence(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed
        .chars()
        .next_back()
        .map(|c| matches!(c, '.' | '!' | '?'))
        .unwrap_or(false)
    {
        trimmed.to_string()
    } else {
        format!("{trimmed}.")
    }
}

fn article_for(text: &str) -> &'static str {
    match text.trim().chars().next().map(|c| c.to_ascii_lowercase()) {
        Some('a' | 'e' | 'i' | 'o' | 'u') => "an",
        _ => "a",
    }
}

pub fn realize_relation(subject: &str, predicate: &str, object: &str) -> String {
    let subject_clean = subject.trim();
    let object_clean = object
        .trim()
        .trim_end_matches(|c: char| matches!(c, '.' | '!' | '?'));
    let object_key = canonical(object_clean);
    let subject_key = canonical(subject_clean);

    let complete_subject_forms = [
        subject_key.clone(),
        format!("a {subject_key}"),
        format!("an {subject_key}"),
        format!("the {subject_key}"),
    ];
    if !subject_key.is_empty()
        && complete_subject_forms.iter().any(|form| {
            object_key.starts_with(&format!("{form} is "))
                || object_key.starts_with(&format!("{form} are "))
                || object_key.starts_with(&format!("{form} means "))
        })
    {
        return finish_sentence(&sentence_case(object_clean));
    }

    let s = sentence_case(subject_clean);
    match predicate {
        "is_a" => finish_sentence(&format!("{s} is {} {object_clean}", article_for(object_clean))),
        "is" | "has_property" | "definition" | "define" | "describes" | "describe" => {
            finish_sentence(&format!("{s} is {object_clean}"))
        }
        "means" => finish_sentence(&format!("{s} means {object_clean}")),
        // `relationship` is the semantic store's intentionally opaque form for
        // a supported relationship sentence it could not safely decompose.
        // Preserve that learned proposition instead of fabricating a relation
        // name or duplicating its subject during realization.
        "relationship" => finish_sentence(&sentence_case(object_clean)),
        "different_from" => finish_sentence(&format!("{s} is different from {object_clean}")),
        other => {
            let relation = other.replace('_', " ");
            finish_sentence(&format!("{s} {relation} {object_clean}"))
        }
    }
}

fn evidence_surface(item: &KnowledgeEvidenceItem) -> String {
    match item.kind.as_str() {
        "claim" => {
            let subject = item.subject.clone().unwrap_or_default();
            let predicate = item.predicate.clone().unwrap_or_else(|| "is".to_string());
            let object = item
                .attributes
                .get("object")
                .and_then(|value| value.as_str())
                .unwrap_or(&item.text);
            realize_relation(&subject, &predicate, object)
        }
        "guidance" | "procedure" | "procedure_step" | "example" => finish_sentence(&sentence_case(&item.text)),
        _ => finish_sentence(&sentence_case(&item.text)),
    }
}

fn complementary(first: &KnowledgeEvidenceItem, second: &KnowledgeEvidenceItem) -> bool {
    if first.id == second.id {
        return false;
    }
    let same_concept = first
        .concept_id
        .as_ref()
        .zip(second.concept_id.as_ref())
        .map(|(a, b)| a == b)
        .unwrap_or(false)
        || first
            .concept_key
            .as_ref()
            .zip(second.concept_key.as_ref())
            .map(|(a, b)| canonical(a) == canonical(b))
            .unwrap_or(false);
    same_concept && first.kind != second.kind && second.relevance >= first.relevance * 0.62
}

pub fn compose_supported_evidence_proposition(
    ranked: &[KnowledgeEvidenceItem],
    frame_id: &FrameId,
) -> CompositionResult {
    let first = match ranked.first() {
        Some(item) => item,
        None => {
            return CompositionResult {
                explanation: "No supported learned evidence was relevant enough to the current semantic goal.".to_string(),
                ..Default::default()
            }
        }
    };

    let mut selected = vec![first.clone()];
    if let Some(second) = ranked.iter().skip(1).find(|item| complementary(first, item)) {
        selected.push(second.clone());
    }

    let surfaces: Vec<String> = selected
        .iter()
        .map(evidence_surface)
        .filter(|surface| !surface.trim().is_empty())
        .collect();
    if surfaces.is_empty() {
        return CompositionResult {
            explanation: "Relevant learned evidence was found, but it did not contain realizable semantic content.".to_string(),
            ..Default::default()
        };
    }
    let surface = surfaces.join(" ");
    let confidence = selected
        .iter()
        .map(|item| item.confidence)
        .fold(0.0_f32, f32::max)
        .clamp(0.1, 1.0);
    let evidence_ids: Vec<EvidenceId> = selected
        .iter()
        .map(|item| EvidenceId(item.id.clone()))
        .collect();
    let support_ids: Vec<String> = selected.iter().map(|item| item.id.clone()).collect();
    let kinds: Vec<String> = selected.iter().map(|item| item.kind.clone()).collect();

    let proposition = Proposition {
        id: PropositionId(format!("rust_goal_prop_{}", first.id)),
        frame_id: frame_id.clone(),
        proposition_type: "supported_goal_relevant_knowledge".to_string(),
        subject: first.subject.clone(),
        subject_id: first.concept_id.as_ref().map(|id| id.0.clone()),
        predicate: first.predicate.clone(),
        object: first
            .attributes
            .get("object")
            .and_then(|value| value.as_str())
            .map(ToString::to_string),
        object_id: first
            .attributes
            .get("objectConceptId")
            .and_then(|value| value.as_str())
            .map(ToString::to_string),
        text: Some(surface.clone()),
        polarity: first.polarity.clone(),
        confidence,
        source: "rust_goal_evidence_composition".to_string(),
        evidence: evidence_ids,
        support: json!({
            "selectedEvidenceIds": support_ids,
            "selectedKinds": kinds,
            "topRelevance": first.relevance,
            "memoryAuthority": "rust_v7_pass_6"
        }),
        attributes: json!({"composedEvidenceCount": selected.len()}),
    };

    CompositionResult {
        proposition: Some(proposition),
        surface: Some(surface),
        used_claim_ids: selected.iter().map(|item| item.id.clone()).collect(),
        explanation: format!(
            "Selected {} supported learned evidence item(s) that best matched the unresolved semantic goal.",
            selected.len()
        ),
    }
}

pub fn compose_supported_concept_proposition(
    knowledge: &KnowledgeSnapshot,
    concept_id: &crate::nova::ids::ConceptId,
    frame_id: &FrameId,
    goal_text: &str,
) -> CompositionResult {
    let concept = match knowledge.concept_by_id(concept_id) {
        Some(value) => value,
        None => {
            return CompositionResult {
                explanation: "The activated concept is not present in the Rust semantic memory.".to_string(),
                ..Default::default()
            }
        }
    };

    let edges = knowledge.collect_edges(concept_id, 3, 96);
    let selected = knowledge.select_relevant_edges(edges, goal_text, 16);
    let edge: &KnowledgeEdge = match selected.first() {
        Some(value) => value,
        None => {
            return CompositionResult {
                explanation: format!(
                    "The concept '{}' opened successfully but has no supported relationship to compose yet.",
                    concept.name
                ),
                ..Default::default()
            }
        }
    };

    let claim = &edge.claim;
    let surface = realize_relation(&claim.subject, &claim.predicate, &claim.object);
    let proposition = Proposition {
        id: PropositionId(format!("rust_knowledge_prop_{}", claim.id)),
        frame_id: frame_id.clone(),
        proposition_type: "supported_knowledge_relation".to_string(),
        subject: Some(claim.subject.clone()),
        subject_id: Some(claim.subject_id.0.clone()),
        predicate: Some(claim.predicate.clone()),
        object: Some(claim.object.clone()),
        object_id: claim.object_concept_id.as_ref().map(|id| id.0.clone()),
        text: Some(surface.clone()),
        polarity: claim.polarity.clone(),
        confidence: claim.confidence,
        source: "rust_concept_graph_composition".to_string(),
        evidence: Vec::new(),
        support: json!({
            "claimId": claim.id.clone(),
            "depth": edge.depth,
            "relevance": edge.relevance,
            "memoryAuthority": "rust_v7_pass_6"
        }),
        attributes: json!({}),
    };

    CompositionResult {
        proposition: Some(proposition),
        surface: Some(surface),
        used_claim_ids: vec![claim.id.clone()],
        explanation: format!(
            "Opened concept '{}' and selected a supported learned relationship from {} candidate edge(s).",
            concept.name,
            selected.len()
        ),
    }
}
