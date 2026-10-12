use serde_json::{json, Value};

use crate::nova::cognition::path::{canon, get_path, get_path_cloned, resolve_template, set_path, value_text};
use crate::nova::mind::unix_ms;

fn scope_matches(scope: &Value, selector: &Value) -> bool {
    if let Some(scope_id) = selector.get("scopeId").and_then(Value::as_str) {
        if value_text(scope.get("id")) != scope_id { return false; }
    }
    if selector.get("nonRoot").and_then(Value::as_bool).unwrap_or(false)
        && scope.get("parentId").is_none_or(Value::is_null)
    {
        return false;
    }
    if let Some(kind) = selector.get("kind").and_then(Value::as_str) {
        if canon(&value_text(scope.get("kind"))) != canon(kind) { return false; }
    }
    if let Some(boundary) = selector.get("boundaryKind").and_then(Value::as_str) {
        if canon(&value_text(scope.get("boundaryKind"))) != canon(boundary) { return false; }
    }
    true
}

fn role_for_reference(reference: &Value) -> &'static str {
    match canon(&value_text(reference.get("grammaticalPerson"))).as_str() {
        "second" | "second_person" => "listener",
        "first_plural" | "first person plural" => "group",
        _ => "speaker",
    }
}

fn participant_target(participant: &Value) -> String {
    for key in ["label", "name", "id"] {
        let value = value_text(participant.get(key));
        if !value.is_empty() { return value; }
    }
    String::new()
}

fn scope_index(scopes: &[Value], id: &str) -> Option<usize> {
    scopes.iter().position(|scope| value_text(scope.get("id")) == id)
}

pub fn execute(op: &str, args: &Value, ctx: &mut Value) -> Result<bool, String> {
    match op {
        "scope_create_child" => {
            let mut scopes = get_path_cloned(ctx, "canonicalSemanticState.scopes")
                .and_then(|value| value.as_array().cloned())
                .unwrap_or_default();
            let active = value_text(get_path(ctx, "canonicalSemanticState.activeScopeId"));
            let parent = args.get("parentId")
                .map(|value| value_text(Some(&resolve_template(value, ctx))))
                .filter(|value| !value.is_empty())
                .or_else(|| args.get("parentPath").and_then(Value::as_str).and_then(|path| get_path_cloned(ctx, path)).map(|value| value_text(Some(&value))))
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| if active.is_empty() { "scope_discourse".to_string() } else { active });
            let identity = args.get("identity").or_else(|| args.get("id"))
                .map(|value| value_text(Some(&resolve_template(value, ctx))))
                .unwrap_or_default();
            if !identity.is_empty() {
                if let Some(existing) = scopes.iter().find(|scope| value_text(scope.get("id")) == identity).cloned() {
                    if let Some(path) = args.get("outputPath").and_then(Value::as_str) { set_path(ctx, path, existing); }
                    return Ok(false);
                }
            }
            let id = if identity.is_empty() { format!("scope_module_{}", unix_ms()) } else { identity };
            let parent_frame = scopes.iter().find(|scope| value_text(scope.get("id")) == parent)
                .and_then(|scope| scope.get("participantFrame")).cloned()
                .or_else(|| get_path_cloned(ctx, "canonicalSemanticState.participants"));
            let scope = json!({
                "id": id,
                "kind": args.get("kind").map(|value| value_text(Some(&resolve_template(value, ctx)))).filter(|value| !value.is_empty()).unwrap_or_else(|| "nested_scope".to_string()),
                "parentId": parent,
                "start": 0,
                "end": 0,
                "boundaryKind": args.get("boundaryKind").map(|value| value_text(Some(&resolve_template(value, ctx)))).filter(|value| !value.is_empty()).unwrap_or_else(|| "learned_boundary".to_string()),
                "participantFrame": args.get("participantFrame").map(|value| resolve_template(value, ctx)).or(parent_frame).unwrap_or(Value::Null),
                "policies": args.get("policies").map(|value| resolve_template(value, ctx)).unwrap_or_else(|| json!({})),
                "source": args.get("source").and_then(Value::as_str).unwrap_or("rust_learned_scope")
            });
            scopes.push(scope.clone());
            let mut changed = set_path(ctx, "canonicalSemanticState.scopes", Value::Array(scopes));
            if let Some(path) = args.get("outputPath").and_then(Value::as_str) { changed = set_path(ctx, path, scope) || changed; }
            Ok(changed)
        }
        "scope_set_active" => {
            let id = args.get("scopeId")
                .map(|value| value_text(Some(&resolve_template(value, ctx))))
                .or_else(|| args.get("fromPath").and_then(Value::as_str).and_then(|path| get_path_cloned(ctx, path)).map(|value| value_text(Some(&value))))
                .unwrap_or_default();
            if id.is_empty() { return Ok(false); }
            let exists = get_path(ctx, "canonicalSemanticState.scopes").and_then(Value::as_array)
                .is_some_and(|scopes| scopes.iter().any(|scope| value_text(scope.get("id")) == id));
            if !exists { return Ok(false); }
            Ok(set_path(ctx, "canonicalSemanticState.activeScopeId", Value::String(id)))
        }
        "scope_restore_parent" => {
            let current = value_text(get_path(ctx, "canonicalSemanticState.activeScopeId"));
            let parent = get_path(ctx, "canonicalSemanticState.scopes").and_then(Value::as_array)
                .and_then(|scopes| scopes.iter().find(|scope| value_text(scope.get("id")) == current))
                .and_then(|scope| scope.get("parentId")).and_then(Value::as_str).map(str::to_string);
            Ok(parent.is_some_and(|parent| set_path(ctx, "canonicalSemanticState.activeScopeId", Value::String(parent))))
        }
        "scope_apply_policy" => {
            let selector = args.get("selector").cloned().unwrap_or_else(|| json!({}));
            let policy = resolve_template(args.get("policy").unwrap_or(&json!({})), ctx);
            let Some(policy_object) = policy.as_object() else { return Ok(false); };
            let mut scopes = get_path_cloned(ctx, "canonicalSemanticState.scopes").and_then(|value| value.as_array().cloned()).unwrap_or_default();
            let mut changed = false;
            for scope in &mut scopes {
                if !scope_matches(scope, &selector) { continue; }
                let Some(scope_object) = scope.as_object_mut() else { continue; };
                let policies = scope_object.entry("policies").or_insert_with(|| json!({}));
                let Some(target) = policies.as_object_mut() else { continue; };
                for (key, value) in policy_object {
                    if target.get(key) != Some(value) {
                        target.insert(key.clone(), value.clone());
                        changed = true;
                    }
                }
            }
            if changed { set_path(ctx, "canonicalSemanticState.scopes", Value::Array(scopes)); }
            Ok(changed)
        }
        "scope_bind_participant" => {
            let selector = args.get("selector").cloned().unwrap_or_else(|| json!({}));
            let role = canon(args.get("role").and_then(Value::as_str).unwrap_or(""));
            if !matches!(role.as_str(), "speaker" | "listener" | "group") { return Ok(false); }
            let participant = args.get("participant")
                .map(|value| resolve_template(value, ctx))
                .or_else(|| args.get("fromPath").and_then(Value::as_str).and_then(|path| get_path_cloned(ctx, path)))
                .unwrap_or(Value::Null);
            if participant.is_null() { return Ok(false); }
            let normalized = if participant.is_object() {
                let mut value = participant;
                let target = participant_target(&value);
                if let Some(object) = value.as_object_mut() {
                    object.entry("role").or_insert(Value::String(role.clone()));
                    object.entry("id").or_insert_with(|| Value::String(target.clone()));
                    object.entry("label").or_insert_with(|| Value::String(target.clone()));
                }
                value
            } else {
                let text = value_text(Some(&participant));
                json!({"id":text,"label":text,"role":role})
            };
            let mut scopes = get_path_cloned(ctx, "canonicalSemanticState.scopes").and_then(|value| value.as_array().cloned()).unwrap_or_default();
            let mut changed = false;
            for scope in &mut scopes {
                if !scope_matches(scope, &selector) { continue; }
                let frame = scope.as_object_mut().and_then(|object| object.get_mut("participantFrame"));
                let Some(frame) = frame.and_then(Value::as_object_mut) else { continue; };
                if frame.get(&role) != Some(&normalized) {
                    frame.insert(role.clone(), normalized.clone());
                    changed = true;
                }
            }
            if changed { set_path(ctx, "canonicalSemanticState.scopes", Value::Array(scopes)); }
            Ok(changed)
        }
        "scope_enforce_isolation" | "scope_resolve_deictics" => {
            let selector = args.get("selector").cloned().unwrap_or_else(|| if op == "scope_enforce_isolation" { json!({"nonRoot":true}) } else { json!({}) });
            let scopes = get_path_cloned(ctx, "canonicalSemanticState.scopes").and_then(|value| value.as_array().cloned()).unwrap_or_default();
            let mut references = get_path_cloned(ctx, "canonicalSemanticState.references").and_then(|value| value.as_array().cloned()).unwrap_or_default();
            let allow_parent = args.get("allowParentFallback").and_then(Value::as_bool).unwrap_or(false);
            let mut changed = false;
            for reference in &mut references {
                let scope_id = value_text(reference.get("scopeId"));
                let Some(index) = scope_index(&scopes, &scope_id) else { continue; };
                if !scope_matches(&scopes[index], &selector) { continue; }
                let role = role_for_reference(reference);
                let mut participant = scopes[index].get("participantFrame").and_then(|frame| frame.get(role)).cloned();
                if participant.is_none() && allow_parent {
                    let mut parent_id = scopes[index].get("parentId").and_then(Value::as_str).map(str::to_string);
                    while let Some(parent) = parent_id {
                        let Some(parent_index) = scope_index(&scopes, &parent) else { break; };
                        participant = scopes[parent_index].get("participantFrame").and_then(|frame| frame.get(role)).cloned();
                        if participant.is_some() { break; }
                        parent_id = scopes[parent_index].get("parentId").and_then(Value::as_str).map(str::to_string);
                    }
                }
                if op == "scope_enforce_isolation" && participant.is_none() {
                    let Some(object) = reference.as_object_mut() else { continue; };
                    if object.get("resolvedEntity").is_some_and(|value| !value.is_null()) || object.get("participantRole").is_some_and(|value| !value.is_null()) || object.get("locked").and_then(Value::as_bool).unwrap_or(false) {
                        object.insert("resolvedEntity".into(), Value::Null);
                        object.insert("participantRole".into(), Value::Null);
                        object.insert("locked".into(), Value::Bool(false));
                        object.insert("resolutionSource".into(), Value::String("scoped_deictic_unresolved".into()));
                        changed = true;
                    }
                    continue;
                }
                let Some(participant) = participant else { continue; };
                let target = participant_target(&participant);
                if target.is_empty() { continue; }
                let Some(object) = reference.as_object_mut() else { continue; };
                if value_text(object.get("resolvedEntity")) != target || value_text(object.get("participantRole")) != role {
                    object.insert("resolvedEntity".into(), Value::String(target));
                    object.insert("participantRole".into(), Value::String(role.to_string()));
                    object.insert("resolutionSource".into(), Value::String("learned_scoped_frame".into()));
                    object.insert("locked".into(), Value::Bool(true));
                    changed = true;
                }
            }
            if changed { set_path(ctx, "canonicalSemanticState.references", Value::Array(references)); }
            Ok(changed)
        }
        "perspective_subject" => {
            let claim = args.get("claimPath").and_then(Value::as_str).and_then(|path| get_path_cloned(ctx, path)).or_else(|| get_path_cloned(ctx, "claim"));
            let owner = claim.as_ref().and_then(|claim| claim.get("qualifiers").and_then(|qualifiers| qualifiers.get("semanticOwnerId")).or_else(|| claim.get("semanticOwnerId"))).cloned()
                .or_else(|| args.get("ownerPath").and_then(Value::as_str).and_then(|path| get_path_cloned(ctx, path)));
            let owner = value_text(owner.as_ref());
            if owner.is_empty() { return Ok(false); }
            let active_scope = args.get("scopeId").and_then(Value::as_str).map(str::to_string).unwrap_or_else(|| value_text(get_path(ctx, "canonicalSemanticState.activeScopeId")));
            let scopes = get_path(ctx, "canonicalSemanticState.scopes").and_then(Value::as_array);
            let frame = scopes.and_then(|scopes| scopes.iter().find(|scope| value_text(scope.get("id")) == active_scope)).and_then(|scope| scope.get("participantFrame"));
            let speaker = frame.and_then(|frame| frame.get("speaker")).or_else(|| get_path(ctx, "canonicalSemanticState.participants.speaker"));
            let listener = frame.and_then(|frame| frame.get("listener")).or_else(|| get_path(ctx, "canonicalSemanticState.participants.listener"));
            let surface = if speaker.is_some_and(|participant| value_text(participant.get("id")) == owner) {
                args.get("firstPerson").and_then(Value::as_str).unwrap_or("I").to_string()
            } else if listener.is_some_and(|participant| value_text(participant.get("id")) == owner) {
                args.get("secondPerson").and_then(Value::as_str).unwrap_or("you").to_string()
            } else {
                claim.as_ref().and_then(|claim| claim.get("subjectLabel").or_else(|| claim.get("subject"))).map(|value| value_text(Some(value))).filter(|value| !value.is_empty()).unwrap_or(owner)
            };
            Ok(set_path(ctx, args.get("outputPath").and_then(Value::as_str).unwrap_or("surfaceSubject"), Value::String(surface)))
        }
        "resolve_participant" => {
            let role = canon(args.get("role").and_then(Value::as_str).unwrap_or("listener"));
            let participant = get_path_cloned(ctx, &format!("canonicalSemanticState.participants.{role}"));
            let Some(participant) = participant else { return Ok(false); };
            let target = participant_target(&participant);
            if target.is_empty() || get_path(ctx, "mention").is_none() { return Ok(false); }
            let mut changed = set_path(ctx, "mention.resolvedEntity", Value::String(target));
            changed = set_path(ctx, "mention.participantRole", Value::String(role)) || changed;
            changed = set_path(ctx, "mention.resolutionSource", Value::String("learned_module".into())) || changed;
            Ok(changed)
        }
        _ => Err("not_scopes".to_string()),
    }
}
