use std::collections::HashSet;

use serde_json::json;

use super::common::{push_unresolved, unique_request};
use super::super::composition::compose_supported_concept_proposition;
use super::super::knowledge::{words, KnowledgeSnapshot};
use super::super::relational::frame_id_for_scope;
use super::super::types::{CanonicalCognitiveState, CognitiveRequest, Conclusion, SurfaceSentence};
use crate::nova::ids::{FrameId, RequestId};

pub(super) fn resolve_concept_question(
    state: &mut CanonicalCognitiveState,
    sentence: &SurfaceSentence,
    knowledge: &KnowledgeSnapshot,
) -> bool {
    let mut matches = knowledge.concepts_in_text(&sentence.text, 12);
    if matches.is_empty() {
        return false;
    }

    // Prefer a grounded concept with actual learned relationships.
    matches.sort_by(|a, b| {
        let a_edges = knowledge
            .claims
            .iter()
            .filter(|claim| claim.subject_id == a.concept.id)
            .count();
        let b_edges = knowledge
            .claims
            .iter()
            .filter(|claim| claim.subject_id == b.concept.id)
            .count();
        b_edges
            .cmp(&a_edges)
            .then_with(|| b.score.total_cmp(&a.score))
    });

    let target_match = matches[0].clone();
    let target = target_match.concept.clone();

    // Mentioning a concept in a question is not enough to infer an explain-
    // concept goal. Only the simple content shape is handled here; richer goal
    // induction remains learned cognition instead of becoming Rust phrase rules.
    let question_terms = words(&sentence.text);
    let target_terms: HashSet<String> = words(&target_match.matched_alias).into_iter().collect();
    if question_terms.iter().any(|term| !target_terms.contains(term)) {
        return false;
    }

    let frame_id = frame_id_for_scope(state, &sentence.scope_id);
    let request = unique_request(
        state,
        CognitiveRequest {
            id: RequestId(format!("rust_concept_request_{}", sentence.id)),
            request_type: "concept_relation_request".to_string(),
            speech_act: "question_candidate".to_string(),
            frame_id: FrameId("frame_discourse".to_string()),
            target_frame_id: Some(frame_id.clone()),
            target: Some(target.name.clone()),
            target_request_id: None,
            answer_relation: "supported_proposition".to_string(),
            unknown_slot_id: None,
            question_surface: sentence.text.clone(),
            status: "unresolved".to_string(),
            answer_entity_id: None,
            source: "rust_concept_goal_construction".to_string(),
            event_pattern: json!({}),
            attribute_pattern: json!({}),
            constraints: Vec::new(),
            attributes: json!({
                "conceptId": target.id.0.clone(),
                "conceptKey": target.key.clone()
            }),
                    extensions: std::collections::BTreeMap::new(),
        },
    );

    let composition =
        compose_supported_concept_proposition(knowledge, &target.id, &frame_id, &sentence.text);
    if let Some(proposition) = composition.proposition {
        let surface = composition.surface.clone();
        state.propositions.push(proposition.clone());
        if let Some(stored) = state.requests.iter_mut().find(|item| item.id == request.id) {
            stored.status = "resolved".to_string();
        }
        state.conclusions.push(Conclusion {
            id: format!("rust_conclusion_{}", request.id.0),
            conclusion_type: "supported_concept_proposition".to_string(),
            request_id: Some(request.id),
            unknown_slot_id: None,
            value_id: proposition.subject_id.clone(),
            value: proposition.text.clone(),
            surface,
            rust_authoritative: true,
            source: "rust_goal_directed_concept_composition".to_string(),
            evidence: proposition.evidence.clone(),
            support: json!({
                "usedClaimIds": composition.used_claim_ids,
                "explanation": composition.explanation
            }),
            attributes: json!({}),
        extensions: std::collections::BTreeMap::new(),
        });
        true
    } else {
        push_unresolved(state, &request, &composition.explanation);
        true
    }
}
