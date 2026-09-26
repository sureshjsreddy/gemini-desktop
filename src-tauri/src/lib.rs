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
    let search_generation = Arc::new(std::sync::atomic::AtomicU64::new(0));

    let state = AppState {
        db,
        supervisor,
        acp_session,
        active_process_workspace,
        search_generation,
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(state)
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
            respond_tool_permission,
            get_prompts,
            search_history,
            export_session,
            list_workspace_files,
            read_workspace_dir,
            search_workspace_files,
            cancel_workspace_search,
            open_workspace_file,
            get_mcp_config,
            save_mcp_config,
            run_terminal_command,
            restart_gemini_session,
            check_app_update,
            launch_winget_upgrade
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
