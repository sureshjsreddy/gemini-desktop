use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
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
        let chosen_option_id = if let Some(oid) = option_id {
            oid
        } else {
            let stored_options = self.pending_permissions.lock().ok().and_then(|mut m| m.remove(&request_id));
            if let Some(opts) = stored_options {
                if allowed {
                    opts.iter()
                        .find(|o| o.kind.contains("allow"))
                        .map(|o| o.option_id.clone())
                        .unwrap_or_else(|| "allow-once".to_string())
                } else {
                    opts.iter()
                        .find(|o| o.kind.contains("reject") || o.kind.contains("deny"))
                        .map(|o| o.option_id.clone())
                        .unwrap_or_else(|| "reject-once".to_string())
                }
            } else if allowed {
                "allow-once".to_string()
            } else {
                "reject-once".to_string()
            }
        };

        let result = serde_json::json!({
            "outcome": {
                "outcome": "selected",
                "optionId": chosen_option_id
            }
        });

        self.send_response(request_id, result).await
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

                let delta = val.pointer("/result/content/text")
                    .or_else(|| val.pointer("/result/content"))
                    .or_else(|| val.pointer("/result/text"))
                    .or_else(|| val.pointer("/result/output"))
                    .and_then(|t| t.as_str())
                    .unwrap_or("")
                    .to_string();

                let _ = app_handle.emit("acp-chunk", StreamChunkPayload {
                    session_id: target_session,
                    delta,
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

                    // Check standard ACP format (update.content.text or update.delta) as well as flat fields
                    let delta = val.pointer("/params/update/content/text")
                        .or_else(|| val.pointer("/params/update/text"))
                        .or_else(|| val.pointer("/params/update/delta"))
                        .or_else(|| val.pointer("/params/update/content"))
                        .or_else(|| val.pointer("/params/delta"))
                        .or_else(|| val.pointer("/params/content"))
                        .or_else(|| val.pointer("/params/text"))
                        .and_then(|t| t.as_str())
                        .unwrap_or("")
                        .to_string();

                    let is_done = val.pointer("/params/done")
                        .or_else(|| val.pointer("/params/update/done"))
                        .and_then(|d| d.as_bool())
                        .unwrap_or(false);

                    let _ = app_handle.emit("acp-chunk", StreamChunkPayload {
                        session_id: matched_session_id,
                        delta,
                        is_done,
                    });
                }
                // Interactive tool confirmation request
                "permission/request" | "session/permission_request" | "session/request_permission" | "tool/confirm" => {
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
                            option_id: "allow-once".to_string(),
                            name: "Allow once".to_string(),
                            kind: "allow_once".to_string(),
                        });
                        options.push(PermissionOption {
                            option_id: "reject-once".to_string(),
                            name: "Reject".to_string(),
                            kind: "reject_once".to_string(),
                        });
                    }

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
                    // Forward generic notification
                    let _ = app_handle.emit("acp-notification", val);
                }
            }
            return;
        }

        // Check if response has a result containing text/content or turn completion
        if val.get("result").is_some() {
            let matched_session_id = val.pointer("/result/sessionId")
                .and_then(|s| s.as_str())
                .and_then(|acp_sid| acp_session.get_local_session_id(acp_sid))
                .or_else(|| acp_session.get_active_local_session())
                .unwrap_or_else(|| active_session_id.to_string());

            let delta = val.pointer("/result/content/text")
                .or_else(|| val.pointer("/result/content"))
                .or_else(|| val.pointer("/result/text"))
                .or_else(|| val.pointer("/result/output"))
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string();

            let _ = app_handle.emit("acp-chunk", StreamChunkPayload {
                session_id: matched_session_id,
                delta,
                is_done: true,
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
    let _ = app_handle.emit("acp-chunk", StreamChunkPayload {
        session_id: active_session_id.to_string(),
        delta: format!("{}\n", trimmed),
        is_done: false,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

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

        session.register_prompt_request(100, "local-123");
        session.clear_sessions();

        assert_eq!(session.get_acp_session_id("local-123"), None);
        assert_eq!(session.get_local_session_id("acp-xyz"), None);
        assert_eq!(session.take_prompt_session(100), None);
    }
}
