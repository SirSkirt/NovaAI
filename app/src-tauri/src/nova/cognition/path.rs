use serde_json::{Map, Value};

pub fn canon(value: &str) -> String {
    value.trim().to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ")
}


const CANONICAL_ALIAS_ROOTS: &[&str] = &[
    "participants","references","discourseContext","structures","cognitiveOutcome","unresolved","scopes","activeScopeId",
    "constraints","entities","events","relations","propositions","requests","unknownSlots","candidateSets","evidence",
    "conclusions","categories","activeCategories","epistemic","mentalState","activities"
];

fn runtime_path(root: &Value, path: &str) -> String {
    if root.get("canonicalSemanticState").is_none() { return path.to_string(); }
    if path == "cognitiveState" { return "canonicalSemanticState".to_string(); }
    if let Some(rest) = path.strip_prefix("cognitiveState.") { return format!("canonicalSemanticState.{rest}"); }
    let first = path.split('.').next().unwrap_or("");
    if CANONICAL_ALIAS_ROOTS.contains(&first) { format!("canonicalSemanticState.{path}") } else { path.to_string() }
}

fn safe_parts(path: &str) -> Vec<&str> {
    path.split('.')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .filter(|part| !matches!(*part, "__proto__" | "prototype" | "constructor"))
        .collect()
}

pub fn get_path<'a>(root: &'a Value, path: &str) -> Option<&'a Value> {
    let path = runtime_path(root, path);
    let mut current = root;
    for part in safe_parts(&path) {
        current = match current {
            Value::Object(object) => object.get(part)?,
            Value::Array(array) => array.get(part.parse::<usize>().ok()?)?,
            _ => return None,
        };
    }
    Some(current)
}

pub fn get_path_cloned(root: &Value, path: &str) -> Option<Value> {
    get_path(root, path).cloned()
}

pub fn set_path(root: &mut Value, path: &str, value: Value) -> bool {
    let path = runtime_path(root, path);
    let parts = safe_parts(&path);
    if parts.is_empty() { return false; }
    if !root.is_object() { *root = Value::Object(Map::new()); }

    let mut current = root;
    for part in &parts[..parts.len() - 1] {
        match current {
            Value::Object(object) => {
                let entry = object
                    .entry((*part).to_string())
                    .or_insert_with(|| Value::Object(Map::new()));
                if !entry.is_object() && !entry.is_array() {
                    *entry = Value::Object(Map::new());
                }
                current = entry;
            }
            Value::Array(array) => {
                let Ok(index) = part.parse::<usize>() else { return false; };
                let Some(entry) = array.get_mut(index) else { return false; };
                if !entry.is_object() && !entry.is_array() {
                    *entry = Value::Object(Map::new());
                }
                current = entry;
            }
            _ => return false,
        }
    }

    let last = parts[parts.len() - 1];
    match current {
        Value::Object(object) => {
            if object.get(last) == Some(&value) { return false; }
            object.insert(last.to_string(), value);
            true
        }
        Value::Array(array) => {
            let Ok(index) = last.parse::<usize>() else { return false; };
            if index < array.len() {
                if array.get(index) == Some(&value) { return false; }
                array[index] = value;
                true
            } else if index == array.len() {
                array.push(value);
                true
            } else {
                false
            }
        }
        _ => false,
    }
}

pub fn delete_path(root: &mut Value, path: &str) -> bool {
    let path = runtime_path(root, path);
    let parts = safe_parts(&path);
    if parts.is_empty() { return false; }
    let mut current = root;
    for part in &parts[..parts.len() - 1] {
        current = match current {
            Value::Object(object) => match object.get_mut(*part) {
                Some(next) => next,
                None => return false,
            },
            Value::Array(array) => {
                let Ok(index) = part.parse::<usize>() else { return false; };
                match array.get_mut(index) {
                    Some(next) => next,
                    None => return false,
                }
            }
            _ => return false,
        };
    }

    let last = parts[parts.len() - 1];
    match current {
        Value::Object(object) => object.remove(last).is_some(),
        Value::Array(array) => {
            let Ok(index) = last.parse::<usize>() else { return false; };
            if index >= array.len() { return false; }
            array.remove(index);
            true
        }
        _ => false,
    }
}

pub fn append_path(root: &mut Value, path: &str, value: Value, unique: bool) -> bool {
    if get_path(root, path).and_then(Value::as_array).is_none() {
        set_path(root, path, Value::Array(Vec::new()));
    }
    let Some(array) = get_path_mut(root, path).and_then(Value::as_array_mut) else { return false; };
    if unique && array.iter().any(|item| item == &value) { return false; }
    array.push(value);
    true
}

pub fn get_path_mut<'a>(root: &'a mut Value, path: &str) -> Option<&'a mut Value> {
    let path = runtime_path(root, path);
    let parts = safe_parts(&path);
    if parts.is_empty() { return Some(root); }
    let mut current = root;
    for part in parts {
        current = match current {
            Value::Object(object) => object.get_mut(part)?,
            Value::Array(array) => array.get_mut(part.parse::<usize>().ok()?)?,
            _ => return None,
        };
    }
    Some(current)
}

pub fn value_text(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Bool(flag)) => flag.to_string(),
        Some(Value::Number(number)) => number.to_string(),
        Some(Value::Null) | None => String::new(),
        Some(other) => other.to_string(),
    }
}

pub fn resolve_template(value: &Value, ctx: &Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.iter().map(|item| resolve_template(item, ctx)).collect()),
        Value::Object(object) => {
            if object.len() == 1 {
                if let Some(path) = object.get("fromPath").and_then(Value::as_str) {
                    return get_path_cloned(ctx, path).unwrap_or(Value::Null);
                }
                if let Some(name) = object.get("fromCapture").and_then(Value::as_str) {
                    return get_path_cloned(ctx, &format!("captures.{name}"))
                        .or_else(|| get_path_cloned(ctx, &format!("patternCaptures.{name}")))
                        .unwrap_or(Value::Null);
                }
            }
            let mut out = Map::new();
            for (key, child) in object { out.insert(key.clone(), resolve_template(child, ctx)); }
            Value::Object(out)
        }
        Value::String(text) => {
            let mut result = text.clone();
            for marker in ["{{", "{%"] {
                loop {
                    let Some(start) = result.find(marker) else { break; };
                    let close = if marker == "{{" { "}}" } else { "%}" };
                    let Some(rel_end) = result[start + 2..].find(close) else { break; };
                    let end = start + 2 + rel_end;
                    let key = result[start + 2..end].trim();
                    let replacement = if marker == "{{" {
                        value_text(get_path(ctx, key))
                    } else {
                        value_text(get_path(ctx, &format!("captures.{key}")))
                    };
                    result.replace_range(start..end + 2, &replacement);
                }
            }
            Value::String(result)
        }
        other => other.clone(),
    }
}

