use std::collections::{BTreeMap, BTreeSet};

use serde_json::{json, Value};

use crate::nova::semantics::{CanonicalCognitiveState, KnowledgeSnapshot};
use crate::nova::development::RustDevelopmentState;

use super::conditions::all_conditions_match;
use super::operations::execute_program;
use super::path::{append_path, canon, get_path, get_path_cloned, set_path, value_text};
use super::scheduler::{schedule, CognitiveSchedule};
use super::types::{CognitiveModule, CognitiveModuleRunRow, CognitiveRuntimeReport, CognitiveRuntimeSnapshot};

const MAX_COGNITIVE_CYCLES: u32 = 8;

fn state_fingerprint(ctx: &Value) -> String {
    serde_json::to_string(&json!({
        "canonicalSemanticState": get_path(ctx, "canonicalSemanticState"),
        "moduleState": get_path(ctx, "moduleState"),
        "learnedState": get_path(ctx, "learnedState"),
        "developmentalState": get_path(ctx, "developmentalState"),
        "epistemic": get_path(ctx, "canonicalSemanticState.epistemic")
    })).unwrap_or_default()
}

#[derive(Clone, Debug, Default)]
struct CapabilityContext {
    known: BTreeSet<String>,
    available: BTreeSet<String>,
}

fn module_outputs(module: &CognitiveModule) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for value in module.produces.iter().chain(module.capabilities.iter()) {
        let key = canon(&value.0);
        if !key.is_empty() && key != "unmapped_cognitive_state_transformation" { out.insert(key); }
    }
    if let Some(form) = &module.cognitive_form {
        let form = canon(form);
        if !form.is_empty() { out.insert(format!("cognitive_form:{form}")); }
    }
    out
}

/// `requires` historically contains both capability dependencies and ordinary
/// runtime/data preconditions.  Pass 4 must not reinterpret every English
/// requirement as a missing CPU primitive.  A requirement gates execution only
/// when it belongs to the capability vocabulary represented by producers,
/// waiters, or module outputs.  Ordinary state needs remain the responsibility
/// of module conditions and programs.
fn capability_context(snapshot: &CognitiveRuntimeSnapshot) -> CapabilityContext {
    let mut known = BTreeSet::new();
    for module in &snapshot.modules {
        known.extend(module_outputs(module));
    }
    for producer in &snapshot.capability_graph.producers {
        let key = canon(&producer.capability.0);
        if !key.is_empty() { known.insert(key); }
    }
    for waiter in &snapshot.capability_graph.waiters {
        let key = canon(&waiter.capability.0);
        if !key.is_empty() { known.insert(key); }
    }

    let mut available = BTreeSet::new();
    loop {
        let before = available.len();
        for module in &snapshot.modules {
            if !module.enabled { continue; }
            let blocked = module.requires.iter().any(|required| {
                let key = canon(&required.0);
                !key.is_empty() && known.contains(&key) && !available.contains(&key)
            });
            if !blocked {
                available.extend(module_outputs(module));
            }
        }
        if available.len() == before { break; }
    }

    CapabilityContext { known, available }
}

fn missing_requirements(module: &CognitiveModule, capabilities: &CapabilityContext) -> Vec<String> {
    module.requires.iter().filter_map(|required| {
        let key = canon(&required.0);
        if key.is_empty() || !capabilities.known.contains(&key) || capabilities.available.contains(&key) {
            None
        } else {
            Some(key)
        }
    }).collect()
}

fn awakenable_waiters(snapshot: &CognitiveRuntimeSnapshot, available: &BTreeSet<String>) -> Vec<String> {
    let mut needs_by_rule: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for waiter in &snapshot.capability_graph.waiters {
        let Some(rule_id) = waiter.rule_id.as_ref().filter(|value| !value.trim().is_empty()) else { continue; };
        let capability = canon(&waiter.capability.0);
        if capability.is_empty() { continue; }
        needs_by_rule.entry(rule_id.clone()).or_default().insert(capability);
    }
    needs_by_rule.into_iter()
        .filter(|(_, needs)| !needs.is_empty() && needs.iter().all(|need| available.contains(need)))
        .map(|(rule_id, _)| rule_id)
        .collect()
}

fn runtime_context(state: &CanonicalCognitiveState, knowledge: &KnowledgeSnapshot, snapshot: &CognitiveRuntimeSnapshot, development: &RustDevelopmentState) -> Value {
    let canonical = serde_json::to_value(state).unwrap_or_else(|_| json!({}));
    json!({
        "rawText": state.source_raw,
        "input": state.source_raw,
        "surface": state.surface.raw,
        "utteranceObservation": state.surface,
        "canonicalSemanticState": canonical,
        "activeCategories": state.active_categories,
        "moduleState": state.module_state,
        "learnedState": state.learned_state,
        "constraints": state.constraints,
        "knowledge": knowledge,
        "developmentalState": development,
        "cognitiveStatus": {
            "modules": snapshot.modules.len(),
            "capabilityProducers": snapshot.capability_graph.producers.len(),
            "capabilityWaiters": snapshot.capability_graph.waiters.len()
        },
        "captures": {},
        "patternCaptures": {}
    })
}

fn validate_and_commit(ctx: &Value, state: &mut CanonicalCognitiveState) -> Result<(), String> {
    let canonical = get_path_cloned(ctx, "canonicalSemanticState").ok_or_else(|| "runtime_missing_canonical_state".to_string())?;
    let mut next: CanonicalCognitiveState = serde_json::from_value(canonical).map_err(|error| format!("runtime_invalid_canonical_state:{error}"))?;
    next.module_state = get_path_cloned(ctx, "moduleState").unwrap_or_else(|| json!({}));
    next.learned_state = get_path_cloned(ctx, "learnedState").unwrap_or_else(|| json!({}));
    *state = next;
    Ok(())
}

fn expose_outcome(ctx: &mut Value) -> bool {
    if get_path(ctx, "canonicalSemanticState.cognitiveOutcome").is_some_and(|value| !value.is_null()) { return false; }
    let requests = get_path(ctx, "canonicalSemanticState.requests").and_then(Value::as_array).map_or(0, Vec::len);
    let unresolved = get_path(ctx, "canonicalSemanticState.unresolved").and_then(Value::as_array).map_or(0, Vec::len);
    let has_meaning = ["frames","entities","events","relations","propositions","requests","unknownSlots","candidateSets","structures","conclusions"]
        .iter().any(|bucket| get_path(ctx, &format!("canonicalSemanticState.{bucket}")).and_then(Value::as_array).is_some_and(|items| !items.is_empty()));
    let outcome = json!({
        "id":"current_cognitive_outcome",
        "type":"cognitive_outcome",
        "state": if has_meaning {"blocked_with_partial_meaning"} else {"interpretation_incomplete"},
        "meaningConstructed":has_meaning,
        "requestIdentified":requests>0,
        "unresolvedGoalCount":unresolved,
        "responseProduced":false,
        "source":"rust_cognitive_runtime_stall"
    });
    let mut changed = set_path(ctx, "canonicalSemanticState.cognitiveOutcome", outcome.clone());
    changed = append_path(ctx, "canonicalSemanticState.structures", outcome, true) || changed;
    for category in ["communication","realization","self_report"] {
        changed = append_path(ctx, "activeCategories", Value::String(category.to_string()), true) || changed;
        changed = append_path(ctx, "canonicalSemanticState.activeCategories", Value::String(category.to_string()), true) || changed;
    }
    changed
}

fn attention_weight(module: &CognitiveModule, schedule: &CognitiveSchedule<'_>) -> f32 {
    module.module_number.and_then(|number| schedule.route_strengths.get(&number)).copied().unwrap_or(0.0)
}

fn module_row(module: &CognitiveModule, schedule: &CognitiveSchedule<'_>, progress: bool, delta_score: u64, error: Option<String>) -> CognitiveModuleRunRow {
    CognitiveModuleRunRow {
        module_id: module.id.clone(),
        module_number: module.module_number,
        name: module.name.clone(),
        phase: module.phase.clone(),
        categories: module.categories.clone(),
        progress,
        delta_score,
        error,
        attention_weight: attention_weight(module, schedule),
    }
}


fn mention_specific(module: &CognitiveModule) -> bool {
    if canon(&module.domain) == "reference" || canon(&module.phase) == "reference" { return true; }
    serde_json::to_string(&(&module.conditions, &module.program))
        .map(|text| text.contains("mention.") || text.contains("\"mention\""))
        .unwrap_or(false)
}

fn mention_count(ctx: &Value) -> usize {
    get_path(ctx, "canonicalSemanticState.references").and_then(Value::as_array).map_or(0, Vec::len)
}

fn install_mention(ctx: &mut Value, index: usize) -> bool {
    let mention = get_path(ctx, "canonicalSemanticState.references").and_then(Value::as_array).and_then(|items| items.get(index)).cloned();
    mention.is_some_and(|mention| set_path(ctx, "mention", mention))
}

fn commit_mention(ctx: &mut Value, index: usize) -> bool {
    let Some(mention) = get_path_cloned(ctx, "mention") else { return false; };
    let mut references = get_path_cloned(ctx, "canonicalSemanticState.references").and_then(|value| value.as_array().cloned()).unwrap_or_default();
    if index >= references.len() { return false; }
    let changed = references[index] != mention;
    if changed {
        references[index] = mention;
        set_path(ctx, "canonicalSemanticState.references", Value::Array(references));
    }
    changed
}

fn merge_row(target: &mut BTreeMap<String, CognitiveModuleRunRow>, row: CognitiveModuleRunRow) {
    if let Some(existing) = target.get_mut(&row.module_id.0) {
        existing.progress = existing.progress || row.progress;
        existing.delta_score = existing.delta_score.saturating_add(row.delta_score);
        existing.attention_weight = existing.attention_weight.max(row.attention_weight);
        if row.error.is_some() { existing.error = row.error; }
    } else {
        target.insert(row.module_id.0.clone(), row);
    }
}

pub fn run_cognitive_runtime(
    state: &mut CanonicalCognitiveState,
    knowledge: &KnowledgeSnapshot,
    snapshot: &CognitiveRuntimeSnapshot,
    development: &mut RustDevelopmentState,
) -> CognitiveRuntimeReport {
    let mut report = CognitiveRuntimeReport { modules_available: snapshot.modules.len(), ..Default::default() };
    if snapshot.modules.is_empty() {
        report.stalled = true;
        report.search_mode = "empty_catalog".to_string();
        return report;
    }

    let capabilities = capability_context(snapshot);
    report.awakenable_stored_only_rule_ids = awakenable_waiters(snapshot, &capabilities.available);
    // Nested `invoke_module` calls must obey the same explicit capability gate as
    // top-level scheduling.  Give the declarative program executor only modules
    // whose known capability requirements are currently available; this avoids
    // a caller bypassing Stored Only/capability gating by invoking a blocked module.
    let invokable_modules: Vec<CognitiveModule> = snapshot.modules.iter()
        .filter(|module| module.enabled && missing_requirements(module, &capabilities).is_empty())
        .cloned()
        .collect();
    let mut ctx = runtime_context(state, knowledge, snapshot, development);
    let mut current_schedule = schedule(state, snapshot, false);
    report.search_mode = current_schedule.mode.clone();
    report.experience_route_ids = current_schedule.route_ids.clone();
    let began_with_experience = current_schedule.mode == "experience_route";

    let mut considered_ids = BTreeSet::new();
    let mut applied_by_id: BTreeMap<String, CognitiveModuleRunRow> = BTreeMap::new();
    let mut changed_by_id: BTreeMap<String, CognitiveModuleRunRow> = BTreeMap::new();
    let mut outcome_exposed = false;
    let mut last_fingerprint = state_fingerprint(&ctx);

    for cycle in 1..=MAX_COGNITIVE_CYCLES {
        report.cycles = cycle;
        let mut cycle_progress = 0u64;
        for module in &current_schedule.modules {
            considered_ids.insert(module.id.0.clone());
            let missing = missing_requirements(module, &capabilities);
            if !missing.is_empty() {
                report.missing_capabilities.extend(missing);
                continue;
            }

            let mention_indices: Vec<Option<usize>> = if mention_specific(module) {
                (0..mention_count(&ctx)).map(Some).collect()
            } else {
                vec![None]
            };
            for mention_index in mention_indices {
                let before_context = ctx.clone();
                if let Some(index) = mention_index { install_mention(&mut ctx, index); }
                let matches = match all_conditions_match(&module.conditions, &mut ctx) {
                    Ok(value) => value,
                    Err(error) => {
                        if let Some(name) = error.strip_prefix("unsupported_condition:") { report.unsupported_operations.push(format!("condition:{name}")); }
                        merge_row(&mut applied_by_id, module_row(module, &current_schedule, false, 0, Some(error)));
                        ctx = before_context;
                        continue;
                    }
                };
                if !matches { ctx = before_context; continue; }

                let before = state_fingerprint(&ctx);
                let execution = execute_program(&module.program, &mut ctx, &invokable_modules, 0);
                match execution {
                    Ok(progress_hint) => {
                        let mention_changed = mention_index.is_some_and(|index| commit_mention(&mut ctx, index));
                        if mention_index.is_some() { super::path::delete_path(&mut ctx, "mention"); }
                        let canonical_valid = get_path_cloned(&ctx, "canonicalSemanticState")
                            .and_then(|value| serde_json::from_value::<CanonicalCognitiveState>(value).ok())
                            .is_some();
                        if !canonical_valid {
                            ctx = before_context;
                            merge_row(&mut applied_by_id, module_row(module, &current_schedule, false, 0, Some("module_produced_invalid_canonical_state".to_string())));
                            continue;
                        }
                        let after = state_fingerprint(&ctx);
                        let changed = progress_hint || mention_changed || before != after;
                        let delta = if changed { 1 } else { 0 };
                        let row = module_row(module, &current_schedule, changed, delta, None);
                        merge_row(&mut applied_by_id, row.clone());
                        if changed {
                            merge_row(&mut changed_by_id, row);
                            cycle_progress += 1;
                        }
                    }
                    Err(error) => {
                        if let Some(name) = error.strip_prefix("unsupported_operation:") { report.unsupported_operations.push(name.to_string()); }
                        ctx = before_context;
                        merge_row(&mut applied_by_id, module_row(module, &current_schedule, false, 0, Some(error)));
                    }
                }
            }
        }

        report.progress_count += cycle_progress;
        set_path(&mut ctx, "moduleState.lastCycleProgress", json!(cycle_progress));
        let surface = value_text(get_path(&ctx, "moduleState.surfaceResponse"));
        if !surface.trim().is_empty() {
            report.surface_response = Some(surface);
            report.stalled = false;
            break;
        }

        let fingerprint = state_fingerprint(&ctx);
        if fingerprint == last_fingerprint {
            if began_with_experience && !report.broadened_after_experience_route {
                current_schedule = schedule(state, snapshot, true);
                report.broadened_after_experience_route = true;
                report.search_mode = "experience_route_then_explore_all".to_string();
                last_fingerprint = state_fingerprint(&ctx);
                continue;
            }
            if !outcome_exposed && expose_outcome(&mut ctx) {
                outcome_exposed = true;
                report.outcome_exposed = true;
                last_fingerprint = state_fingerprint(&ctx);
                continue;
            }
            report.stalled = true;
            break;
        }
        last_fingerprint = fingerprint;
    }

    report.modules_considered = considered_ids.len();
    report.modules_applied = applied_by_id.len();
    report.modules_changed = changed_by_id.len();
    report.applied = applied_by_id.into_values().collect();
    report.changed = changed_by_id.into_values().collect();
    report.unsupported_operations.sort(); report.unsupported_operations.dedup();
    report.missing_capabilities.sort(); report.missing_capabilities.dedup();
    report.awakenable_stored_only_rule_ids.sort(); report.awakenable_stored_only_rule_ids.dedup();

    if let Err(error) = validate_and_commit(&ctx, state) {
        report.stalled = true;
        report.surface_response = None;
        report.unsupported_operations.push(error);
    }
    state.module_state = get_path_cloned(&ctx, "moduleState").unwrap_or_else(|| json!({}));
    state.learned_state = get_path_cloned(&ctx, "learnedState").unwrap_or_else(|| json!({}));
    if let Some(value) = get_path_cloned(&ctx, "developmentalState") {
        match serde_json::from_value::<RustDevelopmentState>(value) {
            Ok(mut next) => { next.normalize(); *development = next; },
            Err(error) => report.unsupported_operations.push(format!("invalid_developmental_state:{error}")),
        }
    }
    for subsystem in ["learned_cognitive_module_runtime", "capability_requirement_gating", "learned_experience_route_scheduler"] {
        state.rust_owned_subsystems.push(subsystem.to_string());
    }
    state.rust_owned_subsystems.sort(); state.rust_owned_subsystems.dedup();
    if report.surface_response.is_some() { state.status = "SUPPORTED".to_string(); }
    report
}
