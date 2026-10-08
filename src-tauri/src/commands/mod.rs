pub mod db;
pub mod session;
pub mod workspace;
pub mod mcp;
pub mod terminal;
pub mod updater;
pub mod setup;

pub use db::*;
pub use session::*;
pub use workspace::*;
pub use mcp::*;
pub use terminal::*;
pub use updater::*;
pub use setup::*;

use crate::acp_client::AcpSession;
use crate::database::DbManager;
use crate::process_manager::ProcessSupervisor;
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use tokio::sync::Mutex;

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
pub async fn check_gemini_env() -> Result<GeminiEnvStatus, String> {
    match crate::process_manager::find_gemini_executable() {
        Some(bin_path) => {
            let path_str = bin_path.to_string_lossy().to_string();
            let is_batch = path_str.to_lowercase().ends_with(".cmd")
                        || path_str.to_lowercase().ends_with(".bat");

            #[cfg(target_os = "windows")]
            let ver_check = if is_batch {
                Command::new("cmd.exe")
                    .args(["/c", &path_str, "--version"])
                    .output()
            } else {
                Command::new(&path_str)
                    .arg("--version")
                    .output()
            };

            #[cfg(not(target_os = "windows"))]
            let ver_check = Command::new(&path_str)
                .arg("--version")
                .output();

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
                details: "Gemini CLI found and verified on system.".to_string(),
            })
        }
        None => Ok(GeminiEnvStatus {
            installed: false,
            path: None,
            version: None,
            details: "Gemini CLI not found in PATH or standard npm/pnpm locations.".to_string(),
        }),
    }
}
