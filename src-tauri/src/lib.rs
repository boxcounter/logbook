mod commands;
pub mod config;
mod error_log;
pub mod files;
pub mod models;

use config::watch_files;
use std::path::PathBuf;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            error_log::install_panic_hook();
            let app_handle = app.handle().clone();
            let app_data_dir = app.path().app_local_data_dir().unwrap_or_else(|_| PathBuf::from("."));
            error_log::init(&app_data_dir);
            if let Some(root_path) = files::read_root_path(&app_data_dir) {
                if root_path.exists() {
                    files::cleanup_tmp_files(&root_path);
                    watch_files(app_handle, root_path);
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::init,
            commands::set_root_path,
            commands::get_entries,
            commands::append_entry,
            commands::update_entry,
            commands::delete_entry,
            commands::set_day_note,
            commands::get_commitments,
            commands::open_in_editor,
            commands::create_starter_files,
            commands::log_error,
            commands::log_info,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
