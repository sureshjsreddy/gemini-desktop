use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use tokio::sync::{oneshot, Mutex};
use tokio::time::{timeout, Duration};
use std::io::Write;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    #[serde(skip_serializing_if = "Value::is_null")]
    pub params: Value,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<u64>,
    pub result: Option<Value>,
    pub error: Option<JsonRpcError>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    pub data: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunkPayload {
    pub session_id: String,
    pub delta: String,
    pub is_done: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionOption {
    pub option_id: String,
    pub name: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPermissionPayload {
    pub request_id: u64,
    pub session_id: String,
    pub tool_call_id: Option<String>,
    pub tool_name: String,
    pub title: Option<String>,
    pub kind: Option<String>,
    pub parameters: Value,
    pub locations: Option<Value>,
    pub content: Option<Value>,
    pub reason: Option<String>,
    pub options: Vec<PermissionOption>,
}

pub struct AcpSession {
    next_id: AtomicU64,
    stdin_writer: Arc<Mutex<Option<Box<dyn Write + Send>>>>,
    pending_requests: Arc<StdMutex<HashMap<u64, oneshot::Sender<Result<Value, JsonRpcError>>>>>,
    pending_permissions: Arc<StdMutex<HashMap<u64, Vec<PermissionOption>>>>,
    pending_prompt_sessions: Arc<StdMutex<HashMap<u64, String>>>,
    local_to_acp: Arc<StdMutex<HashMap<String, String>>>,
    acp_to_local: Arc<StdMutex<HashMap<String, String>>>,
    active_local_session: Arc<StdMutex<Option<String>>>,
    session_approval_modes: Arc<StdMutex<HashMap<String, String>>>,
    active_approval_mode: Arc<StdMutex<String>>,
    workspace_dir: Arc<StdMutex<Option<PathBuf>>>,
    terminal_results: Arc<StdMutex<HashMap<String, (String, i32)>>>,
}

impl AcpSession {
    pub fn new() -> Self {
        Self {
            next_id: AtomicU64::new(1),
            stdin_writer: Arc::new(Mutex::new(None)),
            pending_requests: Arc::new(StdMutex::new(HashMap::new())),
            pending_permissions: Arc::new(StdMutex::new(HashMap::new())),
            pending_prompt_sessions: Arc::new(StdMutex::new(HashMap::new())),
            local_to_acp: Arc::new(StdMutex::new(HashMap::new())),
            acp_to_local: Arc::new(StdMutex::new(HashMap::new())),
            active_local_session: Arc::new(StdMutex::new(None)),
            session_approval_modes: Arc::new(StdMutex::new(HashMap::new())),
            active_approval_mode: Arc::new(StdMutex::new("default".to_string())),
            workspace_dir: Arc::new(StdMutex::new(None)),
            terminal_results: Arc::new(StdMutex::new(HashMap::new())),
        }
    }

    pub async fn set_stdin(&self, writer: Box<dyn Write + Send>) {
        let mut guard = self.stdin_writer.lock().await;
        *guard = Some(writer);
    }

    pub fn next_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::SeqCst)
    }

    pub fn register_session_mapping(&self, local_id: &str, acp_id: &str) {
        if let Ok(mut l2a) = self.local_to_acp.lock() {
            l2a.insert(local_id.to_string(), acp_id.to_string());
        }
        if let Ok(mut a2l) = self.acp_to_local.lock() {
            a2l.insert(acp_id.to_string(), local_id.to_string());
        }
    }

    pub fn get_acp_session_id(&self, local_id: &str) -> Option<String> {
        self.local_to_acp.lock().ok()?.get(local_id).cloned()
    }

    pub fn get_local_session_id(&self, acp_id: &str) -> Option<String> {
        self.acp_to_local.lock().ok()?.get(acp_id).cloned()
    }

    pub fn remove_session(&self, local_id: &str) {
        if let Ok(mut l2a) = self.local_to_acp.lock() {
            if let Some(acp_id) = l2a.remove(local_id) {
                if let Ok(mut a2l) = self.acp_to_local.lock() {
                    a2l.remove(&acp_id);
                }
            }
        }
    }

    pub fn set_active_local_session(&self, local_id: &str) {
        if let Ok(mut guard) = self.active_local_session.lock() {
            *guard = Some(local_id.to_string());
        }
    }

    pub fn get_active_local_session(&self) -> Option<String> {
        self.active_local_session.lock().ok()?.clone()
    }

    pub fn clear_sessions(&self) {
        if let Ok(mut l2a) = self.local_to_acp.lock() {
            l2a.clear();
        }
        if let Ok(mut a2l) = self.acp_to_local.lock() {
            a2l.clear();
        }
        if let Ok(mut map) = self.pending_prompt_sessions.lock() {
            map.clear();
        }
        if let Ok(mut guard) = self.active_local_session.lock() {
            *guard = None;
        }
        if let Ok(mut modes) = self.session_approval_modes.lock() {
            modes.clear();
        }
    }

    pub fn set_session_mode(&self, session_id: &str, mode: &str) {
        if let Ok(mut guard) = self.session_approval_modes.lock() {
            guard.insert(session_id.to_string(), mode.to_string());
        }
        if let Ok(mut guard) = self.active_approval_mode.lock() {
            *guard = mode.to_string();
        }
    }

    pub fn get_session_mode(&self, session_id: &str) -> String {
        if let Ok(guard) = self.session_approval_modes.lock() {
            if let Some(m) = guard.get(session_id) {
                return m.clone();
            }
        }
        if let Ok(guard) = self.active_approval_mode.lock() {
            return guard.clone();
        }
        "default".to_string()
    }

    pub fn register_prompt_request(&self, request_id: u64, local_session_id: &str) {
        if let Ok(mut map) = self.pending_prompt_sessions.lock() {
            map.insert(request_id, local_session_id.to_string());
        }
    }

    pub fn take_prompt_session(&self, request_id: u64) -> Option<String> {
        self.pending_prompt_sessions.lock().ok()?.remove(&request_id)
    }

    pub fn take_pending_request(&self, id: u64) -> Option<oneshot::Sender<Result<Value, JsonRpcError>>> {
        self.pending_requests.lock().ok()?.remove(&id)
    }

    pub fn set_workspace_dir(&self, dir: PathBuf) {
        if let Ok(mut guard) = self.workspace_dir.lock() {
            *guard = Some(dir);
        }
    }

    pub fn get_workspace_dir(&self) -> Option<PathBuf> {
        self.workspace_dir.lock().ok()?.clone()
    }

    pub fn store_terminal_result(&self, id: &str, output: String, exit_code: i32) {
        if let Ok(mut guard) = self.terminal_results.lock() {
            guard.insert(id.to_string(), (output, exit_code));
        }
    }

    pub fn get_terminal_result(&self, id: &str) -> Option<(String, i32)> {
        self.terminal_results.lock().ok()?.get(id).cloned()
    }

    pub fn remove_terminal_result(&self, id: &str) {
        if let Ok(mut guard) = self.terminal_results.lock() {
            guard.remove(id);
        }
    }

    #[allow(dead_code)]
    pub async fn send_request(&self, method: &str, params: Value) -> Result<u64, String> {
        let id = self.next_id();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.to_string(),
            params,
        };

        let json_line = serde_json::to_string(&req).map_err(|e| e.to_string())? + "\n";

        let mut guard = self.stdin_writer.lock().await;
        if let Some(writer) = guard.as_mut() {
            writer.write_all(json_line.as_bytes()).map_err(|e| e.to_string())?;
            writer.flush().map_err(|e| e.to_string())?;
            Ok(id)
        } else {
            Err("CLI stdin is not connected".to_string())
        }
    }

    pub async fn send_prompt_request(&self, local_session_id: &str, params: Value) -> Result<u64, String> {
        let id = self.next_id();
        self.register_prompt_request(id, local_session_id);
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id,
            method: "session/prompt".to_string(),
            params,
        };

        let json_line = serde_json::to_string(&req).map_err(|e| e.to_string())? + "\n";

        let mut guard = self.stdin_writer.lock().await;
        if let Some(writer) = guard.as_mut() {
            writer.write_all(json_line.as_bytes()).map_err(|e| e.to_string())?;
            writer.flush().map_err(|e| e.to_string())?;
            Ok(id)
        } else {
            self.take_prompt_session(id);
            Err("CLI stdin is not connected".to_string())
        }
    }

    pub async fn send_request_with_response(&self, method: &str, params: Value) -> Result<Value, String> {
        let id = self.next_id();
        let (tx, rx) = oneshot::channel();
        if let Ok(mut guard) = self.pending_requests.lock() {
            guard.insert(id, tx);
        }

        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.to_string(),
            params,
        };

        let json_line = serde_json::to_string(&req).map_err(|e| e.to_string())? + "\n";

        {
            let mut guard = self.stdin_writer.lock().await;
            if let Some(writer) = guard.as_mut() {
                writer.write_all(json_line.as_bytes()).map_err(|e| e.to_string())?;
                writer.flush().map_err(|e| e.to_string())?;
            } else {
                if let Ok(mut pguard) = self.pending_requests.lock() {
                    pguard.remove(&id);
                }
                return Err("CLI stdin is not connected".to_string());
            }
        }

        match timeout(Duration::from_secs(20), rx).await {
            Ok(Ok(Ok(val))) => Ok(val),
            Ok(Ok(Err(rpc_err))) => Err(format!("RPC error (code {}): {}", rpc_err.code, rpc_err.message)),
            Ok(Err(_)) => Err("Pending request dropped without response".to_string()),
            Err(_) => {
                if let Ok(mut pguard) = self.pending_requests.lock() {
                    pguard.remove(&id);
                }
                Err(format!("Timed out waiting for response to '{}'", method))
            }
        }
    }

    pub async fn send_response(&self, id: u64, result: Value) -> Result<(), String> {
        let resp = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result
        });
        let json_line = serde_json::to_string(&resp).map_err(|e| e.to_string())? + "\n";

        let mut guard = self.stdin_writer.lock().await;
        if let Some(writer) = guard.as_mut() {
            writer.write_all(json_line.as_bytes()).map_err(|e| e.to_string())?;
            writer.flush().map_err(|e| e.to_string())?;
            Ok(())
        } else {
            Err("CLI stdin is not connected".to_string())
        }
    }

    pub async fn send_error_response(&self, id: u64, code: i64, message: &str) -> Result<(), String> {
        let resp = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": code,
                "message": message
            }
        });
        let json_line = serde_json::to_string(&resp).map_err(|e| e.to_string())? + "\n";

        let mut guard = self.stdin_writer.lock().await;
        if let Some(writer) = guard.as_mut() {
            writer.write_all(json_line.as_bytes()).map_err(|e| e.to_string())?;
            writer.flush().map_err(|e| e.to_string())?;
            Ok(())
        } else {
            Err("CLI stdin is not connected".to_string())
        }
    }

    pub async fn send_notification(&self, method: &str, params: Value) -> Result<(), String> {
        let notif = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });
        let json_line = serde_json::to_string(&notif).map_err(|e| e.to_string())? + "\n";

        let mut guard = self.stdin_writer.lock().await;
        if let Some(writer) = guard.as_mut() {
            writer.write_all(json_line.as_bytes()).map_err(|e| e.to_string())?;
            writer.flush().map_err(|e| e.to_string())?;
            Ok(())
        } else {
            Err("CLI stdin is not connected".to_string())
        }
    }

    pub async fn send_cancel(&self, session_id: Option<&str>, target_request_id: Option<u64>) -> Result<(), String> {
        let mut params = serde_json::Map::new();
        if let Some(sid) = session_id {
            let actual_sid = self.get_acp_session_id(sid).unwrap_or_else(|| sid.to_string());
            params.insert("sessionId".to_string(), serde_json::Value::String(actual_sid));
        }

        // Per ACP specification: session/cancel is a JSON-RPC notification (no 'id' field)
        self.send_notification("session/cancel", serde_json::Value::Object(params)).await?;

        // Also send $/cancel_request notification if target_request_id is provided
        if let Some(rid) = target_request_id {
            let _ = self.send_notification("$/cancel_request", serde_json::json!({ "id": rid })).await;
        }

        Ok(())
    }

    pub fn store_permission_options(&self, request_id: u64, options: Vec<PermissionOption>) {
        if let Ok(mut guard) = self.pending_permissions.lock() {
            guard.insert(request_id, options);
        }
    }

    pub async fn respond_permission(&self, request_id: u64, option_id: Option<String>, allowed: bool) -> Result<(), String> {
        let stored_options = self.pending_permissions.lock().ok().and_then(|mut m| m.remove(&request_id));
        let result = build_permission_response_payload(allowed, option_id, stored_options);
        self.send_response(request_id, result).await
    }
}

/// Builds the official ACP permission response JSON matching Gemini CLI's Zod schema.
/// When allowed: outcome is "selected" with a valid ToolConfirmationOutcome ("proceed_once", "proceed_always", etc.)
/// When rejected: outcome is "cancelled" (discriminated union) without an optionId.
pub fn build_permission_response_payload(
    allowed: bool,
    option_id: Option<String>,
    stored_options: Option<Vec<PermissionOption>>,
) -> Value {
    if !allowed {
        serde_json::json!({
            "outcome": {
                "outcome": "cancelled"
            }
        })
    } else {
        let chosen_id = if let Some(oid) = option_id {
            if let Some(ref opts) = stored_options {
                if opts.iter().any(|o| o.option_id == oid) {
                    oid
                } else {
                    match oid.as_str() {
                        "proceed_once" | "proceed_always" | "proceed_always_and_save"
                        | "proceed_always_server" | "proceed_always_tool" => oid,
                        _ => opts.iter()
                            .find(|o| o.kind.contains("allow") || o.option_id.contains("allow") || o.option_id.contains("proceed"))
                            .map(|o| o.option_id.clone())
                            .unwrap_or_else(|| "proceed_once".to_string()),
                    }
                }
            } else {
                match oid.as_str() {
                    "proceed_once" | "proceed_always" | "proceed_always_and_save"
                    | "proceed_always_server" | "proceed_always_tool" => oid,
                    _ => "proceed_once".to_string(),
                }
            }
        } else if let Some(opts) = stored_options {
            opts.iter()
                .find(|o| o.kind.contains("allow") || o.option_id.contains("allow") || o.option_id.contains("proceed"))
                .map(|o| o.option_id.clone())
                .or_else(|| opts.first().map(|o| o.option_id.clone()))
                .unwrap_or_else(|| "proceed_once".to_string())
        } else {
            "proceed_once".to_string()
        };

        serde_json::json!({
            "outcome": {
                "outcome": "selected",
                "optionId": chosen_id
            }
        })
    }
}

/// Detects and strips internal Gemini CLI control tags like `[MODE_UPDATE] <mode>`
/// from incoming text deltas while extracting the reported mode.
pub fn sanitize_acp_delta(delta: &str) -> (String, Option<String>) {
    if !delta.contains("[MODE_UPDATE]") {
        return (delta.to_string(), None);
    }

    let mut detected_mode = None;
    let mut cleaned_lines = Vec::new();

    for line in delta.lines() {
        if let Some(idx) = line.find("[MODE_UPDATE]") {
            let after = &line[idx + "[MODE_UPDATE]".len()..];
            let trimmed = after.trim_start();
            let mode_token = if let Some(space_idx) = trimmed.find(|c: char| c.is_whitespace()) {
                &trimmed[..space_idx]
            } else {
                trimmed
            };
            if !mode_token.is_empty() {
                detected_mode = Some(mode_token.to_string());
            }

            let before = &line[..idx];
            let after_mode = if let Some(space_idx) = trimmed.find(|c: char| c.is_whitespace()) {
                &trimmed[space_idx..]
            } else {
                ""
            };
            let rebuilt = format!("{}{}", before, after_mode);
            if !rebuilt.trim().is_empty() {
                cleaned_lines.push(rebuilt);
            }
        } else {
            cleaned_lines.push(line.to_string());
        }
    }

    let mut clean_str = cleaned_lines.join("\n");

    // Also strip inline if [MODE_UPDATE] wasn't separated by newlines
    while let Some(idx) = clean_str.find("[MODE_UPDATE]") {
        let after = &clean_str[idx + "[MODE_UPDATE]".len()..];
        let trimmed = after.trim_start();
        let rest = if let Some(space_idx) = trimmed.find(|c: char| c.is_whitespace()) {
            &trimmed[space_idx..]
        } else {
            ""
        };
        clean_str = format!("{}{}", &clean_str[..idx], rest);
    }

    (clean_str, detected_mode)
}

/// Robustly extracts readable text content from any ACP JSON payload:
/// strings, objects with text/content/delta/output/message, or arrays of ContentBlocks.
pub fn extract_acp_text(val: &Value) -> String {
    match val {
        Value::String(s) => s.clone(),
        Value::Array(arr) => {
            let mut pieces = Vec::new();
            for item in arr {
                let piece = extract_acp_text(item);
                if !piece.is_empty() {
                    pieces.push(piece);
                }
            }
            pieces.join("")
        }
        Value::Object(obj) => {
            // Check direct "text" property
            if let Some(text_val) = obj.get("text") {
                let extracted = extract_acp_text(text_val);
                if !extracted.is_empty() {
                    return extracted;
                }
            }
            // Check "delta" property (e.g. streaming delta)
            if let Some(delta_val) = obj.get("delta") {
                let extracted = extract_acp_text(delta_val);
                if !extracted.is_empty() {
                    return extracted;
                }
            }
            // Check "content" property (content block, string, or array of content blocks)
            if let Some(content_val) = obj.get("content") {
                let extracted = extract_acp_text(content_val);
                if !extracted.is_empty() {
                    return extracted;
                }
            }
            // Check "output" property
            if let Some(output_val) = obj.get("output") {
                let extracted = extract_acp_text(output_val);
                if !extracted.is_empty() {
                    return extracted;
                }
            }
            // Check "message" property
            if let Some(msg_val) = obj.get("message") {
                let extracted = extract_acp_text(msg_val);
                if !extracted.is_empty() {
                    return extracted;
                }
            }
            // Check plan "entries" property
            if let Some(entries) = obj.get("entries").and_then(|e| e.as_array()) {
                let mut pieces = Vec::new();
                for entry in entries {
                    let piece = extract_acp_text(entry);
                    if !piece.is_empty() {
                        pieces.push(format!("- {}\n", piece));
                    }
                }
                if !pieces.is_empty() {
                    return pieces.join("");
                }
            }
            String::new()
        }
        _ => String::new(),
    }
}

pub fn handle_acp_line(line: &str, app_handle: &AppHandle, acp_session: &Arc<AcpSession>, active_session_id: &str) {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return;
    }

    // Try parsing as JSON-RPC object
    if let Ok(val) = serde_json::from_str::<Value>(trimmed) {
        // 1. Resolve pending request if this response corresponds to one (e.g. initialize, session/new)
        if let Some(id) = val.get("id").and_then(|i| i.as_u64()) {
            if let Some(tx) = acp_session.take_pending_request(id) {
                if let Some(err_val) = val.get("error") {
                    let err: JsonRpcError = serde_json::from_value(err_val.clone()).unwrap_or_else(|_| JsonRpcError {
                        code: err_val.get("code").and_then(|c| c.as_i64()).unwrap_or(-32603),
                        message: err_val.get("message").and_then(|m| m.as_str()).unwrap_or("Unknown RPC error").to_string(),
                        data: err_val.get("data").cloned(),
                    });
                    let _ = tx.send(Err(err));
                    return;
                } else if let Some(res_val) = val.get("result") {
                    let _ = tx.send(Ok(res_val.clone()));
                    return;
                }
            }

            // 1b. Check if this is the completion response for a session/prompt request
            if let Some(target_session) = acp_session.take_prompt_session(id) {
                if let Some(err_val) = val.get("error") {
                    let code = err_val.get("code").and_then(|c| c.as_i64()).unwrap_or(0);
                    let message = err_val.get("message").and_then(|m| m.as_str()).unwrap_or("Unknown error");
                    if code != -32800 && !message.to_lowercase().contains("cancel") {
                        let _ = app_handle.emit("acp-error", val.clone());
                    }
                }

                let delta = val.get("result")
                    .map(extract_acp_text)
                    .unwrap_or_default();

                let (clean_delta, mode_opt) = sanitize_acp_delta(&delta);
                if let Some(mode) = mode_opt {
                    let _ = app_handle.emit("acp-mode-update", serde_json::json!({
                        "sessionId": target_session,
                        "mode": mode
                    }));
                }

                let stop_reason = val.pointer("/result/stopReason").and_then(|s| s.as_str());
                if matches!(stop_reason, Some("max_turn_requests") | Some("max_tokens")) {
                    acp_session.remove_session(&target_session);
                }

                let final_delta = if clean_delta.is_empty() {
                    match stop_reason {
                        Some("max_turn_requests") => "\n\n*(Session reached maximum turn limit. Next prompt will start a fresh session context.)*".to_string(),
                        Some("max_tokens") => "\n\n*(Context token window limit reached. Next prompt will start a fresh session context.)*".to_string(),
                        Some("cancelled") => "\n\n*(Prompt cancelled)*".to_string(),
                        _ => String::new(),
                    }
                } else {
                    clean_delta
                };

                let _ = app_handle.emit("acp-chunk", StreamChunkPayload {
                    session_id: target_session,
                    delta: final_delta,
                    is_done: true,
                });
                return;
            }
        }

        if let Some(method) = val.get("method").and_then(|m| m.as_str()) {
            match method {
                // Streaming chunk notification
                "session/update" | "acp/chunk" | "content/delta" | "stream" => {
                    let matched_session_id = val.pointer("/params/sessionId")
                        .and_then(|s| s.as_str())
                        .and_then(|acp_sid| acp_session.get_local_session_id(acp_sid))
                        .or_else(|| acp_session.get_active_local_session())
                        .unwrap_or_else(|| active_session_id.to_string());

                    let update_type = val.pointer("/params/update/sessionUpdate").and_then(|s| s.as_str());
                    let mut delta = String::new();

                    // Check if this update represents tool execution progress
                    match update_type {
                        Some("tool_call") => {
                            let title = val.pointer("/params/update/title")
                                .or_else(|| val.pointer("/params/update/toolCallId"))
                                .and_then(|t| t.as_str())
                                .unwrap_or("tool");
                            let status = val.pointer("/params/update/status").and_then(|s| s.as_str()).unwrap_or("in_progress");
                            if status == "in_progress" || status == "pending" {
                                delta = format!("\n\n> ⚙️ **Running tool:** `{}`...\n", title);
                            }
                        }
                        Some("tool_call_update") => {
                            let title = val.pointer("/params/update/title")
                                .or_else(|| val.pointer("/params/update/toolCallId"))
                                .and_then(|t| t.as_str())
                                .unwrap_or("tool");
                            let status = val.pointer("/params/update/status").and_then(|s| s.as_str()).unwrap_or("completed");
                            if status == "completed" {
                                delta = format!("> ✅ **Completed:** `{}`\n\n", title);
                            } else if status == "failed" {
                                let err_msg = val.pointer("/params/update/content/0/content/text")
                                    .or_else(|| val.pointer("/params/update/content/text"))
                                    .and_then(|t| t.as_str())
                                    .unwrap_or("");
                                if !err_msg.is_empty() {
                                    delta = format!("> ❌ **Tool failed:** `{}`: {}\n\n", title, err_msg);
                                } else {
                                    delta = format!("> ❌ **Tool failed:** `{}`\n\n", title);
                                }
                            }
                        }
                        Some("current_mode_update") => {
                            if let Some(mode_id) = val.pointer("/params/update/currentModeId").and_then(|m| m.as_str()) {
                                let _ = app_handle.emit("acp-mode-update", serde_json::json!({
                                    "sessionId": matched_session_id,
                                    "mode": mode_id
                                }));
                            }
                        }
                        _ => {
                            // Extract text content from update or params
                            delta = val.pointer("/params/update")
                                .map(extract_acp_text)
                                .filter(|s| !s.is_empty())
                                .or_else(|| val.pointer("/params").map(extract_acp_text))
                                .unwrap_or_default();
                        }
                    }

                    // Tool execution updates must NEVER mark the turn as done.
                    let is_tool_update = matches!(
                        update_type,
                        Some("tool_call") | Some("tool_call_update")
                    );

                    let is_done = if is_tool_update {
                        false
                    } else {
                        val.pointer("/params/update/sessionUpdate")
                            .and_then(|s| s.as_str())
                            .map(|u| u == "turn_complete" || u == "prompt_complete")
                            .unwrap_or(false)
                            || val.pointer("/params/stopReason")
                                .and_then(|s| s.as_str())
                                .map(|s| !s.is_empty() && s != "null")
                                .unwrap_or(false)
                            || val.pointer("/params/update/stopReason")
                                .and_then(|s| s.as_str())
                                .map(|s| !s.is_empty() && s != "null")
                                .unwrap_or(false)
                    };

                    if is_done {
                        let stop_reason = val.pointer("/params/stopReason")
                            .or_else(|| val.pointer("/params/update/stopReason"))
                            .and_then(|s| s.as_str());
                        if matches!(stop_reason, Some("max_turn_requests") | Some("max_tokens")) {
                            acp_session.remove_session(&matched_session_id);
                        }
                    }

                    let (clean_delta, mode_opt) = sanitize_acp_delta(&delta);
                    if let Some(mode) = mode_opt {
                        let _ = app_handle.emit("acp-mode-update", serde_json::json!({
                            "sessionId": matched_session_id,
                            "mode": mode
                        }));
                    }

                    if clean_delta.is_empty() && !is_done {
                        return;
                    }

                    let _ = app_handle.emit("acp-chunk", StreamChunkPayload {
                        session_id: matched_session_id,
                        delta: clean_delta,
                        is_done,
                    });
                }
                // Interactive tool confirmation request
                "permission/request" | "session/permission_request" | "session/request_permission"
                | "session/requestPermission" | "request_permission" | "requestPermission" | "tool/confirm" => {
                    let req_id = val.get("id").and_then(|id| id.as_u64()).unwrap_or(0);
                    let session_id = val.pointer("/params/sessionId")
                        .and_then(|s| s.as_str())
                        .and_then(|acp_sid| acp_session.get_local_session_id(acp_sid))
                        .or_else(|| acp_session.get_active_local_session())
                        .unwrap_or_else(|| active_session_id.to_string());

                    let title = val.pointer("/params/toolCall/title")
                        .or_else(|| val.pointer("/params/title"))
                        .and_then(|t| t.as_str())
                        .map(|s| s.to_string());

                    let kind = val.pointer("/params/toolCall/kind")
                        .or_else(|| val.pointer("/params/kind"))
                        .and_then(|k| k.as_str())
                        .map(|s| s.to_string());

                    let tool_call_id = val.pointer("/params/toolCall/toolCallId")
                        .or_else(|| val.pointer("/params/toolCallId"))
                        .and_then(|id| id.as_str())
                        .map(|s| s.to_string());

                    let tool_name = val.pointer("/params/toolCall/name")
                        .or_else(|| val.pointer("/params/tool"))
                        .or_else(|| val.pointer("/params/name"))
                        .and_then(|t| t.as_str())
                        .map(|s| s.to_string())
                        .or_else(|| title.clone())
                        .or_else(|| kind.clone())
                        .unwrap_or_else(|| "Tool Execution".to_string());

                    let parameters = val.pointer("/params/toolCall/rawInput")
                        .or_else(|| val.pointer("/params/arguments"))
                        .or_else(|| val.pointer("/params/parameters"))
                        .cloned()
                        .unwrap_or(Value::Null);

                    let locations = val.pointer("/params/toolCall/locations")
                        .or_else(|| val.pointer("/params/locations"))
                        .cloned();

                    let content = val.pointer("/params/toolCall/content")
                        .or_else(|| val.pointer("/params/content"))
                        .cloned();

                    let reason = val.pointer("/params/reason")
                        .and_then(|r| r.as_str())
                        .map(|s| s.to_string());

                    let mut options = Vec::new();
                    if let Some(opts_array) = val.pointer("/params/options").and_then(|o| o.as_array()) {
                        for opt in opts_array {
                            if let (Some(opt_id), Some(name)) = (
                                opt.get("optionId").and_then(|s| s.as_str()),
                                opt.get("name").and_then(|s| s.as_str()),
                            ) {
                                let kind_str = opt.get("kind").and_then(|s| s.as_str()).unwrap_or("allow_once").to_string();
                                options.push(PermissionOption {
                                    option_id: opt_id.to_string(),
                                    name: name.to_string(),
                                    kind: kind_str,
                                });
                            }
                        }
                    }

                    if options.is_empty() {
                        options.push(PermissionOption {
                            option_id: "proceed_once".to_string(),
                            name: "Allow once".to_string(),
                            kind: "allow_once".to_string(),
                        });
                        options.push(PermissionOption {
                            option_id: "cancel".to_string(),
                            name: "Reject".to_string(),
                            kind: "reject_once".to_string(),
                        });
                    }

                    // Check active approval mode for this session
                    let current_mode = acp_session.get_session_mode(&session_id);

                    // 1. YOLO Mode: Backend immediately auto-approves over stdin (0ms latency, immune to WebView2 background throttling)
                    if current_mode == "yolo" {
                        let response_payload = build_permission_response_payload(true, None, Some(options.clone()));
                        let acp_clone = acp_session.clone();
                        tauri::async_runtime::spawn(async move {
                            let _ = acp_clone.send_response(req_id, response_payload).await;
                        });
                        let _ = app_handle.emit("acp-chunk", StreamChunkPayload {
                            session_id: session_id.clone(),
                            delta: format!("\n\n> 🚀 **[YOLO Auto-approved]** `{}`\n", tool_name),
                            is_done: false,
                        });
                        return;
                    }

                    // 2. Auto-Edit Mode: Auto-approve safe workspace inspection (reads, searches, listings) and file edits
                    if current_mode == "auto_edit" {
                        let is_safe = kind.as_deref() == Some("edit")
                            || kind.as_deref() == Some("read")
                            || kind.as_deref() == Some("search")
                            || kind.as_deref() == Some("think")
                            || kind.as_deref() == Some("fetch")
                            || {
                                let name_lower = tool_name.to_lowercase();
                                name_lower.contains("read")
                                    || name_lower.contains("write")
                                    || name_lower.contains("edit")
                                    || name_lower.contains("replace")
                                    || name_lower.contains("patch")
                                    || name_lower.contains("create")
                                    || name_lower.contains("list")
                                    || name_lower.contains("search")
                                    || name_lower.contains("grep")
                                    || name_lower.contains("view")
                                    || name_lower.contains("glob")
                                    || name_lower.contains("find")
                            };

                        if is_safe {
                            let response_payload = build_permission_response_payload(true, None, Some(options.clone()));
                            let acp_clone = acp_session.clone();
                            tauri::async_runtime::spawn(async move {
                                let _ = acp_clone.send_response(req_id, response_payload).await;
                            });
                            let _ = app_handle.emit("acp-chunk", StreamChunkPayload {
                                session_id: session_id.clone(),
                                delta: format!("\n\n> ⚡ **[Auto-approved]** `{}`\n", tool_name),
                                is_done: false,
                            });
                            return;
                        }
                    }

                    // 3. Ask Mode (default) or unsafe terminal execution: Present interactive confirmation UI
                    acp_session.store_permission_options(req_id, options.clone());

                    let _ = app_handle.emit("acp-tool-permission", ToolPermissionPayload {
                        request_id: req_id,
                        session_id,
                        tool_call_id,
                        tool_name,
                        title,
                        kind,
                        parameters,
                        locations,
                        content,
                        reason,
                        options,
                    });
                }
                _ => {
                    // Check if this incoming message is an Agent-to-Client Request (has an 'id')
                    // In YOLO mode or when ACP agents delegate filesystem/terminal actions,
                    // the agent expects a JSON-RPC response. If left unanswered or returned with an error, the CLI deadlocks or crashes!
                    if let Some(req_id) = val.get("id").and_then(|i| i.as_u64()) {
                        let acp_clone = acp_session.clone();
                        let val_clone = val.clone();
                        let method_str = method.to_string();
                        tauri::async_runtime::spawn(async move {
                            match method_str.as_str() {
                                "fs/read_text_file" => {
                                    let path_opt = val_clone.pointer("/params/path").and_then(|p| p.as_str());
                                    if let Some(p) = path_opt {
                                        let path_buf = if Path::new(p).is_relative() {
                                            if let Some(ws_dir) = acp_clone.get_workspace_dir() {
                                                ws_dir.join(p)
                                            } else {
                                                PathBuf::from(p)
                                            }
                                        } else {
                                            PathBuf::from(p)
                                        };

                                        match std::fs::read_to_string(&path_buf) {
                                            Ok(content) => {
                                                let _ = acp_clone.send_response(req_id, serde_json::json!({ "content": content })).await;
                                            }
                                            Err(e) => {
                                                // Per ACP spec and Gemini CLI normalizeFileSystemError, file-not-found must return
                                                // a standard JSON-RPC error containing "Resource not found" and "(ENOENT)"
                                                // so that the agent/CLI identifies that the file does not exist without throwing "content must be a string".
                                                let _ = acp_clone.send_error_response(
                                                    req_id,
                                                    -32002,
                                                    &format!("Resource not found: {} (ENOENT: {})", p, e),
                                                ).await;
                                            }
                                        }
                                    } else {
                                        let _ = acp_clone.send_error_response(
                                            req_id,
                                            -32602,
                                            "Invalid params: path must be a string (ENOENT)",
                                        ).await;
                                    }
                                }
                                "fs/write_text_file" => {
                                    let path_opt = val_clone.pointer("/params/path").and_then(|p| p.as_str());
                                    let content_opt = val_clone.pointer("/params/content").and_then(|c| c.as_str()).unwrap_or("");
                                    if let Some(p) = path_opt {
                                        let path_buf = if Path::new(p).is_relative() {
                                            if let Some(ws_dir) = acp_clone.get_workspace_dir() {
                                                ws_dir.join(p)
                                            } else {
                                                PathBuf::from(p)
                                            }
                                        } else {
                                            PathBuf::from(p)
                                        };
                                        if let Some(parent) = path_buf.parent() {
                                            let _ = std::fs::create_dir_all(parent);
                                        }
                                        match std::fs::write(&path_buf, content_opt) {
                                            Ok(_) => {
                                                let _ = acp_clone.send_response(req_id, serde_json::json!({})).await;
                                            }
                                            Err(e) => {
                                                let _ = acp_clone.send_error_response(
                                                    req_id,
                                                    -32000,
                                                    &format!("Failed to write file {}: {}", p, e),
                                                ).await;
                                            }
                                        }
                                    } else {
                                        let _ = acp_clone.send_error_response(req_id, -32602, "Missing file path").await;
                                    }
                                }
                                "terminal/create" => {
                                    let cmd = val_clone.pointer("/params/command").and_then(|c| c.as_str()).unwrap_or("");
                                    let args_val = val_clone.pointer("/params/args").and_then(|a| a.as_array());
                                    let mut full_cmd = cmd.to_string();
                                    if let Some(args) = args_val {
                                        for arg in args {
                                            if let Some(s) = arg.as_str() {
                                                full_cmd.push(' ');
                                                full_cmd.push_str(s);
                                            }
                                        }
                                    }

                                    let cwd = val_clone.pointer("/params/cwd")
                                        .and_then(|c| c.as_str())
                                        .map(PathBuf::from)
                                        .or_else(|| acp_clone.get_workspace_dir());

                                    let term_id = format!("term_{}", req_id);
                                    let mut command = Command::new("powershell");
                                    command.arg("-NoProfile").arg("-NonInteractive").arg("-Command").arg(&full_cmd);
                                    if let Some(dir) = cwd {
                                        command.current_dir(dir);
                                    }

                                    let output = command.output();
                                    let (stdout_str, stderr_str, exit_code) = match output {
                                        Ok(out) => {
                                            let out_str = String::from_utf8_lossy(&out.stdout).to_string();
                                            let err_str = String::from_utf8_lossy(&out.stderr).to_string();
                                            let code = out.status.code().unwrap_or(0);
                                            (out_str, err_str, code)
                                        }
                                        Err(e) => (String::new(), e.to_string(), 1),
                                    };

                                    let combined = if stderr_str.is_empty() {
                                        stdout_str
                                    } else if stdout_str.is_empty() {
                                        stderr_str
                                    } else {
                                        format!("{}\n{}", stdout_str, stderr_str)
                                    };

                                    acp_clone.store_terminal_result(&term_id, combined, exit_code);
                                    let _ = acp_clone.send_response(req_id, serde_json::json!({
                                        "terminalId": term_id
                                    })).await;
                                }
                                "terminal/output" => {
                                    let term_id = val_clone.pointer("/params/terminalId").and_then(|t| t.as_str()).unwrap_or("");
                                    let (output, exit_code) = acp_clone.get_terminal_result(term_id).unwrap_or((String::new(), 0));
                                    let _ = acp_clone.send_response(req_id, serde_json::json!({
                                        "output": output,
                                        "truncated": false,
                                        "exitStatus": {
                                            "code": exit_code
                                        }
                                    })).await;
                                }
                                "terminal/wait_for_exit" => {
                                    let term_id = val_clone.pointer("/params/terminalId").and_then(|t| t.as_str()).unwrap_or("");
                                    let (_, exit_code) = acp_clone.get_terminal_result(term_id).unwrap_or((String::new(), 0));
                                    let _ = acp_clone.send_response(req_id, serde_json::json!({
                                        "exitCode": exit_code
                                    })).await;
                                }
                                "terminal/release" | "terminal/kill" => {
                                    let term_id = val_clone.pointer("/params/terminalId").and_then(|t| t.as_str()).unwrap_or("");
                                    acp_clone.remove_terminal_result(term_id);
                                    let _ = acp_clone.send_response(req_id, serde_json::json!({})).await;
                                }
                                _ => {
                                    // Acknowledge immediately with an empty result so Gemini CLI async loop never hangs
                                    let _ = acp_clone.send_response(req_id, serde_json::json!({})).await;
                                }
                            }
                        });
                    }
                    // Forward generic notification
                    let _ = app_handle.emit("acp-notification", val);
                }
            }
            return;
        }

        // Check if response has a result containing text/content or explicit turn completion
        if val.pointer("/result/content").and_then(|v| if v.is_null() { None } else { Some(v) }).is_some()
            || val.pointer("/result/text").and_then(|v| if v.is_null() { None } else { Some(v) }).is_some()
            || val.pointer("/result/output").and_then(|v| if v.is_null() { None } else { Some(v) }).is_some()
            || val.pointer("/result/sessionId").and_then(|v| if v.is_null() { None } else { Some(v) }).is_some()
        {
            let matched_session_id = val.pointer("/result/sessionId")
                .and_then(|s| s.as_str())
                .and_then(|acp_sid| acp_session.get_local_session_id(acp_sid))
                .or_else(|| acp_session.get_active_local_session())
                .unwrap_or_else(|| active_session_id.to_string());

            let delta = val.get("result")
                .map(extract_acp_text)
                .unwrap_or_default();

            let (clean_delta, mode_opt) = sanitize_acp_delta(&delta);
            if let Some(mode) = mode_opt {
                let _ = app_handle.emit("acp-mode-update", serde_json::json!({
                    "sessionId": matched_session_id,
                    "mode": mode
                }));
            }

            // Intermediate tool execution results (like { result: { output: ... } }) must NEVER terminate the turn stream!
            // Only terminate turn streaming if an explicit valid stopReason is present in the result.
            let is_done = val.pointer("/result/stopReason")
                .and_then(|s| s.as_str())
                .map(|s| !s.is_empty() && s != "null")
                .unwrap_or(false);

            if is_done {
                let stop_reason = val.pointer("/result/stopReason").and_then(|s| s.as_str());
                if matches!(stop_reason, Some("max_turn_requests") | Some("max_tokens")) {
                    acp_session.remove_session(&matched_session_id);
                }
            }

            let _ = app_handle.emit("acp-chunk", StreamChunkPayload {
                session_id: matched_session_id,
                delta: clean_delta,
                is_done,
            });
            return;
        }

        if let Some(err_val) = val.get("error") {
            let code = err_val.get("code").and_then(|c| c.as_i64()).unwrap_or(0);
            let message = err_val.get("message").and_then(|m| m.as_str()).unwrap_or("");
            // Ignore benign cancellation errors (-32800 is standard JSON-RPC Request Cancelled, or method not found for cancel)
            if code == -32800 || message.to_lowercase().contains("cancel") {
                return;
            }
            let _ = app_handle.emit("acp-error", val);
            return;
        }
    }

    // Fallback: If CLI outputs raw streaming lines or debug logs, emit as text chunk
    let (clean_trimmed, mode_opt) = sanitize_acp_delta(trimmed);
    if let Some(mode) = mode_opt {
        let _ = app_handle.emit("acp-mode-update", serde_json::json!({
            "sessionId": active_session_id,
            "mode": mode
        }));
    }

    if clean_trimmed.is_empty() {
        return;
    }

    let _ = app_handle.emit("acp-chunk", StreamChunkPayload {
        session_id: active_session_id.to_string(),
        delta: format!("{}\n", clean_trimmed),
        is_done: false,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_acp_delta() {
        let chunk = "[MODE_UPDATE] autoEdit";
        let (cleaned, mode) = sanitize_acp_delta(chunk);
        assert_eq!(cleaned, "");
        assert_eq!(mode, Some("autoEdit".to_string()));

        let chunk_default = "[MODE_UPDATE] default";
        let (cleaned2, mode2) = sanitize_acp_delta(chunk_default);
        assert_eq!(cleaned2, "");
        assert_eq!(mode2, Some("default".to_string()));

        let normal_chunk = "I have created the file";
        let (cleaned3, mode3) = sanitize_acp_delta(normal_chunk);
        assert_eq!(cleaned3, "I have created the file");
        assert_eq!(mode3, None);

        let concatenated = "[MODE_UPDATE] autoEdit\nI have successfully created dummy.txt";
        let (cleaned4, mode4) = sanitize_acp_delta(concatenated);
        assert_eq!(cleaned4, "I have successfully created dummy.txt");
        assert_eq!(mode4, Some("autoEdit".to_string()));
    }

    #[test]
    fn test_prompt_session_tracking() {
        let session = AcpSession::new();
        session.register_prompt_request(42, "session-alpha");
        session.register_prompt_request(43, "session-beta");

        assert_eq!(session.take_prompt_session(42), Some("session-alpha".to_string()));
        // Once taken, request id is consumed
        assert_eq!(session.take_prompt_session(42), None);
        // Other sessions remain intact
        assert_eq!(session.take_prompt_session(43), Some("session-beta".to_string()));
    }

    #[test]
    fn test_session_mapping_and_cleanup() {
        let session = AcpSession::new();
        session.register_session_mapping("local-123", "acp-xyz");

        assert_eq!(session.get_acp_session_id("local-123"), Some("acp-xyz".to_string()));
        assert_eq!(session.get_local_session_id("acp-xyz"), Some("local-123".to_string()));

        session.remove_session("local-123");
        assert_eq!(session.get_acp_session_id("local-123"), None);
        assert_eq!(session.get_local_session_id("acp-xyz"), None);

        session.register_session_mapping("local-123", "acp-xyz");
        session.register_prompt_request(100, "local-123");
        session.clear_sessions();

        assert_eq!(session.get_acp_session_id("local-123"), None);
        assert_eq!(session.get_local_session_id("acp-xyz"), None);
        assert_eq!(session.take_prompt_session(100), None);
    }

    #[test]
    fn test_store_and_retrieve_permission_options() {
        let session = AcpSession::new();
        let options = vec![
            PermissionOption {
                option_id: "proceed_once".to_string(),
                name: "Allow".to_string(),
                kind: "allow_once".to_string(),
            },
            PermissionOption {
                option_id: "cancel".to_string(),
                name: "Reject".to_string(),
                kind: "reject_once".to_string(),
            },
        ];
        session.store_permission_options(101, options);
        let retrieved = session.pending_permissions.lock().unwrap().remove(&101);
        assert!(retrieved.is_some());
        let opts = retrieved.unwrap();
        assert_eq!(opts.len(), 2);
        assert_eq!(opts[0].option_id, "proceed_once");
        assert_eq!(opts[1].option_id, "cancel");
    }

    #[test]
    fn test_build_permission_response_payload() {
        // 1. Rejected outcome must follow discriminated union: { "outcome": { "outcome": "cancelled" } }
        let rejected = build_permission_response_payload(false, None, None);
        assert_eq!(
            rejected,
            serde_json::json!({
                "outcome": {
                    "outcome": "cancelled"
                }
            })
        );

        // 2. Allowed with explicit valid optionId
        let allowed_always = build_permission_response_payload(true, Some("proceed_always".to_string()), None);
        assert_eq!(
            allowed_always,
            serde_json::json!({
                "outcome": {
                    "outcome": "selected",
                    "optionId": "proceed_always"
                }
            })
        );

        // 3. Allowed with invalid/legacy optionId falls back to "proceed_once"
        let allowed_invalid = build_permission_response_payload(true, Some("allow-once".to_string()), None);
        assert_eq!(
            allowed_invalid,
            serde_json::json!({
                "outcome": {
                    "outcome": "selected",
                    "optionId": "proceed_once"
                }
            })
        );

        // 4. Allowed without optionId resolves from stored options
        let stored = vec![
            PermissionOption {
                option_id: "proceed_always".to_string(),
                name: "Always allow".to_string(),
                kind: "allow_always".to_string(),
            },
            PermissionOption {
                option_id: "cancel".to_string(),
                name: "Cancel".to_string(),
                kind: "reject_once".to_string(),
            },
        ];
        let allowed_from_stored = build_permission_response_payload(true, None, Some(stored));
        assert_eq!(
            allowed_from_stored,
            serde_json::json!({
                "outcome": {
                    "outcome": "selected",
                    "optionId": "proceed_always"
                }
            })
        );
    }

    #[test]
    fn test_session_mode_tracking() {
        let session = AcpSession::new();
        assert_eq!(session.get_session_mode("default-session"), "default");

        session.set_session_mode("s-1", "yolo");
        session.set_session_mode("s-2", "plan");

        assert_eq!(session.get_session_mode("s-1"), "yolo");
        assert_eq!(session.get_session_mode("s-2"), "plan");

        session.clear_sessions();
        assert_eq!(session.get_session_mode("s-1"), "plan"); // falls back to active_approval_mode
    }

    #[test]
    fn test_extract_acp_text() {
        // 1. Direct string
        assert_eq!(extract_acp_text(&serde_json::json!("Simple text")), "Simple text");

        // 2. Object with type: "text" and text
        let block = serde_json::json!({
            "type": "text",
            "text": "User story overview"
        });
        assert_eq!(extract_acp_text(&block), "User story overview");

        // 3. Array of content blocks
        let blocks = serde_json::json!([
            { "type": "text", "text": "Part 1: Background. " },
            { "type": "text", "text": "Part 2: Acceptance criteria." }
        ]);
        assert_eq!(extract_acp_text(&blocks), "Part 1: Background. Part 2: Acceptance criteria.");

        // 4. Session prompt result with content array
        let prompt_result = serde_json::json!({
            "stopReason": "end_turn",
            "content": [
                {
                    "type": "text",
                    "text": "## Summary of User Story\nHere are the details."
                }
            ]
        });
        assert_eq!(extract_acp_text(&prompt_result), "## Summary of User Story\nHere are the details.");

        // 5. Update notification with agent_message_chunk
        let update_chunk = serde_json::json!({
            "sessionUpdate": "agent_message_chunk",
            "content": {
                "type": "text",
                "text": "streaming token "
            }
        });
        assert_eq!(extract_acp_text(&update_chunk), "streaming token ");

        // 6. Tool update without text content
        let tool_update = serde_json::json!({
            "sessionUpdate": "tool_call_update",
            "status": "completed"
        });
        assert_eq!(extract_acp_text(&tool_update), "");
    }

    #[tokio::test]
    async fn test_send_request_and_prompt_when_disconnected() {
        let session = AcpSession::new();
        let res = session.send_request("test_method", serde_json::json!({})).await;
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), "CLI stdin is not connected");

        let prompt_res = session.send_prompt_request("local-sess-42", serde_json::json!({})).await;
        assert!(prompt_res.is_err());
        assert_eq!(prompt_res.unwrap_err(), "CLI stdin is not connected");
    }

    #[test]
    fn test_stop_reason_turn_exhaustion_cleanup() {
        let session = AcpSession::new();
        let stop_reasons = vec!["max_turn_requests", "max_tokens"];
        for reason in stop_reasons {
            session.register_session_mapping("local-session-1", "acp-session-1");
            assert_eq!(session.get_acp_session_id("local-session-1"), Some("acp-session-1".to_string()));
            if matches!(Some(reason), Some("max_turn_requests") | Some("max_tokens")) {
                session.remove_session("local-session-1");
            }
            assert_eq!(session.get_acp_session_id("local-session-1"), None);
            assert_eq!(session.get_local_session_id("acp-session-1"), None);
        }
    }

    #[test]
    fn test_stop_reason_is_done_evaluation() {
        // Intermediate tool result with output but no stopReason must NOT mark turn done
        let tool_result = serde_json::json!({
            "result": {
                "output": "Directory listed successfully"
            }
        });
        let is_done_tool = tool_result.pointer("/result/stopReason")
            .and_then(|s| s.as_str())
            .map(|s| !s.is_empty() && s != "null")
            .unwrap_or(false);
        assert!(!is_done_tool);

        // Result with explicit null stopReason must NOT mark turn done
        let null_stop_result = serde_json::json!({
            "result": {
                "stopReason": null,
                "text": "Partial output"
            }
        });
        let is_done_null = null_stop_result.pointer("/result/stopReason")
            .and_then(|s| s.as_str())
            .map(|s| !s.is_empty() && s != "null")
            .unwrap_or(false);
        assert!(!is_done_null);

        // Result with valid stopReason "end_turn" marks turn done
        let valid_stop_result = serde_json::json!({
            "result": {
                "stopReason": "end_turn",
                "text": "Finished response"
            }
        });
        let is_done_valid = valid_stop_result.pointer("/result/stopReason")
            .and_then(|s| s.as_str())
            .map(|s| !s.is_empty() && s != "null")
            .unwrap_or(false);
        assert!(is_done_valid);
    }

    #[test]
    fn test_workspace_dir_and_terminal_results() {
        let session = AcpSession::new();
        assert_eq!(session.get_workspace_dir(), None);

        let ws = PathBuf::from("C:\\TestWorkspace");
        session.set_workspace_dir(ws.clone());
        assert_eq!(session.get_workspace_dir(), Some(ws));

        assert_eq!(session.get_terminal_result("term_1"), None);
        session.store_terminal_result("term_1", "output text".to_string(), 0);
        assert_eq!(session.get_terminal_result("term_1"), Some(("output text".to_string(), 0)));

        session.remove_terminal_result("term_1");
        assert_eq!(session.get_terminal_result("term_1"), None);
    }

    #[test]
    fn test_fs_read_error_format_contains_enoent() {
        let fake_path = "summary1.txt";
        let fake_err = "The system cannot find the file specified.";
        let err_msg = format!("Resource not found: {} (ENOENT: {})", fake_path, fake_err);

        assert!(err_msg.contains("Resource not found"));
        assert!(err_msg.contains("ENOENT"));
    }
}

