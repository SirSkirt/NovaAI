mod neural;
mod transformer_lab;
mod hybrid_lab;
mod test_lab;

use neural::NeuralState;
use transformer_lab::TransformerState;
use hybrid_lab::HybridState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(NeuralState::default())
        .manage(TransformerState::default())
        .manage(HybridState::default())
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
            hybrid_lab::hybrid_train,
            hybrid_lab::hybrid_stop,
            hybrid_lab::hybrid_chat,
            hybrid_lab::hybrid_conversation,
            hybrid_lab::hybrid_new_chat,
            hybrid_lab::hybrid_inspect,
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
