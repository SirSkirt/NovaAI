use std::collections::BTreeMap;
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use super::cognition::{run_cognitive_runtime, CapabilityGraph, CognitiveModuleDescriptor, CognitiveRuntimeReport, CognitiveRuntimeSnapshot, ExperienceRoute};
use super::ids::TurnId;
use super::internal::RustInternalCognitionState;
#[cfg(test)]
use super::internal::{import_internal_cognition, InternalCognitionImportInput, InternalCognitionImportReceipt};
use super::development::{reflect_completed_turn, RustDevelopmentState};
#[cfg(test)]
use super::development::{import_development, DevelopmentImportInput, DevelopmentImportReceipt};
use super::learning::{replace_rust_teaching_source, synchronize_learning, LearningSyncInput, RustLearningState};
#[cfg(test)]
use super::learning::LearningSyncReceipt;
use super::trace::refresh_cognitive_trace;
use super::realization::{realize_current, realize_learning, RealizationReport};
use super::semantics::{
    discourse_from_completed_state, interpret_turn, CanonicalCognitiveState, ConversationContextTurn, DiscourseSnapshot,
    KnowledgeSnapshot, SemanticTurnReport,
};
use super::semantics::knowledge::{KnowledgeExample, RustSemanticMemoryState, SemanticMemoryImportInput};
#[cfg(test)]
use super::semantics::knowledge::SemanticMemoryImportReceipt;
use super::teaching::{apply_semantic_learning, parse_source, stable_source_id, translate_cognition, TeachingIngestionReceipt, TeachingSourceInput};
use super::teacher::{FinalizeTeacherRunInput, RecordTeacherAttemptInput, RustTeacherState, TeacherAttemptRecord, TeacherFeedbackReceipt, TeacherRunFinalizeReceipt, UpdateTeacherFeedbackInput};

pub const NOVA_RUST_MIND_SCHEMA: u32 = 12;
pub const NOVA_RUST_ARCHITECTURE_VERSION: &str = "7.0.0-rust-authoritative-mind-alpha-9.3";

pub(crate) fn unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MigrationPhase {
    FoundationOnly,
    PartialRustMind,
    RustAuthoritative,
}

impl Default for MigrationPhase {
    fn default() -> Self {
        Self::PartialRustMind
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnOrigin {
    User,
    Teacher,
    ChatGpt,
    Internal,
    Other,
}

impl Default for TurnOrigin {
    fn default() -> Self {
        Self::Other
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BeginTurnInput {
    pub text: String,
    pub source_label: Option<String>,
    pub origin: Option<TurnOrigin>,
    pub assistant_label: Option<String>,
    pub knowledge: Option<KnowledgeSnapshot>,
    pub cognition: Option<CognitiveRuntimeSnapshot>,
    #[serde(default)]
    pub conversation_context: Vec<ConversationContextTurn>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnEnvelope {
    pub id: TurnId,
    pub origin: TurnOrigin,
    pub source_label: Option<String>,
    pub event_kind: String,
    pub raw_text: String,
    pub started_at: u128,
    pub finished_at: Option<u128>,
    pub semantic_runtime: String,
    pub remaining_reasoning_runtime: String,
    pub terminal_state: Option<String>,
    pub reason_code: Option<String>,
    pub response_produced: Option<bool>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FinishTurnInput {
    pub turn_id: TurnId,
    pub response_produced: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnReceipt {
    pub turn_id: TurnId,
    pub accepted: bool,
    pub migration_phase: String,
    pub reasoning_owner: String,
    pub event_kind: String,
    pub semantic_report: SemanticTurnReport,
    pub cognitive_report: CognitiveRuntimeReport,
    pub realization_report: RealizationReport,
    pub canonical_state: CanonicalCognitiveState,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubsystemOwnership {
    #[serde(default)]
    pub owners: BTreeMap<String, String>,
}

impl SubsystemOwnership {
    fn pass_nine() -> Self {
        let mut owners = BTreeMap::new();
        for name in [
            "turn_lifecycle",
            "surface_observation",
            "scope_construction",
            "bootstrap_deictics",
            "relational_semantic_construction",
            "request_construction",
            "concept_activation",
            "relationship_traversal",
            "supported_proposition_composition",
            "migrated_request_resolution",
            "general_semantic_goal_induction",
            "semantic_reasoning_executive",
            "semantic_evidence_composition",
            "learned_cognitive_module_runtime",
            "capability_requirement_gating",
            "learned_experience_route_scheduler",
            "structured_cognitive_trace",
            "learned_cognitive_state_authority",
            "cognitive_synthesis_authority",
            "capability_dependency_graph",
            "stored_only_executability",
            "fixed_point_stored_only_reconsideration",
            "semantic_memory_authority",
            "lesson_document_parser",
            "teaching_to_semantic_translation",
            "teaching_to_candidate_translation",
            "source_provenance",
            "experience_index",
            "experience_reflection",
            "developmental_category_graph",
            "developmental_consolidation",
            "provisional_experience_abstraction",
            "free_thought",
            "internal_goal_generation",
            "internal_goal_execution",
            "general_response_realization",
            "conversation_teaching_intent_detection",
            "teacher_test_history",
            "teacher_test_evidence_release",
            "teacher_feedback_knowledge",
            "teacher_feedback_provenance",
        ] {
            owners.insert(name.to_string(), "rust_v7_pass_9".to_string());
        }
        for name in [
            "persistent_mind_payload_orchestration",
            "persistent_mind_serialization",
            "persistent_mind_hydration",
            "persistent_mind_recovery",
            "conversation_history_authority",
        ] {
            owners.insert(name.to_string(), "rust_v7_pass_9".to_string());
        }
        for name in [
            "basic_english_bootstrap",
            "teacher_partial_credit",
            "teacher_span_annotations",
            "teacher_span_notes",
            "teacher_reflection_evidence",
        ] {
            owners.insert(name.to_string(), "rust_v7_alpha_9_3".to_string());
        }
        Self { owners }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NovaMind {
    pub schema_version: u32,
    pub architecture_version: String,
    pub migration_phase: MigrationPhase,
    pub reasoning_owner: String,
    pub turn_lifecycle_owner: String,
    pub revision: u64,
    pub current_turn: Option<TurnEnvelope>,
    pub last_completed_turn: Option<TurnEnvelope>,
    pub cognitive_state: CanonicalCognitiveState,
    pub last_semantic_report: Option<SemanticTurnReport>,
    pub last_cognitive_report: Option<CognitiveRuntimeReport>,
    pub last_realization_report: Option<RealizationReport>,
    pub discourse_memory: DiscourseSnapshot,
    #[serde(default)]
    pub conversation_history: Vec<ConversationContextTurn>,
    pub subsystem_ownership: SubsystemOwnership,
    #[serde(default)]
    pub semantic_memory: RustSemanticMemoryState,
    #[serde(default)]
    pub module_catalog: Vec<CognitiveModuleDescriptor>,
    #[serde(default)]
    pub capability_graph: CapabilityGraph,
    #[serde(default)]
    pub experience_index: Vec<ExperienceRoute>,
    #[serde(default)]
    pub learning_state: RustLearningState,
    #[serde(default)]
    pub development_state: RustDevelopmentState,
    #[serde(default)]
    pub internal_cognition: RustInternalCognitionState,
    #[serde(default)]
    pub teacher_state: RustTeacherState,
    pub created_at: u128,
    pub updated_at: u128,
}

impl Default for NovaMind {
    fn default() -> Self {
        let now = unix_ms();
        Self {
            schema_version: NOVA_RUST_MIND_SCHEMA,
            architecture_version: NOVA_RUST_ARCHITECTURE_VERSION.to_string(),
            migration_phase: MigrationPhase::RustAuthoritative,
            reasoning_owner: "rust_v7_authoritative_mind".to_string(),
            turn_lifecycle_owner: "rust_v7_pass_9".to_string(),
            revision: 0,
            current_turn: None,
            last_completed_turn: None,
            cognitive_state: CanonicalCognitiveState::default(),
            last_semantic_report: None,
            last_cognitive_report: None,
            last_realization_report: None,
            discourse_memory: DiscourseSnapshot::default(),
            conversation_history: Vec::new(),
            subsystem_ownership: SubsystemOwnership::pass_nine(),
            semantic_memory: RustSemanticMemoryState::default(),
            module_catalog: Vec::new(),
            capability_graph: CapabilityGraph::default(),
            experience_index: Vec::new(),
            learning_state: RustLearningState::default(),
            development_state: RustDevelopmentState::default(),
            internal_cognition: RustInternalCognitionState::default(),
            teacher_state: RustTeacherState::default(),
            created_at: now,
            updated_at: now,
        }
    }
}

impl NovaMind {
    pub(crate) fn normalize_pass_nine(&mut self) {
        self.schema_version = NOVA_RUST_MIND_SCHEMA;
        self.architecture_version = NOVA_RUST_ARCHITECTURE_VERSION.to_string();
        self.migration_phase = MigrationPhase::RustAuthoritative;
        self.reasoning_owner = "rust_v7_authoritative_mind".to_string();
        self.turn_lifecycle_owner = "rust_v7_pass_9".to_string();
        self.subsystem_ownership = SubsystemOwnership::pass_nine();
        self.teacher_state.normalize();
        self.current_turn = None;
        if self.semantic_memory.initialized || self.learning_state.initialized || self.development_state.initialized || self.internal_cognition.initialized {
            self.refresh_learning_views();
        }
        if self.conversation_history.len() > 160 {
            let keep_from = self.conversation_history.len() - 160;
            self.conversation_history.drain(0..keep_from);
        }
        self.updated_at = unix_ms();
    }

    fn conversation_context_for_turn(&self, legacy_context: &[ConversationContextTurn]) -> Vec<ConversationContextTurn> {
        if self.conversation_history.is_empty() && !legacy_context.is_empty() {
            // Test/one-time migration compatibility only. Native Pass 9 callers
            // no longer select or send conversation history from JavaScript.
            return legacy_context.iter().rev().take(12).cloned().collect::<Vec<_>>().into_iter().rev().collect();
        }
        self.conversation_history.iter().rev().take(12).cloned().collect::<Vec<_>>().into_iter().rev().collect()
    }

    fn remember_conversation_turn(&mut self, turn: ConversationContextTurn) {
        if turn.content.trim().is_empty() { return; }
        self.conversation_history.push(turn);
        if self.conversation_history.len() > 160 { self.conversation_history.remove(0); }
    }

    pub(crate) fn refresh_learning_views(&mut self) {
        self.module_catalog = self.learning_state.modules.iter().map(|module| CognitiveModuleDescriptor {
            id: module.id.clone(),
            module_number: module.module_number,
            short_descriptor: if module.categories.is_empty() { module.name.clone() } else { module.categories.iter().take(3).cloned().collect::<Vec<_>>().join(", ") },
            categories: module.categories.clone(),
            requires: module.requires.clone(),
            produces: module.produces.clone(),
            state: super::cognition::CognitiveModuleState::Executable,
            provenance: serde_json::json!({"sourceType":module.source_type,"sourceName":module.source_name,"sourceId":module.source_id}),
        }).collect();
        self.capability_graph = self.learning_state.capability_graph.clone();
        self.experience_index = self.learning_state.experience_routes.clone();
    }

    pub fn begin_turn(&mut self, input: BeginTurnInput) -> TurnReceipt {
        let now = unix_ms();
        self.revision = self.revision.saturating_add(1);
        let turn_id = TurnId(format!("rust_turn_{}_{}", now, self.revision));
        let origin = input.origin.unwrap_or_else(|| infer_origin(input.source_label.as_deref()));
        let assistant_label = input.assistant_label.as_deref().unwrap_or("Nova");
        let event_probe = parse_source(&TeachingSourceInput {
            source_id: None,
            source_name: "conversation".to_string(),
            text: input.text.clone(),
            source_type: Some("conversation".to_string()),
            kind_hint: Some("conversation".to_string()),
        });
        let event_kind = if event_probe.instructional && !event_probe.has_test_material { "teaching" } else { "cognition" }.to_string();

        // Pass 6 migration bootstrap only. Current native callers no longer send
        // a semantic snapshot each turn. If an old test/client does so before the
        // Rust store initializes, import it once; after initialization it cannot
        // replace current Rust semantic memory.
        let legacy_knowledge = input.knowledge.unwrap_or_default();
        if !self.semantic_memory.initialized && (!legacy_knowledge.concepts.is_empty() || !legacy_knowledge.claims.is_empty() || !legacy_knowledge.guidance.is_empty() || !legacy_knowledge.examples.is_empty() || !legacy_knowledge.procedures.is_empty()) {
            let revision = legacy_knowledge.source_revision;
            let _ = self.semantic_memory.import_once(SemanticMemoryImportInput {
                persisted_state: None,
                knowledge: legacy_knowledge,
                sources: Vec::new(),
                source_revision: revision,
                reason: Some("pass6_turn_legacy_semantic_bootstrap".to_string()),
            });
        }
        if !self.semantic_memory.initialized { self.semantic_memory = RustSemanticMemoryState::new_initialized(); }
        let knowledge = self.semantic_memory.snapshot();

        // Pass 5/6 cognitive migration bootstrap. Current native turns send
        // cognition:null; this exists only for old callers before learning state
        // initialization and never replaces an initialized Rust catalog.
        let bridged_cognition = input.cognition.unwrap_or_default();
        if !self.learning_state.initialized && !bridged_cognition.modules.is_empty() {
            let import = LearningSyncInput {
                persisted_state: None,
                rules: Vec::new(),
                existing_modules: bridged_cognition.modules.clone(),
                candidates: Vec::new(),
                source_candidates: Vec::new(),
                experience_routes: bridged_cognition.experience_routes.clone(),
                source_revision: bridged_cognition.source_revision,
                reason: Some("pass6_turn_legacy_cognition_bootstrap".to_string()),
            };
            self.learning_state = synchronize_learning(import, &self.learning_state);
        }

        let conversation_context = self.conversation_context_for_turn(&input.conversation_context);
        let (mut canonical_state, semantic_report) = interpret_turn(
            &input.text,
            &origin,
            assistant_label,
            &knowledge,
            &self.discourse_memory,
            &conversation_context,
        );
        let cognition = CognitiveRuntimeSnapshot {
            modules: self.learning_state.modules.clone(),
            capability_graph: self.learning_state.capability_graph.clone(),
            experience_routes: self.learning_state.experience_routes.clone(),
            source_revision: self.learning_state.source_revision,
        };
        let cognitive_report = if semantic_report.suggested_surface.is_some() {
            CognitiveRuntimeReport { modules_available: cognition.modules.len(), ..Default::default() }
        } else {
            run_cognitive_runtime(&mut canonical_state, &knowledge, &cognition, &mut self.development_state)
        };
        refresh_cognitive_trace(&mut canonical_state, &self.discourse_memory, Some(&cognitive_report));
        let realization_report = realize_current(&canonical_state, &semantic_report, &origin);
        self.cognitive_state = canonical_state.clone();
        self.last_semantic_report = Some(semantic_report.clone());
        self.last_cognitive_report = Some(cognitive_report.clone());
        self.last_realization_report = Some(realization_report.clone());
        self.refresh_learning_views();
        self.remember_conversation_turn(ConversationContextTurn {
            id: turn_id.0.clone(),
            role: "user".to_string(),
            source_label: input.source_label.clone(),
            content: input.text.clone(),
            created_at: now,
        });

        self.current_turn = Some(TurnEnvelope {
            id: turn_id.clone(),
            origin,
            source_label: input.source_label,
            event_kind: event_kind.clone(),
            raw_text: input.text,
            started_at: now,
            finished_at: None,
            semantic_runtime: "rust_v7_pass_9".to_string(),
            remaining_reasoning_runtime: "rust_v7_pass_9_authoritative_mind".to_string(),
            terminal_state: None,
            reason_code: None,
            response_produced: None,
        });
        self.updated_at = now;

        TurnReceipt {
            turn_id,
            accepted: true,
            migration_phase: "rust_authoritative".to_string(),
            reasoning_owner: self.reasoning_owner.clone(),
            event_kind,
            semantic_report,
            cognitive_report,
            realization_report,
            canonical_state,
        }
    }

    #[cfg(test)]
    pub fn import_semantic_memory(&mut self, input: SemanticMemoryImportInput) -> SemanticMemoryImportReceipt {
        let receipt = self.semantic_memory.import_once(input);
        if receipt.imported {
            self.revision = self.revision.saturating_add(1);
            self.updated_at = unix_ms();
        }
        receipt
    }

    pub fn ingest_source(&mut self, mut input: TeachingSourceInput) -> Result<TeachingIngestionReceipt, String> {
        if input.text.trim().is_empty() { return Err("Nova cannot learn from an empty source.".to_string()); }
        if input.source_name.trim().is_empty() { input.source_name = "document".to_string(); }
        let parsed = parse_source(&input);
        if parsed.document_kind == "test" {
            return Err("Test documents are evaluation material and cannot enter learned memory before Nova attempts them.".to_string());
        }
        let source_id = input.source_id.clone().filter(|id| !id.trim().is_empty()).unwrap_or_else(|| stable_source_id(&input.source_name, &input.text));
        input.source_id = Some(source_id.clone());

        // Re-teaching an existing source replaces that source's contribution in
        // the authoritative Rust mind. It is not layered on top of stale facts
        // or stale cognitive programs from the previous source contents.
        self.semantic_memory.remove_source(&source_id);
        let semantic_report = apply_semantic_learning(&mut self.semantic_memory, &input, &parsed);
        let translation = translate_cognition(&input, &parsed, &source_id);
        self.learning_state = replace_rust_teaching_source(
            &self.learning_state,
            &source_id,
            translation.rules,
            translation.candidates,
            Some(self.semantic_memory.revision),
            format!("rust_pass6_teaching_ingestion:{source_id}"),
        );
        self.refresh_learning_views();
        if !self.development_state.initialized { self.development_state = RustDevelopmentState::new_initialized(); }
        for module in self.learning_state.modules.iter().filter(|module| module.source_id.as_deref() == Some(source_id.as_str())) {
            for category in &module.categories { self.development_state.register_category(category, "learned_module", "ACTIVE", Vec::new(), vec![serde_json::json!({"sourceId":source_id.clone(),"moduleId":module.id.0.clone()})]); }
        }
        self.revision = self.revision.saturating_add(1);
        self.updated_at = unix_ms();
        let synthesis_report = self.learning_state.last_synthesis.clone().unwrap_or_default();
        let realization_report = realize_learning(semantic_report.counts.total(), synthesis_report.upgraded, synthesis_report.stored_only);
        if self.current_turn.as_ref().is_some_and(|turn| turn.event_kind == "teaching") { self.last_realization_report = Some(realization_report.clone()); }
        self.queue_learning_reflection(&source_id, &parsed.source_display_name);
        self.advance_internal_cognition();

        Ok(TeachingIngestionReceipt {
            accepted: true,
            authority: "rust_v7_pass_9".to_string(),
            source_id,
            parsed,
            semantic_report,
            semantic_state: self.semantic_memory.clone(),
            learning_state: self.learning_state.clone(),
            synthesis_report,
            realization_report,
        })
    }

    #[cfg(test)]
    pub fn synchronize_learning(&mut self, input: LearningSyncInput) -> LearningSyncReceipt {
        self.learning_state = synchronize_learning(input, &self.learning_state);
        self.refresh_learning_views();
        self.revision = self.revision.saturating_add(1);
        self.updated_at = unix_ms();
        let report = self.learning_state.last_synthesis.clone().unwrap_or_default();
        LearningSyncReceipt { accepted: true, authority: "rust_v7_pass_9".to_string(), state: self.learning_state.clone(), report }
    }

    #[cfg(test)]
    pub fn import_development(&mut self, input: DevelopmentImportInput) -> DevelopmentImportReceipt {
        let receipt = import_development(input, &self.development_state);
        if receipt.imported { self.development_state = receipt.state.clone(); self.revision = self.revision.saturating_add(1); self.updated_at = unix_ms(); }
        receipt
    }

    #[cfg(test)]
    pub fn import_internal_cognition(&mut self, input: InternalCognitionImportInput) -> InternalCognitionImportReceipt {
        let receipt = import_internal_cognition(input, &self.internal_cognition);
        if receipt.imported { self.internal_cognition = receipt.state.clone(); self.revision = self.revision.saturating_add(1); self.updated_at = unix_ms(); }
        receipt
    }

    #[cfg(test)]
    pub fn reset_developmental_state(&mut self) {
        self.semantic_memory = RustSemanticMemoryState::default();
        self.learning_state = RustLearningState::default();
        self.development_state = RustDevelopmentState::default();
        self.internal_cognition = RustInternalCognitionState::default();
        self.module_catalog.clear();
        self.capability_graph = CapabilityGraph::default();
        self.experience_index.clear();
        self.discourse_memory = DiscourseSnapshot::default();
        self.conversation_history.clear();
        self.cognitive_state = CanonicalCognitiveState::default();
        self.current_turn = None;
        self.last_completed_turn = None;
        self.last_semantic_report = None;
        self.last_cognitive_report = None;
        self.last_realization_report = None;
        self.revision = self.revision.saturating_add(1);
        self.updated_at = unix_ms();
    }

    fn project_teacher_record_to_knowledge(&mut self, record: &TeacherAttemptRecord) {
        if !self.semantic_memory.initialized {
            self.semantic_memory = RustSemanticMemoryState::new_initialized();
        }
        let evidence = serde_json::json!({
            "sourceId": record.knowledge_id.clone(),
            "sourceType": "teacher_feedback",
            "sourceRole": "non_executable_knowledge",
            "attemptId": record.id.clone(),
            "runId": record.run_id.clone(),
            "suiteId": record.suite_id.clone(),
            "caseId": record.case_id.clone(),
            "lessonContext": record.lesson_context.clone(),
            "attemptNumber": record.attempt_number,
            "teacherPointsAwarded": record.teacher_points_awarded,
            "teacherPointsPossible": record.teacher_points_possible,
            "teacherGrade": record.teacher_grade.clone(),
            "effectiveCreditPercent": record.effective_credit_percent(),
            "partialCredit": record.is_partial_credit(),
            "provenanceLabel": "Teacher",
            "nonExecutable": true,
            "provenance": record.provenance.clone(),
            "updatedAt": record.updated_at
        });
        let item = KnowledgeExample {
            id: record.knowledge_id.clone(),
            demonstrates_key: None,
            content: record.knowledge_text(),
            sequence: vec![serde_json::json!({
                "question": record.question.clone(),
                "originalAnswer": record.original_answer.clone(),
                "expectedAnswer": record.expected_answer.clone(),
                "expectedBehavior": record.expected_behavior.clone(),
                "automaticStatus": record.automatic_status.clone(),
                "automaticReason": record.automatic_reason.clone(),
                "cognitiveTrace": record.cognitive_trace.clone(),
                "cognitiveContext": record.cognitive_context.clone(),
                "inspectorContext": record.inspector_context.clone(),
                "lessonContext": record.lesson_context.clone(),
                "attemptNumber": record.attempt_number,
                "teacherPointsAwarded": record.teacher_points_awarded,
                "teacherPointsPossible": record.teacher_points_possible,
                "teacherGrade": record.teacher_grade.clone(),
                "teacherCreditPercent": record.teacher_credit_percent,
                "partialCredit": record.is_partial_credit(),
                "teacherNotes": record.teacher_notes.clone(),
                "annotations": record.annotations.clone(),
                "nonExecutable": true
            })],
            evidence: Vec::new(),
        };
        if let Some(existing) = self.semantic_memory.knowledge.examples.iter_mut().find(|row| row.id == item.id) {
            *existing = item;
            existing.evidence.push(evidence);
        } else {
            self.semantic_memory.store_example(item, evidence);
        }
        self.semantic_memory.revision = self.semantic_memory.revision.saturating_add(1);
        self.semantic_memory.knowledge.source_revision = Some(self.semantic_memory.revision);
        self.semantic_memory.updated_at = unix_ms();
    }

    pub fn record_teacher_attempt(&mut self, input: RecordTeacherAttemptInput) -> Result<TeacherFeedbackReceipt, String> {
        let cognitive_trace = self.cognitive_state.cognitive_trace.clone();
        let cognitive_context = serde_json::json!({
            "turnId": self.last_completed_turn.as_ref().map(|turn| turn.id.clone()),
            "activeConcepts": self.cognitive_state.active_concepts.iter().map(|concept| concept.label.clone()).collect::<Vec<_>>(),
            "relationships": self.cognitive_state.relations.iter().take(12).map(|relation| serde_json::json!({
                "type": relation.relation_type.clone(),
                "subject": relation.subject.clone(),
                "object": relation.object.clone()
            })).collect::<Vec<_>>(),
            "unresolved": self.cognitive_state.unresolved.iter().take(12).map(|need| need.description.clone().unwrap_or_else(|| need.need_type.clone())).collect::<Vec<_>>(),
            "conclusions": self.cognitive_state.conclusions.iter().filter_map(|conclusion| conclusion.surface.clone()).collect::<Vec<_>>(),
            "basicEnglish": self.cognitive_state.english_bootstrap.clone(),
            "terminalState": self.last_realization_report.as_ref().map(|report| report.terminal_state.clone()),
            "reasonCode": self.last_realization_report.as_ref().map(|report| report.reason_code.clone())
        });
        let inspector_context = serde_json::json!({
            "semanticReport": self.last_semantic_report.clone(),
            "cognitiveReport": self.last_cognitive_report.clone(),
            "realizationReport": self.last_realization_report.clone(),
            "requests": self.cognitive_state.requests.iter().take(12).cloned().collect::<Vec<_>>(),
            "unknownSlots": self.cognitive_state.unknown_slots.iter().take(12).cloned().collect::<Vec<_>>(),
            "candidateSets": self.cognitive_state.candidate_sets.iter().take(12).cloned().collect::<Vec<_>>(),
            "evidence": self.cognitive_state.evidence.iter().take(20).cloned().collect::<Vec<_>>()
        });
        let record = self.teacher_state.record_attempt(input, cognitive_trace, cognitive_context, inspector_context)?;
        // Keep grading/answer-key material out of semantic retrieval until the
        // entire run is finished. The attempt itself is already durable Rust
        // state, but later test questions cannot learn from its answer key.
        self.revision = self.revision.saturating_add(1);
        self.updated_at = unix_ms();
        Ok(TeacherFeedbackReceipt {
            accepted: true,
            authority: "rust_v7_alpha_9_3".to_string(),
            knowledge_id: record.knowledge_id.clone(),
            record,
        })
    }

    pub fn finalize_teacher_run(&mut self, input: FinalizeTeacherRunInput) -> Result<TeacherRunFinalizeReceipt, String> {
        let run_id = input.run_id.trim().to_string();
        let released = self.teacher_state.release_run_evidence(&run_id)?;
        let knowledge_ids = released.iter().map(|record| record.knowledge_id.clone()).collect::<Vec<_>>();
        for record in &released {
            self.project_teacher_record_to_knowledge(record);
        }
        if !released.is_empty() {
            self.revision = self.revision.saturating_add(1);
            self.updated_at = unix_ms();
        }
        Ok(TeacherRunFinalizeReceipt {
            accepted: true,
            authority: "rust_v7_alpha_9_3".to_string(),
            run_id,
            released_count: released.len(),
            knowledge_ids,
        })
    }

    pub fn update_teacher_feedback(&mut self, input: UpdateTeacherFeedbackInput) -> Result<TeacherFeedbackReceipt, String> {
        let record = self.teacher_state.update_feedback(input)?;
        if record.evidence_released {
            self.project_teacher_record_to_knowledge(&record);
        }
        self.revision = self.revision.saturating_add(1);
        self.updated_at = unix_ms();
        Ok(TeacherFeedbackReceipt {
            accepted: true,
            authority: "rust_v7_alpha_9_3".to_string(),
            knowledge_id: record.knowledge_id.clone(),
            record,
        })
    }

    fn queue_learning_reflection(&mut self, source_id: &str, display_name: &str) {
        self.internal_cognition.reopen_blocked_after_learning();
        let description = format!("Integrate the recently learned source {} with related existing cognition", display_name);
        let _ = self.internal_cognition.queue_goal(
            "learning_reflection",
            &description,
            "learning",
            0.78,
            vec![serde_json::json!({"sourceId":source_id})],
        );
    }

    fn queue_unresolved_reflection(&mut self, turn_id: &str) {
        if let Some(need) = self.cognitive_state.unresolved.first() {
            let description = need.description.clone().unwrap_or_else(|| need.need_type.replace('_', " "));
            let _ = self.internal_cognition.queue_goal(
                "unresolved_reflection",
                &description,
                "completed_turn",
                0.74,
                vec![serde_json::json!({"turnId":turn_id,"needType":need.need_type})],
            );
        }
    }

    fn advance_internal_cognition(&mut self) {
        let Some(goal) = self.internal_cognition.next_goal() else { return; };
        let origin = TurnOrigin::Internal;
        let knowledge = self.semantic_memory.snapshot();
        let (mut state, semantic_report) = interpret_turn(
            &goal.description,
            &origin,
            "Nova",
            &knowledge,
            &self.discourse_memory,
            &[],
        );
        let cognition = CognitiveRuntimeSnapshot {
            modules: self.learning_state.modules.clone(),
            capability_graph: self.learning_state.capability_graph.clone(),
            experience_routes: self.learning_state.experience_routes.clone(),
            source_revision: self.learning_state.source_revision,
        };
        let cognitive_report = if semantic_report.suggested_surface.is_some() {
            CognitiveRuntimeReport { modules_available: cognition.modules.len(), ..Default::default() }
        } else {
            run_cognitive_runtime(&mut state, &knowledge, &cognition, &mut self.development_state)
        };
        refresh_cognitive_trace(&mut state, &self.discourse_memory, Some(&cognitive_report));
        let realization = realize_current(&state, &semantic_report, &origin);
        self.internal_cognition.mark_result(&goal.id, &realization.terminal_state, &realization.reason_code);
        let internal_turn_id = format!("internal_{}", goal.id);
        let supported = realization.terminal_state == "SUPPORTED";
        let _ = reflect_completed_turn(&mut self.development_state, &mut self.learning_state, &state, &cognitive_report, &internal_turn_id, &goal.description, supported);
        self.refresh_learning_views();
    }

    pub fn finish_turn(&mut self, input: FinishTurnInput) -> Result<(), String> {
        let now = unix_ms();
        let mut turn = self.current_turn.take().ok_or_else(|| "No Rust-owned current turn exists.".to_string())?;
        if turn.id != input.turn_id {
            let expected = turn.id.0.clone();
            self.current_turn = Some(turn);
            return Err(format!("Turn id mismatch. Rust owns {expected}, frontend tried to finish {}.", input.turn_id.0));
        }
        turn.finished_at = Some(now);
        let realization = if let Some(report) = self.last_realization_report.clone() {
            report
        } else {
            let semantic_report = self.last_semantic_report.clone().unwrap_or_default();
            realize_current(&self.cognitive_state, &semantic_report, &turn.origin)
        };
        turn.terminal_state = Some(realization.terminal_state.clone());
        turn.reason_code = Some(realization.reason_code.clone());
        turn.response_produced = Some(input.response_produced);
        let supported = realization.terminal_state == "SUPPORTED" && (!realization.visible_response_required || input.response_produced);
        let report = self.last_cognitive_report.clone().unwrap_or_default();
        let _consolidation = reflect_completed_turn(&mut self.development_state, &mut self.learning_state, &self.cognitive_state, &report, &turn.id.0, &turn.raw_text, supported);
        self.refresh_learning_views();
        self.discourse_memory = discourse_from_completed_state(&self.cognitive_state);
        let completed_turn_id = turn.id.0.clone();
        let was_supported = supported;
        if input.response_produced {
            if let Some(surface) = realization.surface.clone().filter(|text| !text.trim().is_empty()) {
                self.remember_conversation_turn(ConversationContextTurn {
                    id: format!("{}_assistant", completed_turn_id),
                    role: "assistant".to_string(),
                    source_label: Some("Nova".to_string()),
                    content: surface,
                    created_at: now,
                });
            }
        }
        self.last_completed_turn = Some(turn);
        if !was_supported { self.queue_unresolved_reflection(&completed_turn_id); }
        self.advance_internal_cognition();
        self.revision = self.revision.saturating_add(1);
        self.updated_at = now;
        Ok(())
    }
}

fn infer_origin(source_label: Option<&str>) -> TurnOrigin {
    match source_label.unwrap_or("").trim().to_ascii_lowercase().as_str() {
        "teacher" => TurnOrigin::Teacher,
        "chatgpt" => TurnOrigin::ChatGpt,
        "internal" | "nova" => TurnOrigin::Internal,
        "" | "user" => TurnOrigin::User,
        _ => TurnOrigin::Other,
    }
}

pub struct NovaMindState {
    inner: RwLock<NovaMind>,
}

impl Default for NovaMindState {
    fn default() -> Self {
        Self {
            inner: RwLock::new(NovaMind::default()),
        }
    }
}

impl NovaMindState {
    pub(crate) fn replace(&self, mut replacement: NovaMind) -> Result<(), String> {
        replacement.normalize_pass_nine();
        let mut mind = self.inner.write().map_err(|_| "Nova Rust mind write lock was poisoned.".to_string())?;
        *mind = replacement;
        Ok(())
    }

    pub fn snapshot(&self) -> Result<NovaMind, String> {
        self.inner
            .read()
            .map(|mind| mind.clone())
            .map_err(|_| "Nova Rust mind read lock was poisoned.".to_string())
    }

    pub fn begin_turn(&self, input: BeginTurnInput) -> Result<TurnReceipt, String> {
        self.inner
            .write()
            .map_err(|_| "Nova Rust mind write lock was poisoned.".to_string())
            .map(|mut mind| mind.begin_turn(input))
    }

    #[cfg(test)]
    pub fn synchronize_learning(&self, input: LearningSyncInput) -> Result<LearningSyncReceipt, String> {
        self.inner
            .write()
            .map_err(|_| "Nova Rust mind write lock was poisoned.".to_string())
            .map(|mut mind| mind.synchronize_learning(input))
    }

    #[cfg(test)]
    pub fn import_semantic_memory(&self, input: SemanticMemoryImportInput) -> Result<SemanticMemoryImportReceipt, String> {
        self.inner
            .write()
            .map_err(|_| "Nova Rust mind write lock was poisoned.".to_string())
            .map(|mut mind| mind.import_semantic_memory(input))
    }

    #[cfg(test)]
    pub fn import_development(&self, input: DevelopmentImportInput) -> Result<DevelopmentImportReceipt, String> {
        self.inner
            .write()
            .map_err(|_| "Nova Rust mind write lock was poisoned.".to_string())
            .map(|mut mind| mind.import_development(input))
    }

    #[cfg(test)]
    pub fn import_internal_cognition(&self, input: InternalCognitionImportInput) -> Result<InternalCognitionImportReceipt, String> {
        self.inner
            .write()
            .map_err(|_| "Nova Rust mind write lock was poisoned.".to_string())
            .map(|mut mind| mind.import_internal_cognition(input))
    }

    pub fn ingest_source(&self, input: TeachingSourceInput) -> Result<TeachingIngestionReceipt, String> {
        self.inner
            .write()
            .map_err(|_| "Nova Rust mind write lock was poisoned.".to_string())?
            .ingest_source(input)
    }

    #[cfg(test)]
    pub fn reset_developmental_state(&self) -> Result<NovaMind, String> {
        let mut mind = self
            .inner
            .write()
            .map_err(|_| "Nova Rust mind write lock was poisoned.".to_string())?;
        mind.reset_developmental_state();
        Ok(mind.clone())
    }

    pub fn record_teacher_attempt(&self, input: RecordTeacherAttemptInput) -> Result<TeacherFeedbackReceipt, String> {
        self.inner
            .write()
            .map_err(|_| "Nova Rust mind write lock was poisoned.".to_string())?
            .record_teacher_attempt(input)
    }

    pub fn finalize_teacher_run(&self, input: FinalizeTeacherRunInput) -> Result<TeacherRunFinalizeReceipt, String> {
        self.inner
            .write()
            .map_err(|_| "Nova Rust mind write lock was poisoned.".to_string())?
            .finalize_teacher_run(input)
    }

    pub fn update_teacher_feedback(&self, input: UpdateTeacherFeedbackInput) -> Result<TeacherFeedbackReceipt, String> {
        self.inner
            .write()
            .map_err(|_| "Nova Rust mind write lock was poisoned.".to_string())?
            .update_teacher_feedback(input)
    }

    pub fn finish_turn(&self, input: FinishTurnInput) -> Result<NovaMind, String> {
        let mut mind = self
            .inner
            .write()
            .map_err(|_| "Nova Rust mind write lock was poisoned.".to_string())?;
        mind.finish_turn(input)?;
        Ok(mind.clone())
    }
}

#[cfg(test)]
mod tests;
