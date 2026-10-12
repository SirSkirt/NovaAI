use serde_json::{json, Value};

use crate::nova::cognition::CognitiveModule;
use crate::nova::ids::{stable_hash_id, ModuleId};
use crate::nova::learning::{LearnedRuleRecord, RuleSynthesisCandidate};
use crate::nova::mind::unix_ms;

use super::parser::canonical;
use super::types::{CognitiveTeachingTranslation, ParsedTeachingSource, TeachingBlock, TeachingSourceInput};

fn clean(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ").trim_matches(|ch: char| matches!(ch, '.' | ';' | ':' | '"' | '\'' | '“' | '”')).trim().to_string()
}

fn text_atoms(text: &str) -> Vec<String> {
    const STOP: &[&str] = &["a","an","the","is","are","was","were","be","been","being","to","of","for","from","with","when","if","then","that","this","it","and","or","not","may","can","should","must","will","would"];
    let mut out = Vec::new();
    for token in canonical(text).split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_').filter(|token| token.len() > 2 && !STOP.contains(token)) {
        if !out.iter().any(|old| old == token) { out.push(token.to_string()); }
        if out.len() >= 24 { break; }
    }
    out
}

fn curriculum_metadata(role: &str, text: &str) -> bool {
    matches!(canonical(role).as_str(), "metadata" | "heading" | "title" | "summary" | "lesson objective" | "assessment criterion")
        || ["subject:","topic:","purpose:","end:"].iter().any(|prefix| canonical(text).starts_with(prefix))
}

fn operational_kind(block: &TeachingBlock) -> Option<&'static str> {
    let raw = clean(&block.text);
    let text = canonical(&raw);
    let role = canonical(&block.role);
    let scope = canonical(&block.scope);
    if raw.is_empty() || curriculum_metadata(&role, &raw) || matches!(role.as_str(), "example" | "negative example" | "practice" | "question" | "answer") { return None; }

    if matches!(role.as_str(), "definition" | "relationship" | "guidance" | "warning" | "constraint" | "procedure step")
        && ((text.contains(" is ") || text.contains(" are ") || text.contains("testable")) && text.contains(" when ")) {
        return Some("predicate");
    }
    if (text.contains("means revising") || text.contains("means updating") || text.contains("means changing") || text.contains("means reconsidering") || text.contains("means retracting"))
        || ((text.contains(" when ") || text.starts_with("when ")) && ["changes","change","appears","arrives","fails","succeeds"].iter().any(|needle| text.contains(*needle)) && ["revise","update","change","reconsider","retract","preserve","mark","set"].iter().any(|needle| text.contains(*needle)))
        || ((text.contains(" should ") || text.contains(" must ")) && ["revise","update","change","reconsider","retract"].iter().any(|needle| text.contains(*needle))) {
        return Some("state_transition");
    }
    if text.contains("prefer ") || text.contains("may still be useful when") || text.contains("fallback") || (text.contains(" unavailable") && text.contains("when ")) { return Some("preference"); }
    if ["memory","source material","information source","source"].iter().any(|needle| text.contains(*needle))
        && ["consult"," use ","used","allowed","permitted","explicitly defined","closed book","open book"].iter().any(|needle| text.contains(*needle)) { return Some("resource_policy"); }
    if (["wording","verbatim","exact wording","natural wording","semantic meaning","semantic frame"].iter().any(|needle| text.contains(*needle)) || scope.contains("realization"))
        && ["preserve","required","not always","may choose","exact","wording","meaning"].iter().any(|needle| text.contains(*needle)) { return Some("fidelity_policy"); }
    if ["complexity","structure","frame","frames","semantic role","semantic roles","relationship","relationships"].iter().any(|needle| format!("{scope} {text}").contains(*needle))
        && ["follow","preserve","retain","maintain","flatten","collapse","detach","replace","combine","composition"].iter().any(|needle| text.contains(*needle)) { return Some("representation_invariant"); }
    None
}

fn scope_name(block: &TeachingBlock, fallback: &str) -> String {
    let value = canonical(&block.scope).chars().map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' }).collect::<String>().trim_matches('_').to_string();
    if value.is_empty() || value == "general" || value == "lesson" { fallback.to_string() } else { value }
}

fn condition_tail(raw: &str, markers: &[&str]) -> String {
    let lower = raw.to_ascii_lowercase();
    for &marker in markers {
        if let Some(index) = lower.find(marker) { return clean(&raw[index + marker.len()..]); }
    }
    String::new()
}

fn policy_spec(kind: &str, block: &TeachingBlock) -> Value {
    let raw = clean(&block.text);
    let text = canonical(&raw);
    match kind {
        "predicate" => {
            let condition = condition_tail(&raw, &[" when ", " if "]);
            json!({"name":scope_name(block,"learned_predicate"),"definedBy":raw,"conditionText":condition,"conditionAtoms":text_atoms(&condition),"sourceRole":block.role,"scope":block.scope,"kind":"learned_predicate_definition"})
        }
        "state_transition" => {
            let trigger = condition_tail(&raw, &[" when "]);
            let action = if ["preserve","remain","keep","retain"].iter().any(|needle| text.contains(*needle)) { "preserve" }
                else if ["remove","retract","discard"].iter().any(|needle| text.contains(*needle)) { "remove" }
                else if ["mark"," set "].iter().any(|needle| text.contains(*needle)) { "set" } else { "revise" };
            json!({"target":scope_name(block,"cognitive_state"),"triggerText":trigger,"triggerAtoms":text_atoms(&trigger),"action":action,"directive":raw,"scope":block.scope,"kind":"learned_state_transition"})
        }
        "preference" => {
            let mut preferred: Option<String> = None;
            let mut fallback: Option<String> = None;
            let raw_lower = raw.to_ascii_lowercase();
            if let Some(index) = raw_lower.find("prefer ") {
                let rest = clean(&raw[index + "prefer ".len()..]);
                let lower = rest.to_ascii_lowercase();
                let split = [" over ", " to ", " when ", " if "].iter().filter_map(|marker| lower.find(*marker).map(|i| (i,*marker))).min_by_key(|row| row.0);
                if let Some((at, marker)) = split { preferred = Some(clean(&rest[..at])); fallback = Some(clean(&rest[at + marker.len()..])); } else { preferred = Some(rest); }
            }
            let condition = condition_tail(&raw, &[" when ", " if "]);
            json!({"preferred":preferred,"fallback":fallback.unwrap_or_else(||scope_name(block,"fallback_option")),"conditionText":condition,"conditionAtoms":text_atoms(&condition),"directive":raw,"scope":block.scope,"kind":"learned_preference_policy"})
        }
        "resource_policy" => {
            let resources = [("memory","memory"),("source material","source_material"),("source","source_material"),("web","web"),("file","files"),("self knowledge","self_model"),("self model","self_model")].iter().filter_map(|(needle,name)| text.contains(*needle).then(||name.to_string())).fold(Vec::<String>::new(), |mut out,value| { if !out.contains(&value){out.push(value)} out });
            let mode = if ["must not","do not","don't","never","forbid"].iter().any(|needle| text.contains(*needle)) { "forbid" } else if text.contains(" only") || text.contains("closed book") { "restrict" } else { "explicit_if_material" };
            let condition = condition_tail(&raw, &[" if ", " when "]);
            json!({"resources":resources,"mode":mode,"conditionText":condition,"directive":raw,"scope":block.scope,"kind":"learned_task_resource_policy"})
        }
        "fidelity_policy" => {
            let mode = if (text.contains("exact wording") || text.contains("verbatim wording") || text.contains("source wording")) && !text.contains("not always") { "verbatim_when_required" }
                else if text.contains("not always") || text.contains("required only") || text.contains("natural wording") { "semantic_unless_verbatim_required" }
                else if text.contains("preserve") && ["meaning","semantic","role","frame","relationship"].iter().any(|needle| text.contains(*needle)) { "semantic_preservation" } else { "semantic" };
            let condition = condition_tail(&raw, &[" if ", " when ", " unless "]);
            json!({"mode":mode,"conditionText":condition,"directive":raw,"scope":block.scope,"kind":"learned_fidelity_policy"})
        }
        "representation_invariant" => {
            let mode = if ["do not","don't","should not","must not"].iter().any(|needle| text.contains(*needle)) && ["flatten","collapse","detach","replace"].iter().any(|needle| text.contains(*needle)) { "prevent_structural_loss" } else if ["complexity","structure","frame","relationship","role"].iter().any(|needle| text.contains(*needle)) { "source_semantic_shape_fidelity" } else { "preserve_required_structure" };
            json!({"mode":mode,"directive":raw,"scope":block.scope,"requiredAtoms":text_atoms(&raw),"kind":"learned_representation_invariant"})
        }
        _ => json!({"directive":raw,"scope":block.scope}),
    }
}

fn module_for(kind: &str, rule: &LearnedRuleRecord, block: &TeachingBlock) -> CognitiveModule {
    let (name, domain, phase, priority, form, op) = match kind {
        "predicate" => ("Synthesized learned predicate", rule.domain.clone(), "reasoning", 74.0, "operation_program", "declare_learned_predicate"),
        "state_transition" => ("Synthesized general state transition", rule.domain.clone(), "reasoning", 75.0, "state_transformation", "declare_state_transition"),
        "preference" => ("Synthesized conditional preference policy", rule.domain.clone(), "reasoning", 73.0, "selection_policy", "declare_preference_policy"),
        "resource_policy" => ("Synthesized task resource policy", "memory".to_string(), "memory", 82.0, "memory_policy", "declare_task_resource_policy"),
        "fidelity_policy" => ("Synthesized response fidelity policy", "realization".to_string(), "realization", 84.0, "realization_policy", "declare_fidelity_policy"),
        "representation_invariant" => ("Synthesized representation-shape invariant", "interpretation".to_string(), "interpretation", 88.0, "invariant", "declare_representation_invariant"),
        _ => ("Stored learned cognition", rule.domain.clone(), "reasoning", 50.0, "unknown", ""),
    };
    CognitiveModule {
        id: ModuleId(stable_hash_id("rust_module", &canonical(&rule.id))), module_number: None, name: name.to_string(), domain, phase: phase.to_string(), categories: rule.categories.clone(), conditions: vec![json!({"type":"always"})],
        program: if op.is_empty() { Vec::new() } else { vec![json!({"op":op,"args":{"spec":policy_spec(kind,block)}})] }, requires: Vec::new(), produces: Vec::new(), capabilities: Vec::new(), priority, enabled: true,
        source_type: "learned".to_string(), source_name: rule.source_name.clone(), source_id: rule.source_id.clone(), cognitive_form: Some(form.to_string()), evidence: rule.evidence.clone(),
    }
}

fn should_be_rule(block: &TeachingBlock) -> bool {
    matches!(block.role.as_str(), "guidance" | "warning" | "constraint" | "behavior" | "correction" | "validation_rule" | "procedure" | "procedure_step") || operational_kind(block).is_some()
}

pub fn translate_cognition(input: &TeachingSourceInput, parsed: &ParsedTeachingSource, source_id: &str) -> CognitiveTeachingTranslation {
    let now = unix_ms();
    let mut out = CognitiveTeachingTranslation::default();
    for block in parsed.blocks.iter().filter(|block| block.section_kind != "test" && should_be_rule(block)) {
        let directive = clean(&block.text);
        if directive.is_empty() { continue; }
        let kind = operational_kind(block);
        let id = stable_hash_id("rule", &canonical(&format!("{source_id}|{}|{}|{}", block.id, block.role, canonical(&directive))));
        let negative = matches!(block.role.as_str(), "warning" | "constraint") || canonical(&directive).starts_with("do not ") || canonical(&directive).starts_with("avoid ");
        let synthesis_reason = if kind.is_some() {
            "awaiting current Rust synthesis validation"
        } else {
            "teaching is retained but no current generic Rust program can represent it yet"
        };
        let mut rule = LearnedRuleRecord {
            id: id.clone(), directive: directive.clone(), domain: canonical(&block.scope).replace(' ', "_"), primitive: "learned_guidance".to_string(), scope: block.scope.clone(), evidence_role: block.role.clone(),
            cognitive_form: kind.map(|value| match value { "predicate"=>"operation_program", "state_transition"=>"state_transformation", "preference"=>"selection_policy", "resource_policy"=>"memory_policy", "fidelity_policy"=>"realization_policy", "representation_invariant"=>"invariant", _=>"learned_guidance" }.to_string()),
            learned_category: None, categories: vec![block.role.clone()], strength: if directive.to_ascii_lowercase().contains("must") { "required".to_string() } else { "recommended".to_string() }, polarity: if negative { "negative".to_string() } else { "positive".to_string() },
            status: "stored_only".to_string(), module_id: None, source_type: input.source_type.clone().unwrap_or_else(||"learned".to_string()), source_name: input.source_name.clone(), source_display_name: Some(parsed.source_display_name.clone()), source_kind: Some(parsed.document_kind.clone()), source_lesson_number: parsed.lesson_number, source_id: Some(source_id.to_string()),
            evidence: vec![json!({"sourceId":source_id,"sourceName":input.source_name,"sourceRole":"instruction","section":block.scope,"role":block.role,"text":directive,"learnedAt":now})], parameters: json!({}),
            synthesis: json!({"version":7,"status":"stored_only","authority":"rust_v7_pass_9","reason":synthesis_reason,"attemptedAt":now}), learned_at: Some(now as u64), updated_at: Some(now as u64),
        };
        if rule.domain.trim().is_empty() { rule.domain = "reasoning".to_string(); }
        let candidate = kind.map(|kind| {
            let module = module_for(kind, &rule, block);
            RuleSynthesisCandidate { rule_id: rule.id.clone(), module: Some(module.clone()), diagnostic: json!({"status":"synthesized","authority":"rust_v7_pass_9","teachingKind":kind,"requiredOperations":module.program.iter().filter_map(|step|step.get("op").and_then(Value::as_str)).collect::<Vec<_>>()}) }
        });
        out.rules.push(rule);
        if let Some(candidate) = candidate { out.candidates.push(candidate); }
    }
    out
}
