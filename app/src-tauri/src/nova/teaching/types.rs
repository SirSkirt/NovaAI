use serde::{Deserialize, Serialize};

use crate::nova::learning::{LearnedRuleRecord, RuleSynthesisCandidate};
use crate::nova::semantics::knowledge::{RustSemanticMemoryState, SemanticLearningReport};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeachingSourceInput {
    pub source_id: Option<String>,
    pub source_name: String,
    pub text: String,
    pub source_type: Option<String>,
    pub kind_hint: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceIdentity {
    pub kind: String,
    pub lesson_number: Option<u64>,
    pub display_name: String,
    pub source_name: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeachingBlock {
    pub id: String,
    pub label: String,
    pub role: String,
    pub text: String,
    pub scope: String,
    pub line_start: usize,
    pub line_end: usize,
    pub inline: bool,
    pub section_kind: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedTeachingSource {
    pub schema_version: u32,
    pub source_name: String,
    pub source_display_name: String,
    pub document_kind: String,
    pub lesson_number: Option<u64>,
    pub structured: bool,
    pub instructional: bool,
    pub has_test_material: bool,
    #[serde(default)]
    pub blocks: Vec<TeachingBlock>,
}

#[derive(Clone, Debug, Default)]
pub struct CognitiveTeachingTranslation {
    pub rules: Vec<LearnedRuleRecord>,
    pub candidates: Vec<RuleSynthesisCandidate>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeachingIngestionReceipt {
    pub accepted: bool,
    pub authority: String,
    pub source_id: String,
    pub parsed: ParsedTeachingSource,
    pub semantic_report: SemanticLearningReport,
    pub semantic_state: RustSemanticMemoryState,
    pub learning_state: crate::nova::learning::RustLearningState,
    pub synthesis_report: crate::nova::learning::LearningSynthesisReport,
    pub realization_report: crate::nova::realization::RealizationReport,
}
