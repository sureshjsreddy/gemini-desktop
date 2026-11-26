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
    crate::engines::get_adapter(engine.unwrap_or("gemini")).resolve_mcp_config_path(workspace_path)
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

    let parsed: serde_json::Value = serde_json::from_str(crate::commands::strip_utf8_bom(&content))
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
        serde_json::from_str(crate::commands::strip_utf8_bom(&content))
            .unwrap_or_else(|_| serde_json::json!({}))
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

/// Converts an MCP `env` or `headers` value (map `{K: V}` or array `[{name, value}]`)
/// into the strict ACP array format `[{ "name": K, "value": V }]`.
fn format_acp_kv_array(val: Option<&serde_json::Value>) -> serde_json::Value {
    match val {
        Some(serde_json::Value::Array(arr)) => serde_json::Value::Array(arr.clone()),
        Some(serde_json::Value::Object(obj)) => {
            let mut keys: Vec<&String> = obj.keys().collect();
            keys.sort();
            let items: Vec<serde_json::Value> = keys
                .into_iter()
                .map(|k| {
                    let v_str = match &obj[k] {
                        serde_json::Value::String(s) => s.clone(),
                        other => other.to_string(),
                    };
                    serde_json::json!({
                        "name": k,
                        "value": v_str
                    })
                })
                .collect();
            serde_json::Value::Array(items)
        }
        _ => serde_json::json!([]),
    }
}

/// Expands `${PLUGIN_ROOT}` and `${CLAUDE_PLUGIN_ROOT}` placeholders inside plugin MCP definitions.
fn expand_plugin_root_vars(raw: &str, plugin_dir: Option<&std::path::Path>) -> String {
    match plugin_dir {
        Some(dir) => {
            let dir_str = dir.to_string_lossy();
            raw.replace("${PLUGIN_ROOT}", &dir_str)
                .replace("${CLAUDE_PLUGIN_ROOT}", &dir_str)
        }
        None => raw.to_string(),
    }
}

fn expand_kv_array_plugin_vars(val: serde_json::Value, plugin_dir: Option<&std::path::Path>) -> serde_json::Value {
    match (val, plugin_dir) {
        (serde_json::Value::Array(arr), Some(_)) => {
            let mapped = arr
                .into_iter()
                .map(|mut item| {
                    if let Some(v_str) = item.get("value").and_then(|v| v.as_str()).map(|s| s.to_string()) {
                        if let Some(obj) = item.as_object_mut() {
                            obj.insert(
                                "value".to_string(),
                                serde_json::Value::String(expand_plugin_root_vars(&v_str, plugin_dir)),
                            );
                        }
                    }
                    item
                })
                .collect();
            serde_json::Value::Array(mapped)
        }
        (other, _) => other,
    }
}

/// Builds an ACP-compliant `McpServer` entry (`McpServerStdio` or `McpServerHttp`)
/// from a raw `mcpServers` config object.
fn build_acp_mcp_entry(
    name: &str,
    cfg_obj: &serde_json::Map<String, serde_json::Value>,
    resolve_plugin_dir: Option<&std::path::Path>,
    default_cwd: Option<&str>,
) -> serde_json::Value {
    let mut entry_map = serde_json::Map::new();
    entry_map.insert("name".to_string(), serde_json::Value::String(name.to_string()));

    if cfg_obj.get("eager").and_then(|v| v.as_bool()) == Some(true) {
        entry_map.insert("eager".to_string(), serde_json::Value::Bool(true));
    }

    // Streamable HTTP / SSE transport
    if let Some(url_val) = cfg_obj.get("url").or_else(|| cfg_obj.get("serverUrl")) {
        entry_map.insert("type".to_string(), serde_json::Value::String("http".to_string()));
        let resolved_url = url_val
            .as_str()
            .map(|u| serde_json::Value::String(expand_plugin_root_vars(u, resolve_plugin_dir)))
            .unwrap_or_else(|| url_val.clone());
        entry_map.insert("url".to_string(), resolved_url);
        entry_map.insert(
            "headers".to_string(),
            expand_kv_array_plugin_vars(format_acp_kv_array(cfg_obj.get("headers")), resolve_plugin_dir),
        );
        return serde_json::Value::Object(entry_map);
    }

    // Stdio transport
    if let Some(cmd_val) = cfg_obj.get("command") {
        let final_cmd = if let Some(cmd_str) = cmd_val.as_str() {
            let expanded_cmd = expand_plugin_root_vars(cmd_str, resolve_plugin_dir);
            if expanded_cmd == "agy-node"
                && crate::process_manager::find_binary_by_name("agy-node").is_none()
                && crate::process_manager::find_binary_by_name("node").is_some()
            {
                serde_json::Value::String("node".to_string())
            } else {
                serde_json::Value::String(expanded_cmd)
            }
        } else {
            cmd_val.clone()
        };
        entry_map.insert("command".to_string(), final_cmd);
    }

    let resolved_args = if let Some(args_val) = cfg_obj.get("args").and_then(|a| a.as_array()) {
        args_val
            .iter()
            .map(|arg| {
                if let Some(arg_str) = arg.as_str() {
                    let expanded = expand_plugin_root_vars(arg_str, resolve_plugin_dir);
                    if let Some(plugin_dir) = resolve_plugin_dir {
                        let candidate = plugin_dir.join(std::path::Path::new(&expanded));
                        if candidate.exists() && expanded == arg_str {
                            #[cfg(target_os = "windows")]
                            let norm = candidate.to_string_lossy().replace('/', "\\");
                            #[cfg(not(target_os = "windows"))]
                            let norm = candidate.to_string_lossy().to_string();
                            return serde_json::Value::String(norm);
                        }
                    }
                    return serde_json::Value::String(expanded);
                }
                arg.clone()
            })
            .collect()
    } else {
        Vec::new()
    };
    entry_map.insert("args".to_string(), serde_json::Value::Array(resolved_args));
    entry_map.insert(
        "env".to_string(),
        expand_kv_array_plugin_vars(format_acp_kv_array(cfg_obj.get("env")), resolve_plugin_dir),
    );

    if let Some(cwd_val) = cfg_obj.get("cwd") {
        let resolved_cwd = cwd_val
            .as_str()
            .map(|c| serde_json::Value::String(expand_plugin_root_vars(c, resolve_plugin_dir)))
            .unwrap_or_else(|| cwd_val.clone());
        entry_map.insert("cwd".to_string(), resolved_cwd);
    } else if let Some(cwd_fallback) = default_cwd {
        entry_map.insert("cwd".to_string(), serde_json::Value::String(cwd_fallback.to_string()));
    }

    serde_json::Value::Object(entry_map)
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
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(crate::commands::strip_utf8_bom(&content)) {
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

                        let plugin_mcp_file = {
                            let primary = plugin_dir.join("mcp.json");
                            let legacy = plugin_dir.join("mcp_config.json");
                            if primary.is_file() {
                                primary
                            } else if legacy.is_file() {
                                legacy
                            } else {
                                continue;
                            }
                        };

                        if let Ok(content) = std::fs::read_to_string(&plugin_mcp_file) {
                            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(crate::commands::strip_utf8_bom(&content)) {
                                if let Some(servers) = parsed.get("mcpServers").and_then(|s| s.as_object()) {
                                    let plugin_dir_str = plugin_dir.to_string_lossy().to_string();
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

                                            let entry_val = build_acp_mcp_entry(
                                                &namespaced_name,
                                                cfg_obj,
                                                Some(&plugin_dir),
                                                Some(&plugin_dir_str),
                                            );
                                            server_map.insert(namespaced_name, entry_val);
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

                    let entry_val = build_acp_mcp_entry(s_name, cfg_obj, None, None);
                    server_map.insert(s_name.clone(), entry_val);
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

                            let entry_val = build_acp_mcp_entry(s_name, cfg_obj, None, Some(trimmed));
                            server_map.insert(s_name.clone(), entry_val);
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct McpRuntimeDiagnostics {
    pub engine: String,
    pub active_servers: Vec<String>,
    pub preloaded_count: Option<usize>,
    pub admin_blocked: bool,
    pub preload_timeout: bool,
    pub server_errors: Vec<String>,
    pub latest_log_path: Option<String>,
}

pub fn diagnose_agy_log_content(content: &str) -> (Option<usize>, bool, bool, Vec<String>) {
    let mut preloaded_count: Option<usize> = None;
    let mut admin_blocked = false;
    let mut preload_timeout = false;
    let mut server_errors: Vec<String> = Vec::new();

    for line in content.lines() {
        let lower = line.to_lowercase();
        if let Some(idx) = line.find("Preloading ") {
            if let Some(rest) = line[idx + "Preloading ".len()..].split_whitespace().next() {
                if let Ok(n) = rest.parse::<usize>() {
                    preloaded_count = Some(n);
                }
            }
        }
        if lower.contains("mcp disabled by baic admin controls")
            || lower.contains("mcp_server_status_disabled_by_admin")
            || lower.contains("disabled by admin")
        {
            admin_blocked = true;
        }
        if lower.contains("failed to preload the session's mcp servers")
            && lower.contains("context deadline exceeded")
        {
            preload_timeout = true;
        }
        if lower.contains("mcp disabled by baic")
            || lower.contains("mcp_server_status_disabled_by_admin")
            || ((lower.contains("mcp server") || lower.contains("server name "))
                && (lower.contains("failed to load")
                    || lower.contains("executable file not found")
                    || lower.contains("connection closed unexpectedly")))
        {
            let cleaned = line
                .split(']')
                .nth(1)
                .map(|s| s.trim())
                .unwrap_or_else(|| line.trim())
                .to_string();
            if !cleaned.is_empty() && !server_errors.contains(&cleaned) {
                server_errors.push(cleaned);
            }
        }
    }

    (preloaded_count, admin_blocked, preload_timeout, server_errors)
}

pub fn diagnose_mcp_runtime_internal(workspace_path: Option<&str>, engine: &str) -> McpRuntimeDiagnostics {
    let servers = get_active_mcp_servers_for_session(workspace_path, engine);
    let active_servers: Vec<String> = servers
        .iter()
        .filter_map(|s| s.get("name").and_then(|v| v.as_str()).map(|n| n.to_string()))
        .collect();

    let mut preloaded_count = None;
    let mut admin_blocked = false;
    let mut preload_timeout = false;
    let mut server_errors = Vec::new();
    let mut latest_log_path = None;

    if engine.trim().eq_ignore_ascii_case("agy") {
        if let Ok(home) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
            let log_dir = PathBuf::from(home).join(".gemini").join("antigravity-cli").join("log");
            if log_dir.is_dir() {
                if let Ok(entries) = std::fs::read_dir(&log_dir) {
                    let mut latest: Option<(std::time::SystemTime, PathBuf)> = None;
                    for entry in entries.flatten() {
                        let p = entry.path();
                        let fname = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                        if p.is_file() && fname.starts_with("acp-server-") && fname.ends_with(".log") {
                            if let Ok(meta) = entry.metadata() {
                                if let Ok(modified) = meta.modified() {
                                    if latest.as_ref().map(|(t, _)| modified > *t).unwrap_or(true) {
                                        latest = Some((modified, p));
                                    }
                                }
                            }
                        }
                    }
                    if let Some((_, p)) = latest {
                        latest_log_path = Some(p.to_string_lossy().to_string());
                        if let Ok(content) = std::fs::read_to_string(&p) {
                            let (pc, ab, pt, errs) = diagnose_agy_log_content(&content);
                            preloaded_count = pc;
                            admin_blocked = ab;
                            preload_timeout = pt;
                            server_errors = errs;
                        }
                    }
                }
            }
        }
    }

    McpRuntimeDiagnostics {
        engine: engine.to_string(),
        active_servers,
        preloaded_count,
        admin_blocked,
        preload_timeout,
        server_errors,
        latest_log_path,
    }
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

#[tauri::command]
pub async fn diagnose_mcp_runtime(
    state: State<'_, AppState>,
    workspace_path: Option<String>,
    engine: Option<String>,
) -> Result<McpRuntimeDiagnostics, String> {
    let effective_engine = engine.or_else(|| {
        state.db.get_setting("active_cli_engine").ok().flatten()
    }).unwrap_or_else(|| "gemini".to_string());

    Ok(diagnose_mcp_runtime_internal(workspace_path.as_deref(), &effective_engine))
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
        assert_eq!(active_entry["env"][0]["name"], "PORT");
        assert_eq!(active_entry["env"][0]["value"], "3000");

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
        let agy_entry = agy_servers.iter().find(|s| s["name"] == "agy_tool").unwrap();
        assert!(agy_entry["env"].is_array());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_acp_mcp_env_and_http_normalization() {
        let stdio_cfg = serde_json::json!({
            "command": "npx",
            "args": ["-y", "@playwright/mcp@latest"],
            "env": { "Z_KEY": "2", "A_KEY": "1" }
        });
        let stdio_entry = build_acp_mcp_entry("playwright", stdio_cfg.as_object().unwrap(), None, None);
        assert_eq!(stdio_entry["name"], "playwright");
        assert_eq!(stdio_entry["command"], "npx");
        assert_eq!(stdio_entry["args"][0], "-y");
        assert_eq!(stdio_entry["env"][0]["name"], "A_KEY");
        assert_eq!(stdio_entry["env"][0]["value"], "1");
        assert_eq!(stdio_entry["env"][1]["name"], "Z_KEY");
        assert_eq!(stdio_entry["env"][1]["value"], "2");

        let http_cfg = serde_json::json!({
            "serverUrl": "https://mcp.example.com/mcp",
            "headers": { "Authorization": "Bearer tok" }
        });
        let http_entry = build_acp_mcp_entry("remote", http_cfg.as_object().unwrap(), None, None);
        assert_eq!(http_entry["type"], "http");
        assert_eq!(http_entry["url"], "https://mcp.example.com/mcp");
        assert_eq!(http_entry["headers"][0]["name"], "Authorization");
        assert_eq!(http_entry["headers"][0]["value"], "Bearer tok");
    }

    #[test]
    fn test_diagnose_agy_log_content_admin_and_timeout() {
        let sample_log = "\
I1125 09:37:01.000000 10 session_handlers.go:397] Preloading 2 MCP servers of the session
W1125 09:37:06.000000 10 session_handlers.go:401] Failed to preload the session's MCP servers: context deadline exceeded
W1125 09:37:06.100000 11 connector.go:131] MCP server structural-biology_structure_viewer_mcp: exec: \"agy-node\": executable file not found in %PATH%
W1125 09:37:06.200000 12 mcp_manager.go:100] mcp disabled by BAIC admin controls
";
        let (count, admin_blocked, preload_timeout, errs) = diagnose_agy_log_content(sample_log);
        assert_eq!(count, Some(2));
        assert!(admin_blocked);
        assert!(preload_timeout);
        assert!(errs.iter().any(|e| e.contains("agy-node")));
        assert!(errs.iter().any(|e| e.contains("BAIC admin controls")));
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

    #[test]
    fn test_build_acp_mcp_entry_plugin_root_and_eager_http() {
        let plugin_dir = std::path::PathBuf::from(r"C:\Plugins\demo-plugin");

        // Stdio entry with ${PLUGIN_ROOT} and ${CLAUDE_PLUGIN_ROOT} + eager
        let mut stdio_obj = serde_json::Map::new();
        stdio_obj.insert("command".to_string(), serde_json::json!("${PLUGIN_ROOT}/bin/server.exe"));
        stdio_obj.insert("args".to_string(), serde_json::json!(["--root", "${CLAUDE_PLUGIN_ROOT}/data"]));
        stdio_obj.insert("cwd".to_string(), serde_json::json!("${PLUGIN_ROOT}"));
        stdio_obj.insert("eager".to_string(), serde_json::json!(true));
        stdio_obj.insert("env".to_string(), serde_json::json!({ "PLUGIN_HOME": "${PLUGIN_ROOT}/cfg" }));

        let stdio_entry = build_acp_mcp_entry("demo_srv", &stdio_obj, Some(&plugin_dir), None);
        assert_eq!(stdio_entry["eager"], true);
        assert_eq!(stdio_entry["command"], r"C:\Plugins\demo-plugin/bin/server.exe");
        assert_eq!(stdio_entry["args"][1], r"C:\Plugins\demo-plugin/data");
        assert_eq!(stdio_entry["cwd"], r"C:\Plugins\demo-plugin");
        assert_eq!(stdio_entry["env"][0]["value"], r"C:\Plugins\demo-plugin/cfg");

        // HTTP entry with headers and without eager
        let mut http_obj = serde_json::Map::new();
        http_obj.insert("url".to_string(), serde_json::json!("https://mcp.example.com/sse"));
        http_obj.insert("headers".to_string(), serde_json::json!({ "Authorization": "Bearer tok" }));

        let http_entry = build_acp_mcp_entry("remote_mcp", &http_obj, Some(&plugin_dir), None);
        assert_eq!(http_entry["type"], "http");
        assert_eq!(http_entry["url"], "https://mcp.example.com/sse");
        assert!(http_entry.get("eager").is_none());
        assert_eq!(http_entry["headers"][0]["name"], "Authorization");
    }
}
