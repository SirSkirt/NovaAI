use serde_json::json;

use super::common::{push_unresolved, unique_request};
use super::super::knowledge::{canonical, KnowledgeSnapshot};
use super::super::relational::{
    clause_for_question, entity_by_label, frame_id_for_scope,
};
use super::super::types::{
    CandidateSet, CanonicalCognitiveState, CognitiveRequest, Conclusion, EventNode,
    SurfaceSentence, UnknownSlot,
};
use crate::nova::ids::{EntityId, RequestId};

fn event_pool(state: &CanonicalCognitiveState) -> Vec<&EventNode> {
    state
        .events
        .iter()
        .chain(state.discourse_context.events.iter())
        .collect()
}

pub(super) fn resolve_direct_relational_question(
    state: &mut CanonicalCognitiveState,
    sentence: &SurfaceSentence,
    _knowledge: &KnowledgeSnapshot,
) -> bool {
    let clause = match clause_for_question(state, sentence) {
        Some(clause) if !clause.link_text.trim().is_empty() => clause.clone(),
        _ => return false,
    };

    // Predicate identity comes from represented structural events already
    // present in current/discourse state. The bootstrap does not map question
    // words to human semantic roles; it only preserves left/link/right shape.
    let predicate_surface = clause.link_text.clone();
    let predicate_key = canonical(&predicate_surface);
    if !event_pool(state).iter().any(|event| event.predicate_key == predicate_key) {
        return false;
    }
    let leading = clause.left_text.clone();
    let trailing = clause.right_text.clone();
    if leading.is_empty() || trailing.is_empty() { return false; }

    let frame_id = frame_id_for_scope(state, &sentence.scope_id);
    // If the leading argument is already grounded, this is not an unknown-left
    // request (for example a polar question). Preserve the unresolved shape.
    if entity_by_label(state, &leading, Some(&frame_id))
        .or_else(|| entity_by_label(state, &leading, None))
        .is_some()
    {
        return false;
    }

    let target = entity_by_label(state, &trailing, Some(&frame_id))
        .or_else(|| entity_by_label(state, &trailing, None));
    let target = match target {
        Some(value) => value,
        None => return false,
    };

    // Structural position is a substrate primitive. Mapping left_argument to a
    // learned human role such as actor/agent belongs to learned cognition.
    let answer_relation = "left_argument".to_string();
    let slot_id = format!("rust_unknown_{}_{}", sentence.id, answer_relation);
    if !state.unknown_slots.iter().any(|slot| slot.id == slot_id) {
        state.unknown_slots.push(UnknownSlot {
            id: slot_id.clone(),
            frame_id: frame_id.clone(),
            role: answer_relation.clone(),
            value_type: leading,
            status: "unresolved".to_string(),
            source_surface: sentence.text.clone(),
            attributes: json!({"structuralRole": "left_argument"}),
        extensions: std::collections::BTreeMap::new(),
        });
    }

    let request = unique_request(
        state,
        CognitiveRequest {
            id: RequestId(format!("rust_request_{}_{}", sentence.id, answer_relation)),
            request_type: "information_request".to_string(),
            speech_act: "question_candidate".to_string(),
            frame_id: crate::nova::ids::FrameId("frame_discourse".to_string()),
            target_frame_id: Some(frame_id.clone()),
            target: Some(target.label.clone()),
            target_request_id: None,
            answer_relation: answer_relation.clone(),
            unknown_slot_id: Some(slot_id.clone()),
            question_surface: sentence.text.clone(),
            status: "unresolved".to_string(),
            answer_entity_id: None,
            source: "rust_structural_question_reasoning".to_string(),
            event_pattern: json!({
                "predicateSurface": predicate_surface,
                "predicateKey": predicate_key,
                "rightArgumentId": target.id.0.clone(),
                "rightArgumentLabel": target.label.clone()
            }),
            attribute_pattern: json!({}),
            constraints: Vec::new(),
            attributes: json!({"structuralRole": "left_argument"}),
        extensions: std::collections::BTreeMap::new(),
        },
    );

    let mut candidates: Vec<EntityId> = Vec::new();
    for event in event_pool(state) {
        if event.predicate_key != predicate_key {
            continue;
        }
        let right_id = event.right_argument_id.as_ref().or(event.object_id.as_ref());
        let right = right_id.and_then(|id| {
            state
                .entities
                .iter()
                .chain(state.discourse_context.entities.iter())
                .find(|entity| &entity.id == id)
        });
        if right.map(|entity| canonical(&entity.label)) != Some(canonical(&target.label)) {
            continue;
        }
        if let Some(left_id) = event.left_argument_id.as_ref().or(event.actor_id.as_ref()) {
            if !candidates.iter().any(|id| id == left_id) {
                candidates.push(left_id.clone());
            }
        }
    }

    if !candidates.is_empty() {
        state.candidate_sets.push(CandidateSet {
            id: format!("rust_candidates_{}", slot_id),
            frame_id: frame_id.clone(),
            for_unknown_slot_id: slot_id.clone(),
            entity_ids: candidates.clone(),
            source: "rust_matching_relational_events".to_string(),
            constraints: Vec::new(),
        });
    }

    if candidates.len() == 1 {
        let entity = state
            .entities
            .iter()
            .chain(state.discourse_context.entities.iter())
            .find(|entity| entity.id == candidates[0])
            .cloned();
        if let Some(entity) = entity {
            if let Some(stored) = state.requests.iter_mut().find(|item| item.id == request.id) {
                stored.status = "resolved".to_string();
                stored.answer_entity_id = Some(entity.id.clone());
            }
            if let Some(slot) = state.unknown_slots.iter_mut().find(|slot| slot.id == slot_id) {
                slot.status = "resolved".to_string();
            }
            state.conclusions.push(Conclusion {
                id: format!("rust_conclusion_{}", request.id.0),
                conclusion_type: "resolved_information_request".to_string(),
                request_id: Some(request.id.clone()),
                unknown_slot_id: Some(slot_id),
                value_id: Some(entity.id.0.clone()),
                value: Some(entity.label.clone()),
                surface: Some(format!("{}.", entity.label)),
                rust_authoritative: true,
                source: "rust_relational_request_resolution".to_string(),
                evidence: Vec::new(),
                support: json!({"candidateCount": 1, "requestedRelation": answer_relation}),
                attributes: json!({}),
        extensions: std::collections::BTreeMap::new(),
            });
            return true;
        }
    }

    push_unresolved(state, &request, &format!("resolve {}", answer_relation));
    true
}
