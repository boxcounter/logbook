pub mod cli;
pub mod commands;
pub mod config;
mod error_log;
pub mod files;
pub mod models;
pub mod operation_log;
pub mod integrity;
pub mod scan;
pub mod single_instance;
mod window_state;

use std::path::PathBuf;
use tauri::menu::{MenuBuilder, MenuItemBuilder, SubmenuBuilder};
use tauri::Manager;
use tauri_plugin_dialog::DialogExt;

fn show_already_running_dialog(app_name: &str) {
    #[cfg(target_os = "macos")]
    {
        let result = std::process::Command::new("osascript")
            .arg("-e")
            .arg(format!(
                r#"display dialog "{} is already running.\n\nOnly one instance of this application can run at a time." buttons {{"OK"}} default button 1 with icon caution with title "{}""#,
                app_name, app_name
            ))
            .output();
        if let Err(e) = result {
            eprintln!("[logbook] osascript failed: {e}");
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        eprintln!(
            "{} is already running. Only one instance can run at a time.",
            app_name
        );
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            error_log::install_panic_hook();
            app.manage(config::WatcherState::new());
            let app_handle = app.handle().clone();
            let app_data_dir = app
                .path()
                .app_local_data_dir()
                .unwrap_or_else(|_| PathBuf::from("."));
            error_log::init(&app_data_dir);

            match single_instance::InstanceLock::try_acquire(&app_data_dir) {
                Ok(lock) => {
                    app.manage(lock);
                }
                Err(pid) => {
                    error_log::log_info(
                        "single_instance",
                        &format!(
                            "Another instance is already running (PID {}). Exiting.",
                            pid
                        ),
                    );
                    show_already_running_dialog(&app.package_info().name);
                    std::process::exit(0);
                }
            }

            let (width, height, x, y) = window_state::default_window_geometry(&app_handle);
            let title = format!("Logbook v{}", app.package_info().version);
            let _window = tauri::WebviewWindowBuilder::new(
                app,
                "main",
                tauri::WebviewUrl::App("index.html".into()),
            )
            .title(&title)
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
                    config::ensure_watcher(&app_handle, root_path);
                }
            }

            // ── Menu ──────────────────────────────────────────────
            let install_cli_item = MenuItemBuilder::new("Install Command Line Tool…")
                .id("install-cli")
                .build(app)?;

            let app_menu = SubmenuBuilder::new(app, "Logbook")
                .about(Some(Default::default()))
                .separator()
                .item(&install_cli_item)
                .separator()
                .services()
                .separator()
                .hide()
                .hide_others()
                .show_all()
                .separator()
                .quit()
                .build()?;

            let edit_menu = SubmenuBuilder::new(app, "Edit")
                .undo()
                .redo()
                .separator()
                .cut()
                .copy()
                .paste()
                .select_all()
                .build()?;

            let window_menu = SubmenuBuilder::new(app, "Window")
                .minimize()
                .fullscreen()
                .build()?;

            let menu = MenuBuilder::new(app)
                .item(&app_menu)
                .item(&edit_menu)
                .item(&window_menu)
                .build()?;

            app.set_menu(menu)?;

            app.on_menu_event(|app_handle, event| {
                if event.id().0 == "install-cli" {
                    crate::error_log::log_command_enter("install_cli", "menu");
                    let resource_dir = app_handle.path().resource_dir().ok();
                    match crate::cli::install::install_cli(resource_dir) {
                        Ok(msg) => {
                            crate::error_log::log_command_exit("install_cli", true, "");
                            let _ = app_handle
                                .dialog()
                                .message(msg)
                                .title("Logbook")
                                .kind(tauri_plugin_dialog::MessageDialogKind::Info)
                                .show(|_| {});
                        }
                        Err(e) => {
                            crate::error_log::log_error("install_cli", &e);
                            crate::error_log::log_command_exit("install_cli", false, &e);
                            let _ = app_handle
                                .dialog()
                                .message(e)
                                .title("Logbook — Install CLI Failed")
                                .kind(tauri_plugin_dialog::MessageDialogKind::Error)
                                .show(|_| {});
                        }
                    }
                }
            });
            // ────────────────────────────────────────────────────

            // Final safeguard: productName from tauri.conf can override
            // the title during initialization. Set it one last time.
            if let Some(w) = app.get_webview_window("main") {
                if let Err(e) = w.set_title(&title) {
                    crate::error_log::log_error("setup", &format!("set_title failed: {}", e));
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::init,
            commands::set_root_path,
            commands::get_entries,
            commands::get_month_entries,
            commands::append_entry,
            commands::update_entry,
            commands::delete_entry,
            commands::set_day_note,
            commands::get_commitments,
            commands::get_month_dimensions,
            commands::save_dimensions,
            commands::save_dimensions_template,
            commands::get_commitment_progress,
            commands::set_commitments,
            commands::get_available_months,
            commands::reveal_day_file,
            commands::reveal_template_file,
            commands::reveal_file,
            commands::create_starter_files,
            commands::log_error,
            commands::log_info,
            commands::check_watcher_health,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
