use super::*;
use crate::nova::cognition::{CognitiveModule, CognitiveRuntimeSnapshot, ExperienceRoute};
use crate::nova::learning::{LearnedRuleRecord, LearningSyncInput, RuleSynthesisCandidate, RustLearningState};
use crate::nova::teaching::TeachingSourceInput;
use crate::nova::ids::{CapabilityKey, ConceptId, ModuleId};
use crate::nova::semantics::knowledge::{KnowledgeClaim, KnowledgeConcept, KnowledgeGuidance, KnowledgeProcedure, KnowledgeSnapshot, SemanticMemoryImportInput};
use serde_json::json;

fn lesson_knowledge() -> KnowledgeSnapshot {
    KnowledgeSnapshot {
        concepts: vec![
            KnowledgeConcept {
                id: ConceptId("concept_lesson".to_string()),
                key: "lesson".to_string(),
                name: "lesson".to_string(),
                aliases: Vec::new(),
                kinds: Vec::new(),
                subject_eligible: true,
                instructional: true,
                ..Default::default()
            },
            KnowledgeConcept {
                id: ConceptId("concept_relation".to_string()),
                key: "relation".to_string(),
                name: "relation".to_string(),
                aliases: vec!["relationship".to_string()],
                kinds: Vec::new(),
                subject_eligible: true,
                instructional: true,
                ..Default::default()
            },
        ],
        claims: vec![KnowledgeClaim {
            id: "claim_lesson_definition".to_string(),
            subject_id: ConceptId("concept_lesson".to_string()),
            subject_key: "lesson".to_string(),
            subject: "lesson".to_string(),
            predicate: "definition".to_string(),
            object: "A lesson is organized information intended to change understanding or knowledge".to_string(),
            object_concept_id: None,
            polarity: "positive".to_string(),
            confidence: 0.95,
            qualifiers: json!({}),
            evidence: Vec::new(),
        }],
        source_revision: Some(1),
        ..Default::default()
    }
}



fn goal_knowledge() -> KnowledgeSnapshot {
    let mut knowledge = lesson_knowledge();
    knowledge.concepts.extend([
        KnowledgeConcept {
            id: ConceptId("concept_example".to_string()),
            key: "example".to_string(),
            name: "example".to_string(),
            aliases: Vec::new(),
            kinds: Vec::new(),
            subject_eligible: true,
            instructional: true,
            ..Default::default()
        },
        KnowledgeConcept {
            id: ConceptId("concept_teaching_label".to_string()),
            key: "teaching label".to_string(),
            name: "teaching label".to_string(),
            aliases: Vec::new(),
            kinds: Vec::new(),
            subject_eligible: true,
            instructional: true,
            ..Default::default()
        },
        KnowledgeConcept {
            id: ConceptId("concept_correction".to_string()),
            key: "correction".to_string(),
            name: "correction".to_string(),
            aliases: Vec::new(),
            kinds: Vec::new(),
            subject_eligible: true,
            instructional: true,
            ..Default::default()
        },
        KnowledgeConcept {
            id: ConceptId("concept_purpose".to_string()),
            key: "purpose".to_string(),
            name: "purpose".to_string(),
            aliases: Vec::new(),
            kinds: Vec::new(),
            subject_eligible: true,
            instructional: true,
            ..Default::default()
        },
        KnowledgeConcept {
            id: ConceptId("concept_lesson_interpretation".to_string()),
            key: "interpret a structured lesson".to_string(),
            name: "interpret a structured lesson".to_string(),
            aliases: Vec::new(),
            kinds: vec!["procedure".to_string()],
            subject_eligible: false,
            instructional: true,
            ..Default::default()
        },
    ]);
    knowledge.claims.extend([
        KnowledgeClaim {
            id: "claim_teaching_label_role".to_string(),
            subject_id: ConceptId("concept_teaching_label".to_string()),
            subject_key: "teaching label".to_string(),
            subject: "teaching label".to_string(),
            predicate: "determines".to_string(),
            object: "whether its content is a concept name, definition, relationship, guidance item, warning, behavior, procedure, step, example, scope, or lesson metadata".to_string(),
            object_concept_id: None,
            polarity: "positive".to_string(),
            confidence: 0.96,
            qualifiers: json!({}),
            evidence: Vec::new(),
        },
        KnowledgeClaim {
            id: "claim_purpose_definition".to_string(),
            subject_id: ConceptId("concept_purpose".to_string()),
            subject_key: "purpose".to_string(),
            subject: "purpose".to_string(),
            predicate: "definition".to_string(),
            object: "A purpose explains what a lesson is intended to teach".to_string(),
            object_concept_id: None,
            polarity: "positive".to_string(),
            confidence: 0.96,
            qualifiers: json!({}),
            evidence: Vec::new(),
        },
        KnowledgeClaim {
            id: "claim_correction_effect".to_string(),
            subject_id: ConceptId("concept_correction".to_string()),
            subject_key: "correction".to_string(),
            subject: "correction".to_string(),
            predicate: "relationship".to_string(),
            object: "A correction replaces or modifies the incorrect part while preserving related knowledge that remains valid".to_string(),
            object_concept_id: None,
            polarity: "positive".to_string(),
            confidence: 0.96,
            qualifiers: json!({}),
            evidence: Vec::new(),
        },
    ]);
    knowledge.guidance.push(KnowledgeGuidance {
        id: "guidance_example_not_rule".to_string(),
        domain: "instruction".to_string(),
        scope: "learned_concept".to_string(),
        target: "example".to_string(),
        strength: "required".to_string(),
        polarity: "negative".to_string(),
        directive: "Do not make an example into a permanent rule or default response merely because it contains instruction-like wording".to_string(),
        conditions: Vec::new(),
        evidence: Vec::new(),
    });
    knowledge.procedures.push(KnowledgeProcedure {
        id: "procedure_interpret_lesson".to_string(),
        concept_id: Some(ConceptId("concept_lesson_interpretation".to_string())),
        concept_key: "interpret a structured lesson".to_string(),
        name: "interpret a structured lesson".to_string(),
        interaction_mode: "single_turn".to_string(),
        roles: vec!["assistant".to_string()],
        sequence: vec![json!({
            "order": 1,
            "content": "Report honestly what was understood, stored, compiled, applied, or not yet executable"
        })],
        requirements: vec!["ordered_steps".to_string()],
        mechanisms: Vec::new(),
        optional_mechanisms: Vec::new(),
        purpose: None,
        confidence: 0.97,
        evidence: Vec::new(),
    });
    knowledge
}

#[test]
fn pass_eight_is_explicitly_partial_and_rust_owns_live_cognitive_authority_except_payload_orchestration() {
    let mind = NovaMind::default();
    assert!(matches!(mind.migration_phase, MigrationPhase::PartialRustMind));
    for subsystem in [
        "request_construction",
        "learned_cognitive_module_runtime",
        "structured_cognitive_trace",
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
    ] {
        assert_eq!(mind.subsystem_ownership.owners.get(subsystem).map(String::as_str), Some("rust_v7_pass_9"));
    }
    assert_eq!(mind.subsystem_ownership.owners.get("persistent_mind_payload_orchestration").map(String::as_str), Some("rust_v7_pass_9"));
    assert_eq!(mind.migration_phase, crate::nova::mind::MigrationPhase::RustAuthoritative);
}

#[test]
fn pass_six_rust_parser_owns_document_learning_classification_and_test_boundaries() {
    let parsed = crate::nova::teaching::parse_source(&TeachingSourceInput {
        source_id: Some("pass6_parser_probe".to_string()),
        source_name: "Mixed Notes.txt".to_string(),
        text: "Topic: Signals\nConcept: Velorin\nDefinition: A velorin is a marker.\nTest:\nQuestion: What is a velorin?\nExpected Answer: A marker.".to_string(),
        source_type: Some("learned".to_string()),
        kind_hint: None,
    });
    assert_eq!(parsed.document_kind, "document");
    assert!(parsed.instructional);
    assert!(parsed.has_test_material);
    assert!(parsed.blocks.iter().any(|block| block.section_kind != "test" && block.role == "concept"));
    assert!(parsed.blocks.iter().any(|block| block.section_kind == "test" && block.role == "test_expected_answer"));
}

#[test]
fn pass_six_rust_ingests_raw_teaching_and_retains_its_own_synthesis_candidates() {
    let state = NovaMindState::default();
    let receipt = state.ingest_source(TeachingSourceInput {
        source_id: Some("source_pass6_probe".to_string()),
        source_name: "Synthetic Lesson.txt".to_string(),
        text: "Topic: Synthetic signals\nConcept: Velorin\nDefinition: A velorin is a signal marker.\nGuidance: Prefer memory over fallback when evidence is available.".to_string(),
        source_type: Some("learned".to_string()),
        kind_hint: Some("lesson".to_string()),
    }).unwrap();

    assert_eq!(receipt.authority, "rust_v7_pass_9");
    assert!(receipt.semantic_state.initialized);
    assert!(receipt.semantic_state.knowledge.concepts.iter().any(|concept| concept.key == "velorin"));
    assert!(receipt.semantic_state.knowledge.claims.iter().any(|claim| claim.subject_key == "velorin" && claim.object.contains("signal marker")));
    assert!(!receipt.learning_state.candidates.is_empty());
    assert!(receipt.learning_state.rules.iter().any(|rule| rule.source_id.as_deref() == Some("source_pass6_probe")));
    assert!(receipt.learning_state.modules.iter().any(|module| module.source_id.as_deref() == Some("source_pass6_probe")));
}

#[test]
fn pass_six_initialized_semantic_memory_rejects_per_turn_js_replacement() {
    let state = NovaMindState::default();
    state.ingest_source(TeachingSourceInput {
        source_id: Some("source_rust_semantic_authority".to_string()),
        source_name: "Authority Lesson.txt".to_string(),
        text: "Concept: Velorin\nDefinition: A velorin is a Rust-owned marker.".to_string(),
        source_type: Some("learned".to_string()),
        kind_hint: Some("lesson".to_string()),
    }).unwrap();

    let mut attempted_replacement = KnowledgeSnapshot::default();
    attempted_replacement.concepts.push(KnowledgeConcept {
        id: ConceptId("concept_js_intruder".to_string()),
        key: "js intruder".to_string(),
        name: "JS intruder".to_string(),
        subject_eligible: true,
        instructional: false,
        ..Default::default()
    });
    state.begin_turn(BeginTurnInput {
        text: "What is a velorin?".to_string(),
        knowledge: Some(attempted_replacement),
        ..Default::default()
    }).unwrap();

    let mind = state.snapshot().unwrap();
    assert!(mind.semantic_memory.knowledge.concepts.iter().any(|concept| concept.key == "velorin"));
    assert!(!mind.semantic_memory.knowledge.concepts.iter().any(|concept| concept.key == "js intruder"));
}

#[test]
fn pass_six_frontend_sync_cannot_inject_new_brain_rules_after_rust_initializes() {
    let state = NovaMindState::default();
    state.ingest_source(TeachingSourceInput {
        source_id: Some("source_rust_learning_authority".to_string()),
        source_name: "Learning Authority Lesson.txt".to_string(),
        text: "Guidance: Prefer memory over fallback when evidence is available.".to_string(),
        source_type: Some("learned".to_string()),
        kind_hint: Some("lesson".to_string()),
    }).unwrap();
    let before = state.snapshot().unwrap();

    let injected = pass_five_rule("rule_js_attempted_injection_after_pass6");
    let receipt = state.synchronize_learning(LearningSyncInput {
        rules: vec![injected],
        reason: Some("frontend_attempted_brain_replacement".to_string()),
        ..Default::default()
    }).unwrap();

    assert_eq!(receipt.authority, "rust_v7_pass_9");
    assert!(!receipt.state.rules.iter().any(|rule| rule.id == "rule_js_attempted_injection_after_pass6"));
    assert_eq!(receipt.state.rules.len(), before.learning_state.rules.len());
}

#[test]
fn rust_opens_a_learned_concept_and_composes_a_supported_answer() {
    let state = NovaMindState::default();
    let receipt = state
        .begin_turn(BeginTurnInput {
            text: "What is a lesson?".to_string(),
            source_label: Some("Teacher".to_string()),
            origin: None,
            assistant_label: Some("Nova".to_string()),
            knowledge: Some(lesson_knowledge()),
            cognition: None,
            conversation_context: Vec::new(),
        })
        .unwrap();
    assert!(receipt.accepted);
    assert!(receipt.semantic_report.concepts_activated >= 1);
    assert_eq!(receipt.semantic_report.requests_resolved, 1);
    assert!(receipt.semantic_report.suggested_surface.as_deref().unwrap_or("").contains("organized information"));
    assert!(receipt.canonical_state.active_concepts.iter().any(|concept| concept.key == "lesson"));
    assert!(receipt.canonical_state.cognitive_trace.iter().any(|event| event.stage == "goal"));
    assert!(receipt.canonical_state.cognitive_trace.iter().any(|event| event.stage == "conclusion"));
    assert!(!receipt.canonical_state.cognitive_trace.iter().any(|event| event.text.contains("cycle") || event.text.contains("module reference")));
}

#[test]
fn meaning_trace_uses_basic_previous_utterance_link_without_inventing_deeper_semantics() {
    let state = NovaMindState::default();
    let receipt = state.begin_turn(BeginTurnInput {
        text: "Can you say that in a more natural way?".to_string(),
        source_label: None,
        origin: None,
        assistant_label: Some("Nova".to_string()),
        knowledge: Some(lesson_knowledge()),
        cognition: None,
        conversation_context: vec![crate::nova::semantics::ConversationContextTurn {
            id: "assistant_previous".to_string(),
            role: "assistant".to_string(),
            source_label: None,
            content: "I learned definitions and structured lesson roles.".to_string(),
            created_at: 1,
        }],
    }).unwrap();
    let link = receipt.canonical_state.english_bootstrap.context_links.iter()
        .find(|row| row.surface.eq_ignore_ascii_case("that"))
        .expect("basic previous-utterance link");
    assert_eq!(link.referent_id.as_deref(), Some("assistant_previous"));
    let context = receipt.canonical_state.cognitive_trace.iter().find(|event| event.stage == "context").expect("context trace event");
    assert!(context.text.contains("connected this turn"));
    assert!(!receipt.canonical_state.cognitive_trace.iter().any(|event| event.text.contains("Rust semantic executive") || event.text.contains("module references")));
}

#[test]
fn rust_distinguishes_a_quoted_question_from_the_outer_meta_question() {
    let state = NovaMindState::default();
    let receipt = state
        .begin_turn(BeginTurnInput {
            text: "In ‘Mira opened the box. Who opened the box?’, what relation is requested?".to_string(),
            source_label: Some("Teacher".to_string()),
            origin: None,
            assistant_label: Some("Nova".to_string()),
            knowledge: Some(lesson_knowledge()),
            cognition: None,
            conversation_context: Vec::new(),
        })
        .unwrap();
    assert!(receipt.canonical_state.surface.quotes.len() == 1);
    assert!(receipt.canonical_state.requests.iter().any(|request| request.answer_relation == "left_argument"));
    assert!(receipt.canonical_state.requests.iter().any(|request| request.answer_relation == "requested_relation"));
    assert_eq!(receipt.semantic_report.suggested_surface.as_deref(), Some("left argument."));
}

#[test]
fn general_goal_uses_learned_warning_evidence_without_a_why_handler() {
    let state = NovaMindState::default();
    let receipt = state.begin_turn(BeginTurnInput {
        text: "Why should examples not automatically become executable rules?".to_string(),
        source_label: Some("Teacher".to_string()),
        origin: None,
        assistant_label: Some("Nova".to_string()),
        knowledge: Some(goal_knowledge()),
        cognition: None,
        conversation_context: Vec::new(),
    }).unwrap();
    assert_eq!(receipt.semantic_report.requests_resolved, 1);
    assert!(receipt.canonical_state.requests.iter().any(|request| request.source == "rust_general_semantic_goal_induction"));
    let surface = receipt.semantic_report.suggested_surface.unwrap_or_default().to_lowercase();
    assert!(surface.contains("example"));
    assert!(surface.contains("permanent rule"));
}

#[test]
fn general_goal_can_open_a_concept_from_relevant_relationship_evidence() {
    let state = NovaMindState::default();
    let receipt = state.begin_turn(BeginTurnInput {
        text: "What determines the role of structured lesson content?".to_string(),
        source_label: Some("Teacher".to_string()),
        origin: None,
        assistant_label: Some("Nova".to_string()),
        knowledge: Some(goal_knowledge()),
        cognition: None,
        conversation_context: Vec::new(),
    }).unwrap();
    assert_eq!(receipt.semantic_report.requests_resolved, 1);
    assert!(receipt.canonical_state.active_concepts.iter().any(|concept| concept.key == "teaching label"));
    let surface = receipt.semantic_report.suggested_surface.unwrap_or_default().to_lowercase();
    assert!(surface.contains("teaching label"));
    assert!(surface.contains("content"));
}

#[test]
fn general_goal_can_use_a_learned_procedure_step_as_semantic_evidence() {
    let state = NovaMindState::default();
    let receipt = state.begin_turn(BeginTurnInput {
        text: "What should Nova do when a lesson is understood but cannot yet be executed?".to_string(),
        source_label: Some("Teacher".to_string()),
        origin: None,
        assistant_label: Some("Nova".to_string()),
        knowledge: Some(goal_knowledge()),
        cognition: None,
        conversation_context: Vec::new(),
    }).unwrap();
    assert_eq!(receipt.semantic_report.requests_resolved, 1);
    let surface = receipt.semantic_report.suggested_surface.unwrap_or_default().to_lowercase();
    assert!(surface.contains("report honestly"));
    assert!(surface.contains("not yet executable"));
}

#[test]
fn general_goal_can_select_a_learned_effect_relationship() {
    let state = NovaMindState::default();
    let receipt = state.begin_turn(BeginTurnInput {
        text: "What is the purpose of a correction during learning?".to_string(),
        source_label: Some("Teacher".to_string()),
        origin: None,
        assistant_label: Some("Nova".to_string()),
        knowledge: Some(goal_knowledge()),
        cognition: None,
        conversation_context: Vec::new(),
    }).unwrap();
    assert_eq!(receipt.semantic_report.requests_resolved, 1);
    let surface = receipt.semantic_report.suggested_surface.unwrap_or_default().to_lowercase();
    assert!(surface.contains("correction"));
    assert!(surface.contains("incorrect part"));
}

#[test]
fn general_goal_transfers_to_unrelated_learned_content() {
    let mut knowledge = KnowledgeSnapshot::default();
    knowledge.concepts.push(KnowledgeConcept {
        id: ConceptId("concept_velorin_marker".to_string()),
        key: "velorin marker".to_string(),
        name: "velorin marker".to_string(),
        aliases: Vec::new(),
        kinds: Vec::new(),
        subject_eligible: true,
        instructional: false,
        ..Default::default()
    });
    knowledge.claims.push(KnowledgeClaim {
        id: "claim_velorin_marker".to_string(),
        subject_id: ConceptId("concept_velorin_marker".to_string()),
        subject_key: "velorin marker".to_string(),
        subject: "velorin marker".to_string(),
        predicate: "indicates".to_string(),
        object: "which channel carries a velorin packet".to_string(),
        object_concept_id: None,
        polarity: "positive".to_string(),
        confidence: 0.96,
        qualifiers: json!({}),
        evidence: Vec::new(),
    });
    let state = NovaMindState::default();
    let receipt = state.begin_turn(BeginTurnInput {
        text: "What information does a velorin marker provide about a packet?".to_string(),
        source_label: Some("Teacher".to_string()),
        origin: None,
        assistant_label: Some("Nova".to_string()),
        knowledge: Some(knowledge),
        cognition: None,
        conversation_context: Vec::new(),
    }).unwrap();
    assert_eq!(receipt.semantic_report.requests_resolved, 1);
    let surface = receipt.semantic_report.suggested_surface.unwrap_or_default().to_lowercase();
    assert!(surface.contains("channel"));
    assert!(surface.contains("packet"));
}


fn runtime_probe_module(id: &str, number: u64, cue: &str, response: &str) -> CognitiveModule {
    CognitiveModule {
        id: ModuleId(id.to_string()),
        module_number: Some(number),
        name: format!("runtime probe {id}"),
        domain: "reasoning".to_string(),
        phase: "reasoning".to_string(),
        categories: vec!["runtime_probe".to_string()],
        conditions: vec![json!({"type":"contains_any","values":[cue]})],
        program: vec![
            json!({"op":"set_field","args":{"path":"learnedState.runtimeProbe","value":id}}),
            json!({"op":"emit_surface","args":{"text":response}}),
        ],
        requires: Vec::new(),
        produces: vec![CapabilityKey(format!("probe_capability_{id}"))],
        capabilities: Vec::new(),
        priority: 120.0,
        enabled: true,
        source_type: "learned".to_string(),
        source_name: "synthetic transfer test".to_string(),
        source_id: None,
        cognitive_form: None,
        evidence: Vec::new(),
    }
}

#[test]
fn pass_four_executes_a_learned_module_inside_rust() {
    let state = NovaMindState::default();
    let cognition = CognitiveRuntimeSnapshot {
        modules: vec![runtime_probe_module("velorin", 41, "velorin", "velorin route complete")],
        ..Default::default()
    };
    let receipt = state.begin_turn(BeginTurnInput {
        text: "velorin signal".to_string(),
        source_label: Some("Teacher".to_string()),
        origin: None,
        assistant_label: Some("Nova".to_string()),
        knowledge: Some(KnowledgeSnapshot::default()),
        cognition: Some(cognition),
        conversation_context: Vec::new(),
    }).unwrap();
    assert_eq!(receipt.cognitive_report.surface_response.as_deref(), Some("velorin route complete"));
    assert_eq!(receipt.cognitive_report.modules_changed, 1);
    assert_eq!(receipt.cognitive_report.search_mode, "explore_all");
    assert_eq!(receipt.canonical_state.learned_state.get("runtimeProbe").and_then(|value| value.as_str()), Some("velorin"));
    assert!(receipt.canonical_state.rust_owned_subsystems.iter().any(|owner| owner == "learned_cognitive_module_runtime"));
}

#[test]
fn pass_four_gates_a_module_when_its_required_capability_is_missing() {
    let mut module = runtime_probe_module("gated", 51, "gated", "should not appear");
    module.requires = vec![CapabilityKey("missing_general_primitive".to_string())];
    let mut unavailable_producer = runtime_probe_module("producer", 52, "producer", "unused");
    unavailable_producer.enabled = false;
    unavailable_producer.produces = vec![CapabilityKey("missing_general_primitive".to_string())];
    let state = NovaMindState::default();
    let receipt = state.begin_turn(BeginTurnInput {
        text: "gated signal".to_string(),
        source_label: Some("Teacher".to_string()),
        origin: None,
        assistant_label: Some("Nova".to_string()),
        knowledge: Some(KnowledgeSnapshot::default()),
        cognition: Some(CognitiveRuntimeSnapshot { modules: vec![module, unavailable_producer], ..Default::default() }),
        conversation_context: Vec::new(),
    }).unwrap();
    assert!(receipt.cognitive_report.surface_response.is_none());
    assert!(receipt.cognitive_report.missing_capabilities.iter().any(|value| value == "missing_general_primitive"));
}

#[test]
fn pass_four_uses_an_experienced_route_first_and_broadens_if_it_stalls() {
    let stale = runtime_probe_module("stale", 61, "never-present-cue", "stale route");
    let useful = runtime_probe_module("useful", 62, "cobalt", "broad route found it");
    let route = ExperienceRoute {
        id: "route_cobalt".to_string(),
        concept_terms: vec!["cobalt".to_string()],
        goal_terms: Vec::new(),
        relation_terms: Vec::new(),
        fallback_terms: vec!["cobalt".to_string()],
        module_numbers: vec![61],
        module_strengths: json!({"61":0.9}),
        successes: 3,
        failures: 0,
        confidence: 0.8,
        ..Default::default()
    };
    let state = NovaMindState::default();
    let receipt = state.begin_turn(BeginTurnInput {
        text: "cobalt".to_string(),
        source_label: Some("Teacher".to_string()),
        origin: None,
        assistant_label: Some("Nova".to_string()),
        knowledge: Some(KnowledgeSnapshot::default()),
        cognition: Some(CognitiveRuntimeSnapshot {
            modules: vec![stale, useful],
            experience_routes: vec![route],
            ..Default::default()
        }),
        conversation_context: Vec::new(),
    }).unwrap();
    assert_eq!(receipt.cognitive_report.surface_response.as_deref(), Some("broad route found it"));
    assert!(receipt.cognitive_report.broadened_after_experience_route);
    assert_eq!(receipt.cognitive_report.search_mode, "experience_route_then_explore_all");
    assert!(receipt.cognitive_report.experience_route_ids.iter().any(|id| id == "route_cobalt"));
}

#[test]
fn pass_four_does_not_run_modules_over_an_already_supported_rust_semantic_answer() {
    let state = NovaMindState::default();
    let cognition = CognitiveRuntimeSnapshot {
        modules: vec![CognitiveModule {
            id: ModuleId("wrong_override".to_string()),
            module_number: Some(71),
            name: "wrong override".to_string(),
            domain: "reasoning".to_string(),
            phase: "reasoning".to_string(),
            categories: vec!["probe".to_string()],
            conditions: vec![json!({"type":"always"})],
            program: vec![json!({"op":"emit_surface","args":{"text":"WRONG OVERRIDE"}})],
            priority: 999.0,
            enabled: true,
            source_type: "learned".to_string(),
            source_name: "synthetic transfer test".to_string(),
            ..Default::default()
        }],
        ..Default::default()
    };
    let receipt = state.begin_turn(BeginTurnInput {
        text: "What is a lesson?".to_string(),
        source_label: Some("Teacher".to_string()),
        origin: None,
        assistant_label: Some("Nova".to_string()),
        knowledge: Some(lesson_knowledge()),
        cognition: Some(cognition),
        conversation_context: Vec::new(),
    }).unwrap();
    assert!(receipt.semantic_report.suggested_surface.as_deref().unwrap_or("").contains("organized information"));
    assert_eq!(receipt.cognitive_report.modules_applied, 0);
    assert!(receipt.cognitive_report.surface_response.is_none());
}


#[test]
fn pass_four_preserves_open_learned_request_fields_and_array_paths() {
    let create = CognitiveModule {
        id: ModuleId("request_extension_create".to_string()),
        module_number: Some(81),
        name: "create open learned request".to_string(),
        domain: "reasoning".to_string(),
        phase: "reasoning".to_string(),
        categories: vec!["request".to_string()],
        conditions: vec![json!({"type":"contains_any","values":["echoq"]})],
        program: vec![json!({
            "op":"semantic_add_request",
            "args":{
                "type":"action_request",
                "speechAct":"request_candidate",
                "answerRelation":"selected_linguistic_content",
                "actionFamily":"exact_reproduction",
                "selectedContent":"Velorin"
            }
        })],
        priority: 200.0,
        enabled: true,
        source_type: "learned".to_string(),
        source_name: "synthetic transfer test".to_string(),
        ..Default::default()
    };
    let consume = CognitiveModule {
        id: ModuleId("request_extension_consume".to_string()),
        module_number: Some(82),
        name: "consume open learned request".to_string(),
        domain: "reasoning".to_string(),
        phase: "reasoning".to_string(),
        categories: vec!["request".to_string()],
        conditions: vec![json!({"type":"field_equals","field":"requests.0.actionFamily","value":"exact_reproduction"})],
        program: vec![json!({"op":"emit_surface","args":{"text":"extension preserved"}})],
        priority: 100.0,
        enabled: true,
        source_type: "learned".to_string(),
        source_name: "synthetic transfer test".to_string(),
        ..Default::default()
    };
    let state = NovaMindState::default();
    let receipt = state.begin_turn(BeginTurnInput {
        text: "echoq".to_string(),
        source_label: Some("Teacher".to_string()),
        origin: None,
        assistant_label: Some("Nova".to_string()),
        knowledge: Some(KnowledgeSnapshot::default()),
        cognition: Some(CognitiveRuntimeSnapshot { modules: vec![create, consume], ..Default::default() }),
        conversation_context: Vec::new(),
    }).unwrap();
    assert_eq!(receipt.cognitive_report.surface_response.as_deref(), Some("extension preserved"));
    let request = receipt.canonical_state.requests.first().expect("learned request");
    assert_eq!(request.extensions.get("actionFamily").and_then(|value| value.as_str()), Some("exact_reproduction"));
    assert_eq!(request.extensions.get("selectedContent").and_then(|value| value.as_str()), Some("Velorin"));
}

#[test]
fn pass_four_runs_reference_modules_per_mention_and_preserves_reference_extensions() {
    let module = CognitiveModule {
        id: ModuleId("reference_extension_probe".to_string()),
        module_number: Some(91),
        name: "reference extension probe".to_string(),
        domain: "reference".to_string(),
        phase: "reference".to_string(),
        categories: vec!["reference".to_string()],
        conditions: vec![json!({"type":"field_equals","field":"mention.grammaticalPerson","value":"first"})],
        program: vec![json!({"op":"set_field","args":{"path":"mention.learnedProbe","value":"seen"}})],
        priority: 100.0,
        enabled: true,
        source_type: "learned".to_string(),
        source_name: "synthetic transfer test".to_string(),
        ..Default::default()
    };
    let state = NovaMindState::default();
    let receipt = state.begin_turn(BeginTurnInput {
        text: "I wave.".to_string(),
        source_label: Some("Teacher".to_string()),
        origin: None,
        assistant_label: Some("Nova".to_string()),
        knowledge: Some(KnowledgeSnapshot::default()),
        cognition: Some(CognitiveRuntimeSnapshot { modules: vec![module], ..Default::default() }),
        conversation_context: Vec::new(),
    }).unwrap();
    let reference = receipt.canonical_state.references.first().expect("first-person reference");
    assert_eq!(reference.extensions.get("learnedProbe").and_then(|value| value.as_str()), Some("seen"));
    assert!(receipt.cognitive_report.modules_changed >= 1);
}

#[test]
fn pass_four_learned_surface_family_can_capture_quoted_payload_without_hardcoded_action_semantics() {
    let module = CognitiveModule {
        id: ModuleId("learned_surface_family_quote_probe".to_string()),
        module_number: Some(101),
        name: "learned surface family quote probe".to_string(),
        domain: "reasoning".to_string(),
        phase: "reasoning".to_string(),
        categories: vec!["learned_surface_family".to_string()],
        conditions: vec![json!({
            "type":"learned_surface_family",
            "positiveCues":["Repeat after me:"],
            "negativeCues":[],
            "threshold":0.30,
            "payloadMode":"quoted_region",
            "captureName":"selectedContent",
            "familyId":"synthetic_reproduction"
        })],
        program: vec![json!({"op":"emit_surface","args":{"text":"{{ captures.selectedContent }}"}})],
        priority: 100.0,
        enabled: true,
        source_type: "learned".to_string(),
        source_name: "synthetic transfer test".to_string(),
        ..Default::default()
    };
    let state = NovaMindState::default();
    let receipt = state.begin_turn(BeginTurnInput {
        text: "Repeat after me: \"Velorin.\"".to_string(),
        source_label: Some("Teacher".to_string()),
        origin: None,
        assistant_label: Some("Nova".to_string()),
        knowledge: Some(KnowledgeSnapshot::default()),
        cognition: Some(CognitiveRuntimeSnapshot { modules: vec![module], ..Default::default() }),
        conversation_context: Vec::new(),
    }).unwrap();
    assert_eq!(receipt.cognitive_report.surface_response.as_deref(), Some("Velorin."));
}

#[test]
fn pass_four_compound_conditions_require_their_learned_subconditions() {
    let module = CognitiveModule {
        id: ModuleId("compound_condition_probe".to_string()),
        module_number: Some(102),
        name: "compound condition probe".to_string(),
        domain: "reasoning".to_string(),
        phase: "reasoning".to_string(),
        categories: vec!["compound_condition".to_string()],
        conditions: vec![json!({"all":[
            {"type":"contains_any","values":["velorin"]},
            {"not":{"type":"contains_any","values":["blocked"]}}
        ]})],
        program: vec![json!({"op":"emit_surface","args":{"text":"compound condition satisfied"}})],
        priority: 100.0,
        enabled: true,
        source_type: "learned".to_string(),
        source_name: "synthetic transfer test".to_string(),
        ..Default::default()
    };
    let state = NovaMindState::default();
    let receipt = state.begin_turn(BeginTurnInput {
        text: "velorin signal".to_string(),
        source_label: Some("Teacher".to_string()),
        origin: None,
        assistant_label: Some("Nova".to_string()),
        knowledge: Some(KnowledgeSnapshot::default()),
        cognition: Some(CognitiveRuntimeSnapshot { modules: vec![module.clone()], ..Default::default() }),
        conversation_context: Vec::new(),
    }).unwrap();
    assert_eq!(receipt.cognitive_report.surface_response.as_deref(), Some("compound condition satisfied"));

    let blocked_state = NovaMindState::default();
    let blocked = blocked_state.begin_turn(BeginTurnInput {
        text: "velorin blocked signal".to_string(),
        source_label: Some("Teacher".to_string()),
        origin: None,
        assistant_label: Some("Nova".to_string()),
        knowledge: Some(KnowledgeSnapshot::default()),
        cognition: Some(CognitiveRuntimeSnapshot { modules: vec![module], ..Default::default() }),
        conversation_context: Vec::new(),
    }).unwrap();
    assert!(blocked.cognitive_report.surface_response.is_none());
}

#[test]
fn pass_four_does_not_treat_broad_module_categories_as_capabilities() {
    let mut gated = runtime_probe_module("category_is_not_capability", 111, "cobalt", "should not run");
    gated.categories = vec!["general".to_string(), "reasoning".to_string()];
    gated.requires = vec![CapabilityKey("learned_category:general".to_string())];

    // Pass 5 no longer accepts a frontend capability graph as authority.  The
    // explicit waiter therefore enters through Rust-owned Stored Only state.
    // The module's broad `general` category must not satisfy that capability.
    let mut waiter_rule = pass_five_rule("stored_only_general_probe");
    waiter_rule.synthesis = json!({"missingCapabilities":["learned_category:general"]});
    let state = NovaMindState::default();
    state.synchronize_learning(LearningSyncInput {
        rules: vec![waiter_rule],
        existing_modules: vec![gated],
        reason: Some("pass5_broad_category_capability_regression".to_string()),
        ..Default::default()
    }).unwrap();

    let receipt = state.begin_turn(BeginTurnInput {
        text: "cobalt".to_string(),
        source_label: Some("Teacher".to_string()),
        origin: None,
        assistant_label: Some("Nova".to_string()),
        knowledge: Some(KnowledgeSnapshot::default()),
        cognition: None,
        conversation_context: Vec::new(),
    }).unwrap();
    assert!(receipt.cognitive_report.surface_response.is_none());
    assert!(receipt.cognitive_report.missing_capabilities.iter().any(|value| value == "learned_category:general"));
}

#[test]
fn pass_four_nested_module_invocation_cannot_bypass_capability_gating() {
    let mut gated = runtime_probe_module("nested_gated_target", 121, "never_directly_needed", "blocked target ran");
    gated.requires = vec![CapabilityKey("missing_nested_primitive".to_string())];
    let mut unavailable_producer = runtime_probe_module("nested_producer", 122, "producer", "unused");
    unavailable_producer.enabled = false;
    unavailable_producer.produces = vec![CapabilityKey("missing_nested_primitive".to_string())];
    let caller = CognitiveModule {
        id: ModuleId("nested_caller".to_string()),
        module_number: Some(123),
        name: "nested caller".to_string(),
        domain: "reasoning".to_string(),
        phase: "reasoning".to_string(),
        categories: vec!["nested_call".to_string()],
        conditions: vec![json!({"type":"contains_any","values":["cobalt"]})],
        program: vec![json!({"op":"invoke_module","args":{"number":121}})],
        priority: 500.0,
        enabled: true,
        source_type: "learned".to_string(),
        source_name: "synthetic transfer test".to_string(),
        ..Default::default()
    };
    let state = NovaMindState::default();
    let receipt = state.begin_turn(BeginTurnInput {
        text: "cobalt".to_string(),
        source_label: Some("Teacher".to_string()),
        origin: None,
        assistant_label: Some("Nova".to_string()),
        knowledge: Some(KnowledgeSnapshot::default()),
        cognition: Some(CognitiveRuntimeSnapshot {
            modules: vec![caller, gated, unavailable_producer],
            ..Default::default()
        }),
        conversation_context: Vec::new(),
    }).unwrap();
    assert!(receipt.cognitive_report.surface_response.is_none());
}

fn pass_five_rule(id: &str) -> LearnedRuleRecord {
    LearnedRuleRecord {
        id: id.to_string(),
        directive: format!("Synthetic learned cognition {id}"),
        domain: "reasoning".to_string(),
        primitive: "learned_guidance".to_string(),
        scope: "general".to_string(),
        evidence_role: "instruction".to_string(),
        strength: "required".to_string(),
        polarity: "positive".to_string(),
        status: "stored_only".to_string(),
        source_type: "learned".to_string(),
        source_name: "pass5 synthetic test".to_string(),
        ..Default::default()
    }
}

fn pass_five_candidate_module(id: &str, op: &str) -> CognitiveModule {
    CognitiveModule {
        id: ModuleId(id.to_string()),
        name: id.to_string(),
        domain: "reasoning".to_string(),
        phase: "reasoning".to_string(),
        categories: vec!["synthetic".to_string()],
        conditions: vec![json!({"type":"always"})],
        program: vec![json!({"op":op,"args":{"path":"moduleState.pass5Probe","value":true}})],
        priority: 50.0,
        enabled: true,
        source_type: "learned".to_string(),
        source_name: "pass5 synthetic test".to_string(),
        ..Default::default()
    }
}

#[test]
fn pass_five_rust_adopts_a_candidate_only_after_current_substrate_validation() {
    let state = NovaMindState::default();
    let rule = pass_five_rule("rule_supported_candidate");
    let module = pass_five_candidate_module("module_supported_candidate", "set_field");
    let receipt = state.synchronize_learning(LearningSyncInput {
        rules: vec![rule],
        candidates: vec![RuleSynthesisCandidate { rule_id: "rule_supported_candidate".to_string(), module: Some(module), diagnostic: json!({}) }],
        reason: Some("pass5_supported_candidate_test".to_string()),
        ..Default::default()
    }).unwrap();
    assert_eq!(receipt.authority, "rust_v7_pass_9");
    assert_eq!(receipt.report.executable, 1);
    assert_eq!(receipt.report.stored_only, 0);
    assert_eq!(receipt.state.modules.len(), 1);
    assert_eq!(receipt.state.rules[0].status, "module_executable");
}

#[test]
fn pass_five_unsupported_program_or_condition_remains_stored_only() {
    let state = NovaMindState::default();
    let rule = pass_five_rule("rule_unsupported_candidate");
    let mut module = pass_five_candidate_module("module_unsupported_candidate", "developer_magic_answer");
    module.conditions = vec![json!({"type":"developer_knows_best"})];
    let receipt = state.synchronize_learning(LearningSyncInput {
        rules: vec![rule],
        candidates: vec![RuleSynthesisCandidate { rule_id: "rule_unsupported_candidate".to_string(), module: Some(module), diagnostic: json!({}) }],
        reason: Some("pass5_unsupported_candidate_test".to_string()),
        ..Default::default()
    }).unwrap();
    assert_eq!(receipt.report.executable, 0);
    assert_eq!(receipt.report.stored_only, 1);
    let result = receipt.report.results.iter().find(|row| row.rule_id == "rule_unsupported_candidate").expect("rule result");
    assert!(result.missing_operations.iter().any(|op| op == "developer_magic_answer"));
    assert!(result.missing_operations.iter().any(|op| op == "condition:developer_knows_best"));
}

#[test]
fn pass_five_fixed_point_reconsiders_stored_only_after_a_capability_appears() {
    let state = NovaMindState::default();
    let rule_a = pass_five_rule("rule_produces_p");
    let mut rule_b = pass_five_rule("rule_requires_p");
    rule_b.synthesis = json!({"missingCapabilities":["primitive_p"]});

    let mut module_a = pass_five_candidate_module("module_produces_p", "set_field");
    module_a.produces = vec![CapabilityKey("primitive_p".to_string())];
    let mut module_b = pass_five_candidate_module("module_requires_p", "set_field");
    module_b.requires = vec![CapabilityKey("primitive_p".to_string())];

    let receipt = state.synchronize_learning(LearningSyncInput {
        rules: vec![rule_a, rule_b],
        candidates: vec![
            RuleSynthesisCandidate { rule_id: "rule_produces_p".to_string(), module: Some(module_a), diagnostic: json!({}) },
            RuleSynthesisCandidate { rule_id: "rule_requires_p".to_string(), module: Some(module_b), diagnostic: json!({"missingCapabilities":["primitive_p"]}) },
        ],
        reason: Some("pass5_fixed_point_test".to_string()),
        ..Default::default()
    }).unwrap();

    assert_eq!(receipt.report.executable, 2);
    assert_eq!(receipt.report.stored_only, 0);
    assert!(receipt.report.rounds.len() >= 2);
    assert!(receipt.report.rounds[0].upgraded >= 1);
    assert!(receipt.report.rounds.iter().skip(1).any(|round| round.upgraded >= 1));
}

#[test]
fn pass_five_diagnostic_unmapped_state_never_becomes_a_reusable_capability() {
    let state = NovaMindState::default();
    let mut rule = pass_five_rule("rule_unmapped_diagnostic");
    rule.synthesis = json!({"missingCapabilities":["unmapped_cognitive_state_transformation"]});
    let mut module = pass_five_candidate_module("module_unmapped_producer", "set_field");
    module.produces = vec![CapabilityKey("unmapped_cognitive_state_transformation".to_string())];
    let receipt = state.synchronize_learning(LearningSyncInput {
        rules: vec![rule],
        existing_modules: vec![module],
        reason: Some("pass5_unmapped_diagnostic_test".to_string()),
        ..Default::default()
    }).unwrap();
    assert!(!receipt.state.capability_graph.producers.iter().any(|row| row.capability.0 == "unmapped_cognitive_state_transformation"));
    assert!(!receipt.state.capability_graph.waiters.iter().any(|row| row.capability.0 == "unmapped_cognitive_state_transformation"));
}

#[test]
fn pass_five_turns_do_not_replace_rust_learning_authority_with_a_new_js_snapshot() {
    let state = NovaMindState::default();
    let rule = pass_five_rule("rule_rust_owned_module");
    let module_a = pass_five_candidate_module("module_rust_owned", "set_field");
    state.synchronize_learning(LearningSyncInput {
        rules: vec![rule],
        candidates: vec![RuleSynthesisCandidate { rule_id: "rule_rust_owned_module".to_string(), module: Some(module_a), diagnostic: json!({}) }],
        reason: Some("pass5_authority_test".to_string()),
        ..Default::default()
    }).unwrap();

    let module_b = CognitiveModule {
        id: ModuleId("module_js_attempted_replacement".to_string()),
        name: "JS attempted replacement".to_string(),
        domain: "reasoning".to_string(),
        phase: "reasoning".to_string(),
        conditions: vec![json!({"type":"contains_any","values":["beta"]})],
        program: vec![json!({"op":"emit_surface","args":{"text":"wrong authority"}})],
        enabled: true,
        source_type: "learned".to_string(),
        source_name: "pass5 synthetic test".to_string(),
        ..Default::default()
    };
    let receipt = state.begin_turn(BeginTurnInput {
        text: "beta".to_string(),
        source_label: Some("Teacher".to_string()),
        origin: None,
        assistant_label: Some("Nova".to_string()),
        knowledge: Some(KnowledgeSnapshot::default()),
        cognition: Some(CognitiveRuntimeSnapshot { modules: vec![module_b], ..Default::default() }),
        conversation_context: Vec::new(),
    }).unwrap();
    assert_ne!(receipt.cognitive_report.surface_response.as_deref(), Some("wrong authority"));
    let mind = state.snapshot().unwrap();
    assert!(mind.learning_state.modules.iter().any(|module| module.id.0 == "module_rust_owned"));
    assert!(!mind.learning_state.modules.iter().any(|module| module.id.0 == "module_js_attempted_replacement"));
}


#[test]
fn pass_five_rust_validates_and_adopts_whole_source_candidate_modules_without_js_storage_authority() {
    let state = NovaMindState::default();
    let mut source_module = pass_five_candidate_module("source_construction_probe", "emit_surface");
    source_module.program = vec![json!({"op":"emit_surface","args":{"text":"source candidate ran"}})];
    source_module.conditions = vec![json!({"type":"contains_any","values":["velorin"]})];
    source_module.produces = vec![CapabilityKey("source_construction_probe_capability".to_string())];
    let receipt = state.synchronize_learning(LearningSyncInput {
        source_candidates: vec![source_module],
        reason: Some("pass5_source_candidate_test".to_string()),
        ..Default::default()
    }).unwrap();
    assert_eq!(receipt.report.source_candidate_modules, 1);
    assert_eq!(receipt.report.adopted_source_modules, 1);
    assert_eq!(receipt.report.rejected_source_modules, 0);
    assert!(receipt.state.modules.iter().any(|module| module.id.0 == "source_construction_probe"));
}

#[test]
fn pass_five_reset_clears_authoritative_rust_learning_state() {
    let state = NovaMindState::default();
    let rule = pass_five_rule("rule_reset_probe");
    let module = pass_five_candidate_module("module_reset_probe", "set_field");
    state.synchronize_learning(LearningSyncInput {
        rules: vec![rule],
        candidates: vec![RuleSynthesisCandidate { rule_id: "rule_reset_probe".to_string(), module: Some(module), diagnostic: json!({}) }],
        reason: Some("pass5_reset_probe".to_string()),
        ..Default::default()
    }).unwrap();
    assert!(state.snapshot().unwrap().learning_state.initialized);
    assert!(!state.snapshot().unwrap().learning_state.modules.is_empty());
    let reset = state.reset_developmental_state().unwrap();
    assert!(!reset.learning_state.initialized);
    assert!(reset.learning_state.modules.is_empty());
    assert!(reset.module_catalog.is_empty());
    assert!(reset.capability_graph.producers.is_empty());
    assert!(reset.capability_graph.waiters.is_empty());
}


#[test]
fn pass_six_reteaching_same_source_replaces_obsolete_source_backed_state() {
    let state = NovaMindState::default();
    let common = |text: &str| TeachingSourceInput {
        source_id: Some("pass6_replace_source".to_string()),
        source_name: "Replace Lesson.txt".to_string(),
        text: text.to_string(),
        source_type: Some("learned".to_string()),
        kind_hint: Some("lesson".to_string()),
    };

    state.ingest_source(common("Concept: Marker\nDefinition: Marker means old value.\nGuidance: When evidence changes, reconsider the old value.")).unwrap();
    let first = state.snapshot().unwrap();
    assert!(first.semantic_memory.knowledge.claims.iter().any(|claim| claim.object.to_ascii_lowercase().contains("old value")));

    state.ingest_source(common("Concept: Marker\nDefinition: Marker means new value.\nGuidance: When evidence changes, reconsider the new value.")).unwrap();
    let second = state.snapshot().unwrap();
    assert!(second.semantic_memory.knowledge.claims.iter().any(|claim| claim.object.to_ascii_lowercase().contains("new value")));
    assert!(!second.semantic_memory.knowledge.claims.iter().any(|claim| claim.object.to_ascii_lowercase().contains("old value")));
    assert!(second.learning_state.rules.iter().filter(|rule| rule.source_id.as_deref() == Some("pass6_replace_source")).all(|rule| !rule.directive.to_ascii_lowercase().contains("old value")));
}


#[test]
fn pass_six_named_source_identity_survives_content_edits_without_frontend_id() {
    let state = NovaMindState::default();
    let input = |text: &str| TeachingSourceInput {
        source_id: None,
        source_name: "Stable Identity Lesson.txt".to_string(),
        text: text.to_string(),
        source_type: Some("learned".to_string()),
        kind_hint: Some("lesson".to_string()),
    };

    let first = state.ingest_source(input("Concept: Marker\nDefinition: Marker means first value.")).unwrap();
    let second = state.ingest_source(input("Concept: Marker\nDefinition: Marker means corrected value.")).unwrap();
    assert_eq!(first.source_id, second.source_id);
    assert!(!second.semantic_state.knowledge.claims.iter().any(|claim| claim.object.to_ascii_lowercase().contains("first value")));
    assert!(second.semantic_state.knowledge.claims.iter().any(|claim| claim.object.to_ascii_lowercase().contains("corrected value")));
}

#[test]
fn pass_six_restores_persisted_rust_semantic_state_opaquely() {
    let original = NovaMindState::default();
    original.ingest_source(TeachingSourceInput {
        source_id: Some("pass6_semantic_restore".to_string()),
        source_name: "Restore Lesson.txt".to_string(),
        text: "Concept: Velorin\nDefinition: A velorin is a restored marker.".to_string(),
        source_type: Some("learned".to_string()),
        kind_hint: Some("lesson".to_string()),
    }).unwrap();
    let persisted = original.snapshot().unwrap().semantic_memory;
    let last_learning = persisted.last_learning.as_ref().map(|report| report.source_id.clone());

    let fresh = NovaMindState::default();
    let receipt = fresh.import_semantic_memory(SemanticMemoryImportInput {
        persisted_state: Some(persisted.clone()),
        reason: Some("pass6_semantic_restore_test".to_string()),
        ..Default::default()
    }).unwrap();
    assert!(receipt.imported);
    assert_eq!(receipt.state.revision, persisted.revision);
    assert_eq!(receipt.state.last_learning.as_ref().map(|report| report.source_id.clone()), last_learning);
    assert!(receipt.state.knowledge.concepts.iter().any(|concept| concept.key == "velorin"));
}

#[test]
fn pass_six_restores_persisted_rust_learning_state_then_revalidates_it() {
    let original = NovaMindState::default();
    let rule = pass_five_rule("pass6_restore_rule");
    let module = pass_five_candidate_module("pass6_restore_module", "set_field");
    original.synchronize_learning(LearningSyncInput {
        rules: vec![rule],
        candidates: vec![RuleSynthesisCandidate { rule_id: "pass6_restore_rule".to_string(), module: Some(module), diagnostic: json!({}) }],
        reason: Some("pass6_seed_restore_test".to_string()),
        ..Default::default()
    }).unwrap();
    let persisted = original.snapshot().unwrap().learning_state;

    let fresh = NovaMindState::default();
    let receipt = fresh.synchronize_learning(LearningSyncInput {
        persisted_state: Some(persisted.clone()),
        reason: Some("pass6_restore_rust_learning_test".to_string()),
        ..Default::default()
    }).unwrap();
    assert!(receipt.state.initialized);
    assert!(receipt.state.revision > persisted.revision);
    assert!(receipt.state.rules.iter().any(|rule| rule.id == "pass6_restore_rule" && rule.status == "module_executable"));
    assert!(receipt.state.modules.iter().any(|module| module.id.0 == "pass6_restore_module"));
    assert!(receipt.state.candidates.iter().any(|candidate| candidate.rule_id == "pass6_restore_rule"));
}

#[test]
fn pass_six_test_sources_are_rejected_before_learning() {
    let state = NovaMindState::default();
    let result = state.ingest_source(TeachingSourceInput {
        source_id: Some("pass6_test_source".to_string()),
        source_name: "Test Lesson 1.txt".to_string(),
        text: "Question: What is a lesson?\nExpected Answer: A lesson is teaching.".to_string(),
        source_type: Some("learned".to_string()),
        kind_hint: Some("test".to_string()),
    });
    assert!(result.is_err());
    let mind = state.snapshot().unwrap();
    assert!(!mind.semantic_memory.initialized);
    assert!(!mind.learning_state.initialized);
}

#[test]
fn pass_six_legacy_semantic_import_normalizes_named_source_identity_and_evidence() {
    use crate::nova::semantics::knowledge::SemanticSourceRecord;

    let old_source_id = "source_js_generated_old";
    let stable_source_id = crate::nova::ids::stable_source_id("Legacy Lesson.txt", "");
    let state = NovaMindState::default();
    let receipt = state.import_semantic_memory(SemanticMemoryImportInput {
        knowledge: KnowledgeSnapshot {
            concepts: vec![KnowledgeConcept {
                id: ConceptId("legacy_marker".to_string()),
                key: "legacy marker".to_string(),
                name: "legacy marker".to_string(),
                subject_eligible: true,
                instructional: true,
                evidence: vec![json!({"sourceId":old_source_id,"role":"concept"})],
                ..Default::default()
            }],
            source_revision: Some(4),
            ..Default::default()
        },
        sources: vec![SemanticSourceRecord {
            id: old_source_id.to_string(),
            name: "Legacy Lesson.txt".to_string(),
            display_name: "Lesson".to_string(),
            source_type: "learned".to_string(),
            source_role: "teaching".to_string(),
            document_kind: "lesson".to_string(),
            text: "Concept: Legacy marker".to_string(),
            learned_at: 100,
            ..Default::default()
        }],
        source_revision: Some(4),
        reason: Some("pass6_legacy_semantic_identity_test".to_string()),
        ..Default::default()
    }).unwrap();

    assert!(receipt.imported);
    assert_eq!(receipt.state.sources.len(), 1);
    assert_eq!(receipt.state.sources[0].id, stable_source_id);
    assert_eq!(receipt.state.learned_source_ids, vec![stable_source_id.clone()]);
    assert_eq!(
        receipt.state.knowledge.concepts[0].evidence[0].get("sourceId").and_then(|value| value.as_str()),
        Some(stable_source_id.as_str())
    );
}

#[test]
fn pass_six_legacy_learning_import_normalizes_named_source_identity_before_synthesis() {
    let old_source_id = "source_js_generated_learning";
    let expected_source_id = crate::nova::ids::stable_source_id("Legacy Learning Lesson.txt", "");
    let state = NovaMindState::default();

    let mut rule = pass_five_rule("legacy_learning_rule");
    rule.source_name = "Legacy Learning Lesson.txt".to_string();
    rule.source_id = Some(old_source_id.to_string());
    let mut module = pass_five_candidate_module("legacy_learning_module", "set_field");
    module.source_name = "Legacy Learning Lesson.txt".to_string();
    module.source_id = Some(old_source_id.to_string());

    let receipt = state.synchronize_learning(LearningSyncInput {
        rules: vec![rule],
        candidates: vec![RuleSynthesisCandidate {
            rule_id: "legacy_learning_rule".to_string(),
            module: Some(module),
            diagnostic: json!({}),
        }],
        reason: Some("pass6_legacy_learning_identity_test".to_string()),
        ..Default::default()
    }).unwrap();

    let learned_rule = receipt.state.rules.iter().find(|row| row.id == "legacy_learning_rule").expect("legacy rule");
    let learned_module = receipt.state.modules.iter().find(|row| row.id.0 == "legacy_learning_module").expect("legacy module");
    let retained_candidate = receipt.state.candidates.iter().find(|row| row.rule_id == "legacy_learning_rule").expect("legacy candidate");
    assert_eq!(learned_rule.source_id.as_deref(), Some(expected_source_id.as_str()));
    assert_eq!(learned_module.source_id.as_deref(), Some(expected_source_id.as_str()));
    assert_eq!(retained_candidate.module.as_ref().and_then(|row| row.source_id.as_deref()), Some(expected_source_id.as_str()));
}

#[test]
fn pass_six_legacy_conversation_import_preserves_explicit_event_source_identity() {
    use crate::nova::semantics::knowledge::SemanticSourceRecord;

    let event_source_id = "conversation_legacy_event_42";
    let semantic_state = NovaMindState::default();
    let semantic_receipt = semantic_state.import_semantic_memory(SemanticMemoryImportInput {
        knowledge: KnowledgeSnapshot {
            concepts: vec![KnowledgeConcept {
                id: ConceptId("conversation_marker".to_string()),
                key: "conversation marker".to_string(),
                name: "conversation marker".to_string(),
                subject_eligible: true,
                instructional: true,
                evidence: vec![json!({"sourceId":event_source_id,"role":"conversation"})],
                ..Default::default()
            }],
            ..Default::default()
        },
        sources: vec![SemanticSourceRecord {
            id: event_source_id.to_string(),
            name: "conversation".to_string(),
            display_name: "Conversation".to_string(),
            source_type: "conversation".to_string(),
            source_role: "instruction".to_string(),
            document_kind: "conversation".to_string(),
            text: "A single legacy conversation teaching event.".to_string(),
            learned_at: 42,
            ..Default::default()
        }],
        reason: Some("pass6_legacy_conversation_semantic_identity_test".to_string()),
        ..Default::default()
    }).unwrap();
    assert_eq!(semantic_receipt.state.sources[0].id, event_source_id);
    assert_eq!(
        semantic_receipt.state.knowledge.concepts[0].evidence[0].get("sourceId").and_then(|value| value.as_str()),
        Some(event_source_id)
    );

    let learning_state = NovaMindState::default();
    let mut rule = pass_five_rule("legacy_conversation_rule");
    rule.source_name = "conversation".to_string();
    rule.source_type = "conversation".to_string();
    rule.source_id = Some(event_source_id.to_string());
    let mut module = pass_five_candidate_module("legacy_conversation_module", "set_field");
    module.source_name = "conversation".to_string();
    module.source_type = "conversation".to_string();
    module.source_id = Some(event_source_id.to_string());
    let learning_receipt = learning_state.synchronize_learning(LearningSyncInput {
        rules: vec![rule],
        candidates: vec![RuleSynthesisCandidate {
            rule_id: "legacy_conversation_rule".to_string(),
            module: Some(module),
            diagnostic: json!({}),
        }],
        reason: Some("pass6_legacy_conversation_learning_identity_test".to_string()),
        ..Default::default()
    }).unwrap();
    assert_eq!(
        learning_receipt.state.rules.iter().find(|row| row.id == "legacy_conversation_rule").and_then(|row| row.source_id.as_deref()),
        Some(event_source_id)
    );
    assert_eq!(
        learning_receipt.state.modules.iter().find(|row| row.id.0 == "legacy_conversation_module").and_then(|row| row.source_id.as_deref()),
        Some(event_source_id)
    );
}


#[test]
fn pass_seven_rust_reflects_successful_turns_into_experience_routes_and_episodes() {
    let state = NovaMindState::default();
    let rule = pass_five_rule("pass7_route_rule");
    let mut module = pass_five_candidate_module("pass7_route_module", "set_field");
    module.module_number = Some(7001);
    module.conditions = vec![json!({"type":"always"})];
    module.program = vec![json!({"op":"set_field","args":{"path":"learnedState.pass7Probe","value":true}}), json!({"op":"emit_surface","args":{"text":"pass7 route ok"}})];
    state.synchronize_learning(LearningSyncInput { rules: vec![rule], candidates: vec![RuleSynthesisCandidate { rule_id:"pass7_route_rule".to_string(), module:Some(module), diagnostic:json!({}) }], ..Default::default() }).unwrap();
    let turn = state.begin_turn(BeginTurnInput { text:"route reflection probe".to_string(), ..Default::default() }).unwrap();
    assert_eq!(turn.cognitive_report.surface_response.as_deref(), Some("pass7 route ok"));
    state.finish_turn(FinishTurnInput { turn_id:turn.turn_id, response_produced:true }).unwrap();
    let mind=state.snapshot().unwrap();
    assert!(mind.development_state.initialized);
    assert!(mind.development_state.episodes.iter().any(|e|e.supported));
    assert!(mind.learning_state.experience_routes.iter().any(|r|r.module_numbers.contains(&7001) && r.successes>=1));
}

#[test]
fn pass_seven_failed_experienced_route_decays_in_rust_without_js_reflection() {
    let state=NovaMindState::default();
    let mut persisted=RustLearningState::default();persisted.initialized=true;persisted.schema_version=7;let mut route_module=pass_five_candidate_module("route_decay_module","set_field");route_module.module_number=Some(9999);route_module.conditions=vec![json!({"type":"contains_any","values":["never_match_decay"]})];persisted.modules=vec![route_module];persisted.experience_routes=vec![ExperienceRoute{id:"route_decay_probe".to_string(),fallback_terms:vec!["decay".to_string()],module_numbers:vec![9999],confidence:0.55,successes:2,..Default::default()}];
    state.synchronize_learning(LearningSyncInput{persisted_state:Some(persisted),..Default::default()}).unwrap();
    let turn=state.begin_turn(BeginTurnInput{text:"decay route".to_string(),..Default::default()}).unwrap();
    state.finish_turn(FinishTurnInput{turn_id:turn.turn_id,response_produced:false}).unwrap();
    let mind=state.snapshot().unwrap();let route=mind.learning_state.experience_routes.iter().find(|r|r.id=="route_decay_probe").unwrap();
    assert!(route.failures>=1);assert!(route.confidence<0.55);
}

#[test]
fn pass_seven_category_operations_mutate_rust_developmental_graph() {
    let state=NovaMindState::default();
    let mut rule=pass_five_rule("pass7_category_rule");rule.status="stored_only".to_string();
    let mut module=pass_five_candidate_module("pass7_category_module","register_category");module.module_number=Some(7002);module.conditions=vec![json!({"type":"always"})];module.program=vec![json!({"op":"register_category","args":{"name":"novel class","origin":"learned"}}),json!({"op":"relate_categories","args":{"subject":"novel class","predicate":"kind_of","object":"learned class"}}),json!({"op":"emit_surface","args":{"text":"category ok"}})];
    let receipt=state.synchronize_learning(LearningSyncInput{rules:vec![rule],candidates:vec![RuleSynthesisCandidate{rule_id:"pass7_category_rule".to_string(),module:Some(module),diagnostic:json!({})}],..Default::default()}).unwrap();
    assert_eq!(receipt.report.executable,1);
    let turn=state.begin_turn(BeginTurnInput{text:"category mutation probe".to_string(),..Default::default()}).unwrap();
    assert_eq!(turn.cognitive_report.surface_response.as_deref(),Some("category ok"));
    let mind=state.snapshot().unwrap();
    assert!(mind.development_state.categories.iter().any(|c|c.key=="novel class"));
    assert!(mind.development_state.relations.iter().any(|r|r.subject=="novel class"&&r.predicate=="kind of"&&r.object=="learned class"));
}

#[test]
fn pass_seven_development_import_is_one_time_and_rust_authoritative_afterward() {
    use crate::nova::development::{DevelopmentCategory,DevelopmentImportInput};
    let state=NovaMindState::default();
    let first=state.import_development(DevelopmentImportInput{legacy_categories:vec![DevelopmentCategory{id:"legacy_cat".into(),name:"Legacy Cat".into(),key:"legacy cat".into(),origin:"legacy_js_import".into(),status:"ACTIVE".into(),support:1,..Default::default()}],reason:Some("pass7_test".into()),..Default::default()}).unwrap();
    assert!(first.imported);assert!(first.state.categories.iter().any(|c|c.key=="legacy cat"));
    let second=state.import_development(DevelopmentImportInput{legacy_categories:vec![DevelopmentCategory{id:"intruder".into(),name:"JS Intruder".into(),key:"js intruder".into(),..Default::default()}],..Default::default()}).unwrap();
    assert!(!second.imported);assert!(!second.state.categories.iter().any(|c|c.key=="js intruder"));
}


#[test]
fn pass_eight_rust_realization_owns_surface_and_terminal_meaning() {
    let state = NovaMindState::default();
    let receipt = state.begin_turn(BeginTurnInput {
        text: "What is a lesson?".to_string(),
        knowledge: Some(lesson_knowledge()),
        ..Default::default()
    }).unwrap();
    assert_eq!(receipt.realization_report.authority, "rust_v7_pass_9");
    assert_eq!(receipt.realization_report.terminal_state, "SUPPORTED");
    assert!(receipt.realization_report.surface.as_deref().unwrap_or("").contains("organized information"));
}

#[test]
fn pass_eight_rust_classifies_structured_conversation_teaching_without_js_detector() {
    let state = NovaMindState::default();
    let receipt = state.begin_turn(BeginTurnInput {
        text: "Concept: Velorin\nDefinition: A velorin is a signal marker.".to_string(),
        source_label: Some("User".to_string()),
        ..Default::default()
    }).unwrap();
    assert_eq!(receipt.event_kind, "teaching");
    assert_eq!(receipt.realization_report.authority, "rust_v7_pass_9");
}

#[test]
fn pass_eight_learning_creates_and_attempts_a_rust_internal_goal() {
    let state = NovaMindState::default();
    state.ingest_source(TeachingSourceInput {
        source_id: Some("pass8_internal_goal_lesson".to_string()),
        source_name: "Internal Goal Lesson.txt".to_string(),
        text: "Concept: Velorin\nDefinition: A velorin is a signal marker.".to_string(),
        source_type: Some("learned".to_string()),
        kind_hint: Some("lesson".to_string()),
    }).unwrap();
    let mind = state.snapshot().unwrap();
    assert!(mind.internal_cognition.initialized);
    assert!(!mind.internal_cognition.goals.is_empty());
    assert!(mind.internal_cognition.goals.iter().any(|goal| goal.source == "learning" && goal.attempts >= 1));
    assert!(!mind.internal_cognition.history.is_empty());
}

#[test]
fn pass_nine_rust_owns_complete_mind_and_conversation_history() {
    let state = NovaMindState::default();
    let receipt = state.begin_turn(BeginTurnInput {
        text: "What is a lesson?".to_string(),
        knowledge: Some(lesson_knowledge()),
        ..Default::default()
    }).unwrap();
    state.finish_turn(FinishTurnInput { turn_id: receipt.turn_id, response_produced: true, ..Default::default() }).unwrap();
    let mind = state.snapshot().unwrap();
    assert_eq!(mind.migration_phase, crate::nova::mind::MigrationPhase::RustAuthoritative);
    assert_eq!(mind.subsystem_ownership.owners.get("persistent_mind_payload_orchestration").map(String::as_str), Some("rust_v7_pass_9"));
    assert_eq!(mind.subsystem_ownership.owners.get("conversation_history_authority").map(String::as_str), Some("rust_v7_pass_9"));
    assert!(mind.conversation_history.iter().any(|turn| turn.role == "user"));
    assert!(mind.conversation_history.iter().any(|turn| turn.role == "assistant"));
}

#[test]
fn alpha_9_2_teacher_feedback_preserves_original_answer_and_stays_non_executable() {
    use crate::nova::teacher::{FinalizeTeacherRunInput, RecordTeacherAttemptInput, TeacherSpanAnnotation, UpdateTeacherFeedbackInput};

    let mut mind = NovaMind::default();
    let rules_before = mind.learning_state.rules.len();
    let recorded = mind.record_teacher_attempt(RecordTeacherAttemptInput {
        run_id: "run_1".to_string(),
        suite_id: "suite_1".to_string(),
        case_id: "case_1".to_string(),
        source_display_name: "Test Lesson 1".to_string(),
        lesson_context: Some("Lesson 1".to_string()),
        question: "What is a lesson?".to_string(),
        original_answer: "A lesson is information.".to_string(),
        expected_answer: Some("Organized information intended to change understanding or knowledge.".to_string()),
        expected_behavior: None,
        automatic_status: "review".to_string(),
        automatic_reason: "Automatic grader requested teacher review.".to_string(),
        terminal_state: Some("SUPPORTED".to_string()),
        reason_code: Some("GOAL_SATISFIED".to_string()),
        provenance: json!({"runId":"run_1","source":"regression"}),
    }).unwrap();

    let original_answer = recorded.record.original_answer.clone();
    let attempt_id = recorded.record.id.clone();
    assert!(mind.semantic_memory.knowledge.examples.iter().all(|row| row.id != recorded.knowledge_id), "answer-key evidence must stay hidden until the full test run finishes");
    mind.finalize_teacher_run(FinalizeTeacherRunInput { run_id: "run_1".to_string() }).unwrap();
    let updated = mind.update_teacher_feedback(UpdateTeacherFeedbackInput {
        attempt_id,
        teacher_credit_percent: Some(62.5),
        teacher_points_awarded: Some(0.625),
        teacher_points_possible: Some(1.0),
        teacher_grade: Some("Partial".to_string()),
        teacher_notes: "The definition is partly correct but incomplete.".to_string(),
        annotations: vec![TeacherSpanAnnotation {
            id: String::new(),
            target: "answer".to_string(),
            start: 2,
            end: 8,
            style: "highlight".to_string(),
            note: None,
            created_at: 0,
        }],
    }).unwrap();

    assert_eq!(updated.record.original_answer, original_answer);
    assert_eq!(updated.record.teacher_credit_percent, Some(62.5));
    assert_eq!(mind.learning_state.rules.len(), rules_before, "teacher feedback must not synthesize executable cognition");
    let knowledge = mind.semantic_memory.knowledge.examples.iter().find(|row| row.id == updated.knowledge_id).unwrap();
    assert!(knowledge.content.contains("Teacher feedback and test history"));
    assert!(knowledge.content.contains("Teacher highlight marked answer text: \"lesson\"."));
    assert!(knowledge.evidence.iter().any(|value| value.get("nonExecutable").and_then(|v| v.as_bool()) == Some(true)));
    let ranked_item = mind.semantic_memory.knowledge.evidence_items().into_iter().find(|row| row.id == updated.knowledge_id).unwrap();
    assert_eq!(ranked_item.kind, "teacher_feedback");
    assert_eq!(ranked_item.attributes.get("nonExecutable").and_then(|value| value.as_bool()), Some(true));
}

#[test]
fn alpha_9_2_teacher_annotations_are_validated_against_immutable_text() {
    use crate::nova::teacher::{RecordTeacherAttemptInput, TeacherSpanAnnotation, UpdateTeacherFeedbackInput};

    let mut mind = NovaMind::default();
    let recorded = mind.record_teacher_attempt(RecordTeacherAttemptInput {
        run_id: "run_2".to_string(),
        suite_id: "suite_2".to_string(),
        case_id: "case_2".to_string(),
        source_display_name: "Test".to_string(),
        question: "Who opened the red box?".to_string(),
        original_answer: "Mira".to_string(),
        automatic_status: "pass".to_string(),
        ..Default::default()
    }).unwrap();

    let error = mind.update_teacher_feedback(UpdateTeacherFeedbackInput {
        attempt_id: recorded.record.id,
        teacher_credit_percent: Some(100.0),
        teacher_points_awarded: Some(1.0),
        teacher_points_possible: Some(1.0),
        teacher_grade: None,
        teacher_notes: String::new(),
        annotations: vec![TeacherSpanAnnotation {
            id: String::new(),
            target: "answer".to_string(),
            start: 0,
            end: 99,
            style: "underline".to_string(),
            note: None,
            created_at: 0,
        }],
    }).unwrap_err();
    assert!(error.contains("outside the preserved answer text"));
}

#[test]
fn alpha_9_2_test_answer_key_is_not_semantic_evidence_until_run_finalization() {
    use crate::nova::teacher::{FinalizeTeacherRunInput, RecordTeacherAttemptInput};

    let mut mind = NovaMind::default();
    let first = mind.record_teacher_attempt(RecordTeacherAttemptInput {
        run_id: "run_no_leak".to_string(),
        suite_id: "suite_no_leak".to_string(),
        case_id: "q1".to_string(),
        source_display_name: "No Leak Test".to_string(),
        question: "Question one?".to_string(),
        original_answer: "Attempt one".to_string(),
        expected_answer: Some("SECRET ANSWER ONE".to_string()),
        automatic_status: "review".to_string(),
        ..Default::default()
    }).unwrap();

    assert!(mind.semantic_memory.knowledge.examples.iter().all(|row| row.id != first.knowledge_id));
    let before = mind.semantic_memory.knowledge.rank_evidence("SECRET ANSWER ONE", &[], None, 20);
    assert!(before.iter().all(|row| row.id != first.knowledge_id));

    let finalized = mind.finalize_teacher_run(FinalizeTeacherRunInput { run_id: "run_no_leak".to_string() }).unwrap();
    assert_eq!(finalized.released_count, 1);
    let after = mind.semantic_memory.knowledge.rank_evidence("SECRET ANSWER ONE", &[], None, 20);
    assert!(after.iter().any(|row| row.id == first.knowledge_id));
}

#[test]
fn alpha_9_2_declares_teacher_feedback_as_rust_authoritative_without_a_new_reasoner() {
    let mind = NovaMind::default();
    assert_eq!(mind.subsystem_ownership.owners.get("teacher_test_history").map(String::as_str), Some("rust_v7_pass_9"));
    assert_eq!(mind.subsystem_ownership.owners.get("teacher_test_evidence_release").map(String::as_str), Some("rust_v7_pass_9"));
    assert_eq!(mind.subsystem_ownership.owners.get("teacher_feedback_knowledge").map(String::as_str), Some("rust_v7_pass_9"));
    assert_eq!(mind.reasoning_owner, "rust_v7_authoritative_mind");
}

#[test]
fn alpha_9_2_teacher_feedback_is_retrievable_as_evidence_for_ordinary_reflection_context() {
    use crate::nova::teacher::{FinalizeTeacherRunInput, RecordTeacherAttemptInput};

    let mut mind = NovaMind::default();
    let recorded = mind.record_teacher_attempt(RecordTeacherAttemptInput {
        run_id: "run_reflect".to_string(),
        suite_id: "suite_reflect".to_string(),
        case_id: "case_reflect".to_string(),
        source_display_name: "Reflection Test".to_string(),
        question: "What is a lesson?".to_string(),
        original_answer: "A lesson is information.".to_string(),
        expected_answer: Some("A lesson is organized teaching intended to change understanding or knowledge.".to_string()),
        automatic_status: "review".to_string(),
        automatic_reason: "Teacher review requested.".to_string(),
        ..Default::default()
    }).unwrap();
    assert!(mind.semantic_memory.knowledge.examples.iter().all(|row| row.id != recorded.knowledge_id));
    mind.finalize_teacher_run(FinalizeTeacherRunInput { run_id: "run_reflect".to_string() }).unwrap();

    let ranked = mind.semantic_memory.knowledge.rank_evidence(
        "Time to reflect on past tests and teacher feedback.",
        &[],
        None,
        12,
    );
    let evidence = ranked.iter().find(|row| row.id == recorded.knowledge_id).expect("teacher feedback should be retrievable through ordinary evidence ranking");
    assert_eq!(evidence.kind, "teacher_feedback");
    assert_eq!(evidence.attributes.get("nonExecutable").and_then(|value| value.as_bool()), Some(true));
}


#[test]
fn alpha_9_3_basic_english_bootstrap_preserves_simple_year_one_structure_without_school_grammar_labels() {
    let state = NovaMindState::default();
    let receipt = state.begin_turn(BeginTurnInput {
        text: "A person can answer a question.".to_string(),
        ..Default::default()
    }).unwrap();

    let english = &receipt.canonical_state.english_bootstrap;
    assert_eq!(english.utterance_kind, "statement");
    let clause = english.clauses.first().expect("basic English clause");
    assert_eq!(clause.left_text, "A person");
    assert_eq!(clause.operator_text.as_deref(), Some("can"));
    assert_eq!(clause.link_text, "answer");
    assert_eq!(clause.right_text, "a question");
    assert!(!clause.negative);

    let event = receipt.canonical_state.events.first().expect("structural event");
    assert_eq!(event.predicate_key, "answer");
    assert!(event.left_argument_id.is_some());
    assert!(event.right_argument_id.is_some());
    assert!(event.actor_id.is_none(), "bootstrap must not smuggle a learned actor/agent role into the mind");
    assert!(event.object_id.is_none(), "bootstrap must not smuggle a learned object/patient role into the mind");
    assert!(receipt.canonical_state.entities.iter().all(|entity| entity.kind == "discourse_referent" && entity.name.is_none()));

    let serialized = serde_json::to_string(english).unwrap().to_ascii_lowercase();
    for forbidden in ["\"noun\"", "\"pronoun\"", "proper_name", "proper name", "\"agent\"", "\"patient\"", "grammar_rule", "grammar rule"] {
        assert!(!serialized.contains(forbidden), "bootstrap leaked a school grammar concept: {forbidden}");
    }
}

#[test]
fn alpha_9_3_basic_english_distinguishes_simple_question_request_negation_coordination_and_context() {
    let question_state = NovaMindState::default();
    let question = question_state.begin_turn(BeginTurnInput {
        text: "Can a sentence talk about a person?".to_string(),
        ..Default::default()
    }).unwrap();
    assert_eq!(question.canonical_state.english_bootstrap.utterance_kind, "question");
    let question_clause = question.canonical_state.english_bootstrap.clauses.first().expect("question clause");
    assert_eq!(question_clause.left_text, "a sentence");
    assert_eq!(question_clause.operator_text.as_deref(), Some("Can"));
    assert_eq!(question_clause.link_text, "talk about");
    assert_eq!(question_clause.right_text, "a person");

    let ability_question_state = NovaMindState::default();
    let ability_question = ability_question_state.begin_turn(BeginTurnInput {
        text: "Can you know which box contains the coin?".to_string(),
        ..Default::default()
    }).unwrap();
    assert_eq!(ability_question.canonical_state.english_bootstrap.utterance_kind, "question", "ambiguous can-you wording must not be hardcoded as a request");

    let negative_state = NovaMindState::default();
    let negative = negative_state.begin_turn(BeginTurnInput {
        text: "A person is not a thing.".to_string(),
        ..Default::default()
    }).unwrap();
    assert!(negative.canonical_state.english_bootstrap.clauses.first().expect("negative clause").negative);
    assert!(negative.canonical_state.propositions.iter().any(|row| row.polarity == "negative"));

    let coordinated_state = NovaMindState::default();
    let coordinated = coordinated_state.begin_turn(BeginTurnInput {
        text: "Mira opened the red box and Tavik opened the blue box.".to_string(),
        ..Default::default()
    }).unwrap();
    assert!(coordinated.canonical_state.english_bootstrap.clauses.len() >= 2);
    assert!(coordinated.canonical_state.english_bootstrap.coordination_cues.iter().any(|cue| cue.eq_ignore_ascii_case("and")));
    let red_box = coordinated.canonical_state.english_bootstrap.phrases.iter()
        .find(|phrase| phrase.text.eq_ignore_ascii_case("the red box"))
        .expect("modifier-bearing phrase");
    assert_eq!(red_box.head.as_deref(), Some("box"));
    assert!(red_box.modifiers.iter().any(|value| value.eq_ignore_ascii_case("red")));
    assert!(!red_box.modifiers.iter().any(|value| value.eq_ignore_ascii_case("the")));

    let mut mind = NovaMind::default();
    mind.conversation_history.push(ConversationContextTurn {
        id: "previous_assistant".to_string(),
        role: "assistant".to_string(),
        source_label: Some("Nova".to_string()),
        content: "A sentence can carry a message.".to_string(),
        created_at: 1,
    });
    let request = mind.begin_turn(BeginTurnInput {
        text: "Can you say that in a more natural way?".to_string(),
        ..Default::default()
    }).unwrap();
    assert_eq!(request.canonical_state.english_bootstrap.utterance_kind, "request");
    let context = request.canonical_state.english_bootstrap.context_links.iter()
        .find(|link| link.surface.eq_ignore_ascii_case("that"))
        .expect("simple contextual reference");
    assert_eq!(context.referent_kind, "previous_utterance_candidate");
    assert_eq!(context.referent_id.as_deref(), Some("previous_assistant"));
}

#[test]
fn alpha_9_3_teacher_points_grade_span_notes_and_inspector_context_are_preserved_as_non_executable_evidence() {
    use crate::nova::teacher::{FinalizeTeacherRunInput, RecordTeacherAttemptInput, TeacherSpanAnnotation, UpdateTeacherFeedbackInput};

    let mut mind = NovaMind::default();
    let rules_before = mind.learning_state.rules.len();
    let question = "What is a decision?".to_string();
    let answer = "Decision is a decision someone makes.".to_string();
    let recorded = mind.record_teacher_attempt(RecordTeacherAttemptInput {
        run_id: "run_alpha93_teacher".to_string(),
        suite_id: "lesson_1_test".to_string(),
        case_id: "decision_q1".to_string(),
        source_display_name: "Lesson 1 Test".to_string(),
        lesson_context: Some("Lesson 1".to_string()),
        question: question.clone(),
        original_answer: answer.clone(),
        expected_answer: Some("A decision is a choice that someone makes.".to_string()),
        automatic_status: "review".to_string(),
        automatic_reason: "Teacher review requested.".to_string(),
        provenance: json!({"runId":"run_alpha93_teacher","lesson":"Lesson 1","provenance":"Teacher"}),
        ..Default::default()
    }).unwrap();
    assert_eq!(recorded.record.attempt_number, 1);
    assert!(recorded.record.inspector_context.get("semanticReport").is_some());
    assert!(mind.semantic_memory.knowledge.examples.iter().all(|row| row.id != recorded.knowledge_id));

    mind.finalize_teacher_run(FinalizeTeacherRunInput { run_id: "run_alpha93_teacher".to_string() }).unwrap();
    let updated = mind.update_teacher_feedback(UpdateTeacherFeedbackInput {
        attempt_id: recorded.record.id.clone(),
        teacher_credit_percent: None,
        teacher_points_awarded: Some(0.5),
        teacher_points_possible: Some(1.0),
        teacher_grade: Some("Developing".to_string()),
        teacher_notes: "The core idea is partly right, but the opening repeats the term instead of explaining it.".to_string(),
        annotations: vec![TeacherSpanAnnotation {
            id: String::new(),
            target: "answer".to_string(),
            start: 0,
            end: 11,
            style: "strike".to_string(),
            note: Some("No need to repeat the first word twice. This first part is unnecessary.".to_string()),
            created_at: 0,
        }],
    }).unwrap();

    assert_eq!(updated.record.question, question);
    assert_eq!(updated.record.original_answer, answer);
    assert_eq!(updated.record.teacher_points_awarded, Some(0.5));
    assert_eq!(updated.record.teacher_points_possible, Some(1.0));
    assert_eq!(updated.record.teacher_grade.as_deref(), Some("Developing"));
    assert_eq!(updated.record.effective_credit_percent(), Some(50.0));
    assert!(updated.record.is_partial_credit());
    assert_eq!(mind.learning_state.rules.len(), rules_before, "teacher evidence must not synthesize a direct behavior rule");

    let knowledge = mind.semantic_memory.knowledge.examples.iter()
        .find(|row| row.id == updated.knowledge_id)
        .expect("released teacher knowledge");
    assert!(knowledge.content.contains("Teacher score: 0.5/1"));
    assert!(knowledge.content.contains("Teacher grade: Developing"));
    assert!(knowledge.content.contains("Teacher strike marked answer text: \"Decision is\""));
    assert!(knowledge.content.contains("No need to repeat the first word twice"));
    assert!(knowledge.content.contains("non-executable knowledge"));
    assert!(knowledge.evidence.iter().any(|value| {
        value.get("provenanceLabel").and_then(|v| v.as_str()) == Some("Teacher")
            && value.get("nonExecutable").and_then(|v| v.as_bool()) == Some(true)
    }));
}

#[test]
fn alpha_9_3_teacher_rejects_conflicting_point_and_percent_grades() {
    use crate::nova::teacher::{RecordTeacherAttemptInput, UpdateTeacherFeedbackInput};

    let mut mind = NovaMind::default();
    let recorded = mind.record_teacher_attempt(RecordTeacherAttemptInput {
        run_id: "run_conflicting_grade".to_string(),
        suite_id: "suite_conflicting_grade".to_string(),
        case_id: "q1".to_string(),
        source_display_name: "Grade Consistency Test".to_string(),
        question: "A question?".to_string(),
        original_answer: "An answer.".to_string(),
        automatic_status: "review".to_string(),
        ..Default::default()
    }).unwrap();

    let error = mind.update_teacher_feedback(UpdateTeacherFeedbackInput {
        attempt_id: recorded.record.id,
        teacher_credit_percent: Some(75.0),
        teacher_points_awarded: Some(0.5),
        teacher_points_possible: Some(1.0),
        teacher_grade: None,
        teacher_notes: String::new(),
        annotations: Vec::new(),
    }).unwrap_err();
    assert!(error.contains("disagree"));
    let preserved = mind.teacher_state.attempts.first().expect("teacher attempt remains");
    assert!(preserved.teacher_credit_percent.is_none());
    assert!(preserved.teacher_points_awarded.is_none());
    assert!(preserved.teacher_points_possible.is_none());
}

#[test]
fn alpha_9_3_declares_bootstrap_and_teacher_refinements_as_rust_owned_without_changing_reasoning_owner() {
    let mind = NovaMind::default();
    for subsystem in [
        "basic_english_bootstrap",
        "teacher_partial_credit",
        "teacher_span_annotations",
        "teacher_span_notes",
        "teacher_reflection_evidence",
    ] {
        assert_eq!(
            mind.subsystem_ownership.owners.get(subsystem).map(String::as_str),
            Some("rust_v7_alpha_9_3")
        );
    }
    assert_eq!(mind.reasoning_owner, "rust_v7_authoritative_mind");
    assert_eq!(mind.migration_phase, MigrationPhase::RustAuthoritative);
}
