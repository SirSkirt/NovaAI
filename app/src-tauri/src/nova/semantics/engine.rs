use serde::{Deserialize, Serialize};
use serde_json::json;

use super::bootstrap::analyze as analyze_basic_english;
use super::knowledge::KnowledgeSnapshot;
use super::relational::construct_assertions;
use super::executive::run_semantic_executive;
use super::surface::observe;
use super::types::{
    CanonicalCognitiveState, ConversationContextTurn, DiscourseSnapshot, Participant, ParticipantSet, ReferenceNode, SemanticFrame,
    SemanticScope, SemanticTraceStep,
};
use crate::nova::ids::FrameId;
use crate::nova::mind::TurnOrigin;

pub const RUST_SEMANTIC_SCHEMA: u32 = 5;
pub const RUST_SEMANTIC_VERSION: &str = "7.0.0-alpha.9.3.1-basic-english-bootstrap";

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticTurnReport {
    pub surface_observed: bool,
    #[serde(default)]
    pub basic_english_utterance_kind: String,
    #[serde(default)]
    pub basic_english_clauses: usize,
    pub concepts_activated: usize,
    pub assertions_constructed: usize,
    pub requests_constructed: usize,
    pub requests_resolved: usize,
    pub conclusions_produced: usize,
    pub semantic_cycles: usize,
    pub active_concepts_after_reasoning: usize,
    pub semantic_executive_stalled: bool,
    pub suggested_surface: Option<String>,
    #[serde(default)]
    pub trace: Vec<SemanticTraceStep>,
}

fn participants(origin: &TurnOrigin, assistant_label: &str) -> ParticipantSet {
    let (speaker_id, speaker_label, listener_id, listener_label) = match origin {
        TurnOrigin::Internal => ("assistant", assistant_label, "user", "user"),
        _ => ("user", "user", "assistant", assistant_label),
    };
    ParticipantSet {
        speaker: Participant {
            id: speaker_id.to_string(),
            label: speaker_label.to_string(),
            role: "speaker".to_string(),
        },
        listener: Participant {
            id: listener_id.to_string(),
            label: listener_label.to_string(),
            role: "addressee".to_string(),
        },
        group: Participant {
            id: "user+assistant".to_string(),
            label: format!("user and {assistant_label}"),
            role: "group".to_string(),
        },
    }
}

fn scopes(state: &mut CanonicalCognitiveState) {
    state.scopes.push(SemanticScope {
        id: "scope_discourse".to_string(),
        kind: "discourse_scope".to_string(),
        parent_id: None,
        start: 0,
        end: state.source_raw.len(),
        boundary_kind: "root".to_string(),
        participant_frame: Some(state.participants.clone()),
        policies: json!({}),
        source: "rust_surface_scope".to_string(),
    });
    state.frames.push(SemanticFrame {
        id: FrameId("frame_discourse".to_string()),
        identity_key: "frame_discourse".to_string(),
        kind: "discourse_frame".to_string(),
        parent_id: None,
        scope_id: "scope_discourse".to_string(),
        operator_surface: None,
        commitment: "current_discourse".to_string(),
        source_sentence_id: None,
        source: "rust_semantic_core".to_string(),
    });

    for quote in state.surface.quotes.clone() {
        let scope_id = format!("scope_{}", quote.id);
        let frame_id = FrameId(format!("frame_{}", quote.id));
        state.scopes.push(SemanticScope {
            id: scope_id.clone(),
            kind: "bounded_surface_scope".to_string(),
            parent_id: Some("scope_discourse".to_string()),
            start: quote.start,
            end: quote.end,
            boundary_kind: "quoted_text".to_string(),
            participant_frame: None,
            policies: json!({"outerParticipantLeakage": false}),
            source: "rust_surface_scope".to_string(),
        });
        state.frames.push(SemanticFrame {
            id: frame_id,
            identity_key: format!("quote|{}", quote.id),
            kind: "represented_frame".to_string(),
            parent_id: Some(FrameId("frame_discourse".to_string())),
            scope_id,
            operator_surface: None,
            commitment: "represented".to_string(),
            source_sentence_id: None,
            source: "rust_quote_scope".to_string(),
        });
    }
}

fn deictic_references(state: &mut CanonicalCognitiveState) {
    for token in state.surface.tokens.clone().into_iter().filter(|token| token.word) {
        let key = super::knowledge::canonical(&token.surface);
        let (person, role, participant_label) = match key.as_str() {
            "i" | "me" | "my" | "mine" | "myself" => (
                "first",
                "speaker",
                state.participants.speaker.label.clone(),
            ),
            "you" | "your" | "yours" | "yourself" | "yourselves" => (
                "second",
                "listener",
                state.participants.listener.label.clone(),
            ),
            "we" | "us" | "our" | "ours" | "ourselves" => (
                "first_plural",
                "group",
                state.participants.group.label.clone(),
            ),
            _ => continue,
        };
        let root = token.scope_id == "scope_discourse";
        state.references.push(ReferenceNode {
            id: format!("rust_ref_{}", token.start),
            surface: token.surface,
            index: token.start,
            end: token.end,
            grammatical_person: person.to_string(),
            scope_id: token.scope_id,
            resolved_entity: root.then_some(participant_label),
            participant_role: root.then(|| role.to_string()),
            resolution_source: if root {
                "rust_bootstrap_participant_frame"
            } else {
                "rust_scoped_deictic_unresolved"
            }
            .to_string(),
            locked: root,
            extensions: std::collections::BTreeMap::new(),
        });
    }
}

pub fn interpret_turn(
    raw: &str,
    origin: &TurnOrigin,
    assistant_label: &str,
    knowledge: &KnowledgeSnapshot,
    discourse: &DiscourseSnapshot,
    conversation_context: &[ConversationContextTurn],
) -> (CanonicalCognitiveState, SemanticTurnReport) {
    let observation = observe(raw);
    let participant_set = participants(origin, assistant_label);
    let mut state = CanonicalCognitiveState {
        version: RUST_SEMANTIC_VERSION.to_string(),
        schema_version: RUST_SEMANTIC_SCHEMA,
        id: format!("rust_mind_{}", crate::nova::mind::unix_ms()),
        source_raw: raw.to_string(),
        surface: observation,
        participants: participant_set,
        conversation_context: conversation_context.to_vec(),
        active_scope_id: "scope_discourse".to_string(),
        discourse_context: discourse.clone(),
        status: "UNRESOLVED".to_string(),
        rust_owned_subsystems: vec![
            "surface_observation".to_string(),
            "scope_construction".to_string(),
            "bootstrap_deictics".to_string(),
            "basic_english_bootstrap".to_string(),
            "relational_semantic_construction".to_string(),
            "request_construction".to_string(),
            "concept_activation".to_string(),
            "relationship_traversal".to_string(),
            "supported_proposition_composition".to_string(),
            "migrated_request_resolution".to_string(),
            "general_semantic_goal_induction".to_string(),
            "semantic_reasoning_executive".to_string(),
            "semantic_evidence_composition".to_string(),
        ],
        ..Default::default()
    };
    scopes(&mut state);
    deictic_references(&mut state);
    state.english_bootstrap = analyze_basic_english(&state);
    state.semantic_trace.push(SemanticTraceStep {
        stage: "basic_english".to_string(),
        detail: format!(
            "Preserved the utterance as {} with {} structural clause observation(s); no school grammar concepts were assigned.",
            state.english_bootstrap.utterance_kind,
            state.english_bootstrap.clauses.len()
        ),
        data: json!({
            "utteranceKind": state.english_bootstrap.utterance_kind.clone(),
            "clauses": state.english_bootstrap.clauses.clone(),
            "negationCues": state.english_bootstrap.negation_cues.clone(),
            "coordinationCues": state.english_bootstrap.coordination_cues.clone(),
            "connectionCues": state.english_bootstrap.connection_cues.clone(),
            "contextLinks": state.english_bootstrap.context_links.clone()
        }),
    });

    state.active_concepts = knowledge.activate_concepts(raw, 32);
    state.semantic_trace.push(SemanticTraceStep {
        stage: "concept_activation".to_string(),
        detail: format!("Activated {} learned concept(s) from the bridged semantic graph.", state.active_concepts.len()),
        data: json!({"concepts": state.active_concepts.iter().map(|c| &c.label).collect::<Vec<_>>() }),
    });

    let assertions = construct_assertions(&mut state, knowledge);
    state.semantic_trace.push(SemanticTraceStep {
        stage: "relational_construction".to_string(),
        detail: format!("Constructed {assertions} relational event assertion(s) from structural surface evidence."),
        data: json!({}),
    });

    let executive = run_semantic_executive(&mut state, knowledge);
    let requests = executive.requests_constructed;
    let resolved = executive.requests_resolved;
    let resolved_request_ids: std::collections::HashSet<_> = state
        .requests
        .iter()
        .filter(|request| request.status == "resolved")
        .map(|request| request.id.clone())
        .collect();
    state.unresolved.retain(|need| {
        need.request_id
            .as_ref()
            .map(|id| !resolved_request_ids.contains(id))
            .unwrap_or(true)
    });
    let suggested_surface = state
        .conclusions
        .iter()
        .rev()
        .find(|conclusion| conclusion.rust_authoritative)
        .and_then(|conclusion| conclusion.surface.clone());
    state.status = if suggested_surface.is_some() {
        "SUPPORTED".to_string()
    } else if !state.requests.is_empty() || !state.propositions.is_empty() || !state.events.is_empty() || !state.active_concepts.is_empty() {
        "PARTIAL".to_string()
    } else {
        "UNRESOLVED".to_string()
    };
    state.semantic_trace.push(SemanticTraceStep {
        stage: "request_reasoning".to_string(),
        detail: format!(
            "The Rust semantic executive ran {} cycle(s), constructed {requests} request(s), and resolved {resolved} request(s).",
            executive.cycles
        ),
        data: json!({
            "suggestedSurface": suggested_surface.clone(),
            "activeConceptsAfterReasoning": executive.active_concepts_after_reasoning,
            "stalled": executive.stalled
        }),
    });

    let report = SemanticTurnReport {
        surface_observed: true,
        basic_english_utterance_kind: state.english_bootstrap.utterance_kind.clone(),
        basic_english_clauses: state.english_bootstrap.clauses.len(),
        concepts_activated: state.active_concepts.len(),
        assertions_constructed: assertions,
        requests_constructed: requests,
        requests_resolved: resolved,
        conclusions_produced: state.conclusions.len(),
        semantic_cycles: executive.cycles,
        active_concepts_after_reasoning: executive.active_concepts_after_reasoning,
        semantic_executive_stalled: executive.stalled,
        suggested_surface,
        trace: state.semantic_trace.clone(),
    };
    (state, report)
}

pub fn discourse_from_completed_state(state: &CanonicalCognitiveState) -> DiscourseSnapshot {
    let root = FrameId("frame_discourse".to_string());
    DiscourseSnapshot {
        entities: state.entities.iter().filter(|entity| entity.frame_id == root).cloned().collect(),
        events: state.events.iter().filter(|event| event.frame_id == root).cloned().collect(),
        relations: state.relations.iter().filter(|relation| relation.frame_id == root).cloned().collect(),
        propositions: state.propositions.iter().filter(|proposition| proposition.frame_id == root && matches!(proposition.proposition_type.as_str(), "relational_event" | "structural_relation")).cloned().collect(),
    }
}
