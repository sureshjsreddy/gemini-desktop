use crate::acp_client::{handle_acp_line, StreamChunkPayload};
use crate::commands::AppState;
use crate::database::Message;
use chrono::Utc;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::Command;
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

/// Inspects prompt for git context tokens (@git:diff, @git:staged, @git:status)
/// and resolves real working tree git output if workspace_path is a git repository.
pub fn resolve_git_context(prompt: &str, workspace_path: Option<&std::path::Path>) -> String {
    if !prompt.contains("@git:diff") && !prompt.contains("@git:staged") && !prompt.contains("@git:status") {
        return prompt.to_string();
    }

    let mut expanded = prompt.to_string();

    let run_git = |args: &[&str]| -> Option<String> {
        let ws = workspace_path?;
        if !ws.exists() {
            return None;
        }
        let mut cmd = Command::new("git");
        cmd.args(args).current_dir(ws);

        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }

        let out = cmd.output().ok()?;
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
            Some(s)
        } else {
            None
        }
    };

    if expanded.contains("@git:diff") {
        let diff_content = match run_git(&["diff"]) {
            Some(d) if !d.is_empty() => format!("\n```diff\n{}\n```", d),
            Some(_) => "\n*(Working tree clean - no unstaged changes)*".to_string(),
            None => "\n*(Git diff unavailable or not a git repository)*".to_string(),
        };
        expanded = expanded.replace("@git:diff", &format!("\n[Context: Git Diff]{}\n", diff_content));
    }

    if expanded.contains("@git:staged") {
        let staged_content = match run_git(&["diff", "--cached"]) {
            Some(d) if !d.is_empty() => format!("\n```diff\n{}\n```", d),
            Some(_) => "\n*(No changes staged for commit)*".to_string(),
            None => "\n*(Git staged diff unavailable or not a git repository)*".to_string(),
        };
        expanded = expanded.replace("@git:staged", &format!("\n[Context: Git Staged Diff]{}\n", staged_content));
    }

    if expanded.contains("@git:status") {
        let status_content = match run_git(&["status", "--short", "--branch"]) {
            Some(s) if !s.is_empty() => format!("\n```text\n{}\n```", s),
            Some(_) => "\n*(Clean working tree)*".to_string(),
            None => "\n*(Git status unavailable or not a git repository)*".to_string(),
        };
        expanded = expanded.replace("@git:status", &format!("\n[Context: Git Status]{}\n", status_content));
    }

    expanded.trim().to_string()
}

/// Constructs CLI argument list including model selection, approval mode, and workspace flags.
pub fn build_gemini_extra_args(model: &str, approval_mode: &str) -> Vec<String> {
    let mut extra_args = Vec::new();
    let trimmed_model = model.trim();
    if !trimmed_model.is_empty() && trimmed_model != "auto" {
        extra_args.push("--model".to_string());
        extra_args.push(trimmed_model.to_string());
    }

    match approval_mode {
        "yolo" => {
            extra_args.push("--approval-mode".to_string());
            extra_args.push("yolo".to_string());
        }
        "auto_edit" => {
            extra_args.push("--approval-mode".to_string());
            extra_args.push("auto_edit".to_string());
        }
        "plan" => {
            extra_args.push("--approval-mode".to_string());
            extra_args.push("plan".to_string());
        }
        _ => {
            extra_args.push("--approval-mode".to_string());
            extra_args.push("default".to_string());
        }
    }

    extra_args.push("--skip-trust".to_string());
    extra_args
}

#[tauri::command]
pub async fn send_prompt(
    app: AppHandle,
    state: State<'_, AppState>,
    session_id: String,
    workspace_id: String,
    prompt: String,
    model: String,
    approval_mode: Option<String>,
) -> Result<u64, String> {
    let now = Utc::now().to_rfc3339();

    // 1. Save user prompt to local database
    let user_msg = Message {
        id: format!("msg-{}", Uuid::new_v4()),
        session_id: session_id.clone(),
        role: "user".to_string(),
        content: prompt.clone(),
        tool_calls_json: None,
        token_count: 0,
        created_at: now.clone(),
    };
    state.db.save_message(user_msg)?;

    // 2. Determine workspace path
    let workspaces = state.db.list_workspaces()?;
    let ws = workspaces.into_iter().find(|w| w.id == workspace_id);
    let ws_path = ws.as_ref().map(|w| PathBuf::from(&w.path));

    // Resolve effective approval mode and composite process key
    let effective_mode = match approval_mode.as_deref().unwrap_or("auto_edit") {
        "yolo" => "yolo",
        "default" => "default",
        "plan" => "plan",
        _ => "auto_edit",
    };
    let process_key = format!("{}::{}", workspace_id, effective_mode);

    // 3. Ensure CLI process is spawned and ACP connected
    let mut ws_guard = state.active_process_workspace.lock().await;
    let needs_spawn = match &*ws_guard {
        Some(current_key) => current_key != &process_key,
        None => true,
    };

    if needs_spawn {
        // 1. Terminate previous CLI process if still alive
        let mut child_guard = state.active_child.lock().await;
        if let Some(mut old_child) = child_guard.take() {
            let _ = old_child.kill();
            let _ = old_child.wait();
        }

        state.acp_session.clear_sessions();

        let gemini_bin = match crate::process_manager::find_gemini_executable() {
            Some(p) => p,
            None => {
                let assistant_id = format!("msg-{}", Uuid::new_v4());
                let fallback_text = format!(
                    "**[Offline / Mock Mode]**\n\nGemini CLI could not be located on your system PATH or npm global directories.\n\nPrompt received:\n> {}\n\nPlease install Gemini CLI (`npm install -g @google/gemini-cli` or `scoop install gemini-cli`) and ensure it is in your PATH.",
                    prompt
                );
                let assistant_msg = Message {
                    id: assistant_id,
                    session_id: session_id.clone(),
                    role: "assistant".to_string(),
                    content: fallback_text.clone(),
                    tool_calls_json: None,
                    token_count: 50,
                    created_at: Utc::now().to_rfc3339(),
                };
                let _ = state.db.save_message(assistant_msg);

                let _ = app.emit("acp-chunk", StreamChunkPayload {
                    session_id: session_id.clone(),
                    delta: fallback_text,
                    is_done: true,
                });
                return Ok(0);
            }
        };

        let extra_args = build_gemini_extra_args(&model, effective_mode);

        match state.supervisor.spawn_gemini(&gemini_bin, ws_path.clone(), &extra_args) {
            Ok(mut child) => {
                if let Some(stdin) = child.stdin.take() {
                    state.acp_session.set_stdin(Box::new(stdin)).await;
                }

                if let Some(stdout) = child.stdout.take() {
                    let app_clone = app.clone();
                    let session_clone = session_id.clone();
                    let acp_clone = state.acp_session.clone();
                    std::thread::spawn(move || {
                        let reader = BufReader::new(stdout);
                        for line in reader.lines() {
                            if let Ok(l) = line {
                                handle_acp_line(&l, &app_clone, &acp_clone, &session_clone);
                            }
                        }
                    });
                }

                if let Some(stderr) = child.stderr.take() {
                    let app_clone = app.clone();
                    std::thread::spawn(move || {
                        let reader = BufReader::new(stderr);
                        for line in reader.lines() {
                            if let Ok(l) = line {
                                let _ = app_clone.emit("acp-stderr", l);
                            }
                        }
                    });
                }

                *child_guard = Some(child);
                *ws_guard = Some(process_key);

                // Send initialize request (per ACP specification)
                let _ = state.acp_session.send_request_with_response("initialize", serde_json::json!({
                    "protocolVersion": 1,
                    "clientInfo": {
                        "name": "GeminiDesktop",
                        "version": env!("CARGO_PKG_VERSION")
                    }
                })).await;
            }
            Err(e) => {
                let assistant_id = format!("msg-{}", Uuid::new_v4());
                let fallback_text = format!(
                    "**[Offline / Mock Mode]**\n\nFailed to launch Gemini CLI at `{}` (`{}`).\n\nPrompt received:\n> {}\n\nPlease check permissions or verify your Gemini CLI installation.",
                    gemini_bin.display(), e, prompt
                );
                let assistant_msg = Message {
                    id: assistant_id,
                    session_id: session_id.clone(),
                    role: "assistant".to_string(),
                    content: fallback_text.clone(),
                    tool_calls_json: None,
                    token_count: 50,
                    created_at: Utc::now().to_rfc3339(),
                };
                let _ = state.db.save_message(assistant_msg);

                let _ = app.emit("acp-chunk", StreamChunkPayload {
                    session_id: session_id.clone(),
                    delta: fallback_text,
                    is_done: true,
                });
                return Ok(0);
            }
        }
    }

    // 4. Establish session context & send prompt over ACP
    let acp_session_id = match state.acp_session.get_acp_session_id(&session_id) {
        Some(id) => id,
        None => {
            let ws_path_str = ws.as_ref().map(|w| w.path.clone()).unwrap_or_else(|| ".".to_string());
            let new_session_params = serde_json::json!({
                "cwd": ws_path_str,
                "mcpServers": [],
            });

            let res = state.acp_session.send_request_with_response("session/new", new_session_params).await?;

            let assigned_id = res.get("sessionId")
                .and_then(|s| s.as_str())
                .ok_or_else(|| "ACP agent did not return a sessionId from session/new".to_string())?
                .to_string();

            state.acp_session.register_session_mapping(&session_id, &assigned_id);
            assigned_id
        }
    };

    state.acp_session.set_active_local_session(&session_id);

    // Explicitly set the active model on this session via ACP session/set_model
    let trimmed_model = model.trim();
    if !trimmed_model.is_empty() {
        let set_model_params = serde_json::json!({
            "sessionId": acp_session_id,
            "modelId": trimmed_model
        });
        let _ = state.acp_session.send_request_with_response("session/set_model", set_model_params).await;
    }

    // Explicitly set the active policy approval mode on this session via ACP session/set_mode
    let acp_mode_id = match effective_mode {
        "yolo" => "yolo",
        "default" => "default",
        "plan" => "plan",
        _ => "autoEdit",
    };
    let set_mode_params = serde_json::json!({
        "sessionId": acp_session_id,
        "modeId": acp_mode_id
    });
    let _ = state.acp_session.send_request_with_response("session/set_mode", set_mode_params).await;

    let final_prompt = resolve_git_context(&prompt, ws_path.as_deref());

    let prompt_params = serde_json::json!({
        "sessionId": acp_session_id,
        "prompt": [
            {
                "type": "text",
                "text": final_prompt
            }
        ]
    });

    let req_id = state.acp_session.send_request("session/prompt", prompt_params).await?;
    state.acp_session.register_prompt_request(req_id, &session_id);
    Ok(req_id)
}

#[tauri::command]
pub async fn cancel_prompt(
    state: State<'_, AppState>,
    request_id: Option<u64>,
    session_id: Option<String>,
) -> Result<(), String> {
    state.acp_session.send_cancel(session_id.as_deref(), request_id).await
}

#[tauri::command]
pub async fn respond_tool_permission(
    state: State<'_, AppState>,
    request_id: u64,
    option_id: Option<String>,
    allowed: Option<bool>,
) -> Result<(), String> {
    state.acp_session.respond_permission(request_id, option_id, allowed.unwrap_or(true)).await
}

#[tauri::command]
pub async fn restart_gemini_session(state: State<'_, AppState>) -> Result<String, String> {
    let mut child_guard = state.active_child.lock().await;
    if let Some(mut old_child) = child_guard.take() {
        let _ = old_child.kill();
        let _ = old_child.wait();
    }
    let mut ws_guard = state.active_process_workspace.lock().await;
    *ws_guard = None;
    state.acp_session.clear_sessions();
    Ok("Gemini CLI session reset successfully. Next prompt will launch with updated environment.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_git_context() {
        // Without tokens, returns input unchanged
        let raw = "Please explain the architecture";
        assert_eq!(resolve_git_context(raw, None), raw);

        // With @git:diff on None path
        let with_diff = "Review this: @git:diff";
        let resolved = resolve_git_context(with_diff, None);
        assert!(resolved.contains("[Context: Git Diff]"));

        // With @git:status
        let with_status = "Status: @git:status";
        let resolved_status = resolve_git_context(with_status, None);
        assert!(resolved_status.contains("[Context: Git Status]"));
    }

    #[test]
    fn test_resolve_git_context_multiple_tokens() {
        let prompt = "Please check: @git:diff and check status: @git:status for issues";
        let resolved = resolve_git_context(prompt, None);
        assert!(resolved.contains("[Context: Git Diff]"));
        assert!(resolved.contains("[Context: Git Status]"));
        assert!(resolved.contains("Please check:"));
        assert!(resolved.contains("and check status:"));
    }

    #[test]
    fn test_build_gemini_extra_args() {
        let args_auto_edit = build_gemini_extra_args("gemini-3.8-flash", "auto_edit");
        assert!(args_auto_edit.contains(&"--model".to_string()));
        assert!(args_auto_edit.contains(&"gemini-3.8-flash".to_string()));
        assert!(args_auto_edit.contains(&"--approval-mode".to_string()));
        assert!(args_auto_edit.contains(&"auto_edit".to_string()));
        assert!(args_auto_edit.contains(&"--skip-trust".to_string()));

        let args_yolo = build_gemini_extra_args("auto", "yolo");
        assert!(!args_yolo.contains(&"--model".to_string()));
        assert!(args_yolo.contains(&"--approval-mode".to_string()));
        assert!(args_yolo.contains(&"yolo".to_string()));
        assert!(args_yolo.contains(&"--skip-trust".to_string()));

        let args_default = build_gemini_extra_args("", "default");
        assert!(args_default.contains(&"--approval-mode".to_string()));
        assert!(args_default.contains(&"default".to_string()));

        let args_plan = build_gemini_extra_args("gemini-2.5-pro", "plan");
        assert!(args_plan.contains(&"--approval-mode".to_string()));
        assert!(args_plan.contains(&"plan".to_string()));
    }
}
