mod core;
mod scopes;
mod learned_policy;
mod candidates;
mod policy;
mod semantic;
mod semantic_records;

use serde_json::Value;

use super::conditions::condition_matches;
use super::path::{canon, resolve_template};
use super::types::CognitiveModule;

pub fn execute_step(step: &Value, ctx: &mut Value, modules: &[CognitiveModule], depth: usize) -> Result<bool, String> {
    if depth > 12 { return Ok(false); }
    let op = canon(step.get("op").and_then(Value::as_str).unwrap_or("")).replace(' ', "_");
    let args = step.get("args").cloned().unwrap_or_else(|| Value::Object(Default::default()));
    match op.as_str() {
        "branch" => {
            let test = args.get("if").or_else(|| args.get("condition")).cloned().unwrap_or(Value::Null);
            let matched = condition_matches(&test, ctx)?;
            let program = if matched { args.get("then").or_else(|| step.get("then")) } else { args.get("else").or_else(|| step.get("else")) };
            execute_program(program.and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[]), ctx, modules, depth + 1)
        }
        "for_each" => core::for_each(&args, step, ctx, modules, depth),
        "invoke_module" => {
            let wanted = args.get("number").or_else(|| args.get("id")).or_else(|| args.get("name")).or_else(|| args.get("category")).map(|value| resolve_template(value, ctx)).unwrap_or(Value::Null);
            let wanted_text = super::path::value_text(Some(&wanted));
            let target = modules.iter().find(|module| {
                module.module_number.map(|n| n.to_string() == wanted_text.trim_start_matches('#')).unwrap_or(false)
                    || canon(&module.id.0) == canon(&wanted_text)
                    || canon(&module.name) == canon(&wanted_text)
                    || module.categories.iter().any(|category| canon(category) == canon(&wanted_text))
            });
            let Some(module) = target else { return Ok(false); };
            if !module.enabled { return Ok(false); }
            if !super::conditions::all_conditions_match(&module.conditions, ctx)? { return Ok(false); }
            execute_program(&module.program, ctx, modules, depth + 1)
        }
        _ => core::execute(&op, &args, ctx)
            .or_else(|error| if error == "not_core" { candidates::execute(&op, &args, ctx) } else { Err(error) })
            .or_else(|error| if error == "not_candidates" { scopes::execute(&op, &args, ctx) } else { Err(error) })
            .or_else(|error| if error == "not_scopes" { semantic::execute(&op, &args, ctx) } else { Err(error) })
            .or_else(|error| if error == "not_semantic" { policy::execute(&op, &args, ctx) } else { Err(error) })
            .or_else(|error| if error.starts_with("unsupported_operation:") { learned_policy::execute(&op, &args, ctx) } else { Err(error) })
            .or_else(|error| if error == "not_learned_policy" { Err(format!("unsupported_operation:{op}")) } else { Err(error) }),
    }
}

pub fn execute_program(program: &[Value], ctx: &mut Value, modules: &[CognitiveModule], depth: usize) -> Result<bool, String> {
    let mut changed = false;
    for step in program { changed = execute_step(step, ctx, modules, depth)? || changed; }
    Ok(changed)
}

/// Return whether a declarative cognitive operation belongs to the current Rust
/// mental instruction set. This is the synthesis/runtime contract: learning may
/// compose these operations, but a lesson cannot invent executable Rust code.
pub fn operation_supported(name: &str) -> bool {
    let op = canon(name).replace(' ', "_");
    if matches!(op.as_str(),
        "semantic_add_frame" | "semantic_add_entity" | "semantic_add_event" |
        "semantic_add_relation" | "semantic_add_proposition" | "semantic_add_request" |
        "semantic_add_unknown_slot" | "semantic_add_candidate_set" |
        "semantic_add_evidence" | "semantic_add_conclusion"
    ) {
        return true;
    }
    matches!(op.as_str(),
        "branch" | "for_each" | "invoke_module" |
        "copy_field" | "remove_field" | "compare_values" | "count_items" |
        "unique_list" | "intersect_list" | "preserve_field" | "restore_field" |
        "ensure_field_equals" | "set_field" | "append_field" |
        "collect_candidates" | "filter_candidates" | "score_candidates" |
        "measure_candidate_support" | "score_candidates_by_relationship" | "select_best_candidate" |
        "select_matching_item" | "select_preferred_available" |
        "mark_uncertain" | "mark_epistemic_state" | "assign_reference" |
        "resolve_participant" | "perspective_subject" | "second_person" |
        "create_structure" | "compose_structures" | "link_structures" | "query_structures" |
        "preserve_structure" | "restore_structure" | "preserve_ordered_items" |
        "scope_create_child" | "scope_set_active" | "scope_restore_parent" |
        "scope_bind_participant" | "scope_apply_policy" | "scope_enforce_isolation" | "scope_resolve_deictics" |
        "derive_semantic_binding" | "add_constraint" | "register_cognitive_constraint" |
        "create_goal" | "register_unresolved_goal" | "resolve_goal" | "resolve_unresolved_goal" |
        "knowledge_resolve_concept" | "knowledge_collect_relations" |
        "knowledge_select_relevant_relations" | "knowledge_form_supported_proposition" |
        "select_labeled_role" |
        "evaluate_support_conflict" | "summarize_evidence_support" |
        "retrieve_source_evidence" | "inspect_cognitive_status" | "require_preconditions" |
        "record_cognitive_attempt" | "compare_attempt_progress" |
        "register_evidence_relation" | "update_belief_state" |
        "declare_reasoning_policy" | "declare_stop_condition" |
        "declare_invariant" | "declare_ordering" | "assert_paths_present" |
        "declare_learned_predicate" | "evaluate_learned_predicate" |
        "declare_state_transition" | "apply_declared_transition" |
        "declare_preference_policy" | "declare_task_resource_policy" |
        "declare_fidelity_policy" | "declare_representation_invariant" |
        "check_representation_invariant" |
        "activate_category" | "register_category" | "relate_categories" | "emit_surface" | "ensure_state_container"
    )
}

/// Validate a learned program without executing it. Nested programs are checked
/// recursively so Stored Only remains truthful even for branch/loop programs.
pub fn validate_program_operations(program: &[Value]) -> Vec<String> {
    fn visit(step: &Value, missing: &mut Vec<String>) {
        let op = step.get("op").and_then(Value::as_str).unwrap_or("");
        let canonical = canon(op).replace(' ', "_");
        if canonical.is_empty() || !operation_supported(&canonical) {
            if !canonical.is_empty() { missing.push(canonical); }
        }
        for key in ["then", "else", "program"] {
            if let Some(items) = step.get(key).and_then(Value::as_array) {
                for child in items { visit(child, missing); }
            }
        }
        if let Some(args) = step.get("args") {
            for key in ["then", "else", "program"] {
                if let Some(items) = args.get(key).and_then(Value::as_array) {
                    for child in items { visit(child, missing); }
                }
            }
        }
    }
    let mut missing = Vec::new();
    for step in program { visit(step, &mut missing); }
    missing.sort();
    missing.dedup();
    missing
}
