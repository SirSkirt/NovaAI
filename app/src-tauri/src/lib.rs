mod app_control;
mod persistence;
mod nova;

use app_control::AppControlState;
use nova::NovaMindState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppControlState::default())
        .manage(NovaMindState::default())
        .invoke_handler(tauri::generate_handler![
            app_control::app_control_list_windows,
            app_control::app_control_begin,
            app_control::app_control_end,
            app_control::app_control_status,
            app_control::app_control_focus,
            app_control::app_control_type_text,
            app_control::app_control_type_and_enter,
            app_control::app_control_press_enter,
            app_control::app_control_capture_selection,
            nova::commands::nova_mind_snapshot,
            nova::commands::nova_mind_begin_turn,
            nova::commands::nova_mind_parse_source,
            nova::commands::nova_mind_ingest_source,
            nova::commands::nova_teacher_record_attempt,
            nova::commands::nova_teacher_finalize_run,
            nova::commands::nova_teacher_update_feedback,
            nova::commands::nova_mind_finish_turn,
            nova::commands::nova_mind_bootstrap,
            nova::commands::nova_mind_checkpoint,
            nova::commands::nova_mind_reset
        ])
        .run(tauri::generate_context!())
        .expect("error while running Nova");
}
