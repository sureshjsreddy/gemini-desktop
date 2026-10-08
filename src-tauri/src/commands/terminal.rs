use crate::process_manager::ProcessSupervisor;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalCommandResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub duration_ms: u64,
}

#[tauri::command]
pub async fn run_terminal_command(
    command: String,
    workspace_path: Option<String>,
) -> Result<TerminalCommandResult, String> {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return Ok(TerminalCommandResult {
            stdout: String::new(),
            stderr: String::new(),
            exit_code: 0,
            duration_ms: 0,
        });
    }

    let start = std::time::Instant::now();
    let mut cmd = if cfg!(target_os = "windows") {
        let mut c = std::process::Command::new("powershell.exe");
        c.arg("-NoProfile")
            .arg("-NonInteractive")
            .arg("-Command")
            .arg(trimmed);
        c
    } else {
        let mut c = std::process::Command::new("sh");
        c.arg("-c").arg(trimmed);
        c
    };

    let safe_cwd = if let Some(ref wp) = workspace_path {
        let p = PathBuf::from(wp);
        if p.is_dir() {
            // Auto-inject workspace .env and .env.local into terminal commands
            let dot_env = p.join(".env");
            if dot_env.is_file() {
                let envs = ProcessSupervisor::parse_dotenv_file(&dot_env);
                cmd.envs(&envs);
            }
            let dot_env_local = p.join(".env.local");
            if dot_env_local.is_file() {
                let envs = ProcessSupervisor::parse_dotenv_file(&dot_env_local);
                cmd.envs(&envs);
            }
            p
        } else {
            fallback_safe_terminal_dir()
        }
    } else {
        fallback_safe_terminal_dir()
    };
    cmd.current_dir(&safe_cwd);

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    let output = cmd
        .output()
        .map_err(|e| format!("Failed to execute command: {}", e))?;

    let duration_ms = start.elapsed().as_millis() as u64;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code().unwrap_or(-1);

    Ok(TerminalCommandResult {
        stdout,
        stderr,
        exit_code,
        duration_ms,
    })
}

fn fallback_safe_terminal_dir() -> PathBuf {
    if let Ok(home) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
        let p = PathBuf::from(home);
        if p.is_dir() {
            return p;
        }
    }
    if let Ok(local_app) = std::env::var("LOCALAPPDATA") {
        let p = PathBuf::from(local_app).join("GeminiDesktop");
        let _ = std::fs::create_dir_all(&p);
        if p.is_dir() {
            return p;
        }
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_terminal_command_dotenv_injection() {
        let temp_dir = std::env::temp_dir().join(format!("gemini_test_term_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let dot_env = temp_dir.join(".env");
        std::fs::write(&dot_env, "CUSTOM_TEST_ENV=SecretAlphaValue123\n").unwrap();

        let ws_str = temp_dir.to_string_lossy().to_string();
        let cmd_str = if cfg!(target_os = "windows") {
            "Write-Output $env:CUSTOM_TEST_ENV".to_string()
        } else {
            "echo $CUSTOM_TEST_ENV".to_string()
        };

        let res = run_terminal_command(cmd_str, Some(ws_str)).await.expect("run_terminal_command failed");
        assert_eq!(res.exit_code, 0);
        assert!(res.stdout.contains("SecretAlphaValue123"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
