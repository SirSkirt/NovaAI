mod neural;
mod transformer_lab;
mod hybrid_lab;
mod test_lab;
mod diagnostic_log;
mod rehearsal;
mod lesson_zip;
mod checkpoint_transfer;

use neural::NeuralState;
use transformer_lab::TransformerState;
use hybrid_lab::HybridState;
use diagnostic_log::Diagnostics;
use checkpoint_transfer::TransferState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(NeuralState::default())
        .manage(TransformerState::default())
        .manage(HybridState::default())
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
            hybrid_lab::hybrid_boot,
            hybrid_lab::hybrid_status,
            diagnostic_log::nova_log_status,
            diagnostic_log::nova_log_start,
            diagnostic_log::nova_log_stop,
            diagnostic_log::nova_log_read,
            hybrid_lab::hybrid_train,
            lesson_zip::hybrid_zip_preview,
            hybrid_lab::hybrid_checkpoint_export_begin,
            hybrid_lab::hybrid_checkpoint_export_chunk,
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
