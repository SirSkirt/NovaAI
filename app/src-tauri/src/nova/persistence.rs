use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::AppHandle;

use crate::persistence::{nova_state_clear, nova_state_diagnostics, nova_state_load, nova_state_save};

use super::mind::{NovaMind, NovaMindState};
use super::semantics::ConversationContextTurn;

pub const PERSISTENT_MIND_SCHEMA: u32 = 1;
const APP_VERSION: &str = "7.0.0-alpha.9.3.1";

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MindPersistenceMetadata {
    pub schema_version: u32,
    pub app_version: String,
    pub saved_at: u128,
    pub reason: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistentMindEnvelope {
    pub persistence: MindPersistenceMetadata,
    pub mind: NovaMind,
    #[serde(default)]
    pub ui: Value,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MindBootstrapReceipt {
    pub restored: bool,
    pub migrated_legacy: bool,
    pub source: String,
    pub ui_state: Value,
    pub snapshot: NovaMind,
    pub diagnostics: Value,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MindCheckpointReceipt {
    pub saved: bool,
    pub reason: String,
    pub snapshot: NovaMind,
    pub diagnostics: Value,
}

fn now_ms() -> u128 {
    super::mind::unix_ms()
}

fn diagnostics_value(app: AppHandle) -> Value {
    nova_state_diagnostics(app)
        .ok()
        .and_then(|value| serde_json::to_value(value).ok())
        .unwrap_or_else(|| json!({}))
}

fn envelope_for(snapshot: NovaMind, ui: Value, reason: &str) -> PersistentMindEnvelope {
    PersistentMindEnvelope {
        persistence: MindPersistenceMetadata {
            schema_version: PERSISTENT_MIND_SCHEMA,
            app_version: APP_VERSION.to_string(),
            saved_at: now_ms(),
            reason: reason.to_string(),
        },
        mind: snapshot,
        ui,
    }
}

fn legacy_memory<'a>(root: &'a Value) -> Option<&'a Value> {
    root.get("context")?
        .get("cognition")?
        .get("memory")
}

fn legacy_state<T: for<'de> Deserialize<'de>>(root: &Value, key: &str) -> Option<T> {
    let candidate = legacy_memory(root)
        .and_then(|memory| memory.get(key))
        .or_else(|| root.get(key));
    candidate.and_then(|value| serde_json::from_value(value.clone()).ok())
}

fn legacy_conversation(root: &Value) -> Vec<ConversationContextTurn> {
    let Some(messages) = root.get("messages").and_then(Value::as_array) else {
        return Vec::new();
    };
    messages.iter().filter_map(|message| {
        let content = message.get("rawContent")
            .or_else(|| message.get("content"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_string();
        if content.is_empty() { return None; }
        let id = message.get("id").and_then(Value::as_str).unwrap_or("").to_string();
        let role = message.get("role").and_then(Value::as_str).unwrap_or("other").to_string();
        let source_label = message.get("sourceLabel").and_then(Value::as_str).map(str::to_string);
        let created_at = message.get("createdAt").and_then(Value::as_u64).map(u128::from).unwrap_or(0);
        Some(ConversationContextTurn { id, role, source_label, content, created_at })
    }).rev().take(160).collect::<Vec<_>>().into_iter().rev().collect()
}

fn legacy_ui(root: &Value) -> Value {
    json!({
        "messages": root.get("messages").cloned().unwrap_or_else(|| json!([])),
        "thinking": root.get("thinking").cloned().unwrap_or(Value::Null),
        "developerTrace": root.get("developerTrace").cloned().unwrap_or(Value::Null)
    })
}

fn migrate_legacy(root: &Value) -> NovaMind {
    let mut mind = NovaMind::default();
    if let Some(state) = legacy_state(root, "rustSemanticMemoryState") { mind.semantic_memory = state; }
    if let Some(state) = legacy_state(root, "rustLearningState") { mind.learning_state = state; }
    if let Some(state) = legacy_state(root, "rustDevelopmentState") { mind.development_state = state; }
    if let Some(state) = legacy_state(root, "rustInternalCognitionState") { mind.internal_cognition = state; }
    mind.conversation_history = legacy_conversation(root);
    mind.normalize_pass_nine();
    mind
}

fn parse_persisted(raw: &str) -> Result<(NovaMind, Value, bool), String> {
    let root: Value = serde_json::from_str(raw)
        .map_err(|error| format!("Could not parse Nova persistent state: {error}"))?;
    if let Some(mind_value) = root.get("mind") {
        let mut mind: NovaMind = serde_json::from_value(mind_value.clone())
            .map_err(|error| format!("Could not restore Rust-owned NovaMind: {error}"))?;
        mind.normalize_pass_nine();
        return Ok((mind, root.get("ui").cloned().unwrap_or_else(|| json!({})), false));
    }
    Ok((migrate_legacy(&root), legacy_ui(&root), true))
}

fn save_envelope(app: AppHandle, snapshot: NovaMind, ui: Value, reason: &str) -> Result<(), String> {
    let payload = serde_json::to_string(&envelope_for(snapshot, ui, reason))
        .map_err(|error| format!("Could not serialize Rust-owned Nova mind: {error}"))?;
    nova_state_save(app, payload).map(|_| ())
}

pub fn bootstrap(app: AppHandle, state: &NovaMindState) -> Result<MindBootstrapReceipt, String> {
    let raw = nova_state_load(app.clone())?;
    let (mind, ui_state, migrated_legacy, restored, source) = if let Some(raw) = raw {
        let (mind, ui, migrated) = parse_persisted(&raw)?;
        (mind, ui, migrated, true, if migrated { "legacy_v7_migration" } else { "rust_pass9" }.to_string())
    } else {
        let mut mind = NovaMind::default();
        mind.normalize_pass_nine();
        (mind, json!({}), false, false, "new_mind".to_string())
    };

    state.replace(mind)?;
    let snapshot = state.snapshot()?;
    if migrated_legacy {
        save_envelope(app.clone(), snapshot.clone(), ui_state.clone(), "pass9_legacy_migration")?;
    }

    Ok(MindBootstrapReceipt {
        restored,
        migrated_legacy,
        source,
        ui_state,
        snapshot,
        diagnostics: diagnostics_value(app),
    })
}

pub fn checkpoint(app: AppHandle, state: &NovaMindState, ui_state: Value, reason: &str) -> Result<MindCheckpointReceipt, String> {
    let snapshot = state.snapshot()?;
    save_envelope(app.clone(), snapshot.clone(), ui_state, reason)?;
    Ok(MindCheckpointReceipt {
        saved: true,
        reason: reason.to_string(),
        snapshot,
        diagnostics: diagnostics_value(app),
    })
}

pub fn reset(app: AppHandle, state: &NovaMindState, ui_state: Value) -> Result<MindCheckpointReceipt, String> {
    nova_state_clear(app.clone())?;
    let mut fresh = NovaMind::default();
    fresh.normalize_pass_nine();
    state.replace(fresh)?;
    checkpoint(app, state, ui_state, "explicit_reset")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pass_nine_restores_its_own_envelope_without_js_reconstruction() {
        let mut mind = NovaMind::default();
        mind.semantic_memory.initialized = true;
        mind.normalize_pass_nine();
        let raw = serde_json::to_string(&envelope_for(mind.clone(), json!({"messages":[]}), "test")).unwrap();
        let (restored, ui, migrated) = parse_persisted(&raw).unwrap();
        assert!(!migrated);
        assert_eq!(restored.architecture_version, super::super::mind::NOVA_RUST_ARCHITECTURE_VERSION);
        assert!(restored.semantic_memory.initialized);
        assert!(ui.get("messages").is_some());
    }

    #[test]
    fn alpha_9_2_restores_alpha_9_1_mind_without_teacher_state() {
        let mut old_mind = serde_json::to_value(NovaMind::default()).unwrap();
        old_mind.as_object_mut().unwrap().remove("teacherState");
        let raw = serde_json::to_string(&json!({
            "persistence": {
                "schemaVersion": PERSISTENT_MIND_SCHEMA,
                "appVersion": "7.0.0-alpha.9.1",
                "savedAt": 1,
                "reason": "upgrade_test"
            },
            "mind": old_mind,
            "ui": {"messages": []}
        })).unwrap();
        let (restored, _, migrated) = parse_persisted(&raw).unwrap();
        assert!(!migrated);
        assert_eq!(restored.schema_version, super::super::mind::NOVA_RUST_MIND_SCHEMA);
        assert!(restored.teacher_state.initialized);
        assert!(restored.teacher_state.attempts.is_empty());
    }


    #[test]
    fn alpha_9_3_restores_alpha_9_2_mind_without_basic_english_or_new_teacher_fields() {
        let mut old_mind = serde_json::to_value(NovaMind::default()).unwrap();
        let mind_object = old_mind.as_object_mut().unwrap();
        mind_object.insert("schemaVersion".to_string(), json!(11));
        mind_object.insert("architectureVersion".to_string(), json!("7.0.0-rust-authoritative-mind-alpha-9.2"));

        if let Some(cognitive) = mind_object.get_mut("cognitiveState").and_then(Value::as_object_mut) {
            cognitive.remove("englishBootstrap");
        }

        let mut old_report = serde_json::to_value(super::super::semantics::SemanticTurnReport::default()).unwrap();
        if let Some(report) = old_report.as_object_mut() {
            report.remove("basicEnglishUtteranceKind");
            report.remove("basicEnglishClauses");
        }
        mind_object.insert("lastSemanticReport".to_string(), old_report);

        let mut legacy_attempt = serde_json::to_value(super::super::teacher::TeacherAttemptRecord::default()).unwrap();
        if let Some(attempt) = legacy_attempt.as_object_mut() {
            attempt.insert("id".to_string(), json!("alpha92_attempt"));
            attempt.insert("runId".to_string(), json!("alpha92_run"));
            attempt.insert("suiteId".to_string(), json!("alpha92_suite"));
            attempt.insert("caseId".to_string(), json!("alpha92_case"));
            attempt.insert("sourceDisplayName".to_string(), json!("Alpha 9.2 Test"));
            attempt.insert("question".to_string(), json!("Old question?"));
            attempt.insert("originalAnswer".to_string(), json!("Old answer."));
            attempt.insert("automaticStatus".to_string(), json!("review"));
            attempt.insert("knowledgeId".to_string(), json!("alpha92_knowledge"));
            attempt.remove("lessonContext");
            attempt.remove("attemptNumber");
            attempt.remove("inspectorContext");
            attempt.remove("teacherPointsAwarded");
            attempt.remove("teacherPointsPossible");
            attempt.remove("teacherGrade");
        }
        if let Some(teacher) = mind_object.get_mut("teacherState").and_then(Value::as_object_mut) {
            teacher.insert("schemaVersion".to_string(), json!(1));
            teacher.insert("initialized".to_string(), json!(true));
            teacher.insert("attempts".to_string(), json!([legacy_attempt]));
        }

        let raw = serde_json::to_string(&json!({
            "persistence": {
                "schemaVersion": PERSISTENT_MIND_SCHEMA,
                "appVersion": "7.0.0-alpha.9.2",
                "savedAt": 1,
                "reason": "alpha_9_3_upgrade_test"
            },
            "mind": old_mind,
            "ui": {"messages": []}
        })).unwrap();

        let (restored, _, migrated) = parse_persisted(&raw).unwrap();
        assert!(!migrated);
        assert_eq!(restored.schema_version, super::super::mind::NOVA_RUST_MIND_SCHEMA);
        assert_eq!(restored.cognitive_state.english_bootstrap.version, "");
        assert_eq!(restored.last_semantic_report.as_ref().map(|row| row.basic_english_clauses), Some(0));
        assert_eq!(restored.teacher_state.schema_version, super::super::teacher::RUST_TEACHER_SCHEMA);
        let attempt = restored.teacher_state.attempts.first().expect("restored alpha 9.2 attempt");
        assert_eq!(attempt.attempt_number, 1);
        assert!(attempt.lesson_context.is_none());
        assert!(attempt.teacher_grade.is_none());
        assert!(attempt.inspector_context.is_null());
    }

    #[test]
    fn pass_nine_migrates_legacy_rust_mirrors_once_in_rust() {
        let semantic = super::super::semantics::knowledge::RustSemanticMemoryState::new_initialized();
        let legacy = json!({
            "messages":[{"id":"u1","role":"user","content":"hello","createdAt":10}],
            "context":{"cognition":{"memory":{
                "rustSemanticMemoryState": serde_json::to_value(semantic).unwrap()
            }}}
        });
        let raw = serde_json::to_string(&legacy).unwrap();
        let (mind, ui, migrated) = parse_persisted(&raw).unwrap();
        assert!(migrated);
        assert!(mind.semantic_memory.initialized);
        assert_eq!(mind.conversation_history.len(), 1);
        assert!(ui.get("messages").is_some());
    }
}
