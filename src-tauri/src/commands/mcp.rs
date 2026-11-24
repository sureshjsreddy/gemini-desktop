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

/// Gathers all active, enabled MCP servers for an ACP session.
/// Resolves in order of precedence:
/// 1. Plugins' mcp_config.json (when engine == "agy", relative paths resolved against plugin directory)
/// 2. Global MCP configuration (settings.json for Gemini, mcp_config.json for Antigravity)
/// 3. Workspace MCP configuration (overrides global/plugin settings; disabled: true explicitly suppresses the server)
pub fn get_active_mcp_servers_for_session(
    workspace_path: Option<&str>,
    engine: &str,
) -> Vec<serde_json::Value> {
    use std::collections::HashMap;

    let is_agy = engine.trim().eq_ignore_ascii_case("agy");
    let mut server_map: HashMap<String, serde_json::Value> = HashMap::new();

    // 1. If engine is Antigravity (agy), scan installed and enabled plugins
    if is_agy {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .ok();

        if let Some(home_dir) = home {
            let config_dir = PathBuf::from(home_dir).join(".gemini").join("config");
            let plugins_dir = config_dir.join("plugins");

            // Check config.json for enabled status of plugins
            let mut enabled_plugins_map: HashMap<String, bool> = HashMap::new();
            let config_file = config_dir.join("config.json");
            if config_file.is_file() {
                if let Ok(content) = std::fs::read_to_string(&config_file) {
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(plugins_obj) = parsed.get("plugins").and_then(|p| p.as_object()) {
                            for (p_name, p_val) in plugins_obj {
                                let is_enabled = p_val.get("enabled")
                                    .and_then(|e| e.as_bool())
                                    .unwrap_or(true); // Enabled by default unless explicitly false
                                enabled_plugins_map.insert(p_name.clone(), is_enabled);
                            }
                        }
                    }
                }
            }

            if plugins_dir.is_dir() {
                if let Ok(entries) = std::fs::read_dir(&plugins_dir) {
                    for entry in entries.flatten() {
                        let plugin_dir = entry.path();
                        if !plugin_dir.is_dir() {
                            continue;
                        }

                        let plugin_name = plugin_dir
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("")
                            .to_string();

                        if plugin_name.is_empty() {
                            continue;
                        }

                        // Check if plugin is explicitly disabled in config.json
                        if let Some(&is_enabled) = enabled_plugins_map.get(&plugin_name) {
                            if !is_enabled {
                                continue;
                            }
                        }

                        let plugin_mcp_file = plugin_dir.join("mcp_config.json");
                        if !plugin_mcp_file.is_file() {
                            continue;
                        }

                        if let Ok(content) = std::fs::read_to_string(&plugin_mcp_file) {
                            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&content) {
                                if let Some(servers) = parsed.get("mcpServers").and_then(|s| s.as_object()) {
                                    for (raw_s_name, s_cfg) in servers {
                                        if let Some(cfg_obj) = s_cfg.as_object() {
                                            if cfg_obj.get("disabled") == Some(&serde_json::Value::Bool(true)) {
                                                continue;
                                            }

                                            let namespaced_name = if raw_s_name.starts_with(&format!("{}_", plugin_name)) {
                                                raw_s_name.clone()
                                            } else {
                                                format!("{}_{}", plugin_name, raw_s_name)
                                            };

                                            let mut entry_map = serde_json::Map::new();
                                            entry_map.insert("name".to_string(), serde_json::Value::String(namespaced_name.clone()));

                                            if let Some(cmd) = cfg_obj.get("command") {
                                                entry_map.insert("command".to_string(), cmd.clone());
                                            }

                                            // Resolve relative paths in arguments against plugin_dir
                                            if let Some(args_val) = cfg_obj.get("args").and_then(|a| a.as_array()) {
                                                let resolved_args: Vec<serde_json::Value> = args_val.iter().map(|arg| {
                                                    if let Some(arg_str) = arg.as_str() {
                                                        let candidate = plugin_dir.join(std::path::Path::new(arg_str));
                                                        if candidate.exists() {
                                                            #[cfg(target_os = "windows")]
                                                            let norm = candidate.to_string_lossy().replace('/', "\\");
                                                            #[cfg(not(target_os = "windows"))]
                                                            let norm = candidate.to_string_lossy().to_string();
                                                            return serde_json::Value::String(norm);
                                                        }
                                                    }
                                                    arg.clone()
                                                }).collect();
                                                entry_map.insert("args".to_string(), serde_json::Value::Array(resolved_args));
                                            }

                                            if let Some(env_val) = cfg_obj.get("env") {
                                                entry_map.insert("env".to_string(), env_val.clone());
                                            }

                                            // Set cwd to plugin directory if not explicitly provided
                                            if let Some(cwd_val) = cfg_obj.get("cwd") {
                                                entry_map.insert("cwd".to_string(), cwd_val.clone());
                                            } else {
                                                entry_map.insert("cwd".to_string(), serde_json::Value::String(plugin_dir.to_string_lossy().to_string()));
                                            }

                                            if let Some(url_val) = cfg_obj.get("url") {
                                                entry_map.insert("url".to_string(), url_val.clone());
                                            }

                                            server_map.insert(namespaced_name.clone(), serde_json::Value::Object(entry_map.clone()));

                                            // Also provide raw_s_name alias if non-colliding
                                            if raw_s_name != &namespaced_name && !server_map.contains_key(raw_s_name) {
                                                let mut raw_map = entry_map;
                                                raw_map.insert("name".to_string(), serde_json::Value::String(raw_s_name.clone()));
                                                server_map.insert(raw_s_name.clone(), serde_json::Value::Object(raw_map));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 2. Global MCP configuration
    if let Ok(global_cfg) = get_mcp_config_internal(None, Some(engine)) {
        if let Some(servers) = global_cfg.mcp_servers.as_object() {
            for (s_name, s_cfg) in servers {
                if let Some(cfg_obj) = s_cfg.as_object() {
                    if cfg_obj.get("disabled") == Some(&serde_json::Value::Bool(true)) {
                        server_map.remove(s_name);
                        continue;
                    }

                    let mut entry_map = serde_json::Map::new();
                    entry_map.insert("name".to_string(), serde_json::Value::String(s_name.clone()));

                    if let Some(cmd) = cfg_obj.get("command") {
                        entry_map.insert("command".to_string(), cmd.clone());
                    }
                    if let Some(args) = cfg_obj.get("args") {
                        entry_map.insert("args".to_string(), args.clone());
                    }
                    if let Some(env) = cfg_obj.get("env") {
                        entry_map.insert("env".to_string(), env.clone());
                    }
                    if let Some(cwd) = cfg_obj.get("cwd") {
                        entry_map.insert("cwd".to_string(), cwd.clone());
                    }
                    if let Some(url) = cfg_obj.get("url") {
                        entry_map.insert("url".to_string(), url.clone());
                    }

                    server_map.insert(s_name.clone(), serde_json::Value::Object(entry_map));
                }
            }
        }
    }

    // 3. Workspace MCP configuration (overrides global/plugin config; disabled: true explicitly suppresses)
    if let Some(ws) = workspace_path {
        let trimmed = ws.trim();
        if !trimmed.is_empty() {
            if let Ok(ws_cfg) = get_mcp_config_internal(Some(trimmed), Some(engine)) {
                if let Some(servers) = ws_cfg.mcp_servers.as_object() {
                    for (s_name, s_cfg) in servers {
                        if let Some(cfg_obj) = s_cfg.as_object() {
                            if cfg_obj.get("disabled") == Some(&serde_json::Value::Bool(true)) {
                                server_map.remove(s_name);
                                continue;
                            }

                            let mut entry_map = serde_json::Map::new();
                            entry_map.insert("name".to_string(), serde_json::Value::String(s_name.clone()));

                            if let Some(cmd) = cfg_obj.get("command") {
                                entry_map.insert("command".to_string(), cmd.clone());
                            }
                            if let Some(args) = cfg_obj.get("args") {
                                entry_map.insert("args".to_string(), args.clone());
                            }
                            if let Some(env) = cfg_obj.get("env") {
                                entry_map.insert("env".to_string(), env.clone());
                            }
                            if let Some(cwd) = cfg_obj.get("cwd") {
                                entry_map.insert("cwd".to_string(), cwd.clone());
                            } else {
                                entry_map.insert("cwd".to_string(), serde_json::Value::String(trimmed.to_string()));
                            }
                            if let Some(url) = cfg_obj.get("url") {
                                entry_map.insert("url".to_string(), url.clone());
                            }

                            server_map.insert(s_name.clone(), serde_json::Value::Object(entry_map));
                        }
                    }
                }
            }
        }
    }

    // 4. Return sorted by server name for deterministic order
    let mut sorted_keys: Vec<String> = server_map.keys().cloned().collect();
    sorted_keys.sort();

    sorted_keys.into_iter().filter_map(|k| server_map.remove(&k)).collect()
}

#[tauri::command]
pub async fn get_session_mcp_servers(
    state: State<'_, AppState>,
    workspace_path: Option<String>,
    engine: Option<String>,
) -> Result<Vec<serde_json::Value>, String> {
    let effective_engine = engine.or_else(|| {
        state.db.get_setting("active_cli_engine").ok().flatten()
    }).unwrap_or_else(|| "gemini".to_string());

    Ok(get_active_mcp_servers_for_session(workspace_path.as_deref(), &effective_engine))
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

    #[tokio::test]
    async fn test_get_active_mcp_servers_for_session_workspace_and_filtering() {
        let temp_dir = std::env::temp_dir().join(format!("gemini_acp_mcp_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let ws_str = temp_dir.to_string_lossy().to_string();

        // 1. Setup workspace Gemini settings with an active server and a disabled server
        let ws_gemini_servers = serde_json::json!({
            "active_tool": {
                "command": "node",
                "args": ["server.js"],
                "env": { "PORT": "3000" }
            },
            "disabled_tool": {
                "command": "python",
                "args": ["disabled.py"],
                "disabled": true
            }
        });
        save_mcp_config_internal(Some(&ws_str), Some("gemini"), ws_gemini_servers).unwrap();

        // 2. Fetch session servers for Gemini
        let servers = get_active_mcp_servers_for_session(Some(&ws_str), "gemini");
        assert!(servers.iter().any(|s| s["name"] == "active_tool"));
        assert!(!servers.iter().any(|s| s["name"] == "disabled_tool"));

        let active_entry = servers.iter().find(|s| s["name"] == "active_tool").unwrap();
        assert_eq!(active_entry["command"], "node");
        assert_eq!(active_entry["args"][0], "server.js");
        assert_eq!(active_entry["env"]["PORT"], "3000");

        // 3. Setup workspace Antigravity config (.agents/mcp_config.json)
        let ws_agy_servers = serde_json::json!({
            "agy_tool": {
                "command": "npx",
                "args": ["-y", "akbo-cli", "mcp"],
                "env": {}
            }
        });
        save_mcp_config_internal(Some(&ws_str), Some("agy"), ws_agy_servers).unwrap();

        let agy_servers = get_active_mcp_servers_for_session(Some(&ws_str), "agy");
        assert!(agy_servers.iter().any(|s| s["name"] == "agy_tool"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_plugin_mcp_args_resolution_and_cwd_injection() {
        let temp_dir = std::env::temp_dir().join(format!("gemini_plugin_mcp_test_{}", uuid::Uuid::new_v4()));
        let plugin_dir = temp_dir.join("test-plugin");
        let sidecar_dir = plugin_dir.join("sidecars").join("viewer");
        std::fs::create_dir_all(&sidecar_dir).unwrap();

        let dummy_script = sidecar_dir.join("entry.js");
        std::fs::write(&dummy_script, "console.log('mcp');").unwrap();

        // Write plugin mcp_config.json with relative arg
        let plugin_mcp = serde_json::json!({
            "mcpServers": {
                "viewer_mcp": {
                    "command": "node",
                    "args": ["sidecars/viewer/entry.js"]
                }
            }
        });
        std::fs::write(plugin_dir.join("mcp_config.json"), serde_json::to_string(&plugin_mcp).unwrap()).unwrap();

        // Test relative path resolution against plugin directory
        let plugin_dir_str = plugin_dir.to_string_lossy().to_string();
        let plugin_name = "test-plugin";

        let mut entry_map = serde_json::Map::new();
        let namespaced_name = format!("{}_{}", plugin_name, "viewer_mcp");
        entry_map.insert("name".to_string(), serde_json::Value::String(namespaced_name.clone()));
        entry_map.insert("command".to_string(), serde_json::Value::String("node".to_string()));

        let resolved_args: Vec<serde_json::Value> = vec![
            serde_json::Value::String("sidecars/viewer/entry.js".to_string())
        ].iter().map(|arg| {
            if let Some(arg_str) = arg.as_str() {
                let candidate = plugin_dir.join(std::path::Path::new(arg_str));
                if candidate.exists() {
                    #[cfg(target_os = "windows")]
                    let norm = candidate.to_string_lossy().replace('/', "\\");
                    #[cfg(not(target_os = "windows"))]
                    let norm = candidate.to_string_lossy().to_string();
                    return serde_json::Value::String(norm);
                }
            }
            arg.clone()
        }).collect();
        entry_map.insert("args".to_string(), serde_json::Value::Array(resolved_args));
        entry_map.insert("cwd".to_string(), serde_json::Value::String(plugin_dir_str.clone()));

        assert_eq!(namespaced_name, "test-plugin_viewer_mcp");
        assert_eq!(entry_map["cwd"], plugin_dir_str);
        assert_eq!(entry_map["args"][0], dummy_script.to_string_lossy().to_string());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
