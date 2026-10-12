use serde::Deserialize;
use serde_json::Value;
use tauri::{AppHandle, State};

use super::mind::{BeginTurnInput, FinishTurnInput, NovaMind, NovaMindState, TurnReceipt};
use super::persistence::{self as mind_persistence, MindBootstrapReceipt, MindCheckpointReceipt};
use super::teaching::{parse_source, ParsedTeachingSource, TeachingIngestionReceipt, TeachingSourceInput};
use super::teacher::{FinalizeTeacherRunInput, RecordTeacherAttemptInput, TeacherFeedbackReceipt, TeacherRunFinalizeReceipt, UpdateTeacherFeedbackInput};


#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FrontendTurnInput {
    pub text: String,
    pub source_label: Option<String>,
}

#[tauri::command]
pub fn nova_mind_snapshot(state: State<'_, NovaMindState>) -> Result<NovaMind, String> {
    state.snapshot()
}

#[tauri::command]
pub fn nova_mind_begin_turn(state: State<'_, NovaMindState>, input: FrontendTurnInput) -> Result<TurnReceipt, String> {
    if input.text.trim().is_empty() { return Err("Nova cannot begin an empty turn.".to_string()); }
    state.begin_turn(BeginTurnInput {
        text: input.text,
        source_label: input.source_label,
        ..Default::default()
    })
}

#[tauri::command]
pub fn nova_mind_parse_source(input: TeachingSourceInput) -> Result<ParsedTeachingSource, String> {
    if input.text.trim().is_empty() { return Err("Nova cannot parse an empty teaching source.".to_string()); }
    Ok(parse_source(&input))
}

#[tauri::command]
pub fn nova_mind_ingest_source(
    state: State<'_, NovaMindState>,
    input: TeachingSourceInput,
) -> Result<TeachingIngestionReceipt, String> {
    state.ingest_source(input)
}


#[tauri::command]
pub fn nova_teacher_record_attempt(
    state: State<'_, NovaMindState>,
    input: RecordTeacherAttemptInput,
) -> Result<TeacherFeedbackReceipt, String> {
    state.record_teacher_attempt(input)
}

#[tauri::command]
pub fn nova_teacher_finalize_run(
    state: State<'_, NovaMindState>,
    input: FinalizeTeacherRunInput,
) -> Result<TeacherRunFinalizeReceipt, String> {
    state.finalize_teacher_run(input)
}

#[tauri::command]
pub fn nova_teacher_update_feedback(
    state: State<'_, NovaMindState>,
    input: UpdateTeacherFeedbackInput,
) -> Result<TeacherFeedbackReceipt, String> {
    state.update_teacher_feedback(input)
}

#[tauri::command]
pub fn nova_mind_finish_turn(state: State<'_, NovaMindState>, input: FinishTurnInput) -> Result<NovaMind, String> {
    state.finish_turn(input)
}

#[tauri::command]
pub fn nova_mind_bootstrap(app: AppHandle, state: State<'_, NovaMindState>) -> Result<MindBootstrapReceipt, String> {
    mind_persistence::bootstrap(app, state.inner())
}

#[tauri::command]
pub fn nova_mind_checkpoint(app: AppHandle, state: State<'_, NovaMindState>, ui_state: Value, reason: Option<String>) -> Result<MindCheckpointReceipt, String> {
    let reason = reason.unwrap_or_else(|| "frontend_ui_checkpoint".to_string());
    mind_persistence::checkpoint(app, state.inner(), ui_state, &reason)
}

#[tauri::command]
pub fn nova_mind_reset(app: AppHandle, state: State<'_, NovaMindState>, ui_state: Value) -> Result<MindCheckpointReceipt, String> {
    mind_persistence::reset(app, state.inner(), ui_state)
}
