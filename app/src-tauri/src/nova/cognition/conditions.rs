use serde_json::{json, Value};

use super::path::{canon, get_path, get_path_cloned, set_path, value_text};

fn str_field<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn string_values(value: Option<&Value>) -> Vec<String> {
    value.and_then(Value::as_array)
        .map(|items| items.iter().map(|item| value_text(Some(item))).filter(|s| !s.is_empty()).collect())
        .unwrap_or_default()
}

fn token_set(text: &str) -> std::collections::BTreeSet<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .map(canon)
        .filter(|token| token.len() > 1)
        .collect()
}

fn similarity(a: &str, b: &str) -> f64 {
    let canonical_a = canon(a);
    let canonical_b = canon(b);
    if canonical_a.is_empty() || canonical_b.is_empty() { return 0.0; }
    if canonical_a == canonical_b { return 1.0; }
    let a_tokens = token_set(&canonical_a);
    let b_tokens = token_set(&canonical_b);
    if a_tokens.is_empty() || b_tokens.is_empty() { return 0.0; }
    let common = a_tokens.intersection(&b_tokens).count() as f64;
    let union = a_tokens.union(&b_tokens).count() as f64;
    let mut score = common / union.max(1.0);
    let first_a = canonical_a.split_whitespace().next();
    let first_b = canonical_b.split_whitespace().next();
    if first_a.is_some() && first_a == first_b { score += 0.12; }
    if canonical_a.contains(&canonical_b) || canonical_b.contains(&canonical_a) { score += 0.18; }
    score.min(1.0)
}

fn observed_outer_cue(ctx: &Value) -> String {
    let segments = get_path(ctx, "utteranceObservation.segments").and_then(Value::as_array);
    if let Some(segments) = segments {
        let text = segments
            .iter()
            .filter(|segment| value_text(segment.get("kind")).as_str() != "quoted_text")
            .map(|segment| value_text(segment.get("text")))
            .filter(|text| !text.trim().is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        if !text.trim().is_empty() { return text; }
    }
    value_text(get_path(ctx, "rawText"))
}

fn learned_surface_family(condition: &Value, ctx: &mut Value) -> bool {
    let current = observed_outer_cue(ctx);
    let positives = string_values(condition.get("positiveCues"));
    if positives.is_empty() { return false; }
    let negatives = string_values(condition.get("negativeCues"));
    let threshold = condition.get("threshold").and_then(Value::as_f64).unwrap_or(0.58);
    let margin = condition.get("negativeMargin").and_then(Value::as_f64).unwrap_or(0.08);

    let mut best_positive = 0.0;
    let mut matched_cue = String::new();
    for cue in &positives {
        let score = similarity(&current, cue);
        if score > best_positive {
            best_positive = score;
            matched_cue = cue.clone();
        }
    }
    let best_negative = negatives.iter().map(|cue| similarity(&current, cue)).fold(0.0, f64::max);
    if best_positive < threshold || best_negative >= best_positive - margin { return false; }

    set_path(ctx, "captures.familyScore", json!(best_positive));
    set_path(ctx, "captures.matchedCue", Value::String(matched_cue.clone()));
    set_path(ctx, "patternCaptures.familyScore", json!(best_positive));
    set_path(ctx, "patternCaptures.matchedCue", Value::String(matched_cue));
    if let Some(family) = condition.get("familyId").or_else(|| condition.get("familyName")) {
        let family = value_text(Some(family));
        if !family.is_empty() {
            set_path(ctx, "captures.matchedFamily", Value::String(family.clone()));
            set_path(ctx, "patternCaptures.matchedFamily", Value::String(family));
        }
    }

    let payload_mode = canon(str_field(condition, "payloadMode").unwrap_or("")).replace(' ', "_");
    if payload_mode == "quoted_region" {
        let quotes = get_path_cloned(ctx, "utteranceObservation.quotes")
            .and_then(|value| value.as_array().cloned())
            .unwrap_or_default();
        let Some(chosen) = quotes.first() else { return false; };
        let capture_name = str_field(condition, "captureName").unwrap_or("selectedContent");
        let text = value_text(chosen.get("text"));
        let start = chosen.get("start").cloned().unwrap_or(Value::Null);
        let end = chosen.get("end").cloned().unwrap_or(Value::Null);
        let scope_id = get_path(ctx, "canonicalSemanticState.scopes")
            .and_then(Value::as_array)
            .and_then(|scopes| scopes.iter().find(|scope| scope.get("start") == Some(&start) && scope.get("end") == Some(&end)))
            .and_then(|scope| scope.get("id"))
            .cloned()
            .unwrap_or(Value::Null);
        let all = Value::Array(quotes.iter().map(|quote| Value::String(value_text(quote.get("text")))).collect());
        for root in ["captures", "patternCaptures"] {
            set_path(ctx, &format!("{root}.{capture_name}"), Value::String(text.clone()));
            set_path(ctx, &format!("{root}.{capture_name}ScopeId"), scope_id.clone());
            set_path(ctx, &format!("{root}.{capture_name}Start"), start.clone());
            set_path(ctx, &format!("{root}.{capture_name}End"), end.clone());
            set_path(ctx, &format!("{root}.{capture_name}All"), all.clone());
        }
    }
    true
}

fn item_matches(item: &Value, spec: &Value, ctx: &Value) -> bool {
    let Some(object) = spec.as_object() else { return true; };
    object.iter().all(|(path, expected)| {
        let actual = get_path(item, path);
        let expected = super::path::resolve_template(expected, ctx);
        match expected {
            Value::Array(values) => values.iter().any(|value| canon(&value_text(Some(value))) == canon(&value_text(actual))),
            Value::Object(_) => actual == Some(&expected),
            _ => canon(&value_text(actual)) == canon(&value_text(Some(&expected))),
        }
    })
}

fn surface_pattern(pattern: &str, text: &str, ctx: &mut Value) -> bool {
    let mut literals = Vec::new();
    let mut captures = Vec::new();
    let mut rest = pattern;
    while let Some(open) = rest.find('{') {
        let Some(close_rel) = rest[open + 1..].find('}') else { break; };
        let close = open + 1 + close_rel;
        literals.push(rest[..open].to_string());
        captures.push(rest[open + 1..close].trim().to_string());
        rest = &rest[close + 1..];
    }
    if captures.is_empty() { return canon(pattern) == canon(text); }
    literals.push(rest.to_string());
    let lower = text.to_lowercase();
    let mut cursor = 0usize;
    let first = literals.first().cloned().unwrap_or_default().to_lowercase();
    if !first.trim().is_empty() {
        let Some(pos) = lower[cursor..].find(first.trim()) else { return false; };
        if pos != 0 && !pattern.trim_start().starts_with('{') { return false; }
        cursor += pos + first.trim().len();
    }
    for (index, capture) in captures.iter().enumerate() {
        let next_lit = literals.get(index + 1).cloned().unwrap_or_default().to_lowercase();
        let end = if next_lit.trim().is_empty() {
            lower.len()
        } else {
            let Some(pos) = lower[cursor..].find(next_lit.trim()) else { return false; };
            cursor + pos
        };
        let captured = text.get(cursor..end).unwrap_or("").trim().to_string();
        set_path(ctx, &format!("captures.{capture}"), Value::String(captured.clone()));
        set_path(ctx, &format!("patternCaptures.{capture}"), Value::String(captured));
        cursor = end + next_lit.trim().len();
    }
    true
}

pub fn condition_matches(condition: &Value, ctx: &mut Value) -> Result<bool, String> {
    if let Some(all) = condition.get("all").and_then(Value::as_array) {
        let before = ctx.clone();
        for child in all {
            if !condition_matches(child, ctx)? {
                *ctx = before;
                return Ok(false);
            }
        }
        return Ok(true);
    }
    if let Some(any) = condition.get("any").and_then(Value::as_array) {
        for child in any {
            let before = ctx.clone();
            if condition_matches(child, ctx)? { return Ok(true); }
            *ctx = before;
        }
        return Ok(false);
    }
    if let Some(child) = condition.get("not") {
        let before = ctx.clone();
        let matched = condition_matches(child, ctx)?;
        *ctx = before;
        return Ok(!matched);
    }

    let condition_type = canon(str_field(condition, "type").unwrap_or("always")).replace(' ', "_");
    let path = str_field(condition, "field").or_else(|| str_field(condition, "path")).unwrap_or("");
    let result = match condition_type.as_str() {
        "always" => true,
        "field_exists" => get_path(ctx, path).is_some_and(|value| !value.is_null()),
        "field_missing" => get_path(ctx, path).is_none_or(Value::is_null),
        "field_equals" => canon(&value_text(get_path(ctx, path))) == canon(&value_text(condition.get("value"))),
        "field_not_equals" => canon(&value_text(get_path(ctx, path))) != canon(&value_text(condition.get("value"))),
        "field_in" => string_values(condition.get("values")).iter().any(|value| canon(value) == canon(&value_text(get_path(ctx, path)))),
        "field_not_in" => !string_values(condition.get("values")).iter().any(|value| canon(value) == canon(&value_text(get_path(ctx, path)))),
        "contains_any" => {
            let source = str_field(condition, "sourcePath").and_then(|p| get_path(ctx, p)).map(|v| value_text(Some(v))).unwrap_or_else(|| value_text(get_path(ctx, "rawText")));
            let source = canon(&source);
            string_values(condition.get("values")).iter().any(|value| source.contains(&canon(value)))
        }
        "contains_all" => {
            let source = str_field(condition, "sourcePath").and_then(|p| get_path(ctx, p)).map(|v| value_text(Some(v))).unwrap_or_else(|| value_text(get_path(ctx, "rawText")));
            let source = canon(&source);
            string_values(condition.get("values")).iter().all(|value| source.contains(&canon(value)))
        }
        "surface_in" => {
            let surface = get_path(ctx, "mention.surface").or_else(|| get_path(ctx, "surface")).or_else(|| get_path(ctx, "rawText"));
            string_values(condition.get("values")).iter().any(|value| canon(value) == canon(&value_text(surface)))
        }
        "surface_pattern" => {
            let text = str_field(condition, "sourcePath").and_then(|p| get_path_cloned(ctx, p)).map(|v| value_text(Some(&v))).unwrap_or_else(|| value_text(get_path(ctx, "surface").or_else(|| get_path(ctx, "rawText"))));
            surface_pattern(str_field(condition, "pattern").unwrap_or(""), &text, ctx)
        }
        "learned_surface_family" => learned_surface_family(condition, ctx),
        "goal_exists" => {
            let wanted = str_field(condition, "goalType").map(canon);
            get_path(ctx, "canonicalSemanticState.unresolved").and_then(Value::as_array).is_some_and(|goals| goals.iter().any(|goal| wanted.as_ref().is_none_or(|value| canon(&value_text(goal.get("type"))) == *value)))
        }
        "category_active" => {
            let wanted = canon(str_field(condition, "category").unwrap_or(""));
            get_path(ctx, "activeCategories").and_then(Value::as_array).is_some_and(|items| items.iter().any(|item| canon(&value_text(Some(item))) == wanted))
        }
        "structure_exists" => {
            let list_path = str_field(condition, "listPath").unwrap_or("canonicalSemanticState.structures");
            let wanted_kind = str_field(condition, "kind").map(canon);
            let wanted_category = str_field(condition, "category").map(canon);
            get_path(ctx, list_path).and_then(Value::as_array).is_some_and(|items| items.iter().any(|item| {
                wanted_kind.as_ref().is_none_or(|wanted| canon(&value_text(item.get("kind"))) == *wanted)
                    && wanted_category.as_ref().is_none_or(|wanted| canon(&value_text(item.get("category"))) == *wanted)
            }))
        }
        "list_item_matches" => {
            let list_path = str_field(condition, "listPath").unwrap_or("");
            get_path(ctx, list_path).and_then(Value::as_array).is_some_and(|items| items.iter().any(|item| item_matches(item, condition.get("match").unwrap_or(&Value::Null), ctx)))
        }
        "scope_exists" => {
            get_path(ctx, "canonicalSemanticState.scopes").and_then(Value::as_array).is_some_and(|scopes| scopes.iter().any(|scope| {
                let id_ok = str_field(condition, "scopeId").is_none_or(|wanted| value_text(scope.get("id")) == wanted);
                let kind_ok = str_field(condition, "kind").is_none_or(|wanted| canon(&value_text(scope.get("kind"))) == canon(wanted));
                let boundary_ok = str_field(condition, "boundaryKind").is_none_or(|wanted| canon(&value_text(scope.get("boundaryKind"))) == canon(wanted));
                let non_root_ok = !condition.get("nonRoot").and_then(Value::as_bool).unwrap_or(false) || scope.get("parentId").is_some_and(|v| !v.is_null());
                id_ok && kind_ok && boundary_ok && non_root_ok
            }))
        }
        "fields_coexist" | "paths_present" => string_values(condition.get("paths")).iter().all(|p| get_path(ctx, p).is_some_and(|v| !v.is_null())),
        "path_unchanged" => {
            let left = str_field(condition, "leftPath").or_else(|| str_field(condition, "path")).unwrap_or("");
            let right = str_field(condition, "rightPath").or_else(|| str_field(condition, "priorPath")).unwrap_or("");
            get_path(ctx, left) == get_path(ctx, right)
        }
        "resolved_or_no_progress" => {
            let unresolved = get_path(ctx, "canonicalSemanticState.unresolved").and_then(Value::as_array).map_or(0, Vec::len);
            let progress = get_path(ctx, "moduleState.lastCycleProgress").and_then(Value::as_u64).unwrap_or(0);
            unresolved == 0 || progress == 0
        }
        "attempt_repeated_without_progress" => get_path(ctx, "attemptProgress.repeatWithoutNewProgress").and_then(Value::as_bool).unwrap_or(false),
        other => return Err(format!("unsupported_condition:{other}")),
    };
    Ok(result)
}

pub fn all_conditions_match(conditions: &[Value], ctx: &mut Value) -> Result<bool, String> {
    if conditions.is_empty() { return Ok(true); }
    for condition in conditions {
        if !condition_matches(condition, ctx)? { return Ok(false); }
    }
    Ok(true)
}


/// Validate learned condition structures without evaluating them. Synthesis uses
/// the same condition vocabulary as the runtime so a module cannot be marked
/// executable only to fail later on an unknown condition type.
pub fn validate_conditions(conditions: &[Value]) -> Vec<String> {
    fn supported(name: &str) -> bool {
        matches!(canon(name).replace(' ', "_").as_str(),
            "always" | "field_exists" | "field_missing" | "field_equals" | "field_not_equals" |
            "field_in" | "field_not_in" | "contains_any" | "contains_all" | "surface_in" |
            "surface_pattern" | "learned_surface_family" | "goal_exists" | "category_active" |
            "structure_exists" | "list_item_matches" | "scope_exists" | "fields_coexist" |
            "paths_present" | "path_unchanged" | "resolved_or_no_progress" |
            "attempt_repeated_without_progress"
        )
    }

    fn visit(condition: &Value, missing: &mut Vec<String>) {
        if let Some(all) = condition.get("all").and_then(Value::as_array) {
            for child in all { visit(child, missing); }
            return;
        }
        if let Some(any) = condition.get("any").and_then(Value::as_array) {
            for child in any { visit(child, missing); }
            return;
        }
        if let Some(child) = condition.get("not") {
            visit(child, missing);
            return;
        }
        let kind = canon(condition.get("type").and_then(Value::as_str).unwrap_or("always")).replace(' ', "_");
        if !kind.is_empty() && !supported(&kind) { missing.push(format!("condition:{kind}")); }
    }

    let mut missing = Vec::new();
    for condition in conditions { visit(condition, &mut missing); }
    missing.sort();
    missing.dedup();
    missing
}
