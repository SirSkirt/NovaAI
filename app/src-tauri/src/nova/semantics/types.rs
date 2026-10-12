use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::nova::ids::*;


#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationContextTurn {
    pub id: String,
    pub role: String,
    pub source_label: Option<String>,
    pub content: String,
    pub created_at: u128,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Participant {
    pub id: String,
    pub label: String,
    pub role: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParticipantSet {
    pub speaker: Participant,
    pub listener: Participant,
    pub group: Participant,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuoteRange {
    pub id: String,
    pub text: String,
    pub start: usize,
    pub end: usize,
    pub full_start: usize,
    pub full_end: usize,
    pub marker: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceSegment {
    pub kind: String,
    pub text: String,
    pub start: usize,
    pub end: usize,
    pub quote_id: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceSentence {
    pub id: String,
    pub text: String,
    pub start: usize,
    pub end: usize,
    pub terminal: Option<String>,
    pub scope_id: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceToken {
    pub surface: String,
    pub start: usize,
    pub end: usize,
    pub word: bool,
    pub capitalized: bool,
    pub scope_id: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnglishPhraseObservation {
    pub id: String,
    pub scope_id: String,
    pub text: String,
    pub start: usize,
    pub end: usize,
    pub structural_role: String,
    pub head: Option<String>,
    #[serde(default)]
    pub modifiers: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnglishClauseObservation {
    pub id: String,
    pub sentence_id: String,
    pub scope_id: String,
    pub surface: String,
    pub utterance_kind: String,
    pub left_text: String,
    pub link_text: String,
    pub operator_text: Option<String>,
    pub right_text: String,
    pub negative: bool,
    pub coordinator: Option<String>,
    pub question_cue: Option<String>,
    pub inverted: bool,
    pub confidence: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnglishContextLink {
    pub surface: String,
    pub start: usize,
    pub end: usize,
    pub referent_kind: String,
    pub referent_id: Option<String>,
    pub referent_text: Option<String>,
    pub confidence: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnglishBootstrapObservation {
    pub version: String,
    pub utterance_kind: String,
    #[serde(default)]
    pub question_cues: Vec<String>,
    #[serde(default)]
    pub request_cues: Vec<String>,
    #[serde(default)]
    pub negation_cues: Vec<String>,
    #[serde(default)]
    pub coordination_cues: Vec<String>,
    #[serde(default)]
    pub connection_cues: Vec<String>,
    #[serde(default)]
    pub glue_tokens: Vec<String>,
    #[serde(default)]
    pub phrases: Vec<EnglishPhraseObservation>,
    #[serde(default)]
    pub clauses: Vec<EnglishClauseObservation>,
    #[serde(default)]
    pub context_links: Vec<EnglishContextLink>,
    pub note: String,
    #[serde(default)]
    pub diagnostics: Value,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceObservation {
    pub version: String,
    pub raw: String,
    pub normalized_for_display: String,
    #[serde(default)]
    pub quotes: Vec<QuoteRange>,
    #[serde(default)]
    pub segments: Vec<SurfaceSegment>,
    #[serde(default)]
    pub sentences: Vec<SurfaceSentence>,
    #[serde(default)]
    pub tokens: Vec<SurfaceToken>,
    pub note: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticScope {
    pub id: String,
    pub kind: String,
    pub parent_id: Option<String>,
    pub start: usize,
    pub end: usize,
    pub boundary_kind: String,
    pub participant_frame: Option<ParticipantSet>,
    #[serde(default)]
    pub policies: Value,
    pub source: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceNode {
    pub id: String,
    pub surface: String,
    pub index: usize,
    pub end: usize,
    pub grammatical_person: String,
    pub scope_id: String,
    pub resolved_entity: Option<String>,
    pub participant_role: Option<String>,
    pub resolution_source: String,
    pub locked: bool,
    #[serde(default, flatten)]
    pub extensions: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticFrame {
    pub id: FrameId,
    pub identity_key: String,
    pub kind: String,
    pub parent_id: Option<FrameId>,
    pub scope_id: String,
    pub operator_surface: Option<String>,
    pub commitment: String,
    pub source_sentence_id: Option<String>,
    pub source: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityNode {
    pub id: EntityId,
    pub identity_key: String,
    pub frame_id: FrameId,
    pub label: String,
    pub name: Option<String>,
    pub kind: String,
    pub concept_id: Option<ConceptId>,
    pub source: String,
    #[serde(default)]
    pub attributes: Value,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventNode {
    pub id: EventId,
    pub identity_key: String,
    pub frame_id: FrameId,
    pub predicate_surface: String,
    pub predicate_key: String,
    pub left_argument_id: Option<EntityId>,
    pub right_argument_id: Option<EntityId>,
    pub actor_id: Option<EntityId>,
    pub object_id: Option<EntityId>,
    #[serde(default)]
    pub roles: Value,
    pub source: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationEdge {
    pub id: RelationId,
    pub frame_id: FrameId,
    #[serde(rename = "type")]
    pub relation_type: String,
    pub subject_id: Option<String>,
    pub object_id: Option<String>,
    pub event_id: Option<EventId>,
    pub subject: Option<String>,
    pub object: Option<String>,
    pub source: String,
    #[serde(default)]
    pub attributes: Value,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Proposition {
    pub id: PropositionId,
    pub frame_id: FrameId,
    #[serde(rename = "type")]
    pub proposition_type: String,
    pub subject: Option<String>,
    pub subject_id: Option<String>,
    pub predicate: Option<String>,
    pub object: Option<String>,
    pub object_id: Option<String>,
    pub text: Option<String>,
    pub polarity: String,
    pub confidence: f32,
    pub source: String,
    #[serde(default)]
    pub evidence: Vec<EvidenceId>,
    #[serde(default)]
    pub support: Value,
    #[serde(default)]
    pub attributes: Value,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CognitiveRequest {
    pub id: RequestId,
    #[serde(rename = "type")]
    pub request_type: String,
    pub speech_act: String,
    pub frame_id: FrameId,
    pub target_frame_id: Option<FrameId>,
    pub target: Option<String>,
    pub target_request_id: Option<RequestId>,
    pub answer_relation: String,
    pub unknown_slot_id: Option<String>,
    pub question_surface: String,
    pub status: String,
    pub answer_entity_id: Option<EntityId>,
    pub source: String,
    #[serde(default)]
    pub event_pattern: Value,
    #[serde(default)]
    pub attribute_pattern: Value,
    #[serde(default)]
    pub constraints: Vec<String>,
    #[serde(default)]
    pub attributes: Value,
    #[serde(default, flatten)]
    pub extensions: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnknownSlot {
    pub id: String,
    pub frame_id: FrameId,
    pub role: String,
    pub value_type: String,
    pub status: String,
    pub source_surface: String,
    #[serde(default)]
    pub attributes: Value,
    #[serde(default, flatten)]
    pub extensions: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CandidateSet {
    pub id: String,
    pub frame_id: FrameId,
    pub for_unknown_slot_id: String,
    #[serde(default)]
    pub entity_ids: Vec<EntityId>,
    pub source: String,
    #[serde(default)]
    pub constraints: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CognitiveConstraint {
    pub id: String,
    #[serde(rename = "type")]
    pub constraint_type: String,
    pub description: Option<String>,
    #[serde(default)]
    pub attributes: Value,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceItem {
    pub id: EvidenceId,
    pub source_type: String,
    pub source_id: Option<String>,
    pub content: Option<String>,
    #[serde(default)]
    pub attributes: Value,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Conclusion {
    pub id: String,
    #[serde(rename = "type")]
    pub conclusion_type: String,
    pub request_id: Option<RequestId>,
    pub unknown_slot_id: Option<String>,
    pub value_id: Option<String>,
    pub value: Option<String>,
    pub surface: Option<String>,
    pub rust_authoritative: bool,
    pub source: String,
    #[serde(default)]
    pub evidence: Vec<EvidenceId>,
    #[serde(default)]
    pub support: Value,
    #[serde(default)]
    pub attributes: Value,
    #[serde(default, flatten)]
    pub extensions: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnresolvedNeed {
    pub id: String,
    #[serde(rename = "type")]
    pub need_type: String,
    pub description: Option<String>,
    pub goal_id: Option<GoalId>,
    pub request_id: Option<RequestId>,
    #[serde(default)]
    pub required_capabilities: Vec<CapabilityKey>,
    #[serde(default)]
    pub attributes: Value,
    #[serde(default, flatten)]
    pub extensions: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConceptActivation {
    pub concept_id: ConceptId,
    pub key: String,
    pub label: String,
    pub strength: f32,
    pub source: String,
    pub matched_surface: Option<String>,
}


#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CognitiveTraceEvent {
    pub stage: String,
    pub text: String,
    #[serde(default)]
    pub source_ids: Vec<String>,
    #[serde(default)]
    pub data: Value,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticTraceStep {
    pub stage: String,
    pub detail: String,
    #[serde(default)]
    pub data: Value,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscourseSnapshot {
    #[serde(default)]
    pub entities: Vec<EntityNode>,
    #[serde(default)]
    pub events: Vec<EventNode>,
    #[serde(default)]
    pub relations: Vec<RelationEdge>,
    #[serde(default)]
    pub propositions: Vec<Proposition>,
}

/// Shared typed state for the Rust-owned semantic/request/concept subsystem.
/// Open strings remain open data: Rust owns structure, not Nova's final ontology.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalCognitiveState {
    pub version: String,
    pub schema_version: u32,
    pub id: String,
    pub source_raw: String,
    pub surface: SurfaceObservation,
    #[serde(default)]
    pub english_bootstrap: EnglishBootstrapObservation,
    pub participants: ParticipantSet,
    #[serde(default)]
    pub conversation_context: Vec<ConversationContextTurn>,
    #[serde(default)]
    pub scopes: Vec<SemanticScope>,
    pub active_scope_id: String,
    #[serde(default)]
    pub references: Vec<ReferenceNode>,
    #[serde(default)]
    pub frames: Vec<SemanticFrame>,
    #[serde(default)]
    pub entities: Vec<EntityNode>,
    #[serde(default)]
    pub events: Vec<EventNode>,
    #[serde(default)]
    pub relations: Vec<RelationEdge>,
    #[serde(default)]
    pub propositions: Vec<Proposition>,
    #[serde(default)]
    pub requests: Vec<CognitiveRequest>,
    #[serde(default)]
    pub unknown_slots: Vec<UnknownSlot>,
    #[serde(default)]
    pub candidate_sets: Vec<CandidateSet>,
    #[serde(default)]
    pub constraints: Vec<CognitiveConstraint>,
    #[serde(default)]
    pub evidence: Vec<EvidenceItem>,
    #[serde(default)]
    pub conclusions: Vec<Conclusion>,
    #[serde(default)]
    pub structures: Vec<Value>,
    #[serde(default)]
    pub categories: Vec<Value>,
    #[serde(default)]
    pub active_categories: Vec<String>,
    #[serde(default)]
    pub unresolved: Vec<UnresolvedNeed>,
    #[serde(default)]
    pub active_concepts: Vec<ConceptActivation>,
    #[serde(default)]
    pub discourse_context: DiscourseSnapshot,
    #[serde(default)]
    pub semantic_trace: Vec<SemanticTraceStep>,
    #[serde(default)]
    pub cognitive_trace: Vec<CognitiveTraceEvent>,
    #[serde(default)]
    pub learned_state: Value,
    #[serde(default)]
    pub module_state: Value,
    #[serde(default)]
    pub cognitive_outcome: Value,
    #[serde(default)]
    pub epistemic: Value,
    #[serde(default)]
    pub mental_state: Value,
    #[serde(default)]
    pub activities: Vec<Value>,
    pub status: String,
    #[serde(default)]
    pub rust_owned_subsystems: Vec<String>,
}
