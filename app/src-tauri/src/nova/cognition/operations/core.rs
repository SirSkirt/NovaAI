use serde_json::{json, Value};

use crate::nova::cognition::path::{append_path, canon, delete_path, get_path, get_path_cloned, resolve_template, set_path, value_text};
use crate::nova::cognition::types::CognitiveModule;

pub fn for_each(args: &Value, step: &Value, ctx: &mut Value, modules: &[CognitiveModule], depth: usize) -> Result<bool, String> {
    let list_path = args.get("listPath").and_then(Value::as_str).unwrap_or("");
    let list = get_path_cloned(ctx, list_path).and_then(|v| v.as_array().cloned()).unwrap_or_default();
    let local_path = args.get("localPath").and_then(Value::as_str).map(str::to_string)
        .unwrap_or_else(|| format!("locals.{}", args.get("as").and_then(Value::as_str).unwrap_or("item")));
    let prior = get_path_cloned(ctx, &local_path);
    let program = args.get("program").or_else(|| step.get("program")).and_then(Value::as_array).cloned().unwrap_or_default();
    let mut changed = false;
    for (index, item) in list.into_iter().enumerate() {
        set_path(ctx, &local_path, item);
        set_path(ctx, &format!("{local_path}Index"), json!(index));
        changed = super::execute_program(&program, ctx, modules, depth + 1)? || changed;
    }
    if let Some(value) = prior { set_path(ctx, &local_path, value); } else { delete_path(ctx, &local_path); }
    Ok(changed)
}

fn value_key(value: &Value) -> String {
    if let Some(object) = value.as_object() {
        for key in ["id", "name", "label"] { if let Some(value) = object.get(key) { return canon(&value_text(Some(value))); } }
    }
    canon(&value_text(Some(value)))
}

pub fn execute(op: &str, args: &Value, ctx: &mut Value) -> Result<bool, String> {
    match op {
        "set_field" => {
            let value = resolve_template(args.get("value").unwrap_or(&Value::Null), ctx);
            Ok(set_path(ctx, args.get("path").and_then(Value::as_str).unwrap_or(""), value))
        }
        "append_field" => {
            let value = resolve_template(args.get("value").unwrap_or(&Value::Null), ctx);
            Ok(append_path(ctx, args.get("path").and_then(Value::as_str).unwrap_or(""), value, args.get("unique").and_then(Value::as_bool).unwrap_or(false)))
        }
        "copy_field" => {
            let from = args.get("fromPath").and_then(Value::as_str).unwrap_or("");
            let to = args.get("toPath").or_else(|| args.get("path")).and_then(Value::as_str).unwrap_or("");
            Ok(get_path_cloned(ctx, from).is_some_and(|value| set_path(ctx, to, value)))
        }
        "remove_field" => Ok(delete_path(ctx, args.get("path").and_then(Value::as_str).unwrap_or(""))),
        "count_items" => {
            let path = args.get("listPath").and_then(Value::as_str).unwrap_or("");
            let count = get_path(ctx, path).map(|v| v.as_array().map_or(1, Vec::len)).unwrap_or(0);
            Ok(set_path(ctx, args.get("outputPath").and_then(Value::as_str).unwrap_or("count"), json!(count)))
        }
        "compare_values" => {
            let left = args.get("left").map(|v| resolve_template(v, ctx)).or_else(|| args.get("leftPath").and_then(Value::as_str).and_then(|p| get_path_cloned(ctx, p))).unwrap_or(Value::Null);
            let right = args.get("right").map(|v| resolve_template(v, ctx)).or_else(|| args.get("rightPath").and_then(Value::as_str).and_then(|p| get_path_cloned(ctx, p))).unwrap_or(Value::Null);
            let relation = if left == right { "equal" } else { "not_equal" };
            Ok(set_path(ctx, args.get("outputPath").and_then(Value::as_str).unwrap_or("comparison"), json!({"left":left,"right":right,"relation":relation})))
        }
        "unique_list" => {
            let path = args.get("outputPath").or_else(|| args.get("listPath")).or_else(|| args.get("path")).and_then(Value::as_str).unwrap_or("");
            let items = get_path_cloned(ctx, args.get("listPath").or_else(|| args.get("path")).and_then(Value::as_str).unwrap_or("")).and_then(|v| v.as_array().cloned()).unwrap_or_default();
            let mut out = Vec::new();
            for item in items { if !out.iter().any(|existing| existing == &item) { out.push(item); } }
            Ok(set_path(ctx, path, Value::Array(out)))
        }
        "intersect_list" => {
            let target = args.get("targetPath").and_then(Value::as_str).unwrap_or("");
            let allowed = args.get("allowedPath").and_then(Value::as_str).unwrap_or("");
            let current = get_path_cloned(ctx, target).and_then(|v| v.as_array().cloned()).unwrap_or_default();
            let allowed = get_path_cloned(ctx, allowed).and_then(|v| v.as_array().cloned()).unwrap_or_default();
            let allowed_keys: std::collections::BTreeSet<_> = allowed.iter().map(value_key).collect();
            Ok(set_path(ctx, target, Value::Array(current.into_iter().filter(|item| allowed_keys.contains(&value_key(item))).collect())))
        }
        "preserve_field" | "preserve_structure" => {
            let path = args.get("path").and_then(Value::as_str).unwrap_or("");
            let key = args.get("key").and_then(Value::as_str).unwrap_or(path);
            if get_path(ctx, &format!("moduleState.preserved.{key}")).is_some() { return Ok(false); }
            Ok(get_path_cloned(ctx, path).is_some_and(|value| set_path(ctx, &format!("moduleState.preserved.{key}"), value)))
        }
        "restore_field" | "restore_structure" => {
            let path = args.get("path").and_then(Value::as_str).unwrap_or("");
            let key = args.get("key").and_then(Value::as_str).unwrap_or(path);
            Ok(get_path_cloned(ctx, &format!("moduleState.preserved.{key}")).is_some_and(|value| set_path(ctx, path, value)))
        }
        "ensure_field_equals" => {
            let source = args.get("sourcePath").and_then(Value::as_str).unwrap_or("");
            let target = args.get("targetPath").or_else(|| args.get("path")).and_then(Value::as_str).unwrap_or("");
            Ok(get_path_cloned(ctx, source).is_some_and(|value| set_path(ctx, target, value)))
        }
        "add_constraint" => {
            let value = resolve_template(args.get("value").unwrap_or(&Value::Null), ctx);
            Ok(append_path(ctx, "constraints", value, true))
        }
        "mark_uncertain" => {
            let reason = resolve_template(args.get("reason").unwrap_or(&Value::Null), ctx);
            Ok(set_path(ctx, "uncertainty", json!({"status":"uncertain","confidence":args.get("confidence").and_then(Value::as_f64).unwrap_or(0.45),"reason":reason})))
        }
        "emit_surface" => {
            let value = args.get("text").or_else(|| args.get("value")).map(|v| resolve_template(v, ctx)).unwrap_or(Value::String(String::new()));
            Ok(set_path(ctx, args.get("outputPath").and_then(Value::as_str).unwrap_or("moduleState.surfaceResponse"), Value::String(value_text(Some(&value)))))
        }
        "register_category" => {
            let value = args.get("name").or_else(|| args.get("category")).map(|v| resolve_template(v, ctx)).unwrap_or(Value::Null);
            let name = value_text(Some(&value)); if name.trim().is_empty() { return Ok(false); }
            let key = canon(&name);
            let mut rows = get_path_cloned(ctx, "developmentalState.categories").and_then(|v| v.as_array().cloned()).unwrap_or_default();
            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d|d.as_millis()).unwrap_or(0);
            let traits = args.get("traits").map(|v| resolve_template(v,ctx)).and_then(|v|v.as_array().cloned()).unwrap_or_default();
            let evidence = args.get("evidence").map(|v| resolve_template(v,ctx)).and_then(|v|v.as_array().cloned()).unwrap_or_default();
            if rows.iter().any(|r| canon(r.get("key").or_else(||r.get("name")).and_then(Value::as_str).unwrap_or(""))==key) {
                return Ok(false);
            }
            let id=crate::nova::ids::stable_hash_id("category",&key);
            rows.push(json!({"id":id,"name":name,"key":key,"origin":args.get("origin").and_then(Value::as_str).unwrap_or("learned"),"status":args.get("status").and_then(Value::as_str).unwrap_or("ACTIVE"),"traits":traits,"evidence":evidence,"support":1,"createdAt":now,"updatedAt":now}));
            Ok(set_path(ctx,"developmentalState.categories",Value::Array(rows)))
        }
        "relate_categories" => {
            let subject=args.get("subject").map(|v|resolve_template(v,ctx)).map(|v|value_text(Some(&v))).unwrap_or_default();
            let predicate=args.get("predicate").map(|v|resolve_template(v,ctx)).map(|v|value_text(Some(&v))).unwrap_or_else(||"related_to".into());
            let object=args.get("object").map(|v|resolve_template(v,ctx)).map(|v|value_text(Some(&v))).unwrap_or_default();
            if subject.trim().is_empty()||object.trim().is_empty(){return Ok(false);}
            let subject_created = execute("register_category", &json!({"name":subject.clone(),"origin":args.get("origin").and_then(Value::as_str).unwrap_or("learned")}), ctx)?;
            let object_created = execute("register_category", &json!({"name":object.clone(),"origin":args.get("origin").and_then(Value::as_str).unwrap_or("learned")}), ctx)?;
            let s=canon(&subject);let p=canon(&predicate);let o=canon(&object);let sig=format!("{s}|{p}|{o}");let id=crate::nova::ids::stable_hash_id("catrel",&sig);
            let mut rows=get_path_cloned(ctx,"developmentalState.relations").and_then(|v|v.as_array().cloned()).unwrap_or_default();let now=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d|d.as_millis()).unwrap_or(0);
            if rows.iter().any(|r|r.get("id").and_then(Value::as_str)==Some(id.as_str())){return Ok(subject_created||object_created);}
            rows.push(json!({"id":id,"subject":s,"predicate":p,"object":o,"origin":args.get("origin").and_then(Value::as_str).unwrap_or("learned"),"evidence":[],"support":1,"createdAt":now,"updatedAt":now}));
            Ok(set_path(ctx,"developmentalState.relations",Value::Array(rows)))
        }
        "activate_category" => {
            let value = args.get("name").or_else(|| args.get("category")).map(|v| resolve_template(v, ctx)).unwrap_or(Value::Null);
            let name = value_text(Some(&value));
            if name.is_empty() { return Ok(false); }
            let mut changed = append_path(ctx, "activeCategories", Value::String(name.clone()), true);
            changed = append_path(ctx, "canonicalSemanticState.activeCategories", Value::String(name), true) || changed;
            Ok(changed)
        }
        "select_matching_item" => {
            let list = get_path_cloned(ctx, args.get("listPath").and_then(Value::as_str).unwrap_or("")).and_then(|v| v.as_array().cloned()).unwrap_or_default();
            let wanted = args.get("match").cloned().unwrap_or(Value::Null);
            let selected = list.into_iter().find(|item| wanted.as_object().is_none_or(|spec| spec.iter().all(|(path, expected)| canon(&value_text(get_path(item, path))) == canon(&value_text(Some(&resolve_template(expected, ctx)))))));
            Ok(selected.is_some_and(|item| set_path(ctx, args.get("outputPath").and_then(Value::as_str).unwrap_or("selectedItem"), item)))
        }
        "retrieve_source_evidence" => {
            let value = args.get("fromPath").and_then(Value::as_str).and_then(|p| get_path_cloned(ctx,p)).or_else(|| get_path_cloned(ctx,"rawText"));
            Ok(value.is_some_and(|value| set_path(ctx, args.get("outputPath").and_then(Value::as_str).unwrap_or("moduleState.sourceEvidence"), value)))
        }
        "preserve_ordered_items" => {
            let list = get_path_cloned(ctx, args.get("listPath").and_then(Value::as_str).unwrap_or("")).and_then(|v| v.as_array().cloned()).unwrap_or_default();
            let copy = list.into_iter().enumerate().map(|(index,value)| json!({"index":index,"value":value})).collect();
            Ok(set_path(ctx, args.get("outputPath").and_then(Value::as_str).unwrap_or("moduleState.orderedItems"), Value::Array(copy)))
        }
        "require_preconditions" => {
            let paths = args.get("paths").and_then(Value::as_array).cloned().unwrap_or_default();
            let missing: Vec<_> = paths.iter().filter_map(Value::as_str).filter(|path| get_path(ctx,path).is_none_or(Value::is_null)).map(|s| Value::String(s.to_string())).collect();
            let satisfied = missing.is_empty();
            Ok(set_path(ctx, args.get("outputPath").and_then(Value::as_str).unwrap_or("moduleState.preconditions"), json!({"required":paths,"missing":missing,"satisfied":satisfied})))
        }
        "collect_candidates" => {
            let mut out = Vec::new();
            for source in ["canonicalSemanticState.entities", "recentEntities"] {
                if let Some(items) = get_path(ctx, source).and_then(Value::as_array) {
                    for item in items { if !out.iter().any(|existing| value_key(existing)==value_key(item)) { out.push(item.clone()); } }
                }
            }
            Ok(set_path(ctx, args.get("outputPath").and_then(Value::as_str).unwrap_or("candidates"), Value::Array(out)))
        }
        "filter_candidates" | "score_candidates" | "score_candidates_by_relationship" | "measure_candidate_support" | "select_best_candidate" | "assign_reference" => Err("not_core".to_string()),
        _ => Err("not_core".to_string()),
    }
}
