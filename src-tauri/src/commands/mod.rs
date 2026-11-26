pub mod db;
pub mod session;
pub mod workspace;
pub mod mcp;
pub mod terminal;
pub mod updater;
pub mod setup;
pub mod debug_doctor;
pub mod notification;
pub mod session_diff;
pub mod settings;
pub mod git;
pub mod plugins;

pub use db::*;
pub use session::*;
pub use workspace::*;
pub use mcp::*;
pub use terminal::*;
pub use updater::*;
pub use setup::*;
pub use debug_doctor::*;
pub use notification::*;
pub use session_diff::*;
pub use settings::*;
pub use git::*;
pub use plugins::*;

use crate::acp_client::AcpSession;
use crate::database::DbManager;
use crate::process_manager::ProcessSupervisor;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use tokio::sync::Mutex;

pub fn strip_utf8_bom(s: &str) -> &str {
    s.trim_start_matches('\u{feff}')
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeminiEnvStatus {
    pub installed: bool,
    pub path: Option<String>,
    pub version: Option<String>,
    pub details: String,
}

pub struct AppState {
    pub db: DbManager,
    pub supervisor: ProcessSupervisor,
    pub acp_session: Arc<AcpSession>,
    pub active_process_workspace: Arc<Mutex<Option<String>>>,
    pub active_child: Arc<Mutex<Option<std::process::Child>>>,
    pub search_generation: Arc<AtomicU64>,
}

#[tauri::command]
pub async fn check_gemini_env(state: tauri::State<'_, AppState>) -> Result<GeminiEnvStatus, String> {
    let active_engine = state
        .db
        .get_setting("active_cli_engine")
        .ok()
        .flatten()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "gemini".to_string());
    let adapter = crate::engines::get_adapter(&active_engine);
    let desc = adapter.static_descriptor();

    match adapter.find_executable() {
        Some(bin_path) => {
            let path_str = bin_path.to_string_lossy().to_string();
            let ver_check = ProcessSupervisor::build_cli_command(&bin_path, &["--version"]).output();

            let version = if let Ok(ver_out) = ver_check {
                if ver_out.status.success() {
                    let v = String::from_utf8_lossy(&ver_out.stdout).trim().to_string();
                    if v.is_empty() {
                        "installed".to_string()
                    } else {
                        v
                    }
                } else {
                    "installed".to_string()
                }
            } else {
                "installed".to_string()
            };

            Ok(GeminiEnvStatus {
                installed: true,
                path: Some(path_str),
                version: Some(version),
                details: format!("{} found and verified on system.", desc.display_name),
            })
        }
        None => Ok(GeminiEnvStatus {
            installed: false,
            path: None,
            version: None,
            details: format!("{} not found in PATH or standard installation directories.", desc.display_name),
        }),
    }
}
