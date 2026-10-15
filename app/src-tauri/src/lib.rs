// Large, structured teacher-only epoch diagnostics include separate current,
// baseline, autonomous and archived-retention evidence. serde_json::json!
// expands recursively; raise only the macro expansion budget, not neural
// inference depth or the model's recurrent reasoning steps.
#![recursion_limit = "256"]

mod neural;
mod transformer_lab;
mod hybrid_lab;
mod nova2_lab;
mod nova2_tests;
mod targeted_lab;
mod test_lab;
mod diagnostic_log;
mod training_observer;
mod rehearsal;
mod lesson_zip;
mod checkpoint_transfer;
#[cfg(target_os="android")]
mod android_checkpoint_export;
#[cfg(target_os="android")]
mod training_keepalive;
#[cfg(target_os="android")]
mod android_lesson_notification;

use nova2_lab::Nova2State;
use nova2_tests::Nova2TestState;
use diagnostic_log::Diagnostics;
use checkpoint_transfer::TransferState;
use lesson_zip::ZipQueueState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder=tauri::Builder::default();
    #[cfg(target_os="android")]
    let builder=builder
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_background_service::init_with_service(
            || training_keepalive::TrainingKeepalive
        ));
    builder
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(Nova2State::default())
        .manage(Nova2TestState::default())
        .manage(ZipQueueState::default())
        .manage(Diagnostics::default())
        .manage(TransferState::default())
        .invoke_handler(tauri::generate_handler![
            nova2_lab::nova2_boot,
            nova2_lab::nova2_status,
            nova2_lab::nova2_conversation,
            nova2_lab::nova2_new_chat,
            nova2_lab::nova2_chat,
            nova2_lab::nova2_compare_context,
            nova2_lab::nova2_inspect,
            nova2_lab::nova2_reset,
            nova2_lab::nova2_train,
            nova2_lab::nova2_zip_train,
            nova2_lab::nova2_stop,
            nova2_lab::nova2_targeted_train,
            nova2_lab::nova2_targeted_discard,
            nova2_lab::nova2_targeted_accept,
            nova2_lab::nova2_targeted_stage,
            nova2_lab::nova2_observer_read,
            nova2_lab::nova2_rehearsal_info,
            nova2_lab::nova2_rehearsal_add,
            nova2_lab::nova2_rehearsal_clear,
            nova2_lab::nova2_checkpoint_export_begin,
            nova2_lab::nova2_checkpoint_export_chunk,
            nova2_lab::nova2_checkpoint_export_android,
            nova2_lab::nova2_checkpoint_export_cancel,
            nova2_lab::nova2_checkpoint_import_begin,
            nova2_lab::nova2_checkpoint_import_append,
            nova2_lab::nova2_checkpoint_import_cancel,
            nova2_lab::nova2_checkpoint_import_finish,
            nova2_tests::nova2_test_boot,
            nova2_tests::nova2_test_archive,
            nova2_tests::nova2_test_save,
            nova2_tests::nova2_test_delete,
            nova2_tests::nova2_test_pause,
            nova2_tests::nova2_test_resume,
            nova2_tests::nova2_test_stop,
            nova2_tests::nova2_test_start,
            nova2_tests::nova2_test_review,
            lesson_zip::nova2_zip_preview,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Neural Nova");
}
