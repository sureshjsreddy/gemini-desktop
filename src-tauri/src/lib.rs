mod process_manager;
mod acp_client;
mod database;
mod commands;

use commands::*;
use database::DbManager;
use process_manager::ProcessSupervisor;
use acp_client::AcpSession;
use std::sync::Arc;
use tokio::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let db = DbManager::new().expect("Failed to initialize SQLite database");
    let supervisor = ProcessSupervisor::new();
    let acp_session = Arc::new(AcpSession::new());
    let active_process_workspace = Arc::new(Mutex::new(None));
    let active_child = Arc::new(Mutex::new(None));
    let search_generation = Arc::new(std::sync::atomic::AtomicU64::new(0));

    let state = AppState {
        db,
        supervisor,
        acp_session,
        active_process_workspace,
        active_child,
        search_generation,
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .manage(state)
        .setup(|app| {
            commands::ensure_windows_aumid();
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                commands::register_global_debug_hotkey(handle);
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            check_gemini_env,
            get_workspaces,
            save_workspace,
            delete_workspace,
            get_sessions,
            create_session,
            rename_session,
            delete_session,
            get_session_messages,
            save_message,
            send_prompt,
            cancel_prompt,
            steer_prompt,
            rewind_turn,
            respond_tool_permission,
            get_prompts,
            search_history,
            export_session,
            list_workspace_files,
            read_workspace_dir,
            search_workspace_files,
            cancel_workspace_search,
            open_workspace_file,
            check_directory_exists,
            validate_workspace_path,
            get_default_workspace_dir,
            get_mcp_config,
            save_mcp_config,
            run_terminal_command,
            restart_gemini_session,
            check_app_update,
            launch_winget_upgrade,
            get_app_version,
            get_gemini_auth_config,
            save_gemini_auth_config,
            install_gemini_cli,
            detect_gcloud_status,
            test_gemini_cli,
            read_file_snippet,
            get_clipboard_text,
            show_toast_notification,
            focus_app_window,
            get_session_file_changes,
            revert_session_file,
            revert_all_session_files,
            clear_session_snapshots
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
