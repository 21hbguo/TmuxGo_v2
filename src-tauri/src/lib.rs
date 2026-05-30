mod ssh;
mod commands;

use commands::AppState;
use ssh::pool::ConnectionPool;
use ssh::shell::ShellManager;
use std::sync::Arc;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            let data_dir = dirs::data_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("tmuxgo-v2");
            let _ = std::fs::create_dir_all(&data_dir);

            let state = Arc::new(AppState {
                pool: ConnectionPool::new(),
                shell_manager: ShellManager::new(),
                hosts_config_path: data_dir.join("hosts.json"),
                preferences_path: data_dir.join("preferences.json"),
            });

            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Hosts
            commands::list_hosts,
            commands::get_host,
            commands::create_host,
            commands::update_host,
            commands::delete_host,
            commands::connect_host,
            // Sessions
            commands::list_sessions,
            commands::create_session,
            commands::rename_session,
            commands::delete_session,
            // Windows
            commands::list_windows,
            commands::create_window,
            commands::select_window,
            commands::rename_window,
            commands::move_windows,
            commands::kill_window,
            // Panes
            commands::list_panes,
            commands::list_session_panes,
            commands::get_pane_output,
            commands::split_pane,
            commands::zoom_pane,
            commands::select_pane,
            commands::kill_pane,
            // Snapshot
            commands::get_snapshot,
            // Terminal
            commands::attach_terminal,
            commands::detach_terminal,
            commands::send_terminal_input,
            commands::resize_terminal,
            // Files
            commands::list_file_roots,
            commands::list_files,
            commands::read_file_content,
            commands::read_file_preview,
            commands::save_file_content,
            commands::search_files_by_name,
            commands::search_files_by_content,
            commands::get_default_upload_target,
            commands::upload_file,
            // System
            commands::get_system_info,
            // Preferences
            commands::get_preferences,
            commands::update_preferences,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
