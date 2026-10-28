use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SaveGlobalCliSettingsPayload {
    pub chat_compression: Option<ChatCompressionConfig>,
    pub max_session_turns: Option<i64>,
    pub file_filtering: Option<FileFilteringConfig>,
    pub session_retention: Option<SessionRetentionConfig>,
    pub raw_json: Option<String>,
}

pub fn get_global_settings_path() -> Result<PathBuf, String> {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map_err(|_| "Could not determine user home directory".to_string())?;
    Ok(PathBuf::from(home).join(".gemini").join("settings.json"))
}

#[tauri::command]
pub async fn get_global_cli_settings() -> Result<GlobalCliSettings, String> {
    let file_path = get_global_settings_path()?;
    let path_str = file_path.to_string_lossy().to_string();

    if !file_path.exists() {
        return Ok(GlobalCliSettings {
            file_path: path_str,
            chat_compression: ChatCompressionConfig::default(),
            max_session_turns: -1,
            file_filtering: FileFilteringConfig::default(),
            session_retention: SessionRetentionConfig::default(),
            raw_json: "{\n  \"model\": {\n    \"maxSessionTurns\": -1\n  }\n}".to_string(),
        });
    }

    let content = std::fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read settings file {}: {}", path_str, e))?;

    let parsed: serde_json::Value = serde_json::from_str(&content)
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

    Ok(GlobalCliSettings {
        file_path: path_str,
        chat_compression,
        max_session_turns,
        file_filtering,
        session_retention,
        raw_json: content,
    })
}

#[tauri::command]
pub async fn save_global_cli_settings(payload: SaveGlobalCliSettingsPayload) -> Result<GlobalCliSettings, String> {
    let file_path = get_global_settings_path()?;
    let path_str = file_path.to_string_lossy().to_string();

    if let Some(parent) = file_path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create .gemini directory: {}", e))?;
        }
    }

    let mut target_json: serde_json::Value = if let Some(raw) = payload.raw_json {
        let trimmed = raw.trim();
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

    let formatted = serde_json::to_string_pretty(&target_json)
        .map_err(|e| format!("Failed to format settings JSON: {}", e))?;

    std::fs::write(&file_path, &formatted)
        .map_err(|e| format!("Failed to write settings file {}: {}", path_str, e))?;

    get_global_cli_settings().await
}

fn read_or_create_base_json(path: &std::path::Path) -> Result<serde_json::Value, String> {
    if path.exists() {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read existing settings: {}", e))?;
        serde_json::from_str(&content)
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
}
