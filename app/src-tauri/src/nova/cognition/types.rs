use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::nova::ids::*;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", tag = "state")]
pub enum CognitiveModuleState {
    Executable,
    StoredOnly {
        #[serde(default)]
        missing_capabilities: Vec<CapabilityKey>,
        reason: Option<String>,
    },
}

impl Default for CognitiveModuleState {
    fn default() -> Self {
        Self::StoredOnly {
            missing_capabilities: Vec::new(),
            reason: Some("not_migrated_or_synthesized".to_string()),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CognitiveModuleDescriptor {
    pub id: ModuleId,
    pub module_number: Option<u64>,
    pub short_descriptor: String,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub requires: Vec<CapabilityKey>,
    #[serde(default)]
    pub produces: Vec<CapabilityKey>,
    #[serde(default)]
    pub state: CognitiveModuleState,
    #[serde(default)]
    pub provenance: Value,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CognitiveModule {
    pub id: ModuleId,
    pub module_number: Option<u64>,
    pub name: String,
    pub domain: String,
    pub phase: String,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub conditions: Vec<Value>,
    #[serde(default)]
    pub program: Vec<Value>,
    #[serde(default)]
    pub requires: Vec<CapabilityKey>,
    #[serde(default)]
    pub produces: Vec<CapabilityKey>,
    #[serde(default)]
    pub capabilities: Vec<CapabilityKey>,
    pub priority: f32,
    pub enabled: bool,
    pub source_type: String,
    pub source_name: String,
    pub source_id: Option<String>,
    pub cognitive_form: Option<String>,
    #[serde(default)]
    pub evidence: Vec<Value>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleContribution {
    pub module_id: ModuleId,
    pub module_number: Option<u64>,
    pub strength: f32,
    pub state_change_count: u64,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExperienceRoute {
    pub id: String,
    #[serde(default)]
    pub concept_ids: Vec<ConceptId>,
    #[serde(default)]
    pub concept_terms: Vec<String>,
    #[serde(default)]
    pub goal_terms: Vec<String>,
    #[serde(default)]
    pub relation_terms: Vec<String>,
    #[serde(default)]
    pub fallback_terms: Vec<String>,
    #[serde(default)]
    pub module_numbers: Vec<u64>,
    #[serde(default)]
    pub module_strengths: Value,
    #[serde(default)]
    pub contributors: Vec<ModuleContribution>,
    pub successes: u64,
    pub failures: u64,
    pub confidence: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityGraph {
    #[serde(default)]
    pub producers: Vec<CapabilityProducer>,
    #[serde(default)]
    pub waiters: Vec<CapabilityWaiter>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityProducer {
    pub capability: CapabilityKey,
    pub module_id: ModuleId,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityWaiter {
    pub capability: CapabilityKey,
    pub module_id: Option<ModuleId>,
    pub rule_id: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CognitiveRuntimeSnapshot {
    #[serde(default)]
    pub modules: Vec<CognitiveModule>,
    #[serde(default)]
    pub capability_graph: CapabilityGraph,
    #[serde(default)]
    pub experience_routes: Vec<ExperienceRoute>,
    pub source_revision: Option<u64>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CognitiveModuleRunRow {
    pub module_id: ModuleId,
    pub module_number: Option<u64>,
    pub name: String,
    pub phase: String,
    #[serde(default)]
    pub categories: Vec<String>,
    pub progress: bool,
    pub delta_score: u64,
    pub error: Option<String>,
    #[serde(default)]
    pub attention_weight: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CognitiveRuntimeReport {
    pub cycles: u32,
    pub modules_available: usize,
    pub modules_considered: usize,
    pub modules_applied: usize,
    pub modules_changed: usize,
    pub progress_count: u64,
    pub stalled: bool,
    pub outcome_exposed: bool,
    pub surface_response: Option<String>,
    pub search_mode: String,
    #[serde(default)]
    pub experience_route_ids: Vec<String>,
    pub broadened_after_experience_route: bool,
    #[serde(default)]
    pub awakenable_stored_only_rule_ids: Vec<String>,
    #[serde(default)]
    pub applied: Vec<CognitiveModuleRunRow>,
    #[serde(default)]
    pub changed: Vec<CognitiveModuleRunRow>,
    #[serde(default)]
    pub unsupported_operations: Vec<String>,
    #[serde(default)]
    pub missing_capabilities: Vec<String>,
}
