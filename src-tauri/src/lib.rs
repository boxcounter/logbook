pub mod commands;
pub mod config;
mod error_log;
pub mod files;
pub mod models;
mod window_state;

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
            let app_data_dir = app
                .path()
                .app_local_data_dir()
                .unwrap_or_else(|_| PathBuf::from("."));
            error_log::init(&app_data_dir);
            // Restore window geometry (or 90% default) and track changes
            if let Some(window) = app.get_webview_window("main") {
                window_state::restore_window_state(&window, &app_data_dir);
                window_state::register_state_tracking(&window);
            }
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
            commands::get_commitment_progress,
            commands::open_in_editor,
            commands::create_starter_files,
            commands::log_error,
            commands::log_info,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::Exit = event {
                let app_data_dir = app_handle
                    .path()
                    .app_local_data_dir()
                    .unwrap_or_else(|_| PathBuf::from("."));
                window_state::flush_to_disk(&app_data_dir);
            }
        });
}
