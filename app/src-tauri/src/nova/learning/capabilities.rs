use std::collections::BTreeSet;

use crate::nova::cognition::{CapabilityGraph, CapabilityProducer, CapabilityWaiter, CognitiveModule};
use crate::nova::ids::CapabilityKey;

use super::types::LearnedRuleRecord;

fn canon(value: &str) -> String {
    value.trim().to_ascii_lowercase().replace(' ', "_")
}

fn is_diagnostic_only(value: &str) -> bool {
    matches!(canon(value).as_str(), "unmapped_cognitive_state_transformation" | "unmapped_cognitive_state_transformation_")
}

pub fn module_outputs(module: &CognitiveModule) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for value in module.produces.iter().chain(module.capabilities.iter()) {
        let key = canon(&value.0);
        if !key.is_empty() && !is_diagnostic_only(&key) { out.insert(key); }
    }
    if let Some(form) = &module.cognitive_form {
        let key = canon(form);
        if !key.is_empty() { out.insert(format!("cognitive_form:{key}")); }
    }
    out
}

pub fn rule_needs(rule: &LearnedRuleRecord) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for field in ["missingCapabilities", "missingOperations"] {
        if let Some(items) = rule.synthesis.get(field).and_then(|value| value.as_array()) {
            for item in items {
                if let Some(text) = item.as_str() {
                    let key = canon(text);
                    if !key.is_empty() && !is_diagnostic_only(&key) { out.insert(key); }
                }
            }
        }
    }
    out
}

pub fn available_capabilities(modules: &[CognitiveModule], known_waiters: &BTreeSet<String>) -> BTreeSet<String> {
    let mut known = known_waiters.clone();
    for module in modules { known.extend(module_outputs(module)); }
    let mut available = BTreeSet::new();
    loop {
        let before = available.len();
        for module in modules {
            if !module.enabled { continue; }
            let blocked = module.requires.iter().any(|required| {
                let key = canon(&required.0);
                !key.is_empty() && known.contains(&key) && !available.contains(&key)
            });
            if !blocked { available.extend(module_outputs(module)); }
        }
        if before == available.len() { break; }
    }
    available
}

pub fn build_capability_graph(modules: &[CognitiveModule], rules: &[LearnedRuleRecord]) -> CapabilityGraph {
    let mut producers = Vec::new();
    let mut producer_seen = BTreeSet::new();
    for module in modules.iter().filter(|module| module.enabled) {
        for capability in module_outputs(module) {
            let sig = format!("{}|{}", capability, module.id.0);
            if producer_seen.insert(sig) {
                producers.push(CapabilityProducer { capability: CapabilityKey(capability), module_id: module.id.clone() });
            }
        }
    }

    let mut waiters = Vec::new();
    let mut waiter_seen = BTreeSet::new();
    for rule in rules.iter().filter(|rule| rule.status != "module_executable" && rule.status != "executable") {
        for capability in rule_needs(rule) {
            let sig = format!("{}|{}", capability, rule.id);
            if waiter_seen.insert(sig) {
                waiters.push(CapabilityWaiter { capability: CapabilityKey(capability), module_id: None, rule_id: Some(rule.id.clone()) });
            }
        }
    }
    CapabilityGraph { producers, waiters }
}

pub fn missing_candidate_capabilities(module: &CognitiveModule, graph: &CapabilityGraph, modules: &[CognitiveModule]) -> Vec<String> {
    let known_waiters: BTreeSet<String> = graph.waiters.iter().map(|row| canon(&row.capability.0)).collect();
    let available = available_capabilities(modules, &known_waiters);
    let known: BTreeSet<String> = graph.producers.iter().map(|row| canon(&row.capability.0)).chain(known_waiters).collect();
    module.requires.iter().filter_map(|required| {
        let key = canon(&required.0);
        if key.is_empty() || !known.contains(&key) || available.contains(&key) { None } else { Some(key) }
    }).collect()
}

pub fn capability_counts(graph: &CapabilityGraph) -> (usize, usize) {
    let producers: BTreeSet<String> = graph.producers.iter().map(|row| canon(&row.capability.0)).collect();
    let waiters: BTreeSet<String> = graph.waiters.iter().map(|row| canon(&row.capability.0)).collect();
    (producers.len(), waiters.len())
}
