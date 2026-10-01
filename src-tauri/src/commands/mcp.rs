use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct McpConfigResponse {
    pub file_path: String,
    pub mcp_servers: serde_json::Value,
}

pub fn resolve_settings_file(workspace_path: Option<&str>) -> Result<PathBuf, String> {
    if let Some(ws) = workspace_path {
        let trimmed = ws.trim();
        if !trimmed.is_empty() {
            return Ok(PathBuf::from(trimmed).join(".gemini").join("settings.json"));
        }
    }

    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map_err(|_| "Could not determine user home directory".to_string())?;
    Ok(PathBuf::from(home).join(".gemini").join("settings.json"))
}

#[tauri::command]
pub async fn get_mcp_config(workspace_path: Option<String>) -> Result<McpConfigResponse, String> {
    let file_path = resolve_settings_file(workspace_path.as_deref())?;
    let path_str = file_path.to_string_lossy().to_string();

    if !file_path.exists() {
        return Ok(McpConfigResponse {
            file_path: path_str,
            mcp_servers: serde_json::json!({}),
        });
    }

    let content = std::fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read settings file {}: {}", path_str, e))?;

    let parsed: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse settings JSON in {}: {}", path_str, e))?;

    let mcp_servers = if let Some(servers) = parsed.get("mcpServers") {
        if servers.is_object() {
            servers.clone()
        } else {
            serde_json::json!({})
        }
    } else {
        serde_json::json!({})
    };

    Ok(McpConfigResponse {
        file_path: path_str,
        mcp_servers,
    })
}

#[tauri::command]
pub async fn save_mcp_config(workspace_path: Option<String>, mcp_servers: serde_json::Value) -> Result<McpConfigResponse, String> {
    let file_path = resolve_settings_file(workspace_path.as_deref())?;
    let path_str = file_path.to_string_lossy().to_string();

    if let Some(parent) = file_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create settings directory {:?}: {}", parent, e))?;
    }

    let mut root_json: serde_json::Value = if file_path.exists() {
        let content = std::fs::read_to_string(&file_path)
            .unwrap_or_else(|_| "{}".to_string());
        serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    if !root_json.is_object() {
        root_json = serde_json::json!({});
    }

    root_json.as_object_mut().unwrap().insert("mcpServers".to_string(), mcp_servers.clone());

    let pretty_str = serde_json::to_string_pretty(&root_json)
        .map_err(|e| format!("Failed to serialize settings JSON: {}", e))?;

    std::fs::write(&file_path, pretty_str)
        .map_err(|e| format!("Failed to write settings file {}: {}", path_str, e))?;

    Ok(McpConfigResponse {
        file_path: path_str,
        mcp_servers,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mcp_config_save_and_preserve() {
        let temp_dir = std::env::temp_dir().join(format!("gemini_test_ws_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        // 1. Write an initial settings.json with a custom key
        let settings_file = temp_dir.join(".gemini").join("settings.json");
        std::fs::create_dir_all(settings_file.parent().unwrap()).unwrap();
        std::fs::write(&settings_file, r#"{"theme": "cyber-emerald", "telemetry": false}"#).unwrap();

        // 2. Save an MCP config via save_mcp_config
        let ws_str = temp_dir.to_string_lossy().to_string();
        let servers = serde_json::json!({
            "memory": {
                "command": "npx",
                "args": ["-y", "@modelcontextprotocol/server-memory"]
            }
        });

        let res = save_mcp_config(Some(ws_str.clone()), servers).await.expect("save_mcp_config failed");
        assert_eq!(res.mcp_servers["memory"]["command"], "npx");

        // 3. Read back via get_mcp_config
        let fetched = get_mcp_config(Some(ws_str)).await.expect("get_mcp_config failed");
        assert_eq!(fetched.mcp_servers["memory"]["command"], "npx");

        // 4. Verify existing keys were preserved
        let raw = std::fs::read_to_string(&settings_file).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(parsed["theme"], "cyber-emerald");
        assert_eq!(parsed["telemetry"], false);

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
