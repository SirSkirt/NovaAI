use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::nova::ids::{source_has_named_identity, stable_hash_id, stable_source_id, ConceptId};
use crate::nova::mind::unix_ms;

use super::{canonical, KnowledgeClaim, KnowledgeConcept, KnowledgeExample, KnowledgeGuidance, KnowledgeProcedure, KnowledgeSnapshot};

pub const RUST_SEMANTIC_MEMORY_SCHEMA: u32 = 1;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticSourceRecord {
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub source_type: String,
    pub source_role: String,
    pub document_kind: String,
    pub lesson_number: Option<u64>,
    pub text: String,
    pub learned_at: u128,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticLearningCounts {
    pub concepts: usize,
    pub claims: usize,
    pub procedures: usize,
    pub guidance: usize,
    pub examples: usize,
}

impl SemanticLearningCounts {
    pub fn total(&self) -> usize {
        self.concepts + self.claims + self.procedures + self.guidance + self.examples
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticLearningReport {
    pub source_id: String,
    pub source_name: String,
    pub source_role: String,
    pub domain: Option<String>,
    pub counts: SemanticLearningCounts,
    pub created_at: u128,
    pub authority: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticMemoryImportInput {
    /// Opaque Rust-owned state restored from the existing durable payload.
    /// JavaScript persists/transports this value but does not reconstruct it.
    #[serde(default)]
    pub persisted_state: Option<RustSemanticMemoryState>,
    #[serde(default)]
    pub knowledge: KnowledgeSnapshot,
    #[serde(default)]
    pub sources: Vec<SemanticSourceRecord>,
    pub source_revision: Option<u64>,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticMemoryImportReceipt {
    pub accepted: bool,
    pub imported: bool,
    pub authority: String,
    pub state: RustSemanticMemoryState,
    pub reason: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustSemanticMemoryState {
    pub schema_version: u32,
    pub initialized: bool,
    pub revision: u64,
    pub knowledge: KnowledgeSnapshot,
    #[serde(default)]
    pub sources: Vec<SemanticSourceRecord>,
    #[serde(default)]
    pub learned_source_ids: Vec<String>,
    pub last_learning: Option<SemanticLearningReport>,
    pub updated_at: u128,
}

fn clean_text(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ").trim_matches(|c: char| matches!(c, '"' | '\'' | '“' | '”' | '‘' | '’')).trim().to_string()
}

fn concept_key(value: &str) -> String {
    canonical(&clean_text(value)).replace('_', " ").split_whitespace().collect::<Vec<_>>().join(" ")
}

fn merge_unique_text(target: &mut Vec<String>, values: impl IntoIterator<Item = String>) {
    for value in values {
        let value = clean_text(&value);
        if value.is_empty() { continue; }
        let key = canonical(&value);
        if !target.iter().any(|old| canonical(old) == key) { target.push(value); }
    }
}

fn remove_source_evidence(target: &mut Vec<Value>, source_id: &str) -> bool {
    let before = target.len();
    target.retain(|evidence| evidence.get("sourceId").and_then(Value::as_str) != Some(source_id));
    target.len() != before
}

fn merge_evidence(target: &mut Vec<Value>, evidence: Value) {
    if evidence.is_null() { return; }
    let sig = canonical(&evidence.to_string());
    if target.iter().any(|old| canonical(&old.to_string()) == sig) { return; }
    target.push(evidence);
    if target.len() > 32 {
        let excess = target.len() - 32;
        target.drain(0..excess);
    }
}

fn rewrite_source_ids(value: &mut Value, source_ids: &BTreeMap<String, String>) {
    match value {
        Value::Object(map) => {
            if let Some(source_id) = map.get_mut("sourceId") {
                if let Some(old) = source_id.as_str() {
                    if let Some(new) = source_ids.get(old) { *source_id = Value::String(new.clone()); }
                }
            }
            for child in map.values_mut() { rewrite_source_ids(child, source_ids); }
        }
        Value::Array(items) => {
            for child in items { rewrite_source_ids(child, source_ids); }
        }
        _ => {}
    }
}

fn normalize_legacy_source_identity(knowledge: &mut KnowledgeSnapshot, sources: Vec<SemanticSourceRecord>) -> Vec<SemanticSourceRecord> {
    let mut source_ids = BTreeMap::<String, String>::new();
    let mut normalized = Vec::<SemanticSourceRecord>::new();
    for mut source in sources {
        let old_id = source.id.clone();
        let new_id = if source_has_named_identity(&source.name) {
            stable_source_id(&source.name, &source.text)
        } else if !old_id.trim().is_empty() {
            old_id.clone()
        } else {
            stable_source_id(&source.name, &source.text)
        };
        if !old_id.trim().is_empty() { source_ids.insert(old_id, new_id.clone()); }
        source.id = new_id;
        if let Some(index) = normalized.iter().position(|row| row.id == source.id) {
            // Older JS state may contain multiple revisions of one named source.
            // Keep the newest source record and normalize all matching evidence
            // to one stable identity so a later re-teach can remove it cleanly.
            if source.learned_at >= normalized[index].learned_at { normalized[index] = source; }
        } else {
            normalized.push(source);
        }
    }

    for concept in &mut knowledge.concepts { for value in &mut concept.evidence { rewrite_source_ids(value, &source_ids); } }
    for claim in &mut knowledge.claims { for value in &mut claim.evidence { rewrite_source_ids(value, &source_ids); } }
    for guidance in &mut knowledge.guidance {
        for value in &mut guidance.evidence { rewrite_source_ids(value, &source_ids); }
        for value in &mut guidance.conditions { rewrite_source_ids(value, &source_ids); }
    }
    for example in &mut knowledge.examples {
        for value in &mut example.evidence { rewrite_source_ids(value, &source_ids); }
        for value in &mut example.sequence { rewrite_source_ids(value, &source_ids); }
    }
    for procedure in &mut knowledge.procedures {
        for value in &mut procedure.evidence { rewrite_source_ids(value, &source_ids); }
        for value in &mut procedure.sequence { rewrite_source_ids(value, &source_ids); }
    }
    normalized
}

impl RustSemanticMemoryState {
    pub fn new_initialized() -> Self {
        Self { schema_version: RUST_SEMANTIC_MEMORY_SCHEMA, initialized: true, updated_at: unix_ms(), ..Default::default() }
    }

    pub fn snapshot(&self) -> KnowledgeSnapshot {
        let mut knowledge = self.knowledge.clone();
        knowledge.source_revision = Some(self.revision);
        knowledge
    }

    pub fn import_once(&mut self, input: SemanticMemoryImportInput) -> SemanticMemoryImportReceipt {
        if self.initialized {
            return SemanticMemoryImportReceipt {
                accepted: true,
                imported: false,
                authority: "rust_v7_pass_6".to_string(),
                state: self.clone(),
                reason: "Rust semantic memory was already initialized; frontend semantic data was not allowed to replace it.".to_string(),
            };
        }

        if let Some(mut restored) = input.persisted_state {
            if restored.schema_version > RUST_SEMANTIC_MEMORY_SCHEMA {
                return SemanticMemoryImportReceipt {
                    accepted: false,
                    imported: false,
                    authority: "rust_v7_pass_6".to_string(),
                    state: self.clone(),
                    reason: format!("persisted semantic-memory schema {} is newer than supported schema {}", restored.schema_version, RUST_SEMANTIC_MEMORY_SCHEMA),
                };
            }
            restored.schema_version = RUST_SEMANTIC_MEMORY_SCHEMA;
            restored.initialized = true;
            restored.knowledge.source_revision = Some(restored.revision);
            if restored.learned_source_ids.is_empty() {
                restored.learned_source_ids = restored.sources.iter().map(|source| source.id.clone()).collect();
            }
            *self = restored;
            return SemanticMemoryImportReceipt {
                accepted: true,
                imported: true,
                authority: "rust_v7_pass_6".to_string(),
                state: self.clone(),
                reason: input.reason.unwrap_or_else(|| "restore_persisted_rust_semantic_memory".to_string()),
            };
        }

        self.schema_version = RUST_SEMANTIC_MEMORY_SCHEMA;
        self.initialized = true;
        self.revision = input.source_revision.or(input.knowledge.source_revision).unwrap_or(0);
        let mut knowledge = input.knowledge;
        let sources = normalize_legacy_source_identity(&mut knowledge, input.sources);
        self.knowledge = knowledge;
        self.knowledge.source_revision = Some(self.revision);
        self.sources = sources;
        self.learned_source_ids = self.sources.iter().map(|source| source.id.clone()).collect();
        self.updated_at = unix_ms();

        SemanticMemoryImportReceipt {
            accepted: true,
            imported: true,
            authority: "rust_v7_pass_6".to_string(),
            state: self.clone(),
            reason: input.reason.unwrap_or_else(|| "one_time_legacy_semantic_memory_import".to_string()),
        }
    }

    pub(crate) fn remove_source(&mut self, source_id: &str) {
        if source_id.trim().is_empty() { return; }

        self.knowledge.claims.retain_mut(|item| {
            let removed = remove_source_evidence(&mut item.evidence, source_id);
            !removed || !item.evidence.is_empty()
        });
        self.knowledge.guidance.retain_mut(|item| {
            let removed = remove_source_evidence(&mut item.evidence, source_id);
            !removed || !item.evidence.is_empty()
        });
        self.knowledge.examples.retain_mut(|item| {
            let removed = remove_source_evidence(&mut item.evidence, source_id);
            !removed || !item.evidence.is_empty()
        });
        self.knowledge.procedures.retain_mut(|item| {
            let removed = remove_source_evidence(&mut item.evidence, source_id);
            if removed && !item.evidence.is_empty() {
                item.sequence.retain(|step| step.get("sourceId").and_then(Value::as_str) != Some(source_id));
            }
            !removed || !item.evidence.is_empty()
        });
        self.knowledge.concepts.retain_mut(|item| {
            let removed = remove_source_evidence(&mut item.evidence, source_id);
            !removed || !item.evidence.is_empty()
        });
        self.sources.retain(|source| source.id != source_id);
        self.learned_source_ids.retain(|id| id != source_id);
        if matches!(self.last_learning.as_ref(), Some(report) if report.source_id == source_id) { self.last_learning = None; }
    }

    pub(crate) fn upsert_source(&mut self, source: SemanticSourceRecord) {
        if let Some(existing) = self.sources.iter_mut().find(|row| row.id == source.id) {
            *existing = source.clone();
        } else {
            self.sources.push(source.clone());
        }
        if !self.learned_source_ids.iter().any(|id| id == &source.id) { self.learned_source_ids.push(source.id); }
        if self.sources.len() > 160 {
            let excess = self.sources.len() - 160;
            self.sources.drain(0..excess);
        }
        if self.learned_source_ids.len() > 160 {
            let excess = self.learned_source_ids.len() - 160;
            self.learned_source_ids.drain(0..excess);
        }
    }

    pub(crate) fn upsert_concept(
        &mut self,
        name: &str,
        kinds: &[String],
        aliases: &[String],
        subject_eligible: bool,
        instructional: bool,
        domain_key: Option<&str>,
        evidence: Value,
    ) -> Option<KnowledgeConcept> {
        let name = clean_text(name);
        let key = concept_key(&name);
        if name.is_empty() || key.is_empty() { return None; }
        let now = unix_ms();
        let index = self.knowledge.concepts.iter().position(|concept| concept.key == key);
        if let Some(index) = index {
            let concept = &mut self.knowledge.concepts[index];
            merge_unique_text(&mut concept.aliases, aliases.iter().cloned().chain(std::iter::once(name.clone())));
            merge_unique_text(&mut concept.kinds, kinds.iter().cloned());
            if let Some(domain_key) = domain_key { merge_unique_text(&mut concept.domain_keys, [domain_key.to_string()]); }
            concept.subject_eligible |= subject_eligible;
            concept.instructional |= instructional;
            merge_evidence(&mut concept.evidence, evidence);
            concept.updated_at = Some(now);
            return Some(concept.clone());
        }

        let concept = KnowledgeConcept {
            id: ConceptId(stable_hash_id("concept", &canonical(&key))),
            key,
            name: name.clone(),
            aliases: aliases.iter().cloned().filter(|alias| canonical(alias) != canonical(&name)).collect(),
            kinds: kinds.to_vec(),
            domain_keys: domain_key.map(|value| vec![value.to_string()]).unwrap_or_default(),
            subject_eligible,
            instructional,
            evidence: if evidence.is_null() { Vec::new() } else { vec![evidence] },
            created_at: Some(now),
            updated_at: Some(now),
        };
        self.knowledge.concepts.push(concept.clone());
        Some(concept)
    }

    pub(crate) fn concept_by_name(&self, value: &str) -> Option<KnowledgeConcept> {
        let key = concept_key(value);
        self.knowledge.concepts.iter().find(|concept| {
            concept.key == key || canonical(&concept.name) == canonical(value) || concept.aliases.iter().any(|alias| canonical(alias) == canonical(value))
        }).cloned()
    }

    pub(crate) fn store_claim(
        &mut self,
        subject_name: &str,
        predicate: &str,
        object: &str,
        object_concept_name: Option<&str>,
        polarity: &str,
        confidence: f32,
        qualifiers: Value,
        evidence: Value,
    ) -> Option<KnowledgeClaim> {
        let subject = self.upsert_concept(subject_name, &["learned_subject".to_string()], &[], true, false, None, evidence.clone())?;
        let object = clean_text(object);
        let predicate = canonical(predicate).replace(' ', "_");
        if object.is_empty() || predicate.is_empty() { return None; }
        let object_concept_id = object_concept_name.and_then(|name| self.upsert_concept(name, &["related".to_string()], &[], true, false, None, evidence.clone())).map(|concept| concept.id);
        let signature = format!("{}|{}|{}|{}", subject.id.0, predicate, canonical(&object), canonical(polarity));
        let id = stable_hash_id("claim", &canonical(&signature));
        if let Some(existing) = self.knowledge.claims.iter_mut().find(|claim| claim.id == id) {
            existing.confidence = existing.confidence.max(confidence);
            merge_evidence(&mut existing.evidence, evidence);
            return Some(existing.clone());
        }
        let claim = KnowledgeClaim {
            id,
            subject_id: subject.id.clone(),
            subject_key: subject.key.clone(),
            subject: subject.name.clone(),
            predicate,
            object,
            object_concept_id,
            polarity: if polarity.trim().is_empty() { "positive".to_string() } else { polarity.to_string() },
            confidence: confidence.clamp(0.0, 1.0),
            qualifiers,
            evidence: if evidence.is_null() { Vec::new() } else { vec![evidence] },
        };
        self.knowledge.claims.push(claim.clone());
        Some(claim)
    }

    pub(crate) fn store_guidance(&mut self, mut item: KnowledgeGuidance, evidence: Value) -> KnowledgeGuidance {
        let signature = format!("{}|{}|{}|{}", canonical(&item.domain), canonical(&item.target), canonical(&item.directive), canonical(&item.polarity));
        if item.id.trim().is_empty() { item.id = stable_hash_id("guidance", &canonical(&signature)); }
        if let Some(existing) = self.knowledge.guidance.iter_mut().find(|row| row.id == item.id) {
            merge_evidence(&mut existing.evidence, evidence);
            return existing.clone();
        }
        merge_evidence(&mut item.evidence, evidence);
        self.knowledge.guidance.push(item.clone());
        item
    }

    pub(crate) fn store_example(&mut self, mut item: KnowledgeExample, evidence: Value) -> KnowledgeExample {
        let signature = format!("{}|{}", item.demonstrates_key.clone().unwrap_or_default(), canonical(&item.content));
        if item.id.trim().is_empty() { item.id = stable_hash_id("example", &canonical(&signature)); }
        if let Some(existing) = self.knowledge.examples.iter_mut().find(|row| row.id == item.id) {
            merge_evidence(&mut existing.evidence, evidence);
            return existing.clone();
        }
        merge_evidence(&mut item.evidence, evidence);
        self.knowledge.examples.push(item.clone());
        item
    }

    pub(crate) fn store_procedure(&mut self, mut item: KnowledgeProcedure, evidence: Value) -> KnowledgeProcedure {
        let signature = format!("{}|{}", canonical(&item.concept_key), canonical(&item.name));
        if item.id.trim().is_empty() { item.id = stable_hash_id("procedure", &canonical(&signature)); }
        if let Some(existing) = self.knowledge.procedures.iter_mut().find(|row| row.id == item.id) {
            if !item.sequence.is_empty() { existing.sequence = item.sequence.clone(); }
            merge_unique_text(&mut existing.requirements, item.requirements.clone());
            merge_unique_text(&mut existing.mechanisms, item.mechanisms.clone());
            merge_unique_text(&mut existing.optional_mechanisms, item.optional_mechanisms.clone());
            merge_evidence(&mut existing.evidence, evidence);
            return existing.clone();
        }
        merge_evidence(&mut item.evidence, evidence);
        self.knowledge.procedures.push(item.clone());
        item
    }

    pub(crate) fn begin_learning_update(&mut self) {
        if !self.initialized { *self = Self::new_initialized(); }
    }

    pub(crate) fn finish_learning_update(&mut self, report: SemanticLearningReport) {
        self.revision = self.revision.saturating_add(1);
        self.knowledge.source_revision = Some(self.revision);
        self.last_learning = Some(report);
        self.updated_at = unix_ms();
    }

    pub(crate) fn evidence(source_id: &str, source_name: &str, source_role: &str, section: &str, role: &str, text: &str) -> Value {
        json!({
            "sourceId": source_id,
            "sourceName": source_name,
            "sourceRole": source_role,
            "section": section,
            "role": role,
            "text": clean_text(text),
            "learnedAt": unix_ms()
        })
    }
}
