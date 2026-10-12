use serde_json::json;

use super::common::{push_unresolved, unique_request};
use super::super::composition::compose_supported_evidence_proposition;
use super::super::knowledge::{canonical, KnowledgeSnapshot};
use super::super::relational::frame_id_for_scope;
use super::super::types::{CanonicalCognitiveState, CognitiveRequest, Conclusion, SurfaceSentence};
use crate::nova::ids::{FrameId, RequestId};


fn nominal_relation_focus(
    question: &str,
    knowledge: &KnowledgeSnapshot,
) -> Option<crate::nova::ids::ConceptId> {
    // Generic English surface structure: in "X of Y", Y is a plausible
    // semantic target and X names the relation/property being asked about.
    // The substrate does not assign any meaning to X (purpose/color/owner/etc.).
    // It only uses the connector to focus learned evidence on Y.
    let normalized = canonical(question);
    let marker = " of ";
    let index = normalized.rfind(marker)?;
    let tail = normalized.get(index + marker.len()..)?.trim();
    if tail.is_empty() {
        return None;
    }
    knowledge
        .concepts_in_text(tail, 8)
        .into_iter()
        .next()
        .map(|hit| hit.concept.id)
}

/// Construct a general semantic information goal only after more specific
/// structural request constructors have declined the question. The goal does
/// not map question words to answer types. Instead it asks which learned,
/// supported semantic material best resolves the information gap represented
/// by the whole question.
pub(super) fn resolve_general_knowledge_goal(
    state: &mut CanonicalCognitiveState,
    sentence: &SurfaceSentence,
    knowledge: &KnowledgeSnapshot,
) -> bool {
    if sentence.scope_id != "scope_discourse" {
        return false;
    }

    let focus_concept = nominal_relation_focus(&sentence.text, knowledge);
    let ranked = knowledge.rank_evidence(
        &sentence.text,
        &state.active_concepts,
        focus_concept.as_ref(),
        24,
    );
    let top = match ranked.first() {
        Some(item) if item.relevance >= 12.0 => item.clone(),
        _ => return false,
    };

    // A question can refer to a learned relationship without literally naming
    // the concept that owns it. Let relevant evidence open that concept, rather
    // than requiring exact alias lookup before reasoning can begin.
    let inferred = knowledge.infer_concepts_from_evidence(&ranked, &state.active_concepts, 4);
    for activation in inferred {
        if !state
            .active_concepts
            .iter()
            .any(|old| old.concept_id == activation.concept_id)
        {
            state.active_concepts.push(activation);
        }
    }

    let frame_id = frame_id_for_scope(state, &sentence.scope_id);
    let request = unique_request(
        state,
        CognitiveRequest {
            id: RequestId(format!("rust_goal_request_{}", sentence.id)),
            request_type: "semantic_information_goal".to_string(),
            speech_act: "question_candidate".to_string(),
            frame_id: FrameId("frame_discourse".to_string()),
            target_frame_id: Some(frame_id.clone()),
            target: top
                .subject
                .clone()
                .or_else(|| top.concept_key.clone())
                .or_else(|| Some("learned semantic evidence".to_string())),
            target_request_id: None,
            answer_relation: "supported_goal_relevant_knowledge".to_string(),
            unknown_slot_id: None,
            question_surface: sentence.text.clone(),
            status: "unresolved".to_string(),
            answer_entity_id: None,
            source: "rust_general_semantic_goal_induction".to_string(),
            event_pattern: json!({}),
            attribute_pattern: json!({}),
            constraints: Vec::new(),
            attributes: json!({
                "topEvidenceId": top.id,
                "topEvidenceKind": top.kind,
                "topEvidenceRelevance": top.relevance,
                "candidateEvidenceCount": ranked.len(),
                "focusConceptId": focus_concept.as_ref().map(|id| id.0.clone()),
                "goalDerivedFrom": "whole_question_plus_learned_semantic_evidence"
            }),
                    extensions: std::collections::BTreeMap::new(),
        },
    );

    let composition = compose_supported_evidence_proposition(&ranked, &frame_id);
    if let Some(proposition) = composition.proposition {
        let surface = composition.surface.clone();
        let evidence = proposition.evidence.clone();
        let value = proposition.text.clone();
        state.propositions.push(proposition.clone());
        if let Some(stored) = state.requests.iter_mut().find(|item| item.id == request.id) {
            stored.status = "resolved".to_string();
        }
        state.conclusions.push(Conclusion {
            id: format!("rust_conclusion_{}", request.id.0),
            conclusion_type: "supported_semantic_goal_result".to_string(),
            request_id: Some(request.id),
            unknown_slot_id: None,
            value_id: proposition.subject_id.clone(),
            value,
            surface,
            rust_authoritative: true,
            source: "rust_general_semantic_goal_resolution".to_string(),
            evidence,
            support: json!({
                "usedEvidenceIds": composition.used_claim_ids,
                "explanation": composition.explanation,
                "candidateEvidenceCount": ranked.len()
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
