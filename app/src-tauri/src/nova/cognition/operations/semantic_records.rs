use serde_json::{json, Map, Value};

use crate::nova::cognition::path::{get_path, value_text};
use crate::nova::mind::unix_ms;

fn text(value: &Value, keys: &[&str]) -> String {
    for key in keys {
        let candidate = value_text(value.get(*key));
        if !candidate.is_empty() { return candidate; }
    }
    String::new()
}

fn active_scope(ctx: &Value) -> String {
    let value = value_text(get_path(ctx, "canonicalSemanticState.activeScopeId"));
    if value.is_empty() { "scope_discourse".to_string() } else { value }
}

fn frame_id(value: &Value) -> String {
    let frame = text(value, &["frameId"]);
    if frame.is_empty() { "frame_discourse".to_string() } else { frame }
}

fn generated_id(bucket: &str, ctx: &Value) -> String {
    let count = get_path(ctx, &format!("canonicalSemanticState.{bucket}"))
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    format!("rust_module_{bucket}_{}_{}", unix_ms(), count + 1)
}

fn original_attributes(value: &Value) -> Value {
    value.as_object().map(|object| Value::Object(object.clone())).unwrap_or_else(|| json!({"value":value}))
}

fn optional_text(value: &Value, keys: &[&str]) -> Value {
    let result = text(value, keys);
    if result.is_empty() { Value::Null } else { Value::String(result) }
}

/// Preserve learned fields that are not part of the fixed canonical schema.
/// Typed Rust records use `serde(flatten)` for these extensions so learned
/// cognition can carry new structural fields without requiring source changes.
fn merge_extensions(mut normalized: Value, original: &Value) -> Value {
    let (Some(out), Some(input)) = (normalized.as_object_mut(), original.as_object()) else {
        return normalized;
    };
    for (key, value) in input {
        if !out.contains_key(key) {
            out.insert(key.clone(), value.clone());
        }
    }
    normalized
}

pub fn normalize(bucket: &str, value: Value, ctx: &Value) -> Result<Value, String> {
    let original = value.clone();
    let object = value.as_object().cloned().unwrap_or_else(Map::new);
    let value = Value::Object(object);
    let id = text(&value, &["id"]);
    let id = if id.is_empty() { generated_id(bucket, ctx) } else { id };
    let source = {
        let source = text(&value, &["source"]);
        if source.is_empty() { "rust_learned_module".to_string() } else { source }
    };
    let frame = frame_id(&value);

    let normalized = match bucket {
        "frames" => {
            let kind = text(&value, &["kind", "type"]);
            let kind = if kind.is_empty() { "learned_frame".to_string() } else { kind };
            let identity_key = text(&value, &["identityKey"]);
            let identity_key = if identity_key.is_empty() { id.clone() } else { identity_key };
            let scope_id = text(&value, &["scopeId"]);
            let scope_id = if scope_id.is_empty() { active_scope(ctx) } else { scope_id };
            let commitment = text(&value, &["commitment"]);
            let commitment = if commitment.is_empty() { "learned".to_string() } else { commitment };

            json!({
                "id": id,
                "identityKey": identity_key,
                "kind": kind,
                "parentId": optional_text(&value, &["parentId"]),
                "scopeId": scope_id,
                "operatorSurface": optional_text(&value, &["operatorSurface"]),
                "commitment": commitment,
                "sourceSentenceId": optional_text(&value, &["sourceSentenceId"]),
                "source": source
            })
        }
        "entities" => {
            let label = text(&value, &["label", "name", "surface", "entity"]);
            let kind = text(&value, &["kind", "type"]);
            let kind = if kind.is_empty() { "learned_entity".to_string() } else { kind };
            let identity_key = text(&value, &["identityKey"]);
            let identity_key = if identity_key.is_empty() {
                format!("{}|{}", frame, label)
            } else {
                identity_key
            };

            json!({
                "id": id,
                "identityKey": identity_key,
                "frameId": frame,
                "label": label,
                "name": optional_text(&value, &["name"]),
                "kind": kind,
                "conceptId": optional_text(&value, &["conceptId"]),
                "source": source,
                "attributes": original_attributes(&original)
            })
        }
        "events" => {
            let predicate = text(&value, &["predicateKey", "predicateSurface", "predicate", "type"]);
            let identity_key = text(&value, &["identityKey"]);
            let identity_key = if identity_key.is_empty() {
                format!("{}|{}|{}", frame, predicate, id)
            } else {
                identity_key
            };
            let predicate_surface = text(&value, &["predicateSurface", "predicate", "predicateKey"]);
            let predicate_surface = if predicate_surface.is_empty() {
                predicate.clone()
            } else {
                predicate_surface
            };

            json!({
                "id": id,
                "identityKey": identity_key,
                "frameId": frame,
                "predicateSurface": predicate_surface,
                "predicateKey": predicate,
                "leftArgumentId": optional_text(&value, &["leftArgumentId"]),
                "rightArgumentId": optional_text(&value, &["rightArgumentId"]),
                "actorId": optional_text(&value, &["actorId"]),
                "objectId": optional_text(&value, &["objectId"]),
                "roles": value.get("roles").cloned().unwrap_or_else(|| json!({})),
                "source": source
            })
        }
        "relations" => {
            let relation_type = text(&value, &["type", "predicate", "relation"]);
            let relation_type = if relation_type.is_empty() {
                "related_to".to_string()
            } else {
                relation_type
            };

            json!({
                "id": id,
                "frameId": frame,
                "type": relation_type,
                "subjectId": optional_text(&value, &["subjectId"]),
                "objectId": optional_text(&value, &["objectId"]),
                "eventId": optional_text(&value, &["eventId"]),
                "subject": optional_text(&value, &["subject"]),
                "object": optional_text(&value, &["object"]),
                "source": source,
                "attributes": value
                    .get("attributes")
                    .cloned()
                    .or_else(|| value.get("qualifiers").cloned())
                    .unwrap_or_else(|| original_attributes(&original))
            })
        }
        "propositions" => {
            let proposition_type = text(&value, &["type"]);
            let proposition_type = if proposition_type.is_empty() {
                "learned_proposition".to_string()
            } else {
                proposition_type
            };
            let polarity = text(&value, &["polarity"]);
            let polarity = if polarity.is_empty() {
                "positive".to_string()
            } else {
                polarity
            };

            json!({
                "id": id,
                "frameId": frame,
                "type": proposition_type,
                "subject": optional_text(&value, &["subject"]),
                "subjectId": optional_text(&value, &["subjectId"]),
                "predicate": optional_text(&value, &["predicate"]),
                "object": optional_text(&value, &["object", "value"]),
                "objectId": optional_text(&value, &["objectId"]),
                "text": optional_text(&value, &["text", "surface"]),
                "polarity": polarity,
                "confidence": value.get("confidence").and_then(Value::as_f64).unwrap_or(0.7),
                "source": source,
                "evidence": value.get("evidence").cloned().unwrap_or_else(|| json!([])),
                "support": value.get("support").cloned().unwrap_or_else(|| json!({})),
                "attributes": original_attributes(&original)
            })
        }
        "requests" => {
            let request_type = text(&value, &["type", "requestType"]);
            let request_type = if request_type.is_empty() {
                "learned_request".to_string()
            } else {
                request_type
            };
            let speech_act = text(&value, &["speechAct"]);
            let speech_act = if speech_act.is_empty() {
                "request_candidate".to_string()
            } else {
                speech_act
            };
            let answer_relation = text(&value, &["answerRelation", "requestedRole"]);
            let answer_relation = if answer_relation.is_empty() {
                "unspecified".to_string()
            } else {
                answer_relation
            };
            let question = text(&value, &["questionSurface", "surface", "text"]);
            let question_surface = if question.is_empty() {
                value_text(get_path(ctx, "rawText"))
            } else {
                question
            };
            let status = text(&value, &["status"]);
            let status = if status.is_empty() { "open".to_string() } else { status };

            json!({
                "id": id,
                "type": request_type,
                "speechAct": speech_act,
                "frameId": frame,
                "targetFrameId": optional_text(&value, &["targetFrameId"]),
                "target": optional_text(&value, &["target"]),
                "targetRequestId": optional_text(&value, &["targetRequestId"]),
                "answerRelation": answer_relation,
                "unknownSlotId": optional_text(&value, &["unknownSlotId"]),
                "questionSurface": question_surface,
                "status": status,
                "answerEntityId": optional_text(&value, &["answerEntityId"]),
                "source": source,
                "eventPattern": value.get("eventPattern").cloned().unwrap_or_else(|| json!({})),
                "attributePattern": value.get("attributePattern").cloned().unwrap_or_else(|| json!({})),
                "constraints": value.get("constraints").cloned().unwrap_or_else(|| json!([])),
                "attributes": original_attributes(&original)
            })
        }
        "unknownSlots" => {
            let role = text(&value, &["role"]);
            let role = if role.is_empty() { "unknown".to_string() } else { role };
            let value_type = text(&value, &["valueType", "kind", "type"]);
            let value_type = if value_type.is_empty() {
                "unknown".to_string()
            } else {
                value_type
            };
            let status = text(&value, &["status"]);
            let status = if status.is_empty() {
                "unresolved".to_string()
            } else {
                status
            };
            let source_surface = text(&value, &["sourceSurface", "surface"]);
            let source_surface = if source_surface.is_empty() {
                value_text(get_path(ctx, "rawText"))
            } else {
                source_surface
            };

            json!({
                "id": id,
                "frameId": frame,
                "role": role,
                "valueType": value_type,
                "status": status,
                "sourceSurface": source_surface,
                "attributes": original_attributes(&original)
            })
        }
        "candidateSets" => json!({
            "id": id,
            "frameId": frame,
            "forUnknownSlotId": text(&value, &["forUnknownSlotId", "unknownSlotId"]),
            "entityIds": value.get("entityIds").cloned().unwrap_or_else(|| json!([])),
            "source": source,
            "constraints": value.get("constraints").cloned().unwrap_or_else(|| json!([]))
        }),
        "evidence" => {
            let source_type = text(&value, &["sourceType", "type"]);
            let source_type = if source_type.is_empty() {
                "learned_module".to_string()
            } else {
                source_type
            };

            json!({
                "id": id,
                "sourceType": source_type,
                "sourceId": optional_text(&value, &["sourceId"]),
                "content": optional_text(&value, &["content", "text", "value"]),
                "attributes": original_attributes(&original)
            })
        }
        "conclusions" => {
            let conclusion_type = text(&value, &["type"]);
            let conclusion_type = if conclusion_type.is_empty() {
                "learned_conclusion".to_string()
            } else {
                conclusion_type
            };

            json!({
                "id": id,
                "type": conclusion_type,
                "requestId": optional_text(&value, &["requestId"]),
                "unknownSlotId": optional_text(&value, &["unknownSlotId"]),
                "valueId": optional_text(&value, &["valueId"]),
                "value": optional_text(&value, &["value"]),
                "surface": optional_text(&value, &["surface", "response", "text"]),
                "rustAuthoritative": false,
                "source": source,
                "evidence": value.get("evidence").cloned().unwrap_or_else(|| json!([])),
                "support": value.get("support").cloned().unwrap_or_else(|| json!({})),
                "attributes": original_attributes(&original)
            })
        }
        _ => return Err(format!("unsupported_semantic_bucket:{bucket}")),
    };

    let normalized = if matches!(bucket, "requests" | "unknownSlots" | "conclusions") {
        merge_extensions(normalized, &original)
    } else {
        normalized
    };
    Ok(normalized)
}
