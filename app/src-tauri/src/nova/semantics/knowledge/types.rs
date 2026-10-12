use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::nova::ids::ConceptId;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeConcept {
    pub id: ConceptId,
    pub key: String,
    pub name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub kinds: Vec<String>,
    pub subject_eligible: bool,
    pub instructional: bool,
    #[serde(default)]
    pub domain_keys: Vec<String>,
    #[serde(default)]
    pub evidence: Vec<Value>,
    pub created_at: Option<u128>,
    pub updated_at: Option<u128>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeClaim {
    pub id: String,
    pub subject_id: ConceptId,
    pub subject_key: String,
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub object_concept_id: Option<ConceptId>,
    pub polarity: String,
    pub confidence: f32,
    #[serde(default)]
    pub qualifiers: Value,
    #[serde(default)]
    pub evidence: Vec<Value>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeGuidance {
    pub id: String,
    pub domain: String,
    pub scope: String,
    pub target: String,
    pub strength: String,
    pub polarity: String,
    pub directive: String,
    #[serde(default)]
    pub conditions: Vec<Value>,
    #[serde(default)]
    pub evidence: Vec<Value>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeExample {
    pub id: String,
    pub demonstrates_key: Option<String>,
    pub content: String,
    #[serde(default)]
    pub sequence: Vec<Value>,
    #[serde(default)]
    pub evidence: Vec<Value>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeProcedure {
    pub id: String,
    pub concept_id: Option<ConceptId>,
    pub concept_key: String,
    pub name: String,
    pub interaction_mode: String,
    #[serde(default)]
    pub roles: Vec<String>,
    #[serde(default)]
    pub sequence: Vec<Value>,
    #[serde(default)]
    pub requirements: Vec<String>,
    #[serde(default)]
    pub mechanisms: Vec<String>,
    #[serde(default)]
    pub optional_mechanisms: Vec<String>,
    pub purpose: Option<String>,
    pub confidence: f32,
    #[serde(default)]
    pub evidence: Vec<Value>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeSnapshot {
    #[serde(default)]
    pub concepts: Vec<KnowledgeConcept>,
    #[serde(default)]
    pub claims: Vec<KnowledgeClaim>,
    #[serde(default)]
    pub guidance: Vec<KnowledgeGuidance>,
    #[serde(default)]
    pub examples: Vec<KnowledgeExample>,
    #[serde(default)]
    pub procedures: Vec<KnowledgeProcedure>,
    pub source_revision: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConceptMatch {
    pub concept: KnowledgeConcept,
    pub score: f32,
    pub matched_alias: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeEdge {
    pub claim: KnowledgeClaim,
    pub depth: usize,
    pub direction: String,
    pub relevance: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeEvidenceItem {
    pub id: String,
    pub kind: String,
    pub concept_id: Option<ConceptId>,
    pub concept_key: Option<String>,
    pub subject: Option<String>,
    pub predicate: Option<String>,
    pub text: String,
    pub polarity: String,
    pub confidence: f32,
    pub relevance: f32,
    #[serde(default)]
    pub evidence: Vec<Value>,
    #[serde(default)]
    pub attributes: Value,
}
