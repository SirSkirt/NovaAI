use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::nova::cognition::{CapabilityGraph, CognitiveModule, CognitiveModuleState, ExperienceRoute};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LearnedRuleRecord {
    pub id: String,
    pub directive: String,
    pub domain: String,
    pub primitive: String,
    pub scope: String,
    pub evidence_role: String,
    pub cognitive_form: Option<String>,
    pub learned_category: Option<String>,
    #[serde(default)]
    pub categories: Vec<String>,
    pub strength: String,
    pub polarity: String,
    pub status: String,
    pub module_id: Option<String>,
    pub source_type: String,
    pub source_name: String,
    pub source_display_name: Option<String>,
    pub source_kind: Option<String>,
    pub source_lesson_number: Option<u64>,
    pub source_id: Option<String>,
    #[serde(default)]
    pub evidence: Vec<Value>,
    #[serde(default)]
    pub parameters: Value,
    #[serde(default)]
    pub synthesis: Value,
    pub learned_at: Option<u64>,
    pub updated_at: Option<u64>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleSynthesisCandidate {
    pub rule_id: String,
    pub module: Option<CognitiveModule>,
    #[serde(default)]
    pub diagnostic: Value,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LearningSyncInput {
    /// Opaque Rust-owned learning state restored from persistence. Native JS
    /// transports it but does not reinterpret it into a second brain.
    #[serde(default)]
    pub persisted_state: Option<RustLearningState>,
    #[serde(default)]
    pub rules: Vec<LearnedRuleRecord>,
    #[serde(default)]
    pub existing_modules: Vec<CognitiveModule>,
    #[serde(default)]
    pub candidates: Vec<RuleSynthesisCandidate>,
    #[serde(default)]
    pub source_candidates: Vec<CognitiveModule>,
    #[serde(default)]
    pub experience_routes: Vec<ExperienceRoute>,
    pub source_revision: Option<u64>,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SynthesisRoundReport {
    pub round: u32,
    pub scanned: usize,
    pub upgraded: usize,
    pub module_count: usize,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleSynthesisResult {
    pub rule_id: String,
    pub status: String,
    pub module_id: Option<String>,
    #[serde(default)]
    pub missing_capabilities: Vec<String>,
    #[serde(default)]
    pub missing_operations: Vec<String>,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LearningSynthesisReport {
    pub scanned: usize,
    pub upgraded: usize,
    pub executable: usize,
    pub stored_only: usize,
    pub imported_modules: usize,
    pub rejected_imported_modules: usize,
    pub candidate_modules: usize,
    pub source_candidate_modules: usize,
    pub adopted_source_modules: usize,
    pub rejected_source_modules: usize,
    pub producer_capabilities: usize,
    pub waiting_capabilities: usize,
    #[serde(default)]
    pub rounds: Vec<SynthesisRoundReport>,
    #[serde(default)]
    pub results: Vec<RuleSynthesisResult>,
    pub reason: String,
    pub ran_at: u128,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustLearningState {
    pub schema_version: u32,
    pub initialized: bool,
    pub revision: u64,
    pub source_revision: Option<u64>,
    #[serde(default)]
    pub rules: Vec<LearnedRuleRecord>,
    #[serde(default)]
    pub modules: Vec<CognitiveModule>,
    #[serde(default)]
    pub candidates: Vec<RuleSynthesisCandidate>,
    #[serde(default)]
    pub module_states: Vec<(String, CognitiveModuleState)>,
    #[serde(default)]
    pub capability_graph: CapabilityGraph,
    #[serde(default)]
    pub experience_routes: Vec<ExperienceRoute>,
    pub last_synthesis: Option<LearningSynthesisReport>,
    pub updated_at: u128,
}

#[cfg(test)]
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LearningSyncReceipt {
    pub accepted: bool,
    pub authority: String,
    pub state: RustLearningState,
    pub report: LearningSynthesisReport,
}
