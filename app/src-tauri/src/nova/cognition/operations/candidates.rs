use std::collections::BTreeSet;

use serde_json::{json, Value};

use crate::nova::cognition::path::{canon, get_path, get_path_cloned, set_path, value_text};

fn candidate_label(candidate: &Value) -> String {
    for key in ["entity", "label", "name", "id"] {
        let value = value_text(candidate.get(key));
        if !value.is_empty() { return value; }
    }
    String::new()
}

pub fn execute(op: &str, args: &Value, ctx: &mut Value) -> Result<bool, String> {
    match op {
        "filter_candidates" => {
            let path = args.get("listPath").and_then(Value::as_str).unwrap_or("candidates");
            let list = get_path_cloned(ctx, path).and_then(|value| value.as_array().cloned()).unwrap_or_default();
            let wanted_type = args.get("type").and_then(Value::as_str).map(canon);
            let wanted_source = args.get("source").and_then(Value::as_str).map(canon);
            let excluded: BTreeSet<String> = args.get("excludeSources").and_then(Value::as_array).into_iter().flatten().map(|value| canon(&value_text(Some(value)))).collect();
            let filtered = list.into_iter().filter(|candidate| {
                let type_ok = wanted_type.as_ref().is_none_or(|wanted| {
                    let actual = canon(&value_text(candidate.get("type")));
                    actual.is_empty() || actual == *wanted
                });
                let source = canon(&value_text(candidate.get("source")));
                let source_ok = wanted_source.as_ref().is_none_or(|wanted| source == *wanted) && !excluded.contains(&source);
                type_ok && source_ok
            }).collect();
            Ok(set_path(ctx, args.get("outputPath").and_then(Value::as_str).unwrap_or(path), Value::Array(filtered)))
        }
        "score_candidates" => {
            let path = args.get("listPath").and_then(Value::as_str).unwrap_or("candidates");
            let mut list = get_path_cloned(ctx, path).and_then(|value| value.as_array().cloned()).unwrap_or_default();
            let criteria = args.get("criteria").and_then(Value::as_array).cloned().unwrap_or_default();
            for candidate in &mut list {
                let mut score = candidate.get("score").and_then(Value::as_f64).unwrap_or(0.0);
                for criterion in &criteria {
                    let kind = canon(&value_text(criterion.get("kind")));
                    let weight = criterion.get("weight").and_then(Value::as_f64).unwrap_or(1.0);
                    if kind == "source_equals" && canon(&value_text(candidate.get("source"))) == canon(&value_text(criterion.get("value"))) { score += weight; }
                    if kind == "field_equals" {
                        let field = criterion.get("field").and_then(Value::as_str).unwrap_or("");
                        if canon(&value_text(candidate.get(field))) == canon(&value_text(criterion.get("value"))) { score += weight; }
                    }
                }
                if let Some(object) = candidate.as_object_mut() { object.insert("score".into(), json!(score)); }
            }
            list.sort_by(|left, right| right.get("score").and_then(Value::as_f64).unwrap_or(0.0).partial_cmp(&left.get("score").and_then(Value::as_f64).unwrap_or(0.0)).unwrap_or(std::cmp::Ordering::Equal));
            Ok(set_path(ctx, args.get("outputPath").and_then(Value::as_str).unwrap_or(path), Value::Array(list)))
        }
        "measure_candidate_support" | "score_candidates_by_relationship" => {
            let path = args.get("listPath").and_then(Value::as_str).unwrap_or("candidates");
            let mut list = get_path_cloned(ctx, path).and_then(|value| value.as_array().cloned()).unwrap_or_default();
            let roots = ["canonicalSemanticState.relations", "canonicalSemanticState.events", "canonicalSemanticState.propositions", "canonicalSemanticState.evidence"];
            let root_text = roots.iter().filter_map(|path| get_path(ctx, path)).map(Value::to_string).map(|text| canon(&text)).collect::<Vec<_>>();
            let weight = args.get("matchWeight").or_else(|| args.get("explicitWeight")).and_then(Value::as_f64).unwrap_or(1.0);
            for candidate in &mut list {
                let label = canon(&candidate_label(candidate));
                let mut score = candidate.get("support").or_else(|| candidate.get("score")).and_then(Value::as_f64).unwrap_or(0.0);
                let mut reasons = candidate.get("supportReasons").or_else(|| candidate.get("scoreReasons")).and_then(Value::as_array).cloned().unwrap_or_default();
                if !label.is_empty() && root_text.iter().any(|root| root.contains(&label)) {
                    score += weight;
                    reasons.push(Value::String("represented_relationship_support".into()));
                }
                if let Some(object) = candidate.as_object_mut() {
                    object.insert("support".into(), json!(score));
                    object.insert("score".into(), json!(score));
                    object.insert("supportReasons".into(), Value::Array(reasons));
                }
            }
            list.sort_by(|left, right| right.get("score").and_then(Value::as_f64).unwrap_or(0.0).partial_cmp(&left.get("score").and_then(Value::as_f64).unwrap_or(0.0)).unwrap_or(std::cmp::Ordering::Equal));
            Ok(set_path(ctx, args.get("outputPath").and_then(Value::as_str).unwrap_or(path), Value::Array(list)))
        }
        "select_best_candidate" => {
            let list = get_path_cloned(ctx, args.get("listPath").and_then(Value::as_str).unwrap_or("candidates")).and_then(|value| value.as_array().cloned()).unwrap_or_default();
            let Some(best) = list.first().cloned() else { return Ok(false); };
            let mut changed = set_path(ctx, args.get("outputPath").and_then(Value::as_str).unwrap_or("selectedCandidate"), best.clone());
            if args.get("assignToMention").and_then(Value::as_bool).unwrap_or(false) && get_path(ctx, "mention").is_some() {
                let target = candidate_label(&best);
                if !target.is_empty() {
                    changed = set_path(ctx, "mention.resolvedEntity", Value::String(target)) || changed;
                    changed = set_path(ctx, "mention.resolutionSource", Value::String("learned_module".into())) || changed;
                }
            }
            Ok(changed)
        }
        "assign_reference" => {
            if get_path(ctx, "mention").is_none() { return Ok(false); }
            let best = get_path_cloned(ctx, "selectedCandidate").or_else(|| get_path(ctx, "candidates").and_then(Value::as_array).and_then(|items| items.first()).cloned());
            let Some(best) = best else { return Ok(false); };
            let target = candidate_label(&best);
            if target.is_empty() { return Ok(false); }
            let mut changed = set_path(ctx, "mention.resolvedEntity", Value::String(target));
            changed = set_path(ctx, "mention.resolutionSource", Value::String("learned_module".into())) || changed;
            Ok(changed)
        }
        _ => Err("not_candidates".to_string()),
    }
}
