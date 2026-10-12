use std::collections::{BTreeMap, BTreeSet};

use serde_json::{json, Value};

use crate::nova::cognition::{validate_conditions, validate_program_operations, CognitiveModule, CognitiveModuleState, ExperienceRoute};
use crate::nova::ids::{source_has_named_identity, stable_source_id, ModuleId};
use crate::nova::mind::unix_ms;

use super::capabilities::{build_capability_graph, capability_counts, missing_candidate_capabilities};
use super::types::{
    LearnedRuleRecord, LearningSynthesisReport, LearningSyncInput, RuleSynthesisCandidate, RuleSynthesisResult,
    RustLearningState, SynthesisRoundReport,
};

const MAX_SYNTHESIS_ROUNDS: u32 = 8;

fn canon(value: &str) -> String { value.trim().to_ascii_lowercase().replace(' ', "_") }

fn module_signature(module: &CognitiveModule) -> String {
    let source = module.source_id.as_deref().unwrap_or(&module.source_name);
    let conditions = serde_json::to_string(&module.conditions).unwrap_or_default();
    let program = serde_json::to_string(&module.program).unwrap_or_default();
    format!("{}|{}|{}|{}|{}|{}", canon(source), canon(&module.name), canon(&module.domain), canon(&module.phase), conditions, program)
}

fn merge_modules(existing: Vec<CognitiveModule>) -> Vec<CognitiveModule> {
    let mut by_id = BTreeMap::new();
    let mut by_signature = BTreeSet::new();
    for mut module in existing {
        if module.id.0.trim().is_empty() { continue; }
        if module.domain.trim().is_empty() { module.domain = "reasoning".to_string(); }
        if module.phase.trim().is_empty() { module.phase = module.domain.clone(); }
        let signature = module_signature(&module);
        if by_signature.insert(signature) { by_id.insert(module.id.0.clone(), module); }
    }
    by_id.into_values().collect()
}

fn merge_rules(prior: &[LearnedRuleRecord], incoming: Vec<LearnedRuleRecord>) -> Vec<LearnedRuleRecord> {
    let mut by_id: BTreeMap<String, LearnedRuleRecord> = prior.iter().cloned().map(|rule| (rule.id.clone(), rule)).collect();
    for rule in incoming {
        if rule.id.trim().is_empty() { continue; }
        by_id.insert(rule.id.clone(), rule);
    }
    by_id.into_values().collect()
}

fn merge_candidates(prior: &[RuleSynthesisCandidate], incoming: Vec<RuleSynthesisCandidate>) -> Vec<RuleSynthesisCandidate> {
    let mut by_rule: BTreeMap<String, RuleSynthesisCandidate> = prior.iter().cloned().map(|row| (row.rule_id.clone(), row)).collect();
    for row in incoming {
        if row.rule_id.trim().is_empty() { continue; }
        by_rule.insert(row.rule_id.clone(), row);
    }
    by_rule.into_values().collect()
}

fn candidate_map(candidates: &[RuleSynthesisCandidate]) -> BTreeMap<String, RuleSynthesisCandidate> {
    candidates.iter().cloned().filter(|row| !row.rule_id.trim().is_empty()).map(|row| (row.rule_id.clone(), row)).collect()
}

fn legacy_named_source_id(source_name: &str, current: Option<&str>) -> Option<String> {
    if !source_has_named_identity(source_name) {
        return current.filter(|value| !value.trim().is_empty()).map(str::to_string);
    }
    Some(stable_source_id(source_name, ""))
}

fn normalize_legacy_learning_source_identity(
    rules: &mut [LearnedRuleRecord],
    modules: &mut [CognitiveModule],
    candidates: &mut [RuleSynthesisCandidate],
    source_candidates: &mut [CognitiveModule],
) {
    for rule in rules {
        rule.source_id = legacy_named_source_id(&rule.source_name, rule.source_id.as_deref());
    }
    for module in modules {
        module.source_id = legacy_named_source_id(&module.source_name, module.source_id.as_deref());
    }
    for candidate in candidates {
        if let Some(module) = candidate.module.as_mut() {
            module.source_id = legacy_named_source_id(&module.source_name, module.source_id.as_deref());
        }
    }
    for module in source_candidates {
        module.source_id = legacy_named_source_id(&module.source_name, module.source_id.as_deref());
    }
}

fn diagnostic_list(value: &Value, key: &str) -> Vec<String> {
    value.get(key).and_then(Value::as_array).map(|items| {
        items.iter().filter_map(Value::as_str).map(canon).filter(|item| !item.is_empty()).collect()
    }).unwrap_or_default()
}

fn stored_only_reason(rule: &LearnedRuleRecord, candidate: Option<&RuleSynthesisCandidate>, missing_ops: &[String], missing_caps: &[String]) -> String {
    if !missing_ops.is_empty() { return format!("unsupported_operations:{}", missing_ops.join(",")); }
    if !missing_caps.is_empty() { return format!("missing_capabilities:{}", missing_caps.join(",")); }
    if let Some(reason) = candidate.and_then(|row| row.diagnostic.get("reason")).and_then(Value::as_str).filter(|text| !text.trim().is_empty()) { return reason.to_string(); }
    if let Some(reason) = rule.synthesis.get("reason").and_then(Value::as_str).filter(|text| !text.trim().is_empty()) { return reason.to_string(); }
    "no_valid_current_rust_program_yet".to_string()
}

fn next_module_number(modules: &[CognitiveModule]) -> u64 {
    modules.iter().filter_map(|module| module.module_number).max().unwrap_or(0).saturating_add(1)
}

fn prepare_candidate(mut module: CognitiveModule, rule: &LearnedRuleRecord, next_number: u64) -> CognitiveModule {
    if module.id.0.trim().is_empty() { module.id = ModuleId(format!("rust_module_{}", rule.id)); }
    if module.module_number.is_none() { module.module_number = Some(next_number); }
    if module.name.trim().is_empty() { module.name = format!("Synthesized cognition for {}", rule.id); }
    if module.domain.trim().is_empty() { module.domain = if rule.domain.trim().is_empty() { "reasoning".to_string() } else { rule.domain.clone() }; }
    if module.phase.trim().is_empty() { module.phase = module.domain.clone(); }
    if module.categories.is_empty() { module.categories = rule.categories.clone(); }
    if module.source_type.trim().is_empty() { module.source_type = "learned".to_string(); }
    if module.source_name.trim().is_empty() { module.source_name = rule.source_name.clone(); }
    if module.source_id.is_none() { module.source_id = rule.source_id.clone(); }
    if module.cognitive_form.is_none() { module.cognitive_form = rule.cognitive_form.clone(); }
    module.enabled = true;
    module
}

fn prepare_source_candidate(mut module: CognitiveModule, next_number: u64) -> Option<CognitiveModule> {
    if module.id.0.trim().is_empty() { return None; }
    if module.module_number.is_none() { module.module_number = Some(next_number); }
    if module.name.trim().is_empty() { module.name = module.id.0.clone(); }
    if module.domain.trim().is_empty() { module.domain = "reasoning".to_string(); }
    if module.phase.trim().is_empty() { module.phase = module.domain.clone(); }
    if module.source_type.trim().is_empty() { module.source_type = "learned".to_string(); }
    if module.source_name.trim().is_empty() { module.source_name = "learned source construction".to_string(); }
    module.enabled = true;
    Some(module)
}

fn run_synthesis(
    mut rules: Vec<LearnedRuleRecord>,
    existing_modules: Vec<CognitiveModule>,
    retained_candidates: Vec<RuleSynthesisCandidate>,
    source_candidates: Vec<CognitiveModule>,
    experience_routes: Vec<ExperienceRoute>,
    source_revision: Option<u64>,
    reason: String,
    prior_revision: u64,
    imported_modules: usize,
) -> RustLearningState {
    let now = unix_ms();
    let source_candidate_modules = source_candidates.len();
    let mut rejected_imports = BTreeMap::<String, Vec<String>>::new();
    let mut modules = Vec::new();
    for module in merge_modules(existing_modules) {
        let mut unsupported = validate_program_operations(&module.program);
        unsupported.extend(validate_conditions(&module.conditions));
        unsupported.sort(); unsupported.dedup();
        if unsupported.is_empty() { modules.push(module); } else { rejected_imports.insert(module.id.0.clone(), unsupported); }
    }

    let mut adopted_source_modules = 0usize;
    let mut rejected_source_modules = 0usize;
    for source_candidate in source_candidates {
        let Some(source_candidate) = prepare_source_candidate(source_candidate, next_module_number(&modules)) else { rejected_source_modules += 1; continue; };
        let mut unsupported = validate_program_operations(&source_candidate.program);
        unsupported.extend(validate_conditions(&source_candidate.conditions));
        unsupported.sort(); unsupported.dedup();
        if !unsupported.is_empty() { rejected_source_modules += 1; continue; }
        let signature = module_signature(&source_candidate);
        if modules.iter().any(|module| module.id == source_candidate.id || module_signature(module) == signature) { continue; }
        modules.push(source_candidate); adopted_source_modules += 1;
    }

    let candidates = candidate_map(&retained_candidates);
    let current_module_ids: BTreeSet<String> = modules.iter().map(|module| module.id.0.clone()).collect();
    for rule in &mut rules {
        let referenced = rule.module_id.clone();
        let valid = referenced.as_ref().is_some_and(|id| current_module_ids.contains(id));
        if !valid {
            if let Some(id) = referenced.as_ref() {
                if let Some(missing) = rejected_imports.get(id) {
                    rule.synthesis = json!({"version":7,"status":"stored_only","authority":"rust_v7_pass_9","missingOperations":missing,"reason":"previous executable module is not executable in the current Rust substrate","attemptedAt":now});
                }
            }
            rule.status = "stored_only".to_string(); rule.module_id = None;
        } else { rule.status = "module_executable".to_string(); }
    }

    let mut results = BTreeMap::<String, RuleSynthesisResult>::new();
    let mut rounds = Vec::new();
    let mut total_upgraded = 0usize;

    for round in 1..=MAX_SYNTHESIS_ROUNDS {
        let graph = build_capability_graph(&modules, &rules);
        let round_modules = modules.clone();
        let mut round_upgraded = 0usize;
        let mut scanned = 0usize;
        for rule in &mut rules {
            if matches!(rule.status.as_str(), "module_executable" | "executable") { continue; }
            scanned += 1;
            let candidate = candidates.get(&rule.id);
            let Some(candidate_module) = candidate.and_then(|row| row.module.clone()) else {
                let mut missing_ops = diagnostic_list(&rule.synthesis, "missingOperations");
                if let Some(row) = candidate { missing_ops.extend(diagnostic_list(&row.diagnostic, "missingOperations")); }
                missing_ops.sort(); missing_ops.dedup();
                let mut missing_caps = diagnostic_list(&rule.synthesis, "missingCapabilities");
                if let Some(row) = candidate { missing_caps.extend(diagnostic_list(&row.diagnostic, "missingCapabilities")); }
                missing_caps.sort(); missing_caps.dedup();
                results.insert(rule.id.clone(), RuleSynthesisResult { rule_id: rule.id.clone(), status: "stored_only".to_string(), module_id: None, missing_capabilities: missing_caps.clone(), missing_operations: missing_ops.clone(), reason: Some(stored_only_reason(rule, candidate, &missing_ops, &missing_caps)) });
                continue;
            };

            let candidate_module = prepare_candidate(candidate_module, rule, next_module_number(&modules));
            let mut missing_ops = validate_program_operations(&candidate_module.program);
            missing_ops.extend(validate_conditions(&candidate_module.conditions));
            missing_ops.sort(); missing_ops.dedup();
            let missing_caps = missing_candidate_capabilities(&candidate_module, &graph, &round_modules);
            if !missing_ops.is_empty() || !missing_caps.is_empty() {
                results.insert(rule.id.clone(), RuleSynthesisResult { rule_id: rule.id.clone(), status: "stored_only".to_string(), module_id: None, missing_capabilities: missing_caps.clone(), missing_operations: missing_ops.clone(), reason: Some(stored_only_reason(rule, candidate, &missing_ops, &missing_caps)) });
                continue;
            }

            let candidate_signature = module_signature(&candidate_module);
            let existing_id = modules.iter().find(|module| module.id == candidate_module.id || module_signature(module) == candidate_signature).map(|module| module.id.0.clone());
            let module_id = if let Some(existing_id) = existing_id { existing_id } else { let id = candidate_module.id.0.clone(); modules.push(candidate_module); id };
            rule.status = "module_executable".to_string(); rule.module_id = Some(module_id.clone());
            rule.synthesis = json!({"version":7,"status":"synthesized","authority":"rust_v7_pass_9","moduleId":module_id,"reason":"validated Rust-generated declarative cognitive program in the current Rust substrate","attemptedAt":now});
            results.insert(rule.id.clone(), RuleSynthesisResult { rule_id: rule.id.clone(), status: "module_executable".to_string(), module_id: rule.module_id.clone(), missing_capabilities: Vec::new(), missing_operations: Vec::new(), reason: None });
            total_upgraded += 1; round_upgraded += 1;
        }
        rounds.push(SynthesisRoundReport { round, scanned, upgraded: round_upgraded, module_count: modules.len() });
        if round_upgraded == 0 { break; }
    }

    let graph = build_capability_graph(&modules, &rules);
    let (producer_capabilities, waiting_capabilities) = capability_counts(&graph);
    let executable = rules.iter().filter(|rule| matches!(rule.status.as_str(), "module_executable" | "executable")).count();
    let stored_only = rules.len().saturating_sub(executable);
    let report = LearningSynthesisReport {
        scanned: results.len(), upgraded: total_upgraded, executable, stored_only, imported_modules,
        rejected_imported_modules: rejected_imports.len(), candidate_modules: retained_candidates.iter().filter(|row| row.module.is_some()).count(), source_candidate_modules,
        adopted_source_modules, rejected_source_modules, producer_capabilities, waiting_capabilities, rounds, results: results.into_values().collect(), reason, ran_at: now,
    };
    let module_states = modules.iter().map(|module| (module.id.0.clone(), CognitiveModuleState::Executable)).collect();
    RustLearningState {
        schema_version: 2, initialized: true, revision: prior_revision.saturating_add(1), source_revision, rules, modules, candidates: retained_candidates,
        module_states, capability_graph: graph, experience_routes, last_synthesis: Some(report), updated_at: now,
    }
}

/// One-time migration/import boundary for the old JavaScript cognitive stores.
/// After Rust learning initializes, frontend rules/modules/candidates are not
/// allowed to replace the authoritative Rust catalog. Experience routes remain
/// frontend-owned temporarily and may still be mirrored here until their pass.
pub fn synchronize_learning(input: LearningSyncInput, prior: &RustLearningState) -> RustLearningState {
    if prior.initialized {
        let mut next = prior.clone();
        // Pass 7: initialized Rust learning state owns experience routes. Frontend sync
        // cannot replace learned routing history after authority has migrated.
        next.revision = next.revision.saturating_add(1);
        next.updated_at = unix_ms();
        let (producer_capabilities, waiting_capabilities) = capability_counts(&next.capability_graph);
        next.last_synthesis = Some(LearningSynthesisReport {
            scanned: 0, upgraded: 0,
            executable: next.rules.iter().filter(|rule| matches!(rule.status.as_str(), "module_executable" | "executable")).count(),
            stored_only: next.rules.iter().filter(|rule| !matches!(rule.status.as_str(), "module_executable" | "executable")).count(),
            imported_modules: 0, rejected_imported_modules: 0, candidate_modules: next.candidates.len(), source_candidate_modules: 0, adopted_source_modules: 0, rejected_source_modules: 0,
            producer_capabilities, waiting_capabilities, rounds: Vec::new(), results: Vec::new(),
            reason: input.reason.unwrap_or_else(|| "frontend_sync_after_rust_authority".to_string()), ran_at: unix_ms(),
        });
        return next;
    }

    if let Some(persisted) = input.persisted_state {
        // Pass 6 restart restoration: JavaScript transports the Rust state as an
        // opaque payload. Rust revalidates every executable module/candidate
        // against the current substrate before it becomes live again.
        let experience_routes = if input.experience_routes.is_empty() { persisted.experience_routes.clone() } else { input.experience_routes };
        let source_revision = input.source_revision.or(persisted.source_revision);
        let imported_modules = persisted.modules.len();
        return run_synthesis(
            persisted.rules, persisted.modules, persisted.candidates, Vec::new(), experience_routes, source_revision,
            input.reason.unwrap_or_else(|| "restore_persisted_rust_learning_state".to_string()), persisted.revision, imported_modules,
        );
    }

    let imported_modules = input.existing_modules.len();
    let mut rules = input.rules;
    let mut existing_modules = input.existing_modules;
    let mut candidates = input.candidates;
    let mut source_candidates = input.source_candidates;
    normalize_legacy_learning_source_identity(&mut rules, &mut existing_modules, &mut candidates, &mut source_candidates);
    run_synthesis(rules, existing_modules, candidates, source_candidates, input.experience_routes, input.source_revision, input.reason.unwrap_or_else(|| "one_time_legacy_learning_import".to_string()), prior.revision, imported_modules)
}

/// Authoritative Pass 7 learning/source replacement. Re-teaching the same source ID
/// removes that source's prior Rust-generated rules/modules/candidates before
/// integrating its current contents. This prevents obsolete teaching from
/// surviving merely because the new source changed wording or removed a rule.
pub fn replace_rust_teaching_source(
    prior: &RustLearningState,
    source_id: &str,
    new_rules: Vec<LearnedRuleRecord>,
    new_candidates: Vec<RuleSynthesisCandidate>,
    source_revision: Option<u64>,
    reason: String,
) -> RustLearningState {
    let removed_rule_ids: BTreeSet<String> = prior.rules.iter()
        .filter(|rule| rule.source_id.as_deref() == Some(source_id))
        .map(|rule| rule.id.clone())
        .collect();
    let retained_rules: Vec<LearnedRuleRecord> = prior.rules.iter()
        .filter(|rule| rule.source_id.as_deref() != Some(source_id))
        .cloned()
        .collect();
    let retained_candidates: Vec<RuleSynthesisCandidate> = prior.candidates.iter()
        .filter(|candidate| !removed_rule_ids.contains(&candidate.rule_id))
        .cloned()
        .collect();
    let retained_modules: Vec<CognitiveModule> = prior.modules.iter()
        .filter(|module| module.source_id.as_deref() != Some(source_id))
        .cloned()
        .collect();

    let rules = merge_rules(&retained_rules, new_rules);
    let candidates = merge_candidates(&retained_candidates, new_candidates);
    run_synthesis(
        rules,
        retained_modules,
        candidates,
        Vec::new(),
        prior.experience_routes.clone(),
        source_revision.or(prior.source_revision),
        reason,
        prior.revision,
        0,
    )
}

