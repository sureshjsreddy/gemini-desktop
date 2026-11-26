use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::State;
use crate::AppState;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ChatCompressionConfig {
    pub enabled: bool,
    pub context_percentage_threshold: f64,
}

impl Default for ChatCompressionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            context_percentage_threshold: 0.75,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FileFilteringConfig {
    pub respect_git_ignore: bool,
    pub respect_gemini_ignore: bool,
}

impl Default for FileFilteringConfig {
    fn default() -> Self {
        Self {
            respect_git_ignore: true,
            respect_gemini_ignore: true,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SessionRetentionConfig {
    pub enabled: bool,
    pub max_age: String,
    pub max_count: Option<u64>,
}

impl Default for SessionRetentionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_age: "30d".to_string(),
            max_count: Some(50),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GlobalCliSettings {
    pub file_path: String,
    pub chat_compression: ChatCompressionConfig,
    pub max_session_turns: i64,
    pub file_filtering: FileFilteringConfig,
    pub session_retention: SessionRetentionConfig,
    pub raw_json: String,
    #[serde(default)]
    pub agy_agent_mode: Option<String>,
    #[serde(default)]
    pub show_thinking: Option<bool>,
    #[serde(default)]
    pub orchestrator: Option<serde_json::Value>,
    #[serde(default)]
    pub agy_auto_compact_override: Option<String>,
    #[serde(default)]
    pub agy_disable_embedded_ripgrep: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SaveGlobalCliSettingsPayload {
    pub chat_compression: Option<ChatCompressionConfig>,
    pub max_session_turns: Option<i64>,
    pub file_filtering: Option<FileFilteringConfig>,
    pub session_retention: Option<SessionRetentionConfig>,
    pub raw_json: Option<String>,
    #[serde(default)]
    pub agy_agent_mode: Option<String>,
    #[serde(default)]
    pub show_thinking: Option<bool>,
    #[serde(default)]
    pub orchestrator: Option<serde_json::Value>,
    #[serde(default)]
    pub agy_auto_compact_override: Option<String>,
    #[serde(default)]
    pub agy_disable_embedded_ripgrep: Option<bool>,
}

pub fn get_global_settings_path(engine: Option<&str>) -> Result<PathBuf, String> {
    crate::engines::get_adapter(engine.unwrap_or("gemini")).resolve_settings_path()
}

#[tauri::command]
pub async fn get_global_cli_settings(
    state: State<'_, AppState>,
) -> Result<GlobalCliSettings, String> {
    let engine = state.db.get_setting("active_cli_engine").ok().flatten().unwrap_or_else(|| "gemini".to_string());
    let file_path = get_global_settings_path(Some(&engine))?;
    let path_str = file_path.to_string_lossy().to_string();

    let agy_auto_compact_override = state
        .db
        .get_setting("agy_auto_compact_override")
        .ok()
        .flatten()
        .filter(|s| !s.trim().is_empty());
    let agy_disable_embedded_ripgrep = state
        .db
        .get_setting("agy_disable_embedded_ripgrep")
        .ok()
        .flatten()
        .map(|v| v.trim() == "1" || v.trim().eq_ignore_ascii_case("true"));

    if !file_path.exists() {
        let default_raw = crate::engines::get_adapter(&engine)
            .default_global_settings_json()
            .to_string();
        return Ok(GlobalCliSettings {
            file_path: path_str,
            chat_compression: ChatCompressionConfig::default(),
            max_session_turns: -1,
            file_filtering: FileFilteringConfig::default(),
            session_retention: SessionRetentionConfig::default(),
            raw_json: default_raw,
            agy_agent_mode: if engine == "agy" { Some("auto".to_string()) } else { None },
            show_thinking: if engine == "agy" { Some(true) } else { None },
            orchestrator: None,
            agy_auto_compact_override,
            agy_disable_embedded_ripgrep,
        });
    }

    let content = std::fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read settings file {}: {}", path_str, e))?;

    let parsed: serde_json::Value = serde_json::from_str(crate::commands::strip_utf8_bom(&content))
        .unwrap_or_else(|_| serde_json::json!({}));

    // Extract chatCompression
    let compression_val = parsed.pointer("/model/chatCompression");
    let chat_compression = ChatCompressionConfig {
        enabled: compression_val
            .and_then(|v| v.get("enabled"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        context_percentage_threshold: compression_val
            .and_then(|v| v.get("contextPercentageThreshold"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.75),
    };

    // Extract maxSessionTurns
    let max_session_turns = parsed.pointer("/model/maxSessionTurns")
        .and_then(|v| v.as_i64())
        .unwrap_or(-1);

    // Extract fileFiltering
    let filtering_val = parsed.pointer("/context/fileFiltering");
    let file_filtering = FileFilteringConfig {
        respect_git_ignore: filtering_val
            .and_then(|v| v.get("respectGitIgnore"))
            .and_then(|v| v.as_bool())
            .unwrap_or(true),
        respect_gemini_ignore: filtering_val
            .and_then(|v| v.get("respectGeminiIgnore"))
            .and_then(|v| v.as_bool())
            .unwrap_or(true),
    };

    // Extract sessionRetention
    let retention_val = parsed.pointer("/general/sessionRetention");
    let session_retention = SessionRetentionConfig {
        enabled: retention_val
            .and_then(|v| v.get("enabled"))
            .and_then(|v| v.as_bool())
            .unwrap_or(true),
        max_age: retention_val
            .and_then(|v| v.get("maxAge"))
            .and_then(|v| v.as_str())
            .unwrap_or("30d")
            .to_string(),
        max_count: retention_val
            .and_then(|v| v.get("maxCount"))
            .and_then(|v| v.as_u64()),
    };

    let agy_agent_mode = parsed
        .get("agentMode")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let show_thinking = parsed.get("showThinking").and_then(|v| v.as_bool());
    let orchestrator = parsed.get("orchestrator").cloned();

    Ok(GlobalCliSettings {
        file_path: path_str,
        chat_compression,
        max_session_turns,
        file_filtering,
        session_retention,
        raw_json: content,
        agy_agent_mode,
        show_thinking,
        orchestrator,
        agy_auto_compact_override,
        agy_disable_embedded_ripgrep,
    })
}

#[tauri::command]
pub async fn save_global_cli_settings(
    state: State<'_, AppState>,
    payload: SaveGlobalCliSettingsPayload,
) -> Result<GlobalCliSettings, String> {
    let engine = state.db.get_setting("active_cli_engine").ok().flatten().unwrap_or_else(|| "gemini".to_string());
    let file_path = get_global_settings_path(Some(&engine))?;
    let path_str = file_path.to_string_lossy().to_string();

    if let Some(parent) = file_path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create settings parent directory: {}", e))?;
        }
    }

    let mut target_json: serde_json::Value = if let Some(raw) = payload.raw_json {
        let trimmed = crate::commands::strip_utf8_bom(&raw).trim();
        if !trimmed.is_empty() {
            serde_json::from_str(trimmed)
                .map_err(|e| format!("Invalid JSON syntax in raw settings: {}", e))?
        } else {
            read_or_create_base_json(&file_path)?
        }
    } else {
        read_or_create_base_json(&file_path)?
    };

    if !target_json.is_object() {
        target_json = serde_json::json!({});
    }

    // Apply structured updates if raw_json wasn't explicitly overriding
    if let Some(comp) = payload.chat_compression {
        ensure_object(&mut target_json, "model");
        if let Some(model_obj) = target_json.get_mut("model").and_then(|v| v.as_object_mut()) {
            model_obj.insert("chatCompression".to_string(), serde_json::json!({
                "enabled": comp.enabled,
                "contextPercentageThreshold": comp.context_percentage_threshold
            }));
        }
    }

    if let Some(turns) = payload.max_session_turns {
        ensure_object(&mut target_json, "model");
        if let Some(model_obj) = target_json.get_mut("model").and_then(|v| v.as_object_mut()) {
            model_obj.insert("maxSessionTurns".to_string(), serde_json::json!(turns));
        }
    }

    if let Some(filt) = payload.file_filtering {
        ensure_object(&mut target_json, "context");
        if let Some(ctx_obj) = target_json.get_mut("context").and_then(|v| v.as_object_mut()) {
            ctx_obj.insert("fileFiltering".to_string(), serde_json::json!({
                "respectGitIgnore": filt.respect_git_ignore,
                "respectGeminiIgnore": filt.respect_gemini_ignore
            }));
        }
    }

    if let Some(ret) = payload.session_retention {
        ensure_object(&mut target_json, "general");
        if let Some(gen_obj) = target_json.get_mut("general").and_then(|v| v.as_object_mut()) {
            let mut ret_val = serde_json::json!({
                "enabled": ret.enabled,
                "maxAge": ret.max_age
            });
            if let Some(count) = ret.max_count {
                if let Some(m) = ret_val.as_object_mut() {
                    m.insert("maxCount".to_string(), serde_json::json!(count));
                }
            }
            gen_obj.insert("sessionRetention".to_string(), ret_val);
        }
    }

    if engine == "agy" {
        if let Some(mode) = payload.agy_agent_mode {
            let trimmed = mode.trim();
            if !trimmed.is_empty() {
                if let Some(obj) = target_json.as_object_mut() {
                    obj.insert("agentMode".to_string(), serde_json::Value::String(trimmed.to_string()));
                }
            }
        }
        if let Some(show) = payload.show_thinking {
            if let Some(obj) = target_json.as_object_mut() {
                obj.insert("showThinking".to_string(), serde_json::Value::Bool(show));
            }
        }
        if let Some(orch) = payload.orchestrator {
            if let Some(obj) = target_json.as_object_mut() {
                if orch.is_null() {
                    obj.remove("orchestrator");
                } else {
                    obj.insert("orchestrator".to_string(), orch);
                }
            }
        }
    }

    if let Some(compact_override) = payload.agy_auto_compact_override {
        let _ = state.db.set_setting("agy_auto_compact_override", compact_override.trim());
    }
    if let Some(disable_rg) = payload.agy_disable_embedded_ripgrep {
        let _ = state.db.set_setting(
            "agy_disable_embedded_ripgrep",
            if disable_rg { "true" } else { "false" },
        );
    }

    let formatted = serde_json::to_string_pretty(&target_json)
        .map_err(|e| format!("Failed to format settings JSON: {}", e))?;

    std::fs::write(&file_path, &formatted)
        .map_err(|e| format!("Failed to write settings file {}: {}", path_str, e))?;

    get_global_cli_settings(state).await
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CliEnginesStatus {
    pub active_engine: String,
    pub gemini_installed: bool,
    pub gemini_path: Option<String>,
    pub agy_installed: bool,
    pub agy_path: Option<String>,
}

#[tauri::command]
pub async fn get_active_cli_engine(state: State<'_, AppState>) -> Result<String, String> {
    let engine = state.db.get_setting("active_cli_engine")
        .map_err(|e| e.to_string())?
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "gemini".to_string());
    Ok(engine)
}

#[tauri::command]
pub async fn set_active_cli_engine(
    state: State<'_, AppState>,
    engine: String,
) -> Result<String, String> {
    let normalized = crate::engines::normalize_engine_id(&engine);

    state.db.set_setting("active_cli_engine", normalized)
        .map_err(|e| e.to_string())?;

    // Terminate any currently running CLI process and reset ACP state so subsequent turns launch the newly active engine
    let mut child_guard = state.active_child.lock().await;
    if let Some(old_child) = child_guard.take() {
        crate::process_manager::ProcessSupervisor::terminate_child(old_child);
    }
    let mut ws_guard = state.active_process_workspace.lock().await;
    *ws_guard = None;
    state.acp_session.clear_sessions();
    crate::process_manager::invalidate_cli_path_cache();

    Ok(normalized.to_string())
}

#[tauri::command]
pub async fn get_engines_catalog() -> Result<Vec<crate::engines::EngineDescriptor>, String> {
    tokio::task::spawn_blocking(|| crate::engines::build_engines_catalog(true, false))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_cli_engines_status(state: State<'_, AppState>) -> Result<CliEnginesStatus, String> {
    let active = state.db.get_setting("active_cli_engine")
        .map_err(|e| e.to_string())?
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "gemini".to_string());

    let gemini_exe = crate::process_manager::find_gemini_executable();
    let agy_exe = crate::process_manager::find_agy_executable();

    Ok(CliEnginesStatus {
        active_engine: active,
        gemini_installed: gemini_exe.is_some(),
        gemini_path: gemini_exe.map(|p| p.to_string_lossy().to_string()),
        agy_installed: agy_exe.is_some(),
        agy_path: agy_exe.map(|p| p.to_string_lossy().to_string()),
    })
}

fn read_or_create_base_json(path: &std::path::Path) -> Result<serde_json::Value, String> {
    if path.exists() {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read existing settings: {}", e))?;
        serde_json::from_str(crate::commands::strip_utf8_bom(&content))
            .map_err(|e| format!("Failed to parse existing settings JSON: {}", e))
    } else {
        Ok(serde_json::json!({}))
    }
}

fn ensure_object(root: &mut serde_json::Value, key: &str) {
    if root.get(key).is_none() || !root[key].is_object() {
        if let Some(obj) = root.as_object_mut() {
            obj.insert(key.to_string(), serde_json::json!({}));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_compression_defaults() {
        let def = ChatCompressionConfig::default();
        assert!(!def.enabled);
        assert!((def.context_percentage_threshold - 0.75).abs() < f64::EPSILON);
    }

    #[test]
    fn test_file_filtering_defaults() {
        let def = FileFilteringConfig::default();
        assert!(def.respect_git_ignore);
        assert!(def.respect_gemini_ignore);
    }

    #[test]
    fn test_ensure_object_preserves_existing() {
        let mut val = serde_json::json!({
            "model": {
                "existingKey": "preserved"
            }
        });
        ensure_object(&mut val, "model");
        assert_eq!(val["model"]["existingKey"], "preserved");
    }

    #[test]
    fn test_active_cli_engine_persistence() {
        let db = crate::database::DbManager::new_in_memory().unwrap();
        // Initially empty
        assert_eq!(db.get_setting("active_cli_engine").unwrap(), None);

        // When unpersisted, default fallback is strictly "gemini"
        let fallback_engine = db.get_setting("active_cli_engine")
            .unwrap()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "gemini".to_string());
        assert_eq!(fallback_engine, "gemini");

        // Set to agy
        db.set_setting("active_cli_engine", "agy").unwrap();
        assert_eq!(db.get_setting("active_cli_engine").unwrap().as_deref(), Some("agy"));

        // Switch to gemini
        db.set_setting("active_cli_engine", "gemini").unwrap();
        assert_eq!(db.get_setting("active_cli_engine").unwrap().as_deref(), Some("gemini"));
    }

    #[test]
    fn test_get_global_settings_path_routing() {
        let p_gemini = get_global_settings_path(Some("gemini")).unwrap();
        let s_gemini = p_gemini.to_string_lossy().to_lowercase().replace('/', "\\");
        assert!(s_gemini.ends_with(".gemini\\settings.json"));

        let p_agy = get_global_settings_path(Some("agy")).unwrap();
        let s_agy = p_agy.to_string_lossy().to_lowercase().replace('/', "\\");
        assert!(s_agy.ends_with("antigravity-cli\\settings.json"));
    }

    #[test]
    fn test_global_settings_bom_and_agy_keys_parsing() {
        let bom_raw = "\u{FEFF}{\"agentMode\":\"auto\",\"showThinking\":true,\"orchestrator\":{\"plannerModel\":\"orchestrator\",\"effort\":\"high\"}}";
        let stripped = crate::commands::strip_utf8_bom(bom_raw);
        let parsed: serde_json::Value = serde_json::from_str(stripped).unwrap();
        assert_eq!(parsed["agentMode"], "auto");
        assert_eq!(parsed["showThinking"], true);
        assert_eq!(parsed["orchestrator"]["effort"], "high");
    }
}
