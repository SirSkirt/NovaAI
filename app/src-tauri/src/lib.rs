mod neural;

use neural::NeuralState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(NeuralState::default())
        .invoke_handler(tauri::generate_handler![
            neural::neural_boot,
            neural::neural_status,
            neural::neural_inspect,
            neural::neural_generate,
            neural::neural_train,
            neural::neural_stop,
            neural::neural_reset,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Neural Nova");
}
