pub mod cli;
pub mod commands;
pub mod config;
mod error_log;
pub mod files;
pub mod models;
pub mod operation_log;
pub mod scan;
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

            let (width, height, x, y) = window_state::default_window_geometry(&app_handle);
            let _window = tauri::WebviewWindowBuilder::new(
                app,
                "main",
                tauri::WebviewUrl::App("index.html".into()),
            )
            .title("Logbook")
            .inner_size(width as f64, height as f64)
            .position(x as f64, y as f64)
            // Tauri's OS drag-drop handler (enabled by default) intercepts the
            // native HTML5 dragover/drop events Sortable.js needs for in-app
            // drag-reorder. We don't use OS file-drop, so disable it.
            .disable_drag_drop_handler()
            .build()
            .expect("failed to create main window");

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
            commands::set_commitments,
            commands::get_available_months,
            commands::open_in_editor,
            commands::create_starter_files,
            commands::log_error,
            commands::log_info,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
