use serde_json::{json, Value};

use crate::nova::cognition::path::{canon, get_path, get_path_cloned, resolve_template, set_path, value_text};

fn policy_list(ctx: &Value, path: &str) -> Vec<Value> {
    get_path_cloned(ctx, path).and_then(|value| value.as_array().cloned()).unwrap_or_default()
}

fn item_identity(value: &Value) -> String {
    if let Some(object) = value.as_object() {
        for key in ["id", "name", "label"] {
            let text = value_text(object.get(key));
            if !text.is_empty() { return canon(&text); }
        }
    }
    canon(&value_text(Some(value)))
}

pub fn execute(op: &str, args: &Value, ctx: &mut Value) -> Result<bool, String> {
    match op {
        "ensure_state_container" => {
            let path = args.get("path").and_then(Value::as_str).unwrap_or("cognitiveState");
            if get_path(ctx, path).is_some() { return Ok(false); }
            let value = if args.get("kind").and_then(Value::as_str) == Some("list") { json!([]) } else { json!({}) };
            Ok(set_path(ctx, path, value))
        }
        "evaluate_learned_predicate" => {
            let name = canon(args.get("name").or_else(|| args.get("predicate")).and_then(Value::as_str).unwrap_or(""));
            let predicates = policy_list(ctx, "moduleState.policyStore.predicates");
            let spec = predicates.iter().find(|value| canon(&value_text(value.get("name"))) == name).cloned();
            let text = args.get("contextText").map(|value| value_text(Some(&resolve_template(value, ctx))))
                .or_else(|| args.get("contextPath").and_then(Value::as_str).and_then(|path| get_path_cloned(ctx, path)).map(|value| value_text(Some(&value))))
                .unwrap_or_default();
            let atoms = spec.as_ref().and_then(|value| value.get("conditionAtoms")).and_then(Value::as_array).cloned().unwrap_or_default();
            let normalized = canon(&text);
            let matched = spec.is_some() && (atoms.is_empty() || atoms.iter().all(|atom| normalized.contains(&canon(&value_text(Some(atom))))));
            let result = json!({"predicate":name,"known":spec.is_some(),"matched":matched,"spec":spec});
            Ok(set_path(ctx, args.get("outputPath").and_then(Value::as_str).unwrap_or("moduleState.predicateEvaluation"), result))
        }
        "apply_declared_transition" => {
            let target = canon(args.get("target").and_then(Value::as_str).unwrap_or(""));
            let transitions = policy_list(ctx, "moduleState.policyStore.transitions");
            let spec = transitions.iter().find(|value| canon(&value_text(value.get("target"))) == target || canon(&value_text(value.get("id"))) == target).cloned();
            let Some(spec) = spec else { return Ok(false); };
            let spec_target = value_text(spec.get("target"));
            let state_path = args.get("statePath").and_then(Value::as_str).map(str::to_string).unwrap_or_else(|| format!("learnedState.{spec_target}"));
            let current = get_path_cloned(ctx, &state_path).unwrap_or(Value::Null);
            let value = args.get("valueFromPath").and_then(Value::as_str).and_then(|path| get_path_cloned(ctx, path)).or_else(|| args.get("value").cloned()).unwrap_or_else(|| current.clone());
            let next = json!({"previous":current,"value":value,"action":spec.get("action").and_then(Value::as_str).unwrap_or("revise"),"reason":spec.get("directive").cloned().unwrap_or(Value::Null)});
            Ok(set_path(ctx, &state_path, next))
        }
        "select_preferred_available" => {
            let list = args.get("optionsPath").and_then(Value::as_str).and_then(|path| get_path_cloned(ctx, path)).or_else(|| args.get("options").cloned()).and_then(|value| value.as_array().cloned()).unwrap_or_default();
            let preferences = policy_list(ctx, "moduleState.policyStore.preferences");
            let policy = args.get("policyId").and_then(Value::as_str)
                .and_then(|id| preferences.iter().find(|value| value_text(value.get("id")) == id))
                .or_else(|| preferences.last());
            let preferred = policy.and_then(|value| value.get("preferred")).map(|value| canon(&value_text(Some(value))));
            let fallback = policy.and_then(|value| value.get("fallback")).map(|value| canon(&value_text(Some(value))));
            let selected = preferred.as_ref().and_then(|wanted| list.iter().find(|item| item_identity(item) == *wanted).cloned())
                .or_else(|| fallback.as_ref().and_then(|wanted| list.iter().find(|item| item_identity(item) == *wanted).cloned()))
                .or_else(|| list.first().cloned());
            Ok(selected.is_some_and(|value| set_path(ctx, args.get("outputPath").and_then(Value::as_str).unwrap_or("selectedPreferred"), value)))
        }
        "check_representation_invariant" => {
            let source = args.get("sourcePath").and_then(Value::as_str).and_then(|path| get_path(ctx, path));
            let target = args.get("targetPath").and_then(Value::as_str).and_then(|path| get_path(ctx, path));
            let count = |value: Option<&Value>| -> usize {
                value.map_or(0, |value| value.as_object().map(|object| object.len()).or_else(|| value.as_array().map(Vec::len)).unwrap_or(1))
            };
            let source_count = count(source);
            let target_count = count(target);
            Ok(set_path(ctx, args.get("outputPath").and_then(Value::as_str).unwrap_or("representationInvariantCheck"), json!({"sourceCount":source_count,"targetCount":target_count,"preservesNontrivialShape":source_count<=1||target_count>0})))
        }
        _ => Err("not_learned_policy".to_string()),
    }
}
