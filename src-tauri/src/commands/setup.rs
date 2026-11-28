use crate::AppState;
use serde::{Deserialize, Serialize};
use serde_json::Value;
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
    #[serde(default)]
    pub agy_adc_auth: Option<bool>,
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

pub fn read_auth_config_from_db(db: &crate::database::DbManager) -> Result<GeminiAuthConfig, String> {
    let non_empty = |v: Option<String>| -> Option<String> {
        v.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
    };

    let auth_mode = db.get_setting("gemini_auth_mode")
        .map_err(|e| e.to_string())?
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| {
            if let Ok(Some(key)) = db.get_setting("gemini_api_key") {
                if !key.trim().is_empty() {
                    return "api_key".to_string();
                }
            }
            "vertex_ai".to_string()
        });
    let api_key = non_empty(db.get_setting("gemini_api_key").map_err(|e| e.to_string())?);
    let google_cloud_project = non_empty(db.get_setting("google_cloud_project").map_err(|e| e.to_string())?);
    let google_cloud_location = non_empty(db.get_setting("google_cloud_location").map_err(|e| e.to_string())?)
        .or_else(|| Some("us-central1".to_string()));
    let google_app_credentials = non_empty(db.get_setting("google_app_credentials").map_err(|e| e.to_string())?);
    let agy_adc_auth = db.get_setting("agy_adc_auth")
        .map_err(|e| e.to_string())?
        .map(|s| s.trim().eq_ignore_ascii_case("true"));

    Ok(GeminiAuthConfig {
        auth_mode,
        api_key,
        google_cloud_project,
        google_cloud_location,
        google_app_credentials,
        agy_adc_auth,
    })
}

pub fn write_auth_config_to_db(db: &crate::database::DbManager, config: &GeminiAuthConfig) -> Result<(), String> {
    // Persist empty strings on cleared optional fields so stale values never linger
    db.set_setting("gemini_auth_mode", config.auth_mode.trim()).map_err(|e| e.to_string())?;
    db.set_setting("gemini_api_key", config.api_key.as_deref().unwrap_or("").trim()).map_err(|e| e.to_string())?;
    db.set_setting("google_cloud_project", config.google_cloud_project.as_deref().unwrap_or("").trim()).map_err(|e| e.to_string())?;
    db.set_setting("google_cloud_location", config.google_cloud_location.as_deref().unwrap_or("").trim()).map_err(|e| e.to_string())?;
    db.set_setting("google_app_credentials", config.google_app_credentials.as_deref().unwrap_or("").trim()).map_err(|e| e.to_string())?;
    if let Some(adc) = config.agy_adc_auth {
        db.set_setting("agy_adc_auth", if adc { "true" } else { "false" }).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn build_env_local_content(existing_content: Option<&str>, config: &GeminiAuthConfig) -> String {
    let mut env_lines = Vec::new();
    if let Some(existing) = existing_content {
        for line in existing.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("GEMINI_API_KEY=")
                || trimmed.starts_with("GOOGLE_GENAI_USE_VERTEXAI=")
                || trimmed.starts_with("GOOGLE_CLOUD_PROJECT=")
                || trimmed.starts_with("GOOGLE_CLOUD_LOCATION=")
                || trimmed.starts_with("GOOGLE_APPLICATION_CREDENTIALS=")
                || trimmed.starts_with("AGY_ADC_AUTH=")
            {
                continue;
            }
            env_lines.push(line.to_string());
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

    if config.agy_adc_auth == Some(true) {
        env_lines.push("AGY_ADC_AUTH=true".to_string());
    }

    env_lines.join("\n") + "\n"
}

#[tauri::command]
pub async fn get_gemini_auth_config(
    state: State<'_, AppState>,
) -> Result<GeminiAuthConfig, String> {
    read_auth_config_from_db(&state.db)
}

#[tauri::command]
pub async fn save_gemini_auth_config(
    state: State<'_, AppState>,
    config: GeminiAuthConfig,
    apply_to_workspace: Option<String>,
) -> Result<(), String> {
    // 1. Save in SQLite
    write_auth_config_to_db(&state.db, &config)?;

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
            let existing = if env_local_path.is_file() {
                std::fs::read_to_string(&env_local_path).ok()
            } else {
                None
            };
            let new_content = build_env_local_content(existing.as_deref(), &config);
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
                    crate::process_manager::invalidate_cli_path_cache();
                    let bin_path = crate::process_manager::find_gemini_executable();
                    let ver = bin_path.as_ref().and_then(|p| get_cli_binary_version(p));

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

        // Fast in-memory check without spawning cmd.exe or where.exe
        let gcloud_exe = crate::process_manager::find_binary_by_name("gcloud");
        let installed = gcloud_exe.is_some();
        let path = gcloud_exe.map(|p| p.to_string_lossy().to_string());

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
            // Direct in-process config parsing from disk (avoids heavy cmd.exe / python.exe gcloud subshells)
            let config_dirs = [
                std::env::var("APPDATA").ok().map(|a| std::path::PathBuf::from(a).join("gcloud")),
                std::env::var("USERPROFILE").ok().map(|u| std::path::PathBuf::from(u).join(".config").join("gcloud")),
            ];

            for gcloud_dir in config_dirs.into_iter().flatten() {
                let active_cfg_name = std::fs::read_to_string(gcloud_dir.join("active_config"))
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|_| "default".to_string());
                let config_file = gcloud_dir.join("configurations").join(format!("config_{}", active_cfg_name));
                if config_file.is_file() {
                    if let Ok(content) = std::fs::read_to_string(&config_file) {
                        for line in content.lines() {
                            let line = line.trim();
                            if let Some(rest) = line.strip_prefix("project") {
                                if let Some(val) = rest.trim().strip_prefix('=') {
                                    let v = val.trim().to_string();
                                    if !v.is_empty() && v != "(unset)" {
                                        active_project = Some(v);
                                    }
                                }
                            }
                            if let Some(rest) = line.strip_prefix("account") {
                                if let Some(val) = rest.trim().strip_prefix('=') {
                                    let v = val.trim().to_string();
                                    if !v.is_empty() && v != "(unset)" {
                                        active_account = Some(v);
                                    }
                                }
                            }
                        }
                    }
                }
                if active_project.is_some() && active_account.is_some() {
                    break;
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

        let mut cmd = crate::process_manager::ProcessSupervisor::build_cli_command(
            &bin,
            &[
                "--skip-trust",
                "-p",
                "Respond with 'OK' and nothing else.",
                "--model",
                test_model.as_str(),
                "--output-format",
                "text",
            ],
        );

        let safe_fallback = crate::process_manager::ProcessSupervisor::fallback_safe_dir();

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
            let friendly_msg = if combined_err.contains("no longer available")
                || combined_err.contains("ModelNotFoundError")
                || combined_err.contains("deprecated")
                || combined_err.contains("not found for API version")
                || combined_err.contains("models/gemini-1.5")
                || combined_err.contains("models/gemini-2.5-flash-lite-preview")
            {
                "The requested model is deprecated or retired by Google. Please switch to gemini-3.8-flash or gemini-3.5-flash-lite."
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
            } else if combined_err.contains("RESOURCE_EXHAUSTED")
                || combined_err.contains("Quota")
                || combined_err.to_lowercase().contains("spending cap")
                || combined_err.to_lowercase().contains("spend cap")
            {
                "API Quota or monthly spending cap exceeded for this key or project."
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CliModelInfo {
    pub id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub input_token_limit: Option<u64>,
    pub output_token_limit: Option<u64>,
    pub is_default: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliModelDiscoveryResult {
    pub supported: bool,
    pub models: Vec<CliModelInfo>,
    pub message: String,
    pub engine: Option<String>,
    #[serde(rename = "cli_version")]
    pub cli_version: Option<String>,
}

pub fn parse_cli_models_json(stdout: &str) -> Vec<CliModelInfo> {
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    if let Ok(val) = serde_json::from_str::<Value>(trimmed) {
        let items: Option<&Vec<Value>> = if let Some(arr) = val.as_array() {
            Some(arr)
        } else if let Some(arr) = val.get("models").and_then(|m| m.as_array()) {
            Some(arr)
        } else {
            None
        };

        if let Some(arr) = items {
            let mut result = Vec::new();
            for item in arr {
                if let Some(id) = item
                    .get("id")
                    .or_else(|| item.get("modelId"))
                    .or_else(|| item.get("model"))
                    .or_else(|| item.get("name"))
                    .and_then(|v| v.as_str())
                {
                    let id_str = id.trim().to_string();
                    if !id_str.is_empty() {
                        let name = item
                            .get("displayName")
                            .or_else(|| item.get("name"))
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        let description = item
                            .get("description")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        let input_token_limit = item
                            .get("inputTokenLimit")
                            .or_else(|| item.get("input_token_limit"))
                            .or_else(|| item.pointer("/contextLimits/inputTokens"))
                            .and_then(|v| v.as_u64());
                        let output_token_limit = item
                            .get("outputTokenLimit")
                            .or_else(|| item.get("output_token_limit"))
                            .or_else(|| item.pointer("/contextLimits/outputTokens"))
                            .and_then(|v| v.as_u64());
                        let is_default = item
                            .get("isDefault")
                            .or_else(|| item.get("default"))
                            .and_then(|v| v.as_bool())
                            .or_else(|| Some(id_str.eq_ignore_ascii_case("auto")));

                        result.push(CliModelInfo {
                            id: id_str,
                            name,
                            description,
                            input_token_limit,
                            output_token_limit,
                            is_default,
                        });
                    }
                }
            }
            return result;
        }
    }

    // Fallback: parse plain text line outputs (e.g. from agy models or gemini --list-models)
    let mut text_models = Vec::new();
    for line in trimmed.lines() {
        let clean = line.trim().trim_start_matches(|c| c == '-' || c == '*' || c == '•').trim();
        if clean.starts_with("Fetching") || clean.is_empty() {
            continue;
        }

        if let Some((id, display_name)) = clean.split_once('\t') {
            let id = id.trim();
            if !id.is_empty() {
                text_models.push(CliModelInfo {
                    id: id.to_string(),
                    name: Some(display_name.trim().to_string()),
                    description: None,
                    input_token_limit: None,
                    output_token_limit: None,
                    is_default: Some(id.starts_with("auto")),
                });
                continue;
            }
        }

        if clean.starts_with("gemini-")
            || clean.starts_with("auto")
            || clean.starts_with("claude-")
            || clean.starts_with("gpt-")
            || clean.starts_with("orchestrator")
        {
            let parts: Vec<&str> = clean.split_whitespace().collect();
            if let Some(first) = parts.first() {
                text_models.push(CliModelInfo {
                    id: first.to_string(),
                    name: Some(clean.to_string()),
                    description: None,
                    input_token_limit: None,
                    output_token_limit: None,
                    is_default: Some(first.starts_with("auto")),
                });
            }
        }
    }
    text_models
}

pub fn get_cli_binary_version(bin_path: &std::path::Path) -> Option<String> {
    let v_out = crate::process_manager::ProcessSupervisor::build_cli_command(
        bin_path,
        &["--version"],
    )
    .output();

    v_out.ok().and_then(|o| {
        if o.status.success() {
            let v = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if v.is_empty() {
                None
            } else {
                Some(v)
            }
        } else {
            None
        }
    })
}

#[tauri::command]
pub async fn discover_gemini_models(
    state: State<'_, AppState>,
) -> Result<CliModelDiscoveryResult, String> {
    let active_engine = state.db.get_setting("active_cli_engine").ok().flatten().unwrap_or_else(|| "gemini".to_string());
    if active_engine == "agy" {
        let bin = match crate::process_manager::find_agy_executable() {
            Some(p) => p,
            None => {
                return Ok(CliModelDiscoveryResult {
                    supported: false,
                    models: Vec::new(),
                    message: "Antigravity CLI (agy) executable not found on PATH or standard directories.".to_string(),
                    engine: Some("agy".to_string()),
                    cli_version: None,
                });
            }
        };

        let cli_version = get_cli_binary_version(&bin);

        return tokio::task::spawn_blocking(move || {
            let mut cmd = crate::process_manager::ProcessSupervisor::build_cli_command(&bin, &["models"]);

            cmd.stdout(std::process::Stdio::piped());
            cmd.stderr(std::process::Stdio::piped());

            let output = match cmd.output() {
                Ok(o) => o,
                Err(e) => {
                    return Ok(CliModelDiscoveryResult {
                        supported: false,
                        models: Vec::new(),
                        message: format!("Failed to spawn agy models: {}", e),
                        engine: Some("agy".to_string()),
                        cli_version,
                    });
                }
            };

            let stdout_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let parsed = parse_cli_models_json(&stdout_str);
            if !parsed.is_empty() {
                let ver_str = cli_version.as_deref().map(|v| format!(" (v{})", v.trim_start_matches('v'))).unwrap_or_default();
                return Ok(CliModelDiscoveryResult {
                    supported: true,
                    models: parsed,
                    message: format!("Discovered models successfully from Antigravity CLI (agy){}.", ver_str),
                    engine: Some("agy".to_string()),
                    cli_version,
                });
            }

            Ok(CliModelDiscoveryResult {
                supported: false,
                models: Vec::new(),
                message: "No models returned from agy models.".to_string(),
                engine: Some("agy".to_string()),
                cli_version,
            })
        }).await.map_err(|e| e.to_string())?;
    }

    let bin = match crate::process_manager::find_gemini_executable() {
        Some(p) => p,
        None => {
            return Ok(CliModelDiscoveryResult {
                supported: false,
                models: Vec::new(),
                message: "Gemini CLI executable not found on PATH.".to_string(),
                engine: Some("gemini".to_string()),
                cli_version: None,
            });
        }
    };

    let cli_version = get_cli_binary_version(&bin);
    let effective_config = match get_gemini_auth_config(state.clone()).await {
        Ok(c) => c,
        Err(_) => GeminiAuthConfig {
            auth_mode: "api_key".to_string(),
            api_key: None,
            google_cloud_project: None,
            google_cloud_location: None,
            google_app_credentials: None,
            agy_adc_auth: None,
        },
    };

    tokio::task::spawn_blocking(move || {
        // Run: gemini models --output-format json
        let mut cmd = crate::process_manager::ProcessSupervisor::build_cli_command(
            &bin,
            &["models", "--output-format", "json"],
        );

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

        let output = match cmd.output() {
            Ok(o) => o,
            Err(e) => {
                return Ok(CliModelDiscoveryResult {
                    supported: false,
                    models: Vec::new(),
                    message: format!("Failed to spawn gemini models: {}", e),
                    engine: Some("gemini".to_string()),
                    cli_version,
                });
            }
        };

        let stdout_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr_str = String::from_utf8_lossy(&output.stderr).trim().to_string();

        let parsed = parse_cli_models_json(&stdout_str);
        if !parsed.is_empty() {
            let ver_str = cli_version.as_deref().map(|v| format!(" (v{})", v.trim_start_matches('v'))).unwrap_or_else(|| " (v0.67+)".to_string());
            return Ok(CliModelDiscoveryResult {
                supported: true,
                models: parsed,
                message: format!("Discovered models successfully from Gemini CLI{}.", ver_str),
                engine: Some("gemini".to_string()),
                cli_version,
            });
        }

        // If 'models' subcommand failed, try 'gemini --list-models' as fallback (v0.67.0 flag)
        let mut cmd_flag = crate::process_manager::ProcessSupervisor::build_cli_command(
            &bin,
            &["--list-models"],
        );
        cmd_flag.stdout(std::process::Stdio::piped());
        cmd_flag.stderr(std::process::Stdio::piped());

        if let Ok(flag_output) = cmd_flag.output() {
            let flag_stdout = String::from_utf8_lossy(&flag_output.stdout).trim().to_string();
            let flag_parsed = parse_cli_models_json(&flag_stdout);
            if !flag_parsed.is_empty() {
                let ver_str = cli_version.as_deref().map(|v| format!(" (v{})", v.trim_start_matches('v'))).unwrap_or_else(|| " (v0.67+)".to_string());
                return Ok(CliModelDiscoveryResult {
                    supported: true,
                    models: flag_parsed,
                    message: format!("Discovered models successfully via Gemini CLI --list-models{}.", ver_str),
                    engine: Some("gemini".to_string()),
                    cli_version,
                });
            }
        }

        let combined = format!("{}\n{}", stdout_str, stderr_str);
        let notice = if combined.contains("Usage: gemini") || combined.contains("Unknown argument") {
            "Dynamic model discovery is supported in Gemini CLI v0.67.0+. Using official standard models.".to_string()
        } else if !stderr_str.is_empty() {
            stderr_str
        } else {
            "No models returned from CLI.".to_string()
        };

        Ok(CliModelDiscoveryResult {
            supported: false,
            models: Vec::new(),
            message: notice,
            engine: Some("gemini".to_string()),
            cli_version,
        })
    }).await.map_err(|e| e.to_string())?
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CliPathStatus {
    pub gemini_path: Option<String>,
    pub gemini_dir: Option<String>,
    pub gemini_in_user_path: bool,
    pub agy_path: Option<String>,
    pub agy_dir: Option<String>,
    pub agy_in_user_path: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PathAddResult {
    pub success: bool,
    pub message: String,
    pub target_dir: String,
    pub already_in_path: bool,
}

/// Non-secret GCP & CLI configuration variables eligible for Windows User Environment sync.
pub const MANAGED_GCP_USER_ENV_KEYS: [&str; 4] = [
    "GOOGLE_CLOUD_PROJECT",
    "GOOGLE_CLOUD_LOCATION",
    "GOOGLE_GENAI_USE_VERTEXAI",
    "AGY_ADC_AUTH",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UserEnvVarStatus {
    pub name: String,
    pub value: Option<String>,
    pub is_set: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserEnvSyncRequest {
    pub google_cloud_project: Option<String>,
    pub google_cloud_location: Option<String>,
    #[serde(default)]
    pub google_genai_use_vertexai: Option<bool>,
    #[serde(default)]
    pub agy_adc_auth: Option<bool>,
    #[serde(default)]
    pub clear_unspecified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserEnvSyncResult {
    pub success: bool,
    pub message: String,
    pub updated_vars: Vec<String>,
    pub removed_vars: Vec<String>,
    pub vars: Vec<UserEnvVarStatus>,
}

/// Validates and translates a `UserEnvSyncRequest` into deterministic set (`Some`) or delete (`None`)
/// Registry operations scoped strictly to `MANAGED_GCP_USER_ENV_KEYS`.
pub fn build_managed_user_env_plan(
    req: &UserEnvSyncRequest,
) -> Result<Vec<(&'static str, Option<String>)>, String> {
    let validate_value = |key: &str, raw: &str| -> Result<String, String> {
        let trimmed = raw.trim();
        if trimmed.contains('\n') || trimmed.contains('\r') || trimmed.contains('\0') {
            return Err(format!("{} cannot contain newlines or control characters.", key));
        }
        Ok(trimmed.to_string())
    };

    let mut plan: Vec<(&'static str, Option<String>)> = Vec::new();

    match req.google_cloud_project.as_deref().map(str::trim) {
        Some(p) if !p.is_empty() => {
            plan.push(("GOOGLE_CLOUD_PROJECT", Some(validate_value("GOOGLE_CLOUD_PROJECT", p)?)));
        }
        _ if req.clear_unspecified => plan.push(("GOOGLE_CLOUD_PROJECT", None)),
        _ => {}
    }

    match req.google_cloud_location.as_deref().map(str::trim) {
        Some(loc) if !loc.is_empty() => {
            plan.push(("GOOGLE_CLOUD_LOCATION", Some(validate_value("GOOGLE_CLOUD_LOCATION", loc)?)));
        }
        _ if req.clear_unspecified => plan.push(("GOOGLE_CLOUD_LOCATION", None)),
        _ => {}
    }

    match req.google_genai_use_vertexai {
        Some(true) => plan.push(("GOOGLE_GENAI_USE_VERTEXAI", Some("true".to_string()))),
        Some(false) => plan.push(("GOOGLE_GENAI_USE_VERTEXAI", None)),
        None if req.clear_unspecified => plan.push(("GOOGLE_GENAI_USE_VERTEXAI", None)),
        None => {}
    }

    match req.agy_adc_auth {
        Some(true) => plan.push(("AGY_ADC_AUTH", Some("true".to_string()))),
        Some(false) => plan.push(("AGY_ADC_AUTH", None)),
        None if req.clear_unspecified => plan.push(("AGY_ADC_AUTH", None)),
        None => {}
    }

    Ok(plan)
}

/// Reads a Windows User Environment Variable directly from `HKCU\Environment` (0 subprocesses).
pub fn read_windows_user_env_var(name: &str) -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::System::Registry::{
            RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_CURRENT_USER, KEY_READ,
        };

        unsafe {
            let subkey: Vec<u16> = OsStr::new("Environment")
                .encode_wide()
                .chain(Some(0))
                .collect();
            let val_name: Vec<u16> = OsStr::new(name)
                .encode_wide()
                .chain(Some(0))
                .collect();

            let mut hkey: HKEY = std::ptr::null_mut();
            if RegOpenKeyExW(HKEY_CURRENT_USER, subkey.as_ptr(), 0, KEY_READ, &mut hkey) == 0 && !hkey.is_null() {
                let mut data_len: u32 = 0;
                if RegQueryValueExW(hkey, val_name.as_ptr(), std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut(), &mut data_len) == 0 && data_len > 0 {
                    let mut buffer: Vec<u8> = vec![0u8; data_len as usize + 2];
                    if RegQueryValueExW(hkey, val_name.as_ptr(), std::ptr::null_mut(), std::ptr::null_mut(), buffer.as_mut_ptr(), &mut data_len) == 0 {
                        let _ = RegCloseKey(hkey);
                        let u16_slice: &[u16] = std::slice::from_raw_parts(buffer.as_ptr() as *const u16, data_len as usize / 2);
                        let end = u16_slice.iter().position(|&c| c == 0).unwrap_or(u16_slice.len());
                        let val_str = String::from_utf16_lossy(&u16_slice[..end]).trim().to_string();
                        if !val_str.is_empty() {
                            return Some(val_str);
                        }
                        return None;
                    }
                }
                let _ = RegCloseKey(hkey);
            }
        }
        None
    }

    #[cfg(not(target_os = "windows"))]
    {
        std::env::var(name)
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    }
}

pub fn read_managed_user_env_statuses() -> Vec<UserEnvVarStatus> {
    MANAGED_GCP_USER_ENV_KEYS
        .iter()
        .map(|&name| {
            let value = read_windows_user_env_var(name);
            let is_set = value.is_some();
            UserEnvVarStatus {
                name: name.to_string(),
                value,
                is_set,
            }
        })
        .collect()
}

pub fn get_user_path_dirs() -> Vec<std::path::PathBuf> {
    #[cfg(target_os = "windows")]
    {
        if let Some(path_str) = read_windows_user_env_var("Path") {
            return path_str
                .split(';')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .map(std::path::PathBuf::from)
                .collect();
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(path_var) = std::env::var("PATH") {
            return std::env::split_paths(&path_var).collect();
        }
    }

    Vec::new()
}

#[tauri::command]
pub async fn get_user_env_vars_status() -> Result<Vec<UserEnvVarStatus>, String> {
    tokio::task::spawn_blocking(read_managed_user_env_statuses)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn sync_user_env_vars(
    request: UserEnvSyncRequest,
) -> Result<UserEnvSyncResult, String> {
    let plan = build_managed_user_env_plan(&request)?;

    tokio::task::spawn_blocking(move || {
        if plan.is_empty() {
            return Ok(UserEnvSyncResult {
                success: false,
                message: "No Google Cloud Project or Vertex settings provided to sync.".to_string(),
                updated_vars: Vec::new(),
                removed_vars: Vec::new(),
                vars: read_managed_user_env_statuses(),
            });
        }

        #[cfg(target_os = "windows")]
        {
            let mut updated_vars = Vec::new();
            let mut removed_vars = Vec::new();
            let mut cmd = Command::new("powershell.exe");
            let ps_script = r#"
$count = [int]$env:GEMINI_UENV_COUNT
for ($i = 0; $i -lt $count; $i++) {
  $name = [Environment]::GetEnvironmentVariable("GEMINI_UENV_NAME_$i", 'Process')
  $op = [Environment]::GetEnvironmentVariable("GEMINI_UENV_OP_$i", 'Process')
  $val = [Environment]::GetEnvironmentVariable("GEMINI_UENV_VAL_$i", 'Process')
  if ($op -eq 'remove') {
    [Environment]::SetEnvironmentVariable($name, $null, 'User')
  } else {
    [Environment]::SetEnvironmentVariable($name, $val, 'User')
  }
}
"#;
            cmd.args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", ps_script]);
            cmd.creation_flags(0x08000000);
            cmd.env("GEMINI_UENV_COUNT", plan.len().to_string());

            for (idx, (key, val_opt)) in plan.iter().enumerate() {
                cmd.env(format!("GEMINI_UENV_NAME_{}", idx), key);
                match val_opt {
                    Some(v) => {
                        cmd.env(format!("GEMINI_UENV_OP_{}", idx), "set");
                        cmd.env(format!("GEMINI_UENV_VAL_{}", idx), v);
                        updated_vars.push((*key).to_string());
                    }
                    None => {
                        cmd.env(format!("GEMINI_UENV_OP_{}", idx), "remove");
                        removed_vars.push((*key).to_string());
                    }
                }
            }

            match cmd.output() {
                Ok(out) if out.status.success() => {
                    // Keep the desktop app's own process environment in sync immediately too
                    for (key, val_opt) in &plan {
                        match val_opt {
                            Some(v) => std::env::set_var(key, v),
                            None => std::env::remove_var(key),
                        }
                    }

                    let mut parts = Vec::new();
                    if !updated_vars.is_empty() {
                        parts.push(format!("Set {} in Windows User Env", updated_vars.join(", ")));
                    }
                    if !removed_vars.is_empty() {
                        parts.push(format!("Cleared {} from Windows User Env", removed_vars.join(", ")));
                    }
                    let summary = format!("{}. New terminals will pick these up automatically.", parts.join("; "));

                    Ok(UserEnvSyncResult {
                        success: true,
                        message: summary,
                        updated_vars,
                        removed_vars,
                        vars: read_managed_user_env_statuses(),
                    })
                }
                Ok(out) => {
                    let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
                    Ok(UserEnvSyncResult {
                        success: false,
                        message: if err.is_empty() {
                            "Failed to update Windows User Environment variables.".to_string()
                        } else {
                            format!("Failed to update Windows User Environment variables: {}", err)
                        },
                        updated_vars: Vec::new(),
                        removed_vars: Vec::new(),
                        vars: read_managed_user_env_statuses(),
                    })
                }
                Err(e) => Ok(UserEnvSyncResult {
                    success: false,
                    message: format!("Failed to invoke PowerShell to update User Environment: {}", e),
                    updated_vars: Vec::new(),
                    removed_vars: Vec::new(),
                    vars: read_managed_user_env_statuses(),
                }),
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            Ok(UserEnvSyncResult {
                success: false,
                message: "Automatic User Environment sync is only supported on Windows.".to_string(),
                updated_vars: Vec::new(),
                removed_vars: Vec::new(),
                vars: read_managed_user_env_statuses(),
            })
        }
    }).await.map_err(|e| e.to_string())?
}

pub fn is_dir_in_path_list(target_dir: &std::path::Path, path_list: &[std::path::PathBuf]) -> bool {
    let norm_target = target_dir.to_string_lossy().to_lowercase().replace('/', "\\").trim_end_matches('\\').to_string();
    for d in path_list {
        let norm_d = d.to_string_lossy().to_lowercase().replace('/', "\\").trim_end_matches('\\').to_string();
        if norm_d == norm_target {
            return true;
        }
    }
    false
}

pub fn is_dir_in_user_path(target_dir: &std::path::Path) -> bool {
    let dirs = get_user_path_dirs();
    is_dir_in_path_list(target_dir, &dirs)
}

#[tauri::command]
pub async fn check_cli_path_status() -> Result<CliPathStatus, String> {
    tokio::task::spawn_blocking(|| {
        let gemini_exe = crate::process_manager::find_gemini_executable();
        let gemini_dir = gemini_exe.as_ref().and_then(|p| p.parent().map(|d| d.to_path_buf()))
            .or_else(|| {
                std::env::var("APPDATA").ok().map(|a| std::path::PathBuf::from(a).join("npm"))
            });

        let agy_exe = crate::process_manager::find_agy_executable();
        let agy_dir = agy_exe.as_ref().and_then(|p| p.parent().map(|d| d.to_path_buf()))
            .or_else(|| {
                std::env::var("LOCALAPPDATA").ok().map(|l| std::path::PathBuf::from(l).join("agy").join("bin"))
            });

        let user_dirs = get_user_path_dirs();
        let gemini_in_user_path = gemini_dir.as_ref().map(|d| is_dir_in_path_list(d, &user_dirs)).unwrap_or(false);
        let agy_in_user_path = agy_dir.as_ref().map(|d| is_dir_in_path_list(d, &user_dirs)).unwrap_or(false);

        Ok(CliPathStatus {
            gemini_path: gemini_exe.map(|p| p.to_string_lossy().to_string()),
            gemini_dir: gemini_dir.map(|d| d.to_string_lossy().to_string()),
            gemini_in_user_path,
            agy_path: agy_exe.map(|p| p.to_string_lossy().to_string()),
            agy_dir: agy_dir.map(|d| d.to_string_lossy().to_string()),
            agy_in_user_path,
        })
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn add_cli_to_user_path(engine: String) -> Result<PathAddResult, String> {
    tokio::task::spawn_blocking(move || {
        let is_agy = engine.trim().eq_ignore_ascii_case("agy");
        let target_dir = if is_agy {
            crate::process_manager::find_agy_executable()
                .and_then(|p| p.parent().map(|d| d.to_path_buf()))
                .unwrap_or_else(|| {
                    let localapp = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| "C:\\Users\\Default\\AppData\\Local".to_string());
                    std::path::PathBuf::from(localapp).join("agy").join("bin")
                })
        } else {
            crate::process_manager::find_gemini_executable()
                .and_then(|p| p.parent().map(|d| d.to_path_buf()))
                .unwrap_or_else(|| {
                    let appdata = std::env::var("APPDATA").unwrap_or_else(|_| "C:\\Users\\Default\\AppData\\Roaming".to_string());
                    std::path::PathBuf::from(appdata).join("npm")
                })
        };

        let target_dir_str = target_dir.to_string_lossy().to_string();

        if is_dir_in_user_path(&target_dir) {
            return Ok(PathAddResult {
                success: true,
                message: format!("'{}' is already present in your User PATH environment variable.", target_dir_str),
                target_dir: target_dir_str,
                already_in_path: true,
            });
        }

        #[cfg(target_os = "windows")]
        {
            let escaped_dir = target_dir_str.replace('\'', "''");
            let ps_script = format!(
                "$curr = [Environment]::GetEnvironmentVariable('Path', 'User'); if (-not $curr) {{ $curr = '' }}; $new = if ($curr.Trim().Length -gt 0) {{ $curr.TrimEnd(';') + ';' + '{}' }} else {{ '{}' }}; [Environment]::SetEnvironmentVariable('Path', $new, 'User')",
                escaped_dir, escaped_dir
            );

            let mut cmd = Command::new("powershell.exe");
            cmd.args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &ps_script]);
            cmd.creation_flags(0x08000000);

            match cmd.output() {
                Ok(out) => {
                    if out.status.success() {
                        if let Ok(curr_proc_path) = std::env::var("PATH") {
                            let updated_proc_path = format!("{};{}", target_dir_str, curr_proc_path);
                            std::env::set_var("PATH", updated_proc_path);
                        }
                        crate::process_manager::invalidate_cli_path_cache();

                        Ok(PathAddResult {
                            success: true,
                            message: format!(
                                "Successfully added '{}' to your User PATH. All new terminals and command prompts will now recognize '{}' globally.",
                                target_dir_str,
                                if is_agy { "agy" } else { "gemini" }
                            ),
                            target_dir: target_dir_str,
                            already_in_path: false,
                        })
                    } else {
                        let err = String::from_utf8_lossy(&out.stderr).to_string();
                        Ok(PathAddResult {
                            success: false,
                            message: format!("Failed to update User PATH via PowerShell: {}", err.trim()),
                            target_dir: target_dir_str,
                            already_in_path: false,
                        })
                    }
                }
                Err(e) => {
                    Ok(PathAddResult {
                        success: false,
                        message: format!("Failed to invoke PowerShell process to update User PATH: {}", e),
                        target_dir: target_dir_str,
                        already_in_path: false,
                    })
                }
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            Ok(PathAddResult {
                success: false,
                message: format!("Automatic PATH modification is only supported on Windows. Please manually add '{}' to your PATH.", target_dir_str),
                target_dir: target_dir_str,
                already_in_path: false,
            })
        }
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn update_or_install_agy_cli() -> Result<CliInstallResult, String> {
    tokio::task::spawn_blocking(move || {
        let user_home = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME"))
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")));

        let existing_bin = crate::process_manager::find_agy_executable();

        #[cfg(target_os = "windows")]
        let (output, method) = if let Some(ref bin_path) = existing_bin {
            let mut c = crate::process_manager::ProcessSupervisor::build_cli_command(bin_path, &["update"]);
            c.current_dir(&user_home);
            (c.output(), "agy update")
        } else {
            let mut c = Command::new("powershell.exe");
            c.args([
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                "irm https://antigravity.google/cli/install.ps1 | iex"
            ]);
            c.current_dir(&user_home).creation_flags(0x08000000);
            (c.output(), "antigravity.google/cli/install.ps1")
        };

        #[cfg(not(target_os = "windows"))]
        let (output, method) = if let Some(ref bin_path) = existing_bin {
            let mut c = Command::new(bin_path);
            c.arg("update").current_dir(&user_home);
            (c.output(), "agy update")
        } else {
            let mut c = Command::new("bash");
            c.args(["-c", "curl -fsSL https://antigravity.google/cli/install.sh | bash"]);
            c.current_dir(&user_home);
            (c.output(), "install.sh")
        };

        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);
                if out.status.success() {
                    crate::process_manager::invalidate_cli_path_cache();
                    let bin_path = crate::process_manager::find_agy_executable();
                    let ver = bin_path.as_ref().and_then(|p| get_cli_binary_version(p));

                    Ok(CliInstallResult {
                        success: true,
                        message: format!("Antigravity CLI (agy) successfully updated via {}!", method),
                        installed_path: bin_path.map(|p| p.to_string_lossy().to_string()),
                        version: ver,
                    })
                } else {
                    let err_msg = if !stderr.trim().is_empty() { stderr.trim().to_string() } else { stdout.trim().to_string() };
                    Ok(CliInstallResult {
                        success: false,
                        message: format!("Antigravity CLI update via {} exited with error: {}", method, err_msg),
                        installed_path: None,
                        version: None,
                    })
                }
            }
            Err(e) => {
                Ok(CliInstallResult {
                    success: false,
                    message: format!("Failed to launch {} updater process: {}", method, e),
                    installed_path: None,
                    version: None,
                })
            }
        }
    }).await.map_err(|e| e.to_string())?
}

pub fn classify_agy_test_error(raw: &str) -> String {
    let lower = raw.to_lowercase();
    if lower.contains("unauthenticated") || lower.contains("401") || lower.contains("not logged in") || lower.contains("oauth") {
        format!("Antigravity CLI authentication required: {}. Click 'Sign In via agy' or configure a Gemini API key.", raw.trim())
    } else if lower.contains("prepaid") || lower.contains("402") || lower.contains("quota") || lower.contains("billing") {
        format!("Antigravity quota or prepaid wallet error: {}. Check your license or billing status.", raw.trim())
    } else if lower.contains("not_found") || lower.contains("model") && lower.contains("not found") {
        format!("Model unavailable or deprecated: {}. Refresh discovered models in Workspace settings.", raw.trim())
    } else {
        raw.trim().to_string()
    }
}

#[tauri::command]
pub async fn test_agy_cli(
    state: State<'_, AppState>,
    config: Option<GeminiAuthConfig>,
    workspace_path: Option<String>,
    verify_auth: Option<bool>,
) -> Result<CliTestResult, String> {
    let effective_config = match config {
        Some(c) => Some(c),
        None => get_gemini_auth_config(state.clone()).await.ok(),
    };

    tokio::task::spawn_blocking(move || {
        let bin = match crate::process_manager::find_agy_executable() {
            Some(p) => p,
            None => {
                return Ok(CliTestResult {
                    success: false,
                    latency_ms: 0,
                    message: "Antigravity CLI (agy) not found on system PATH or standard directories.".to_string(),
                    details: Some("Please install Antigravity CLI or add its directory to your PATH.".to_string()),
                });
            }
        };

        let start = Instant::now();
        let path_str = bin.to_string_lossy().to_string();

        let user_home = crate::process_manager::ProcessSupervisor::fallback_safe_dir();
        let work_dir = workspace_path
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(std::path::PathBuf::from)
            .filter(|p| p.exists())
            .unwrap_or_else(|| user_home.clone());

        let run_agy = |args: &[&str], inject_auth: bool| -> std::io::Result<std::process::Output> {
            let mut cmd = crate::process_manager::ProcessSupervisor::build_cli_command(&bin, args);
            cmd.current_dir(&work_dir);
            cmd.env("CI", "true");
            if inject_auth {
                if let Some(ref cfg) = effective_config {
                    if cfg.agy_adc_auth == Some(true) {
                        cmd.env("AGY_ADC_AUTH", "true");
                    }
                    if cfg.auth_mode == "vertex_ai" {
                        if let Some(ref proj) = cfg.google_cloud_project {
                            if !proj.trim().is_empty() {
                                cmd.env("GOOGLE_GENAI_USE_VERTEXAI", "true");
                                cmd.env("GOOGLE_CLOUD_PROJECT", proj.trim());
                            }
                        }
                        if let Some(ref loc) = cfg.google_cloud_location {
                            if !loc.trim().is_empty() {
                                cmd.env("GOOGLE_CLOUD_LOCATION", loc.trim());
                            }
                        }
                        if let Some(ref creds) = cfg.google_app_credentials {
                            if !creds.trim().is_empty() {
                                cmd.env("GOOGLE_APPLICATION_CREDENTIALS", creds.trim());
                            }
                        }
                    } else if let Some(ref key) = cfg.api_key {
                        if !key.trim().is_empty() {
                            cmd.env("GEMINI_API_KEY", key.trim());
                        }
                    }
                }
            }
            cmd.output()
        };

        let version_out = run_agy(&["--version"], false);
        let latency_ms = start.elapsed().as_millis() as u64;

        match version_out {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
                let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
                if !out.status.success() {
                    let raw_err = if !stderr.is_empty() { stderr } else { stdout };
                    return Ok(CliTestResult {
                        success: false,
                        latency_ms,
                        message: format!("Antigravity CLI test failed with exit code: {:?}", out.status.code()),
                        details: Some(classify_agy_test_error(&raw_err)),
                    });
                }

                // Optionally perform a live prompt probe when explicitly requested
                if verify_auth.unwrap_or(false) {
                    let live_start = Instant::now();
                    match run_agy(&["-p", "Respond with 'OK' and nothing else.", "--output-format", "json"], true) {
                        Ok(live_out) => {
                            let live_ms = live_start.elapsed().as_millis() as u64;
                            let live_stdout = String::from_utf8_lossy(&live_out.stdout).trim().to_string();
                            let live_stderr = String::from_utf8_lossy(&live_out.stderr).trim().to_string();
                            if live_out.status.success() {
                                return Ok(CliTestResult {
                                    success: true,
                                    latency_ms: live_ms,
                                    message: format!(
                                        "Antigravity CLI & authentication verified (version {}, {} ms).",
                                        if stdout.is_empty() { "verified" } else { &stdout },
                                        live_ms
                                    ),
                                    details: Some(format!("Binary: {}\nLive JSON probe succeeded.", path_str)),
                                });
                            } else {
                                let err_text = if !live_stderr.is_empty() { &live_stderr } else { &live_stdout };
                                return Ok(CliTestResult {
                                    success: false,
                                    latency_ms: live_ms,
                                    message: classify_agy_test_error(err_text),
                                    details: Some(format!("Version: {}\nStderr: {}", stdout, err_text)),
                                });
                            }
                        }
                        Err(e) => {
                            return Ok(CliTestResult {
                                success: false,
                                latency_ms,
                                message: format!("Live auth probe failed: {}", e),
                                details: Some(path_str),
                            });
                        }
                    }
                }

                Ok(CliTestResult {
                    success: true,
                    latency_ms,
                    message: format!("Antigravity CLI is operational (version {}).", if stdout.is_empty() { "verified" } else { &stdout }),
                    details: Some(format!("Binary: {}\nOutput: {}", path_str, stdout)),
                })
            }
            Err(e) => {
                Ok(CliTestResult {
                    success: false,
                    latency_ms,
                    message: format!("Failed to execute Antigravity CLI binary: {}", e),
                    details: Some(path_str),
                })
            }
        }
    }).await.map_err(|e| e.to_string())?
}

/// Launches Antigravity CLI's interactive authentication flow (`agy acp --login`).
#[tauri::command]
pub async fn launch_agy_login() -> Result<CliInstallResult, String> {
    let bin = crate::process_manager::find_agy_executable()
        .ok_or_else(|| "Antigravity CLI (agy) executable not found on PATH.".to_string())?;
    let path_str = bin.to_string_lossy().to_string();
    let ver = get_cli_binary_version(&bin);

    #[cfg(target_os = "windows")]
    {
        Command::new("cmd.exe")
            .args(["/c", "start", "Antigravity CLI Login", &path_str, "acp", "--login"])
            .spawn()
            .map_err(|e| format!("Failed to launch agy acp --login: {}", e))?;
    }

    #[cfg(not(target_os = "windows"))]
    {
        Command::new(&path_str)
            .args(["acp", "--login"])
            .spawn()
            .map_err(|e| format!("Failed to launch agy acp --login: {}", e))?;
    }

    Ok(CliInstallResult {
        success: true,
        message: "Opened Antigravity CLI sign-in window (`agy acp --login`). Complete authentication, then test connection or discover models.".to_string(),
        installed_path: Some(path_str),
        version: ver,
    })
}

/// Launches Gemini CLI in an interactive terminal window so the user can complete browser OAuth sign-in.
#[tauri::command]
pub async fn launch_gemini_login(state: State<'_, AppState>) -> Result<CliInstallResult, String> {
    let bin = crate::process_manager::find_gemini_executable()
        .ok_or_else(|| "Gemini CLI (gemini) executable not found on PATH or standard npm directories.".to_string())?;
    let path_str = bin.to_string_lossy().to_string();
    let ver = get_cli_binary_version(&bin);
    let saved_cfg = get_gemini_auth_config(state.clone()).await.ok();

    let mut env_pairs: Vec<(&'static str, String)> = Vec::new();
    if let Some(cfg) = saved_cfg {
        if cfg.auth_mode == "vertex_ai" {
            env_pairs.push(("GOOGLE_GENAI_USE_VERTEXAI", "true".to_string()));
            if let Some(proj) = cfg.google_cloud_project {
                env_pairs.push(("GOOGLE_CLOUD_PROJECT", proj));
            }
            if let Some(loc) = cfg.google_cloud_location {
                env_pairs.push(("GOOGLE_CLOUD_LOCATION", loc));
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        let mut cmd = Command::new("cmd.exe");
        cmd.args(["/c", "start", "Gemini CLI Login", &path_str]);
        for (k, v) in &env_pairs {
            cmd.env(k, v);
        }
        cmd.spawn()
            .map_err(|e| format!("Failed to launch Gemini CLI login terminal: {}", e))?;
    }

    #[cfg(not(target_os = "windows"))]
    {
        let mut cmd = Command::new(&path_str);
        for (k, v) in &env_pairs {
            cmd.env(k, v);
        }
        cmd.spawn()
            .map_err(|e| format!("Failed to launch Gemini CLI login: {}", e))?;
    }

    Ok(CliInstallResult {
        success: true,
        message: "Opened Gemini CLI sign-in window. Follow the browser OAuth prompt in the terminal, then return here to run Connection Test.".to_string(),
        installed_path: Some(path_str),
        version: ver,
    })
}

/// Opens a native file dialog on Windows to pick a Service Account JSON credentials file.
#[tauri::command]
pub async fn pick_credentials_file(initial_path: Option<String>) -> Result<Option<String>, String> {
    #[cfg(target_os = "windows")]
    {
        let init_dir = initial_path
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .and_then(|s| {
                let p = std::path::PathBuf::from(s);
                if p.is_file() {
                    p.parent().map(|par| par.to_string_lossy().to_string())
                } else if p.is_dir() {
                    Some(s.to_string())
                } else {
                    p.parent().filter(|par| par.is_dir()).map(|par| par.to_string_lossy().to_string())
                }
            })
            .or_else(|| std::env::var("USERPROFILE").ok())
            .unwrap_or_default();

        tokio::task::spawn_blocking(move || {
            let script = r#"
Add-Type -AssemblyName System.Windows.Forms
$owner = New-Object System.Windows.Forms.Form
$owner.TopMost = $true
$owner.StartPosition = 'Manual'
$owner.Size = New-Object System.Drawing.Size(0, 0)
$dialog = New-Object System.Windows.Forms.OpenFileDialog
$dialog.Title = 'Select Google Service Account JSON Key'
$dialog.Filter = 'JSON Credentials (*.json)|*.json|All Files (*.*)|*.*'
if ($env:GEMINI_PICKER_INIT_DIR -and (Test-Path -LiteralPath $env:GEMINI_PICKER_INIT_DIR)) {
  $dialog.InitialDirectory = $env:GEMINI_PICKER_INIT_DIR
}
$result = $dialog.ShowDialog($owner)
if ($result -eq [System.Windows.Forms.DialogResult]::OK -and $dialog.FileName) {
  [Console]::OutputEncoding = [System.Text.Encoding]::UTF8
  Write-Output $dialog.FileName
}
$dialog.Dispose()
$owner.Dispose()
"#;
            let mut cmd = Command::new("powershell.exe");
            cmd.args(["-NoProfile", "-NonInteractive", "-STA", "-Command", script]);
            cmd.env("GEMINI_PICKER_INIT_DIR", &init_dir);
            cmd.creation_flags(0x08000000);
            let output = cmd.output().map_err(|e| format!("Failed to launch file picker: {}", e))?;
            if !output.status.success() {
                let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
                return Err(if err.is_empty() {
                    "File picker exited unexpectedly.".to_string()
                } else {
                    err
                });
            }
            let picked = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if picked.is_empty() {
                Ok(None)
            } else {
                Ok(Some(picked))
            }
        })
        .await
        .map_err(|e| format!("File picker task failed: {}", e))?
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = initial_path;
        Ok(None)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PluginImportResult {
    pub success: bool,
    pub message: String,
    pub output: String,
}

#[tauri::command]
pub async fn import_gemini_plugins_via_agy() -> Result<PluginImportResult, String> {
    let bin = match crate::process_manager::find_agy_executable() {
        Some(p) => p,
        None => {
            return Ok(PluginImportResult {
                success: false,
                message: "Antigravity CLI (agy) executable not found.".to_string(),
                output: String::new(),
            });
        }
    };

    tokio::task::spawn_blocking(move || {
        let user_home = crate::process_manager::ProcessSupervisor::fallback_safe_dir();

        let mut cmd = crate::process_manager::ProcessSupervisor::build_cli_command(
            &bin,
            &["plugin", "import", "gemini"],
        );
        cmd.current_dir(&user_home);
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        match cmd.output() {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
                let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
                let combined = if !stderr.is_empty() {
                    format!("{}\n{}", stdout, stderr).trim().to_string()
                } else {
                    stdout
                };

                Ok(PluginImportResult {
                    success: out.status.success(),
                    message: if out.status.success() {
                        "Gemini extensions processed by Antigravity CLI.".to_string()
                    } else {
                        format!("Import exited with code: {:?}", out.status.code())
                    },
                    output: combined,
                })
            }
            Err(e) => Ok(PluginImportResult {
                success: false,
                message: format!("Failed to execute agy plugin import: {}", e),
                output: String::new(),
            }),
        }
    }).await.map_err(|e| e.to_string())?
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SkillsSyncResult {
    pub success: bool,
    pub synced_count: usize,
    pub skill_names: Vec<String>,
    pub message: String,
}

pub fn sync_workspace_skills_internal(workspace_path: &str) -> Result<SkillsSyncResult, String> {
    let ws = std::path::PathBuf::from(workspace_path);
    let gemini_skills = ws.join(".gemini").join("skills");
    let agy_skills = ws.join(".agents").join("skills");

    if !gemini_skills.is_dir() {
        return Ok(SkillsSyncResult {
            success: true,
            synced_count: 0,
            skill_names: Vec::new(),
            message: "No .gemini/skills directory found in workspace.".to_string(),
        });
    }

    std::fs::create_dir_all(&agy_skills)
        .map_err(|e| format!("Failed to create .agents/skills directory: {}", e))?;

    let mut synced_names = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&gemini_skills) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(folder_name) = path.file_name() {
                    let target_dir = agy_skills.join(folder_name);
                    if copy_dir_all_recursive(&path, &target_dir).is_ok() {
                        synced_names.push(folder_name.to_string_lossy().to_string());
                    }
                }
            }
        }
    }

    let count = synced_names.len();
    Ok(SkillsSyncResult {
        success: true,
        synced_count: count,
        skill_names: synced_names.clone(),
        message: format!("Successfully synced {} skill(s) to .agents/skills: {}", count, synced_names.join(", ")),
    })
}

fn copy_dir_all_recursive(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all_recursive(&entry.path(), &dst.join(entry.file_name()))?;
        } else {
            std::fs::copy(entry.path(), dst.join(entry.file_name()))?;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn sync_workspace_skills_for_agy(workspace_path: String) -> Result<SkillsSyncResult, String> {
    tokio::task::spawn_blocking(move || {
        sync_workspace_skills_internal(&workspace_path)
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
            agy_adc_auth: Some(true),
        };

        let json = serde_json::to_string(&config).expect("failed to serialize");
        assert!(json.contains("corp-ai-sample-123"));
        assert!(json.contains("vertex_ai"));

        let deserialized: GeminiAuthConfig = serde_json::from_str(&json).expect("failed to deserialize");
        assert_eq!(deserialized.auth_mode, "vertex_ai");
        assert_eq!(deserialized.google_cloud_project.as_deref(), Some("corp-ai-sample-123"));
        assert_eq!(deserialized.agy_adc_auth, Some(true));
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

    #[test]
    fn test_deprecated_model_error_detection() {
        let sample_err_15 = "Error: models/gemini-1.5-flash-8b is not found for API version v1beta.";
        let is_deprecated = sample_err_15.contains("deprecated")
            || sample_err_15.contains("not found for API version")
            || sample_err_15.contains("models/gemini-1.5");
        assert!(is_deprecated);

        let sample_err_preview = "Error: model gemini-2.5-flash-lite-preview-09-2025 has been discontinued.";
        let is_preview_dead = sample_err_preview.contains("models/gemini-2.5-flash-lite-preview")
            || sample_err_preview.contains("discontinued")
            || sample_err_preview.contains("gemini-2.5-flash-lite-preview");
        assert!(is_preview_dead);
    }

    #[test]
    fn test_parse_cli_models_json_array() {
        let json_input = r#"[
            {
                "id": "gemini-3.8-flash",
                "displayName": "Gemini 3.8 Flash",
                "description": "Ultra-responsive next-gen Flash",
                "inputTokenLimit": 1048576,
                "outputTokenLimit": 8192,
                "isDefault": true
            },
            {
                "id": "gemini-3.5-flash-lite",
                "displayName": "Gemini 3.5 Flash-Lite",
                "inputTokenLimit": 1048576,
                "outputTokenLimit": 8192,
                "isDefault": false
            }
        ]"#;

        let models = parse_cli_models_json(json_input);
        assert_eq!(models.len(), 2);
        assert_eq!(models[0].id, "gemini-3.8-flash");
        assert_eq!(models[0].name.as_deref(), Some("Gemini 3.8 Flash"));
        assert_eq!(models[0].input_token_limit, Some(1048576));
        assert_eq!(models[0].is_default, Some(true));

        assert_eq!(models[1].id, "gemini-3.5-flash-lite");
        assert_eq!(models[1].is_default, Some(false));
    }

    #[test]
    fn test_parse_cli_models_json_object() {
        let json_input = r#"{
            "models": [
                {
                    "model": "gemini-3.1-pro-preview",
                    "name": "Gemini 3.1 Pro Preview"
                }
            ]
        }"#;

        let models = parse_cli_models_json(json_input);
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].id, "gemini-3.1-pro-preview");
    }

    #[test]
    fn test_parse_cli_models_text_fallback() {
        let text_input = r#"
- auto (Auto-selects best model)
- gemini-3.8-flash (Latest Preview)
- gemini-3.5-flash-lite
        "#;

        let models = parse_cli_models_json(text_input);
        assert_eq!(models.len(), 3);
        assert_eq!(models[0].id, "auto");
        assert_eq!(models[1].id, "gemini-3.8-flash");
        assert_eq!(models[2].id, "gemini-3.5-flash-lite");
    }

    #[test]
    fn test_parse_agy_models_output() {
        let text_input = "Fetching available models...\ngemini-3.8-flash-high\tGemini 3.8 Flash (High)\nclaude-sonnet-4-6\tClaude Sonnet 4.6 (Thinking)\ngpt-oss-120b-medium\tGPT-OSS 120B (Medium)\n";
        let models = parse_cli_models_json(text_input);
        assert_eq!(models.len(), 3);
        assert_eq!(models[0].id, "gemini-3.8-flash-high");
        assert_eq!(models[0].name.as_deref(), Some("Gemini 3.8 Flash (High)"));
        assert_eq!(models[1].id, "claude-sonnet-4-6");
        assert_eq!(models[2].id, "gpt-oss-120b-medium");
    }

    #[test]
    fn test_is_dir_in_path_list_case_and_slash_insensitivity() {
        let list = vec![
            std::path::PathBuf::from("C:\\Users\\test\\AppData\\Local\\agy\\bin"),
            std::path::PathBuf::from("C:\\Program Files\\nodejs"),
        ];
        assert!(is_dir_in_path_list(&std::path::PathBuf::from("c:/users/test/appdata/local/agy/bin/"), &list));
        assert!(is_dir_in_path_list(&std::path::PathBuf::from("C:\\Program Files\\nodejs"), &list));
        assert!(!is_dir_in_path_list(&std::path::PathBuf::from("C:\\Program Files\\git"), &list));
    }

    #[test]
    fn test_cli_path_status_structure() {
        let status = CliPathStatus {
            gemini_path: Some("C:\\Users\\test\\AppData\\Roaming\\npm\\gemini.cmd".to_string()),
            gemini_dir: Some("C:\\Users\\test\\AppData\\Roaming\\npm".to_string()),
            gemini_in_user_path: true,
            agy_path: Some("C:\\Users\\test\\AppData\\Local\\agy\\bin\\agy.exe".to_string()),
            agy_dir: Some("C:\\Users\\test\\AppData\\Local\\agy\\bin".to_string()),
            agy_in_user_path: false,
        };
        assert!(status.gemini_in_user_path);
        assert!(!status.agy_in_user_path);
        assert_eq!(status.agy_dir.as_deref(), Some("C:\\Users\\test\\AppData\\Local\\agy\\bin"));
    }

    #[test]
    fn test_path_add_result_serialization() {
        let res = PathAddResult {
            success: true,
            message: "Added to PATH".to_string(),
            target_dir: "C:\\Users\\test\\bin".to_string(),
            already_in_path: false,
        };
        let json = serde_json::to_string(&res).unwrap();
        assert!(json.contains("\"alreadyInPath\":false"));
        assert!(json.contains("\"targetDir\":\"C:\\\\Users\\\\test\\\\bin\""));
    }

    #[test]
    fn test_sync_workspace_skills() {
        let temp_dir = std::env::temp_dir().join(format!("gemini_skills_test_{}", uuid::Uuid::new_v4()));
        let gemini_skills = temp_dir.join(".gemini").join("skills").join("test-skill");
        std::fs::create_dir_all(&gemini_skills).unwrap();
        std::fs::write(gemini_skills.join("SKILL.md"), "---\nname: test-skill\n---\n# Test").unwrap();

        let res = sync_workspace_skills_internal(&temp_dir.to_string_lossy()).unwrap();
        assert!(res.success);
        assert_eq!(res.synced_count, 1);
        assert_eq!(res.skill_names, vec!["test-skill".to_string()]);

        let agy_skill_file = temp_dir.join(".agents").join("skills").join("test-skill").join("SKILL.md");
        assert!(agy_skill_file.exists());
        let content = std::fs::read_to_string(&agy_skill_file).unwrap();
        assert!(content.contains("name: test-skill"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_cli_model_discovery_result_serialization() {
        let res_agy = CliModelDiscoveryResult {
            supported: true,
            models: vec![CliModelInfo {
                id: "gemini-3.8-flash-high".to_string(),
                name: Some("Gemini 3.8 Flash (High)".to_string()),
                description: None,
                input_token_limit: None,
                output_token_limit: None,
                is_default: Some(false),
            }],
            message: "Discovered models successfully from Antigravity CLI (agy) (v1.5.8).".to_string(),
            engine: Some("agy".to_string()),
            cli_version: Some("1.5.8".to_string()),
        };

        let json = serde_json::to_string(&res_agy).expect("failed to serialize");
        assert!(json.contains("\"engine\":\"agy\""));
        assert!(json.contains("\"cli_version\":\"1.5.8\""));

        let deserialized: CliModelDiscoveryResult = serde_json::from_str(&json).expect("failed to deserialize");
        assert_eq!(deserialized.engine.as_deref(), Some("agy"));
        assert_eq!(deserialized.cli_version.as_deref(), Some("1.5.8"));
        assert_eq!(deserialized.models.len(), 1);
    }

    #[test]
    fn test_classify_agy_test_error_and_orchestrator_parsing() {
        assert!(classify_agy_test_error("RPC error: UNAUTHENTICATED").contains("authentication required"));
        assert!(classify_agy_test_error("HTTP 402 prepaid wallet exhausted").contains("prepaid wallet"));
        assert!(classify_agy_test_error("model foo not found").contains("Model unavailable"));

        let list_out = "orchestrator\tOrchestrator Agent\ngemini-4-hydrogen-high\tGemini 4 Hydrogen (High)\n";
        let models = parse_cli_models_json(list_out);
        assert_eq!(models.len(), 2);
        assert_eq!(models[0].id, "orchestrator");
        assert_eq!(models[1].id, "gemini-4-hydrogen-high");
    }

    #[test]
    fn test_parse_cli_models_nested_context_limits_gemini_071() {
        let json_input = r#"[
            {
                "modelId": "gemini-3.1-pro-preview-customtools",
                "id": "gemini-3.1-pro-preview-customtools",
                "displayName": "gemini-3.1-pro-preview",
                "contextLimits": {
                    "inputTokens": 1048576,
                    "outputTokens": 65536
                }
            }
        ]"#;
        let models = parse_cli_models_json(json_input);
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].id, "gemini-3.1-pro-preview-customtools");
        assert_eq!(models[0].input_token_limit, Some(1048576));
        assert_eq!(models[0].output_token_limit, Some(65536));
        assert_eq!(models[0].is_default, Some(false));
    }

    #[test]
    fn test_build_managed_user_env_plan_non_secret_gcp_vars() {
        let req = UserEnvSyncRequest {
            google_cloud_project: Some("  my-gcp-proj-42  ".to_string()),
            google_cloud_location: Some(" us-central1 ".to_string()),
            google_genai_use_vertexai: Some(true),
            agy_adc_auth: Some(true),
            clear_unspecified: false,
        };

        let plan = build_managed_user_env_plan(&req).expect("plan should build");
        assert_eq!(
            plan,
            vec![
                ("GOOGLE_CLOUD_PROJECT", Some("my-gcp-proj-42".to_string())),
                ("GOOGLE_CLOUD_LOCATION", Some("us-central1".to_string())),
                ("GOOGLE_GENAI_USE_VERTEXAI", Some("true".to_string())),
                ("AGY_ADC_AUTH", Some("true".to_string())),
            ]
        );

        let clear_req = UserEnvSyncRequest {
            google_cloud_project: None,
            google_cloud_location: Some("   ".to_string()),
            google_genai_use_vertexai: Some(false),
            agy_adc_auth: None,
            clear_unspecified: true,
        };
        let clear_plan = build_managed_user_env_plan(&clear_req).expect("clear plan should build");
        assert_eq!(
            clear_plan,
            vec![
                ("GOOGLE_CLOUD_PROJECT", None),
                ("GOOGLE_CLOUD_LOCATION", None),
                ("GOOGLE_GENAI_USE_VERTEXAI", None),
                ("AGY_ADC_AUTH", None),
            ]
        );

        let bad_req = UserEnvSyncRequest {
            google_cloud_project: Some("bad\nvalue".to_string()),
            google_cloud_location: None,
            google_genai_use_vertexai: None,
            agy_adc_auth: None,
            clear_unspecified: false,
        };
        assert!(build_managed_user_env_plan(&bad_req).is_err());
    }

    #[test]
    fn test_auth_config_db_clear_and_env_local_merge() {
        let db = crate::database::DbManager::new_in_memory().unwrap();
        let initial = GeminiAuthConfig {
            auth_mode: "vertex_ai".to_string(),
            api_key: Some("AIzaOldKey".to_string()),
            google_cloud_project: Some("corp-proj-1".to_string()),
            google_cloud_location: Some("europe-west1".to_string()),
            google_app_credentials: Some("C:\\keys\\sa.json".to_string()),
            agy_adc_auth: Some(true),
        };
        write_auth_config_to_db(&db, &initial).unwrap();
        let loaded = read_auth_config_from_db(&db).unwrap();
        assert_eq!(loaded.google_cloud_project.as_deref(), Some("corp-proj-1"));
        assert_eq!(loaded.agy_adc_auth, Some(true));

        // Clearing optional fields must overwrite stale DB values rather than leaving them behind
        let cleared = GeminiAuthConfig {
            auth_mode: "vertex_ai".to_string(),
            api_key: None,
            google_cloud_project: None,
            google_cloud_location: Some("us-central1".to_string()),
            google_app_credentials: None,
            agy_adc_auth: Some(false),
        };
        write_auth_config_to_db(&db, &cleared).unwrap();
        let after_clear = read_auth_config_from_db(&db).unwrap();
        assert_eq!(after_clear.api_key, None);
        assert_eq!(after_clear.google_cloud_project, None);
        assert_eq!(after_clear.google_app_credentials, None);
        assert_eq!(after_clear.agy_adc_auth, Some(false));

        // .env.local merge preserves unrelated variables and refreshes Vertex + AGY_ADC_AUTH keys
        let existing_env = "CUSTOM_TOKEN=keep-me\nGEMINI_API_KEY=stale\nGOOGLE_CLOUD_PROJECT=old-proj\nAGY_ADC_AUTH=true\n";
        let merged = build_env_local_content(Some(existing_env), &initial);
        assert!(merged.contains("CUSTOM_TOKEN=keep-me"));
        assert!(!merged.contains("GEMINI_API_KEY=stale"));
        assert!(merged.contains("GOOGLE_GENAI_USE_VERTEXAI=true"));
        assert!(merged.contains("GOOGLE_CLOUD_PROJECT=corp-proj-1"));
        assert!(merged.contains("GOOGLE_CLOUD_LOCATION=europe-west1"));
        assert!(merged.contains("AGY_ADC_AUTH=true"));
    }
}
