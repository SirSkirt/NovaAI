use std::collections::BTreeMap;

use serde_json::json;

use super::super::types::{CanonicalCognitiveState, CognitiveRequest, UnresolvedNeed};

pub(super) fn unique_request(
    state: &mut CanonicalCognitiveState,
    request: CognitiveRequest,
) -> CognitiveRequest {
    if let Some(existing) = state.requests.iter().find(|old| {
        old.request_type == request.request_type
            && old.target_frame_id == request.target_frame_id
            && old.answer_relation == request.answer_relation
            && old.question_surface == request.question_surface
    }) {
        return existing.clone();
    }
    state.requests.push(request.clone());
    request
}

pub(super) fn push_unresolved(
    state: &mut CanonicalCognitiveState,
    request: &CognitiveRequest,
    description: &str,
) {
    if state
        .unresolved
        .iter()
        .any(|need| need.request_id.as_ref() == Some(&request.id))
    {
        return;
    }
    state.unresolved.push(UnresolvedNeed {
        id: format!("goal_{}", request.id.0),
        need_type: "resolve_requested_relation".to_string(),
        description: Some(description.to_string()),
        goal_id: None,
        request_id: Some(request.id.clone()),
        required_capabilities: Vec::new(),
        attributes: json!({"source": "rust_request_construction"}),
        extensions: BTreeMap::new(),
    });
}
