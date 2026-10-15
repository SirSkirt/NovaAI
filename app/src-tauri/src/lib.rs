// Large, structured teacher-only epoch diagnostics include separate current,
// baseline, autonomous and archived-retention evidence. serde_json::json!
// expands recursively; raise only the macro expansion budget, not neural
// inference depth or the model's recurrent reasoning steps.
#![recursion_limit = "256"]

mod neural;
mod transformer_lab;
mod hybrid_lab;
mod nova2_lab;
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

use neural::NeuralState;
use transformer_lab::TransformerState;
use hybrid_lab::HybridState;
use nova2_lab::Nova2State;
use targeted_lab::TargetedState;
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
        .manage(NeuralState::default())
        .manage(TransformerState::default())
        .manage(HybridState::default())
        .manage(Nova2State::default())
        .manage(TargetedState::default())
        .manage(ZipQueueState::default())
        .manage(Diagnostics::default())
        .manage(TransferState::default())
        .invoke_handler(tauri::generate_handler![
            neural::neural_boot,
            neural::neural_status,
            neural::neural_inspect,
            neural::neural_generate,
            neural::neural_train,
            neural::neural_train_dialogues,
            neural::neural_chat,
            neural::neural_stop,
            neural::neural_reset,
            transformer_lab::transformer_boot,
            transformer_lab::transformer_status,
            transformer_lab::transformer_train,
            transformer_lab::transformer_stop,
            transformer_lab::transformer_chat,
            transformer_lab::transformer_reset,
            nova2_lab::nova2_boot,
            nova2_lab::nova2_status,
            nova2_lab::nova2_conversation,
            nova2_lab::nova2_new_chat,
            nova2_lab::nova2_chat,
            nova2_lab::nova2_reset,
            nova2_lab::nova2_train,
            nova2_lab::nova2_zip_train,
            nova2_lab::nova2_stop,
            hybrid_lab::hybrid_boot,
            hybrid_lab::hybrid_status,
            diagnostic_log::nova_log_status,
            diagnostic_log::nova_log_start,
            diagnostic_log::nova_log_stop,
            diagnostic_log::nova_log_read,
            training_observer::nova_observer_sessions,
            training_observer::nova_observer_read,
            hybrid_lab::hybrid_train,
            targeted_lab::targeted_status,
            targeted_lab::targeted_import_begin,
            targeted_lab::targeted_import_append,
            targeted_lab::targeted_import_cancel,
            targeted_lab::targeted_import_finish,
            targeted_lab::targeted_train,
            targeted_lab::targeted_stop,
            targeted_lab::targeted_discard,
            targeted_lab::targeted_accept,
            lesson_zip::hybrid_zip_preview,
            lesson_zip::hybrid_zip_train,
            lesson_zip::hybrid_zip_stop,
            lesson_zip::hybrid_zip_status,
            hybrid_lab::hybrid_checkpoint_export_begin,
            hybrid_lab::hybrid_checkpoint_export_chunk,
            hybrid_lab::hybrid_checkpoint_export_android,
            hybrid_lab::hybrid_checkpoint_export_cancel,
            hybrid_lab::hybrid_checkpoint_import_begin,
            hybrid_lab::hybrid_checkpoint_import_append,
            hybrid_lab::hybrid_checkpoint_import_finish,
            hybrid_lab::hybrid_checkpoint_import_cancel,
            hybrid_lab::hybrid_rehearsal_info,
            hybrid_lab::hybrid_rehearsal_add,
            hybrid_lab::hybrid_rehearsal_clear,
            hybrid_lab::hybrid_stop,
            hybrid_lab::hybrid_chat,
            hybrid_lab::hybrid_conversation,
            hybrid_lab::hybrid_new_chat,
            hybrid_lab::hybrid_inspect,
            hybrid_lab::hybrid_tokenizer_probe,
            hybrid_lab::hybrid_bridge_probe,
            hybrid_lab::hybrid_diagnostic_probe,
            hybrid_lab::hybrid_reset,
            hybrid_lab::hybrid_test_archive,
            hybrid_lab::hybrid_test_save,
            hybrid_lab::hybrid_test_delete,
            hybrid_lab::hybrid_test_start,
            hybrid_lab::hybrid_test_pause,
            hybrid_lab::hybrid_test_resume,
            hybrid_lab::hybrid_test_stop,
            hybrid_lab::hybrid_test_review,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Neural Nova");
}
