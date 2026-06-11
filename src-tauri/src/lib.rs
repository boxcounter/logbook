mod commands;
mod config;
mod files;
mod models;

use config::watch_files;
use std::path::PathBuf;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_handle = app.handle().clone();
            let app_data_dir = app.path().app_local_data_dir().unwrap_or_else(|_| PathBuf::from("."));
            if let Some(root_path) = files::read_root_path(&app_data_dir) {
                if root_path.exists() {
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
