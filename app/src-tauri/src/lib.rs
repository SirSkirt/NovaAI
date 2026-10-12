mod neural;
mod transformer_lab;

use neural::NeuralState;
use transformer_lab::TransformerState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(NeuralState::default())
        .manage(TransformerState::default())
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running Neural Nova");
}
