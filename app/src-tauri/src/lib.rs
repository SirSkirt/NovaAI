// Nova Messenger: only the Bicameral inference runtime and checkpoint transfer
// are registered in the application. Historical teaching code is archived.
mod nova2_lab;
mod checkpoint_transfer;
#[cfg(target_os="android")]
mod android_checkpoint_export;

use nova2_lab::Nova2State;
use checkpoint_transfer::TransferState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run(){
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(Nova2State::default())
        .manage(TransferState::default())
        .invoke_handler(tauri::generate_handler![
            nova2_lab::nova2_boot,
            nova2_lab::nova2_status,
            nova2_lab::nova2_conversation,
            nova2_lab::nova2_new_chat,
            nova2_lab::nova2_chat,
            nova2_lab::nova2_compare_context,
            nova2_lab::nova2_inspect,
            nova2_lab::nova2_checkpoint_export_begin,
            nova2_lab::nova2_checkpoint_export_chunk,
            nova2_lab::nova2_checkpoint_export_android,
            nova2_lab::nova2_checkpoint_export_cancel,
            nova2_lab::nova2_checkpoint_import_begin,
            nova2_lab::nova2_checkpoint_import_append,
            nova2_lab::nova2_checkpoint_import_cancel,
            nova2_lab::nova2_checkpoint_import_finish,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Nova Messenger");
}
