use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::State;
use crate::AppState;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct McpConfigResponse {
    pub file_path: String,
    pub mcp_servers: serde_json::Value,
}

pub fn resolve_mcp_config_file(workspace_path: Option<&str>, engine: Option<&str>) -> Result<PathBuf, String> {
    let is_agy = engine.map(|e| e == "agy").unwrap_or(false);

    if let Some(ws) = workspace_path {
        let trimmed = ws.trim();
        if !trimmed.is_empty() {
            if is_agy {
                return Ok(PathBuf::from(trimmed).join(".agents").join("mcp_config.json"));
            } else {
                return Ok(PathBuf::from(trimmed).join(".gemini").join("settings.json"));
            }
        }
    }

    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map_err(|_| "Could not determine user home directory".to_string())?;

    if is_agy {
        Ok(PathBuf::from(home).join(".gemini").join("config").join("mcp_config.json"))
    } else {
        Ok(PathBuf::from(home).join(".gemini").join("settings.json"))
    }
}

#[allow(dead_code)]
pub fn resolve_settings_file(workspace_path: Option<&str>) -> Result<PathBuf, String> {
    resolve_mcp_config_file(workspace_path, Some("gemini"))
}

pub fn get_mcp_config_internal(workspace_path: Option<&str>, engine: Option<&str>) -> Result<McpConfigResponse, String> {
    let file_path = resolve_mcp_config_file(workspace_path, engine)?;
    let path_str = file_path.to_string_lossy().to_string();

    if !file_path.exists() {
        return Ok(McpConfigResponse {
            file_path: path_str,
            mcp_servers: serde_json::json!({}),
        });
    }

    let content = std::fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read MCP configuration file {}: {}", path_str, e))?;

    let parsed: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse MCP configuration JSON in {}: {}", path_str, e))?;

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

pub fn save_mcp_config_internal(
    workspace_path: Option<&str>,
    engine: Option<&str>,
    mcp_servers: serde_json::Value,
) -> Result<McpConfigResponse, String> {
    let file_path = resolve_mcp_config_file(workspace_path, engine)?;
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

#[tauri::command]
pub async fn get_mcp_config(
    state: State<'_, AppState>,
    workspace_path: Option<String>,
    engine: Option<String>,
) -> Result<McpConfigResponse, String> {
    let effective_engine = engine.or_else(|| {
        state.db.get_setting("active_cli_engine").ok().flatten()
    }).unwrap_or_else(|| "gemini".to_string());

    get_mcp_config_internal(workspace_path.as_deref(), Some(&effective_engine))
}

#[tauri::command]
pub async fn save_mcp_config(
    state: State<'_, AppState>,
    workspace_path: Option<String>,
    engine: Option<String>,
    mcp_servers: serde_json::Value,
) -> Result<McpConfigResponse, String> {
    let effective_engine = engine.or_else(|| {
        state.db.get_setting("active_cli_engine").ok().flatten()
    }).unwrap_or_else(|| "gemini".to_string());

    save_mcp_config_internal(workspace_path.as_deref(), Some(&effective_engine), mcp_servers)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct McpImportResult {
    pub success: bool,
    pub imported_count: usize,
    pub server_names: Vec<String>,
    pub message: String,
}

pub fn detect_importable_gemini_mcp_internal(workspace_path: Option<&str>) -> Result<Vec<String>, String> {
    let gemini_cfg = get_mcp_config_internal(workspace_path, Some("gemini"))?;
    let gemini_servers = match gemini_cfg.mcp_servers.as_object() {
        Some(obj) if !obj.is_empty() => obj,
        _ => return Ok(Vec::new()),
    };

    let agy_cfg = get_mcp_config_internal(workspace_path, Some("agy"))?;
    let agy_servers = agy_cfg.mcp_servers.as_object();

    let mut importable = Vec::new();
    for key in gemini_servers.keys() {
        if agy_servers.map(|s| !s.contains_key(key)).unwrap_or(true) {
            importable.push(key.clone());
        }
    }
    Ok(importable)
}

pub fn import_mcp_from_gemini_internal(workspace_path: Option<&str>) -> Result<McpImportResult, String> {
    let gemini_cfg = get_mcp_config_internal(workspace_path, Some("gemini"))?;
    let gemini_servers = match gemini_cfg.mcp_servers.as_object() {
        Some(obj) if !obj.is_empty() => obj.clone(),
        _ => {
            return Ok(McpImportResult {
                success: true,
                imported_count: 0,
                server_names: Vec::new(),
                message: "No Gemini CLI MCP servers found to import.".to_string(),
            });
        }
    };

    let agy_cfg = get_mcp_config_internal(workspace_path, Some("agy"))?;
    let mut agy_servers = agy_cfg.mcp_servers.as_object().cloned().unwrap_or_default();

    let mut imported = Vec::new();
    for (k, v) in gemini_servers {
        if !agy_servers.contains_key(&k) {
            agy_servers.insert(k.clone(), v);
            imported.push(k);
        }
    }

    if imported.is_empty() {
        return Ok(McpImportResult {
            success: true,
            imported_count: 0,
            server_names: Vec::new(),
            message: "All Gemini CLI MCP servers are already present in Antigravity configuration.".to_string(),
        });
    }

    let count = imported.len();
    save_mcp_config_internal(
        workspace_path,
        Some("agy"),
        serde_json::Value::Object(agy_servers),
    )?;

    Ok(McpImportResult {
        success: true,
        imported_count: count,
        server_names: imported.clone(),
        message: format!("Successfully imported {} server(s) from Gemini CLI: {}", count, imported.join(", ")),
    })
}

#[tauri::command]
pub async fn detect_importable_gemini_mcp(
    workspace_path: Option<String>,
) -> Result<Vec<String>, String> {
    detect_importable_gemini_mcp_internal(workspace_path.as_deref())
}

#[tauri::command]
pub async fn import_mcp_from_gemini(
    workspace_path: Option<String>,
) -> Result<McpImportResult, String> {
    import_mcp_from_gemini_internal(workspace_path.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_mcp_config_file_routing() {
        let home = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")).unwrap();
        let home_path = PathBuf::from(home);

        // Global Gemini
        let gemini_global = resolve_mcp_config_file(None, Some("gemini")).unwrap();
        assert_eq!(gemini_global, home_path.join(".gemini").join("settings.json"));

        // Global Agy
        let agy_global = resolve_mcp_config_file(None, Some("agy")).unwrap();
        assert_eq!(agy_global, home_path.join(".gemini").join("config").join("mcp_config.json"));

        // Workspace Gemini
        let gemini_ws = resolve_mcp_config_file(Some("C:\\test\\workspace"), Some("gemini")).unwrap();
        assert_eq!(gemini_ws, PathBuf::from("C:\\test\\workspace").join(".gemini").join("settings.json"));

        // Workspace Agy
        let agy_ws = resolve_mcp_config_file(Some("C:\\test\\workspace"), Some("agy")).unwrap();
        assert_eq!(agy_ws, PathBuf::from("C:\\test\\workspace").join(".agents").join("mcp_config.json"));
    }

    #[tokio::test]
    async fn test_mcp_config_save_and_preserve() {
        let temp_dir = std::env::temp_dir().join(format!("gemini_test_ws_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        // 1. Write an initial settings.json with a custom key
        let settings_file = temp_dir.join(".gemini").join("settings.json");
        std::fs::create_dir_all(settings_file.parent().unwrap()).unwrap();
        std::fs::write(&settings_file, r#"{"theme": "cyber-emerald", "telemetry": false}"#).unwrap();

        // 2. Save an MCP config via save_mcp_config_internal (Gemini)
        let ws_str = temp_dir.to_string_lossy().to_string();
        let servers = serde_json::json!({
            "memory": {
                "command": "npx",
                "args": ["-y", "@modelcontextprotocol/server-memory"]
            }
        });

        let res = save_mcp_config_internal(Some(&ws_str), Some("gemini"), servers).expect("save_mcp_config failed");
        assert_eq!(res.mcp_servers["memory"]["command"], "npx");

        // 3. Read back via get_mcp_config_internal
        let fetched = get_mcp_config_internal(Some(&ws_str), Some("gemini")).expect("get_mcp_config failed");
        assert_eq!(fetched.mcp_servers["memory"]["command"], "npx");

        // 4. Verify existing keys were preserved
        let raw = std::fs::read_to_string(&settings_file).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(parsed["theme"], "cyber-emerald");
        assert_eq!(parsed["telemetry"], false);

        // 5. Test Antigravity config saving (.agents/mcp_config.json)
        let agy_servers = serde_json::json!({
            "github": {
                "command": "docker",
                "args": ["run", "-i", "ghcr.io/github/github-mcp-server:latest"]
            }
        });
        let agy_res = save_mcp_config_internal(Some(&ws_str), Some("agy"), agy_servers).expect("save agy failed");
        assert_eq!(agy_res.file_path, temp_dir.join(".agents").join("mcp_config.json").to_string_lossy().to_string());
        assert_eq!(agy_res.mcp_servers["github"]["command"], "docker");

        // Read back Agy
        let agy_fetched = get_mcp_config_internal(Some(&ws_str), Some("agy")).expect("get agy failed");
        assert_eq!(agy_fetched.mcp_servers["github"]["command"], "docker");

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[tokio::test]
    async fn test_import_mcp_from_gemini() {
        let temp_dir = std::env::temp_dir().join(format!("gemini_import_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let ws_str = temp_dir.to_string_lossy().to_string();

        // 1. Populate Gemini CLI config with 2 servers
        let gemini_servers = serde_json::json!({
            "memory": { "command": "npx", "args": ["-y", "@modelcontextprotocol/server-memory"] },
            "sqlite": { "command": "uvx", "args": ["mcp-server-sqlite"] }
        });
        save_mcp_config_internal(Some(&ws_str), Some("gemini"), gemini_servers).unwrap();

        // 2. Pre-seed Antigravity with 1 server (memory)
        let agy_servers = serde_json::json!({
            "memory": { "command": "npx", "args": ["-y", "@modelcontextprotocol/server-memory"] }
        });
        save_mcp_config_internal(Some(&ws_str), Some("agy"), agy_servers).unwrap();

        // 3. Detect importable
        let importable = detect_importable_gemini_mcp_internal(Some(&ws_str)).unwrap();
        assert_eq!(importable, vec!["sqlite".to_string()]);

        // 4. Run import
        let res = import_mcp_from_gemini_internal(Some(&ws_str)).unwrap();
        assert!(res.success);
        assert_eq!(res.imported_count, 1);
        assert_eq!(res.server_names, vec!["sqlite".to_string()]);

        // 5. Verify Antigravity now has both
        let fetched = get_mcp_config_internal(Some(&ws_str), Some("agy")).unwrap();
        assert!(fetched.mcp_servers.get("memory").is_some());
        assert!(fetched.mcp_servers.get("sqlite").is_some());

        // 6. Second import should do nothing
        let res2 = import_mcp_from_gemini_internal(Some(&ws_str)).unwrap();
        assert_eq!(res2.imported_count, 0);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
