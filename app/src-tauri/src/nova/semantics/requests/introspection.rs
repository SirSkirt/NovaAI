use serde_json::json;

use super::common::unique_request;
use super::super::knowledge::KnowledgeSnapshot;
use super::super::types::{CanonicalCognitiveState, CognitiveRequest, Conclusion, SurfaceSentence};
use crate::nova::ids::{FrameId, RequestId};

pub(super) fn resolve_meta_relation_question(
    state: &mut CanonicalCognitiveState,
    sentence: &SurfaceSentence,
    knowledge: &KnowledgeSnapshot,
) -> bool {
    if sentence.scope_id != "scope_discourse"
        || !knowledge.relation_concept_is_present_in(&sentence.text)
    {
        return false;
    }

    let inner = state
        .requests
        .iter()
        .rev()
        .find(|request| {
            request
                .target_frame_id
                .as_ref()
                .map(|frame| frame.0.as_str() != "frame_discourse")
                .unwrap_or(false)
        })
        .cloned();
    let inner = match inner {
        Some(value) => value,
        None => return false,
    };

    let request = unique_request(
        state,
        CognitiveRequest {
            id: RequestId(format!("rust_meta_request_{}", sentence.id)),
            request_type: "semantic_introspection_request".to_string(),
            speech_act: "question_candidate".to_string(),
            frame_id: FrameId("frame_discourse".to_string()),
            target_frame_id: Some(FrameId("frame_discourse".to_string())),
            target: Some("represented request".to_string()),
            target_request_id: Some(inner.id.clone()),
            answer_relation: "requested_relation".to_string(),
            unknown_slot_id: None,
            question_surface: sentence.text.clone(),
            status: "resolved".to_string(),
            answer_entity_id: None,
            source: "rust_meta_semantic_request".to_string(),
            event_pattern: json!({}),
            attribute_pattern: json!({}),
            constraints: Vec::new(),
            attributes: json!({"inspects": "request.answerRelation"}),
        extensions: std::collections::BTreeMap::new(),
        },
    );

    let value = inner.answer_relation.clone();
    state.conclusions.push(Conclusion {
        id: format!("rust_conclusion_{}", request.id.0),
        conclusion_type: "reported_semantic_relation".to_string(),
        request_id: Some(request.id),
        unknown_slot_id: None,
        value_id: None,
        value: Some(value.clone()),
        surface: Some(format!("{}.", value.replace('_', " "))),
        rust_authoritative: true,
        source: "rust_meta_semantic_request".to_string(),
        evidence: Vec::new(),
        support: json!({
            "innerRequestId": inner.id.0.clone(),
            "innerQuestion": inner.question_surface.clone()
        }),
        attributes: json!({}),
        extensions: std::collections::BTreeMap::new(),
    });
    true
}
