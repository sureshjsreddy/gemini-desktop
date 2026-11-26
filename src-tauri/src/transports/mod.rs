use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Emitter};

/// Wire protocol spoken by a CLI backend engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TransportProtocol {
    /// Standard Agent Client Protocol JSON-RPC 2.0 over stdin/stdout (Gemini CLI, Antigravity CLI).
    Acp,
    /// Line-delimited JSON stream protocol (e.g. Claude Code `--output-format stream-json`).
    StreamJson,
    /// App-server JSON-RPC protocol (e.g. OpenAI Codex `codex app-server`).
    CodexRpc,
}

impl TransportProtocol {
    pub fn as_str(self) -> &'static str {
        match self {
            TransportProtocol::Acp => "acp",
            TransportProtocol::StreamJson => "stream-json",
            TransportProtocol::CodexRpc => "codex-rpc",
        }
    }
}

/// Runtime capabilities exposed by a transport layer.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransportCapabilities {
    pub supports_bidirectional_permissions: bool,
    pub supports_mid_stream_reset: bool,
    pub supports_concurrent_steering: bool,
    pub supports_resume: bool,
}

/// Canonical desktop events emitted to the Svelte frontend regardless of CLI wire format.
/// Future non-ACP CLI adapters translate their stdout stream into these normalized variants.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum NormalizedAgentEvent {
    Chunk {
        session_id: String,
        delta: String,
        is_done: bool,
        reset: bool,
    },
    ThoughtChunk {
        session_id: String,
        delta: String,
    },
    UsageUpdate {
        session_id: String,
        used_tokens: u64,
        context_window: u64,
        percentage: f64,
    },
    ModeUpdate {
        session_id: String,
        mode: String,
    },
    SessionInfoUpdate {
        session_id: String,
        title: String,
    },
    PlanUpdate {
        session_id: String,
        entries: Vec<Value>,
    },
}

/// Emits a normalized agent event onto the Tauri event bus using stable channel names.
#[allow(dead_code)]
pub fn emit_normalized_event(app_handle: &AppHandle, event: NormalizedAgentEvent) {
    match event {
        NormalizedAgentEvent::Chunk {
            session_id,
            delta,
            is_done,
            reset,
        } => {
            let _ = app_handle.emit(
                "acp-chunk",
                serde_json::json!({
                    "session_id": session_id,
                    "delta": delta,
                    "is_done": is_done,
                    "reset": reset,
                }),
            );
        }
        NormalizedAgentEvent::ThoughtChunk { session_id, delta } => {
            let _ = app_handle.emit(
                "acp-thought-chunk",
                serde_json::json!({
                    "sessionId": session_id,
                    "delta": delta,
                }),
            );
        }
        NormalizedAgentEvent::UsageUpdate {
            session_id,
            used_tokens,
            context_window,
            percentage,
        } => {
            let _ = app_handle.emit(
                "acp-usage-update",
                serde_json::json!({
                    "sessionId": session_id,
                    "usedTokens": used_tokens,
                    "contextWindow": context_window,
                    "percentage": percentage,
                }),
            );
        }
        NormalizedAgentEvent::ModeUpdate { session_id, mode } => {
            let _ = app_handle.emit(
                "acp-mode-update",
                serde_json::json!({
                    "sessionId": session_id,
                    "mode": mode,
                }),
            );
        }
        NormalizedAgentEvent::SessionInfoUpdate { session_id, title } => {
            let _ = app_handle.emit(
                "acp-session-info-update",
                serde_json::json!({
                    "sessionId": session_id,
                    "title": title,
                }),
            );
        }
        NormalizedAgentEvent::PlanUpdate {
            session_id,
            entries,
        } => {
            let _ = app_handle.emit(
                "acp-plan-update",
                serde_json::json!({
                    "sessionId": session_id,
                    "entries": entries,
                }),
            );
        }
    }
}

/// Seam between the desktop UI/session coordinator and wire-specific CLI stream handlers.
#[allow(dead_code)]
pub trait AgentTransport: Send + Sync {
    fn protocol(&self) -> TransportProtocol;
    fn capabilities(&self) -> TransportCapabilities;
}

/// Standard ACP JSON-RPC 2.0 stdio transport shared by Gemini CLI and Antigravity CLI.
#[allow(dead_code)]
pub struct AcpTransport;

impl AgentTransport for AcpTransport {
    fn protocol(&self) -> TransportProtocol {
        TransportProtocol::Acp
    }

    fn capabilities(&self) -> TransportCapabilities {
        TransportCapabilities {
            supports_bidirectional_permissions: true,
            supports_mid_stream_reset: true,
            supports_concurrent_steering: false,
            supports_resume: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transport_protocol_strings() {
        assert_eq!(TransportProtocol::Acp.as_str(), "acp");
        assert_eq!(TransportProtocol::StreamJson.as_str(), "stream-json");
        assert_eq!(TransportProtocol::CodexRpc.as_str(), "codex-rpc");
    }

    #[test]
    fn test_acp_transport_contract_and_capabilities() {
        let transport = AcpTransport;
        assert_eq!(transport.protocol(), TransportProtocol::Acp);
        let caps = transport.capabilities();
        assert!(caps.supports_bidirectional_permissions);
        assert!(caps.supports_mid_stream_reset);
        assert!(!caps.supports_concurrent_steering);
        assert!(caps.supports_resume);
    }
}
