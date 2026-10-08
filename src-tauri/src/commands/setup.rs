use crate::AppState;
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::time::Instant;
use tauri::State;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiAuthConfig {
    pub auth_mode: String, // "api_key" | "vertex_ai"
    pub api_key: Option<String>,
    pub google_cloud_project: Option<String>,
    pub google_cloud_location: Option<String>,
    pub google_app_credentials: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliInstallResult {
    pub success: bool,
    pub message: String,
    pub installed_path: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliTestResult {
    pub success: bool,
    pub latency_ms: u64,
    pub message: String,
    pub details: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GCloudStatus {
    pub installed: bool,
    pub active_project: Option<String>,
    pub active_account: Option<String>,
    pub has_adc: bool,
    pub has_gemini_oauth: bool,
    pub path: Option<String>,
}

#[tauri::command]
pub async fn get_gemini_auth_config(
    state: State<'_, AppState>,
) -> Result<GeminiAuthConfig, String> {
    let auth_mode = state.db.get_setting("gemini_auth_mode")
        .map_err(|e| e.to_string())?
        .unwrap_or_else(|| {
            if let Ok(Some(key)) = state.db.get_setting("gemini_api_key") {
                if !key.trim().is_empty() {
                    return "api_key".to_string();
                }
            }
            "vertex_ai".to_string()
        });
    let api_key = state.db.get_setting("gemini_api_key")
        .map_err(|e| e.to_string())?;
    let google_cloud_project = state.db.get_setting("google_cloud_project")
        .map_err(|e| e.to_string())?;
    let google_cloud_location = state.db.get_setting("google_cloud_location")
        .map_err(|e| e.to_string())?
        .or_else(|| Some("us-central1".to_string()));
    let google_app_credentials = state.db.get_setting("google_app_credentials")
        .map_err(|e| e.to_string())?;

    Ok(GeminiAuthConfig {
        auth_mode,
        api_key,
        google_cloud_project,
        google_cloud_location,
        google_app_credentials,
    })
}

#[tauri::command]
pub async fn save_gemini_auth_config(
    state: State<'_, AppState>,
    config: GeminiAuthConfig,
    apply_to_workspace: Option<String>,
) -> Result<(), String> {
    // 1. Save in SQLite
    state.db.set_setting("gemini_auth_mode", &config.auth_mode).map_err(|e| e.to_string())?;
    if let Some(key) = &config.api_key {
        state.db.set_setting("gemini_api_key", key).map_err(|e| e.to_string())?;
    }
    if let Some(proj) = &config.google_cloud_project {
        state.db.set_setting("google_cloud_project", proj).map_err(|e| e.to_string())?;
    }
    if let Some(loc) = &config.google_cloud_location {
        state.db.set_setting("google_cloud_location", loc).map_err(|e| e.to_string())?;
    }
    if let Some(creds) = &config.google_app_credentials {
        state.db.set_setting("google_app_credentials", creds).map_err(|e| e.to_string())?;
    }

    // 2. Sync to ~/.gemini/settings.json so official CLI also picks up selected auth mode
    let home_dir = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")).ok();
    if let Some(home) = home_dir {
        let gemini_dir = std::path::PathBuf::from(home).join(".gemini");
        let _ = std::fs::create_dir_all(&gemini_dir);
        let settings_path = gemini_dir.join("settings.json");

        let mut settings_json: serde_json::Value = if settings_path.is_file() {
            std::fs::read_to_string(&settings_path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_else(|| serde_json::json!({}))
        } else {
            serde_json::json!({})
        };

        let selected_type = if config.auth_mode == "vertex_ai" {
            "vertex-ai"
        } else {
            "gemini-api-key"
        };

        if !settings_json.is_object() {
            settings_json = serde_json::json!({});
        }

        if settings_json.get("security").is_none() {
            settings_json["security"] = serde_json::json!({});
        }
        if settings_json["security"].get("auth").is_none() {
            settings_json["security"]["auth"] = serde_json::json!({});
        }
        settings_json["security"]["auth"]["selectedType"] = serde_json::Value::String(selected_type.to_string());

        if let Ok(formatted) = serde_json::to_string_pretty(&settings_json) {
            let _ = std::fs::write(&settings_path, formatted);
        }
    }

    // 3. If workspace path provided, update or write .env.local in the workspace directory
    if let Some(ws_path) = apply_to_workspace {
        let ws_dir = std::path::PathBuf::from(&ws_path);
        if ws_dir.is_dir() {
            let env_local_path = ws_dir.join(".env.local");
            let mut env_lines = Vec::new();
            if env_local_path.is_file() {
                if let Ok(existing) = std::fs::read_to_string(&env_local_path) {
                    for line in existing.lines() {
                        let trimmed = line.trim();
                        if trimmed.starts_with("GEMINI_API_KEY=")
                            || trimmed.starts_with("GOOGLE_GENAI_USE_VERTEXAI=")
                            || trimmed.starts_with("GOOGLE_CLOUD_PROJECT=")
                            || trimmed.starts_with("GOOGLE_CLOUD_LOCATION=")
                            || trimmed.starts_with("GOOGLE_APPLICATION_CREDENTIALS=") {
                            continue;
                        }
                        env_lines.push(line.to_string());
                    }
                }
            }

            if config.auth_mode == "vertex_ai" {
                env_lines.push("GOOGLE_GENAI_USE_VERTEXAI=true".to_string());
                if let Some(proj) = &config.google_cloud_project {
                    if !proj.trim().is_empty() {
                        env_lines.push(format!("GOOGLE_CLOUD_PROJECT={}", proj.trim()));
                    }
                }
                if let Some(loc) = &config.google_cloud_location {
                    if !loc.trim().is_empty() {
                        env_lines.push(format!("GOOGLE_CLOUD_LOCATION={}", loc.trim()));
                    }
                }
                if let Some(creds) = &config.google_app_credentials {
                    if !creds.trim().is_empty() {
                        env_lines.push(format!("GOOGLE_APPLICATION_CREDENTIALS={}", creds.trim()));
                    }
                }
            } else if let Some(key) = &config.api_key {
                if !key.trim().is_empty() {
                    env_lines.push(format!("GEMINI_API_KEY={}", key.trim()));
                }
            }

            let new_content = env_lines.join("\n") + "\n";
            let _ = std::fs::write(&env_local_path, new_content);
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn install_gemini_cli() -> Result<CliInstallResult, String> {
    tokio::task::spawn_blocking(move || {
        let user_home = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME"))
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")));

        #[cfg(target_os = "windows")]
        let (output, method) = {
            // Check npm
            let mut npm_cmd = Command::new("cmd.exe");
            npm_cmd.args(["/c", "where.exe", "npm"])
                .current_dir(&user_home)
                .creation_flags(0x08000000);
            let npm_check = npm_cmd.output();
            let has_npm = npm_check.map(|o| o.status.success()).unwrap_or(false);

            if has_npm {
                let mut c = Command::new("cmd.exe");
                c.args(["/c", "npm", "install", "-g", "@google/gemini-cli@latest"])
                    .current_dir(&user_home)
                    .creation_flags(0x08000000);
                (c.output(), "npm")
            } else {
                let mut c = Command::new("cmd.exe");
                c.args(["/c", "winget", "install", "--id", "Google.GeminiCLI", "--accept-source-agreements", "--accept-package-agreements"])
                    .current_dir(&user_home)
                    .creation_flags(0x08000000);
                (c.output(), "winget")
            }
        };

        #[cfg(not(target_os = "windows"))]
        let (output, method) = {
            let mut c = Command::new("npm");
            c.args(["install", "-g", "@google/gemini-cli@latest"])
                .current_dir(&user_home);
            (c.output(), "npm")
        };

        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);
                if out.status.success() {
                    let bin_path = crate::process_manager::find_gemini_executable();
                    let ver = bin_path.as_ref().and_then(|p| {
                        let path_str = p.to_string_lossy().to_string();
                        let is_batch = path_str.to_lowercase().ends_with(".cmd") || path_str.to_lowercase().ends_with(".bat");
                        #[cfg(target_os = "windows")]
                        let v_out = if is_batch {
                            Command::new("cmd.exe").args(["/c", &path_str, "--version"]).creation_flags(0x08000000).output()
                        } else {
                            Command::new(&path_str).arg("--version").creation_flags(0x08000000).output()
                        };
                        #[cfg(not(target_os = "windows"))]
                        let v_out = Command::new(&path_str).arg("--version").output();

                        v_out.ok().and_then(|o| {
                            if o.status.success() {
                                let v = String::from_utf8_lossy(&o.stdout).trim().to_string();
                                if v.is_empty() { None } else { Some(v) }
                            } else {
                                None
                            }
                        })
                    });

                    Ok(CliInstallResult {
                        success: true,
                        message: format!("Gemini CLI successfully installed via {}!", method),
                        installed_path: bin_path.map(|p| p.to_string_lossy().to_string()),
                        version: ver,
                    })
                } else {
                    let err_msg = if !stderr.trim().is_empty() { stderr.trim().to_string() } else { stdout.trim().to_string() };
                    Ok(CliInstallResult {
                        success: false,
                        message: format!("Installation via {} exited with error: {}", method, err_msg),
                        installed_path: None,
                        version: None,
                    })
                }
            }
            Err(e) => {
                Ok(CliInstallResult {
                    success: false,
                    message: format!("Failed to launch {} installer process: {}", method, e),
                    installed_path: None,
                    version: None,
                })
            }
        }
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn detect_gcloud_status() -> Result<GCloudStatus, String> {
    tokio::task::spawn_blocking(move || {
        let user_home = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME"))
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")));

        #[cfg(target_os = "windows")]
        let which_gcloud = Command::new("cmd.exe")
            .args(["/c", "where.exe", "gcloud"])
            .current_dir(&user_home)
            .creation_flags(0x08000000)
            .output();

        #[cfg(not(target_os = "windows"))]
        let which_gcloud = Command::new("which")
            .arg("gcloud")
            .current_dir(&user_home)
            .output();

        let (installed, path) = match which_gcloud {
            Ok(out) if out.status.success() => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let first_line = stdout.lines().next().unwrap_or("").trim().to_string();
                (true, Some(first_line))
            }
            _ => (false, None),
        };

        let mut has_adc = false;
        if let Ok(appdata) = std::env::var("APPDATA") {
            let adc_path = std::path::PathBuf::from(appdata).join("gcloud").join("application_default_credentials.json");
            if adc_path.exists() {
                has_adc = true;
            }
        }
        if !has_adc {
            if let Ok(home) = std::env::var("USERPROFILE") {
                let adc_path = std::path::PathBuf::from(home).join(".config").join("gcloud").join("application_default_credentials.json");
                if adc_path.exists() {
                    has_adc = true;
                }
            }
        }

        let mut has_gemini_oauth = false;
        let gemini_dir = user_home.join(".gemini");
        if gemini_dir.join("oauth_creds.json").is_file() {
            has_gemini_oauth = true;
        }

        let mut active_project = None;
        let mut active_account = None;

        if installed {
            #[cfg(target_os = "windows")]
            let proj_out = Command::new("cmd.exe")
                .args(["/c", "gcloud", "config", "get-value", "project"])
                .current_dir(&user_home)
                .creation_flags(0x08000000)
                .output();
            #[cfg(not(target_os = "windows"))]
            let proj_out = Command::new("gcloud")
                .args(["config", "get-value", "project"])
                .current_dir(&user_home)
                .output();

            if let Ok(po) = proj_out {
                if po.status.success() {
                    let p = String::from_utf8_lossy(&po.stdout).trim().to_string();
                    if !p.is_empty() && p != "(unset)" {
                        active_project = Some(p);
                    }
                }
            }

            #[cfg(target_os = "windows")]
            let acc_out = Command::new("cmd.exe")
                .args(["/c", "gcloud", "auth", "list", "--filter=status:ACTIVE", "--format=value(account)"])
                .current_dir(&user_home)
                .creation_flags(0x08000000)
                .output();
            #[cfg(not(target_os = "windows"))]
            let acc_out = Command::new("gcloud")
                .args(["auth", "list", "--filter=status:ACTIVE", "--format=value(account)"])
                .current_dir(&user_home)
                .output();

            if let Ok(ao) = acc_out {
                if ao.status.success() {
                    let a = String::from_utf8_lossy(&ao.stdout).trim().to_string();
                    if !a.is_empty() {
                        active_account = Some(a);
                    }
                }
            }
        }

        // If active_account not found via gcloud, read from ~/.gemini/google_accounts.json
        if active_account.is_none() {
            let accounts_path = gemini_dir.join("google_accounts.json");
            if accounts_path.is_file() {
                if let Ok(content) = std::fs::read_to_string(&accounts_path) {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(act) = val.get("active").and_then(|v| v.as_str()) {
                            if !act.trim().is_empty() {
                                active_account = Some(act.trim().to_string());
                            }
                        }
                    }
                }
            }
        }

        Ok(GCloudStatus {
            installed,
            active_project,
            active_account,
            has_adc,
            has_gemini_oauth,
            path,
        })
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn test_gemini_cli(
    state: State<'_, AppState>,
    config: Option<GeminiAuthConfig>,
    workspace_path: Option<String>,
    model: Option<String>,
) -> Result<CliTestResult, String> {
    let bin = match crate::process_manager::find_gemini_executable() {
        Some(p) => p,
        None => {
            return Ok(CliTestResult {
                success: false,
                latency_ms: 0,
                message: "Gemini CLI not found on system PATH or standard npm directories.".to_string(),
                details: Some("Please install Gemini CLI using the 1-click installer above.".to_string()),
            });
        }
    };

    let bin_path_str = bin.to_string_lossy().to_string();
    let is_batch = bin_path_str.to_lowercase().ends_with(".cmd") || bin_path_str.to_lowercase().ends_with(".bat");

    // Resolve credentials to test
    let effective_config = match config {
        Some(c) => c,
        None => get_gemini_auth_config(state.clone()).await?,
    };

    let test_model = model.unwrap_or_else(|| "gemini-3.8-flash".to_string());
    let test_model = if test_model.trim().is_empty() {
        "gemini-3.8-flash".to_string()
    } else {
        test_model.trim().to_string()
    };

    tokio::task::spawn_blocking(move || {
        let start = Instant::now();

        #[cfg(target_os = "windows")]
        let mut cmd = if is_batch {
            let mut c = Command::new("cmd.exe");
            c.args([
                "/c",
                &bin_path_str,
                "--skip-trust",
                "-p",
                "Respond with 'OK' and nothing else.",
                "--model",
                &test_model,
                "--output-format",
                "text",
            ]);
            c
        } else {
            let mut c = Command::new(&bin_path_str);
            c.args([
                "--skip-trust",
                "-p",
                "Respond with 'OK' and nothing else.",
                "--model",
                &test_model,
                "--output-format",
                "text",
            ]);
            c
        };

        #[cfg(not(target_os = "windows"))]
        let mut cmd = {
            let mut c = Command::new(&bin_path_str);
            c.args([
                "--skip-trust",
                "-p",
                "Respond with 'OK' and nothing else.",
                "--model",
                &test_model,
                "--output-format",
                "text",
            ]);
            c
        };

        // Determine an enterprise-grade test directory.
        // Priority 1: Valid active workspace project directory.
        // Priority 2: User Home Directory (%USERPROFILE%, e.g. C:\Users\<username>), which is the standard
        //             working directory for Windows developer shells and pre-whitelisted in EDR profiles.
        // Priority 3: LocalAppData application directory (%LOCALAPPDATA%\GeminiDesktop).
        // We strictly avoid %TEMP% to eliminate CrowdStrike / Defender EDR temporary folder IOA alerts.
        let safe_fallback = if let Ok(home) = std::env::var("USERPROFILE") {
            let p = std::path::PathBuf::from(home);
            if p.is_dir() {
                p
            } else if let Ok(local_app) = std::env::var("LOCALAPPDATA") {
                let p2 = std::path::PathBuf::from(local_app).join("GeminiDesktop");
                let _ = std::fs::create_dir_all(&p2);
                p2
            } else {
                std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
            }
        } else if let Ok(local_app) = std::env::var("LOCALAPPDATA") {
            let p = std::path::PathBuf::from(local_app).join("GeminiDesktop");
            let _ = std::fs::create_dir_all(&p);
            p
        } else {
            std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
        };

        let test_dir = if let Some(ref ws) = workspace_path {
            let d = std::path::PathBuf::from(ws);
            if d.is_dir() && !d.to_string_lossy().ends_with(":\\") && !d.to_string_lossy().ends_with(":/") {
                crate::process_manager::ensure_folder_trusted(&d);
                d
            } else {
                crate::process_manager::ensure_folder_trusted(&safe_fallback);
                safe_fallback
            }
        } else {
            crate::process_manager::ensure_folder_trusted(&safe_fallback);
            safe_fallback
        };
        cmd.current_dir(&test_dir);

        // Enforce non-interactive environment
        cmd.env("CI", "true");
        cmd.env("PAGER", "cat");
        cmd.env("GIT_PAGER", "cat");

        // Inject tested credentials
        if effective_config.auth_mode == "vertex_ai" {
            cmd.env("GOOGLE_GENAI_USE_VERTEXAI", "true");
            if let Some(ref proj) = effective_config.google_cloud_project {
                cmd.env("GOOGLE_CLOUD_PROJECT", proj);
            }
            if let Some(ref loc) = effective_config.google_cloud_location {
                cmd.env("GOOGLE_CLOUD_LOCATION", loc);
            }
            if let Some(ref creds) = effective_config.google_app_credentials {
                cmd.env("GOOGLE_APPLICATION_CREDENTIALS", creds);
            }
        } else if let Some(ref key) = effective_config.api_key {
            cmd.env("GEMINI_API_KEY", key);
        }

        #[cfg(target_os = "windows")]
        {
            cmd.creation_flags(0x08000000);
        }

        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                return Ok(CliTestResult {
                    success: false,
                    latency_ms: start.elapsed().as_millis() as u64,
                    message: format!("Failed to spawn test process: {}", e),
                    details: None,
                });
            }
        };

        // Poll up to 15 seconds
        let timeout_duration = std::time::Duration::from_secs(15);
        let poll_interval = std::time::Duration::from_millis(100);
        let poll_start = Instant::now();
        let mut exited = false;

        while poll_start.elapsed() < timeout_duration {
            match child.try_wait() {
                Ok(Some(_status)) => {
                    exited = true;
                    break;
                }
                Ok(None) => {
                    std::thread::sleep(poll_interval);
                }
                Err(e) => {
                    return Ok(CliTestResult {
                        success: false,
                        latency_ms: start.elapsed().as_millis() as u64,
                        message: format!("Error waiting for CLI response: {}", e),
                        details: None,
                    });
                }
            }
        }

        if !exited {
            let _ = child.kill();
            return Ok(CliTestResult {
                success: false,
                latency_ms: 15000,
                message: "Connection test timed out after 15 seconds.".to_string(),
                details: Some("The model API did not respond in time. Please check your network connection, proxy settings, and project/quota permissions.".to_string()),
            });
        }

        let output = match child.wait_with_output() {
            Ok(o) => o,
            Err(e) => {
                return Ok(CliTestResult {
                    success: false,
                    latency_ms: start.elapsed().as_millis() as u64,
                    message: format!("Failed to read process output: {}", e),
                    details: None,
                });
            }
        };

        let latency_ms = start.elapsed().as_millis() as u64;
        let stdout_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr_str = String::from_utf8_lossy(&output.stderr).trim().to_string();

        if output.status.success() && (!stdout_str.is_empty() || stderr_str.is_empty()) {
            Ok(CliTestResult {
                success: true,
                latency_ms,
                message: "Gemini CLI successfully connected and verified!".to_string(),
                details: Some(stdout_str),
            })
        } else {
            let combined_err = format!("{}\n{}", stderr_str, stdout_str).trim().to_string();
            let friendly_msg = if combined_err.contains("no longer available") || combined_err.contains("ModelNotFoundError") {
                "Model is no longer available to new users. Please switch to gemini-3.8-flash."
            } else if combined_err.contains("503") || combined_err.contains("high demand") {
                "Model is currently experiencing high demand. Please retry or test with gemini-3.8-flash."
            } else if combined_err.contains("GEMINI_API_KEY") {
                "GEMINI_API_KEY is not set or empty. Please enter your API key."
            } else if combined_err.contains("API_KEY_INVALID") || combined_err.contains("401") || combined_err.contains("403") {
                "Invalid API Key. Please verify your API key in Google AI Studio."
            } else if combined_err.contains("GOOGLE_CLOUD_PROJECT") || combined_err.contains("GOOGLE_CLOUD_LOCATION") {
                "Missing GOOGLE_CLOUD_PROJECT or GOOGLE_CLOUD_LOCATION for Vertex AI mode."
            } else if combined_err.contains("application-default") || combined_err.contains("Could not automatically determine credentials") {
                "Google Cloud credentials not found. Run 'gcloud auth application-default login' or provide a Service Account key."
            } else if combined_err.contains("aiplatform.googleapis.com") {
                "Vertex AI API (aiplatform.googleapis.com) is disabled on your Google Cloud Project."
            } else if combined_err.contains("RESOURCE_EXHAUSTED") || combined_err.contains("Quota") {
                "API Quota exceeded for this key or project."
            } else if combined_err.contains("trusted") || combined_err.contains("trust") {
                "Workspace directory is not trusted. Pass --skip-trust or add folder to trusted folders."
            } else {
                "Gemini CLI returned an error during execution."
            };

            Ok(CliTestResult {
                success: false,
                latency_ms,
                message: friendly_msg.to_string(),
                details: if !combined_err.is_empty() { Some(combined_err) } else { None },
            })
        }
    }).await.map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gemini_auth_config_serialization() {
        let config = GeminiAuthConfig {
            auth_mode: "vertex_ai".to_string(),
            api_key: None,
            google_cloud_project: Some("corp-ai-sample-123".to_string()),
            google_cloud_location: Some("us-central1".to_string()),
            google_app_credentials: None,
        };

        let json = serde_json::to_string(&config).expect("failed to serialize");
        assert!(json.contains("corp-ai-sample-123"));
        assert!(json.contains("vertex_ai"));

        let deserialized: GeminiAuthConfig = serde_json::from_str(&json).expect("failed to deserialize");
        assert_eq!(deserialized.auth_mode, "vertex_ai");
        assert_eq!(deserialized.google_cloud_project.as_deref(), Some("corp-ai-sample-123"));
    }

    #[test]
    fn test_cli_test_result_success_and_error() {
        let res_ok = CliTestResult {
            success: true,
            latency_ms: 120,
            message: "Connected".to_string(),
            details: Some("OK".to_string()),
        };
        assert!(res_ok.success);
        assert_eq!(res_ok.latency_ms, 120);

        let res_err = CliTestResult {
            success: false,
            latency_ms: 450,
            message: "Missing credentials".to_string(),
            details: None,
        };
        assert!(!res_err.success);
    }
}
