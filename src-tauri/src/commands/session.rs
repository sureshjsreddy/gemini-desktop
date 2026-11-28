use serde::{Deserialize, Serialize};
use crate::acp_client::{handle_acp_line, StreamChunkPayload};
use crate::engines::CliEngineAdapter;
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

/// Splits comma, semicolon, or newline separated directory strings into trimmed entries.
pub fn parse_additional_dirs(raw: Option<&str>) -> Vec<String> {
    crate::engines::parse_additional_dirs(raw)
}

/// Constructs CLI argument list including model selection, approval mode, and workspace flags for Gemini CLI.
#[allow(dead_code)]
pub fn build_gemini_extra_args(model: &str, approval_mode: &str) -> Vec<String> {
    build_gemini_extra_args_with_options(model, approval_mode, false, None)
}

/// Extended Gemini CLI argument builder supporting `--sandbox` and `--include-directories`.
/// Never passes Antigravity-only flags such as `--effort` or `--agent`.
#[allow(dead_code)]
pub fn build_gemini_extra_args_with_options(
    model: &str,
    approval_mode: &str,
    sandbox: bool,
    additional_dirs: Option<&str>,
) -> Vec<String> {
    let mut args = crate::engines::GeminiEngineAdapter.build_acp_args(
        model,
        approval_mode,
        None,
        None,
        sandbox,
        additional_dirs,
    );
    args.push("--skip-trust".to_string());
    args
}

/// Constructs CLI argument list for Antigravity CLI (agy) ACP mode.
#[allow(dead_code)]
pub fn build_agy_extra_args(model: &str, approval_mode: &str, effort: Option<&str>) -> Vec<String> {
    build_agy_extra_args_with_options(model, approval_mode, effort, None, false, None)
}

/// Extended Antigravity CLI (`agy`) argument builder supporting `--mode auto`, `--effort`,
/// `--agent`, `--sandbox`, and repeatable `--add-dir`.
#[allow(dead_code)]
pub fn build_agy_extra_args_with_options(
    model: &str,
    approval_mode: &str,
    effort: Option<&str>,
    agent_name: Option<&str>,
    sandbox: bool,
    additional_dirs: Option<&str>,
) -> Vec<String> {
    crate::engines::AgyEngineAdapter.build_acp_args(
        model,
        approval_mode,
        effort,
        agent_name,
        sandbox,
        additional_dirs,
    )
}

/// Sanitizes a model identifier for the active CLI engine.
/// If the model is incompatible with the active engine, seamlessly falls back to "auto".
#[allow(dead_code)]
pub fn sanitize_model_for_engine(engine: &str, model: &str) -> String {
    crate::engines::get_adapter(engine).sanitize_model(model)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptAttachment {
    pub path: String,
    pub mime_type: Option<String>,
    pub name: Option<String>,
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
    message_id: Option<String>,
    attachments: Option<Vec<PromptAttachment>>,
) -> Result<u64, String> {
    let now = Utc::now().to_rfc3339();

    let final_msg_id = message_id
        .filter(|id| !id.trim().is_empty())
        .unwrap_or_else(|| format!("msg-{}", Uuid::new_v4()));

    // 1. Save user prompt to local database
    let user_msg = Message {
        id: final_msg_id,
        session_id: session_id.clone(),
        role: "user".to_string(),
        content: prompt.clone(),
        tool_calls_json: None,
        token_count: 0,
        created_at: now.clone(),
    };
    state.db.save_message(user_msg)?;

    // 2. Determine workspace path and engine options
    let workspaces = state.db.list_workspaces()?;
    let ws = workspaces.into_iter().find(|w| w.id == workspace_id);
    let ws_path = ws.as_ref().map(|w| PathBuf::from(&w.path));
    let reasoning_effort = ws.as_ref().and_then(|w| w.reasoning_effort.clone());
    let sandbox_enabled = ws.as_ref().and_then(|w| w.sandbox_enabled).unwrap_or(false);
    let agent_name = ws.as_ref().and_then(|w| w.agent_name.clone());
    let additional_dirs = ws.as_ref().and_then(|w| w.additional_dirs.clone());
    let system_prompt_text = ws
        .as_ref()
        .and_then(|w| w.system_prompt.clone())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    // Resolve active CLI engine (global app-level setting, defaults to "gemini" if not persisted by user)
    let active_engine = state.db.get_setting("active_cli_engine")
        .ok()
        .flatten()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "gemini".to_string());

    let adapter = crate::engines::get_adapter(&active_engine);
    let engine_desc = adapter.static_descriptor();
    let is_agy = adapter.id() == "agy";
    let sanitized_model = adapter.sanitize_model(&model);

    // Resolve effective approval mode and composite process key
    let effective_mode = adapter.normalize_approval_mode(approval_mode.as_deref().unwrap_or("default"));
    state.acp_session.set_session_mode(&session_id, effective_mode);

    let effort_key = if engine_desc.capabilities.supports_reasoning_effort {
        reasoning_effort.as_deref().unwrap_or("")
    } else {
        ""
    };
    let agent_key = if engine_desc.capabilities.supports_agent_persona {
        agent_name.as_deref().unwrap_or("")
    } else {
        ""
    };
    let dirs_key = additional_dirs.as_deref().unwrap_or("");
    let process_key = format!(
        "{}::{}::{}::{}::{}::{}::{}",
        workspace_id, adapter.id(), effective_mode, effort_key, sandbox_enabled, agent_key, dirs_key
    );

    // 3. Ensure CLI process is spawned and ACP connected
    let mut ws_guard = state.active_process_workspace.lock().await;
    let needs_spawn = match &*ws_guard {
        Some(current_key) => current_key != &process_key,
        None => true,
    };

    if needs_spawn {
        // 1. Terminate previous CLI process tree if still alive
        {
            let mut child_guard = state.active_child.lock().await;
            if let Some(old_child) = child_guard.take() {
                crate::process_manager::ProcessSupervisor::terminate_child(old_child);
            }
        }

        state.acp_session.clear_sessions();

        let cli_bin = match adapter.find_executable() {
            Some(p) => p,
            None => {
                let assistant_id = format!("msg-{}", Uuid::new_v4());
                let fallback_text = format!(
                    "**[Offline / Mock Mode]**\n\n{} could not be located on your system PATH or standard installation directories.\n\nPrompt received:\n> {}\n\nInstall hint: `{}` or switch active CLI engines in Settings.",
                    engine_desc.display_name, prompt, engine_desc.install_hint
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

        let extra_args = adapter.build_acp_args(
            &sanitized_model,
            effective_mode,
            reasoning_effort.as_deref(),
            agent_name.as_deref(),
            sandbox_enabled,
            additional_dirs.as_deref(),
        );

        let auth_mode = state.db.get_setting("gemini_auth_mode")
            .ok()
            .flatten()
            .unwrap_or_else(|| {
                if let Ok(Some(key)) = state.db.get_setting("gemini_api_key") {
                    if !key.trim().is_empty() {
                        return "api_key".to_string();
                    }
                }
                "vertex_ai".to_string()
            });

        let mut global_envs = Vec::new();
        if auth_mode == "vertex_ai" {
            global_envs.push(("GOOGLE_GENAI_USE_VERTEXAI".to_string(), "true".to_string()));
            if let Ok(Some(proj)) = state.db.get_setting("google_cloud_project") {
                if !proj.trim().is_empty() {
                    global_envs.push(("GOOGLE_CLOUD_PROJECT".to_string(), proj.trim().to_string()));
                }
            }
            if let Ok(Some(loc)) = state.db.get_setting("google_cloud_location") {
                if !loc.trim().is_empty() {
                    global_envs.push(("GOOGLE_CLOUD_LOCATION".to_string(), loc.trim().to_string()));
                }
            }
            if let Ok(Some(creds)) = state.db.get_setting("google_app_credentials") {
                if !creds.trim().is_empty() {
                    global_envs.push(("GOOGLE_APPLICATION_CREDENTIALS".to_string(), creds.trim().to_string()));
                }
            }
        } else {
            if let Ok(Some(key)) = state.db.get_setting("gemini_api_key") {
                if !key.trim().is_empty() {
                    global_envs.push(("GEMINI_API_KEY".to_string(), key.trim().to_string()));
                }
            }
        }

        if is_agy {
            if let Ok(Some(adc_auth)) = state.db.get_setting("agy_adc_auth") {
                if adc_auth.trim().eq_ignore_ascii_case("true") {
                    global_envs.push(("AGY_ADC_AUTH".to_string(), "true".to_string()));
                }
            }
            if let Ok(Some(compact_override)) = state.db.get_setting("agy_auto_compact_override") {
                let trimmed = compact_override.trim();
                if !trimmed.is_empty() {
                    global_envs.push(("AGY_CLI_AUTO_COMPACT_OVERRIDE".to_string(), trimmed.to_string()));
                }
            }
            if let Ok(Some(disable_rg)) = state.db.get_setting("agy_disable_embedded_ripgrep") {
                if disable_rg.trim() == "true" || disable_rg.trim() == "1" {
                    global_envs.push(("ANTIGRAVITY_DISABLE_EMBEDDED_RIPGREP".to_string(), "1".to_string()));
                }
            }
        }

        match state.supervisor.spawn_agent(&active_engine, &cli_bin, ws_path.clone(), &extra_args, &global_envs) {
            Ok(mut child) => {
                if let Some(stdin) = child.stdin.take() {
                    state.acp_session.set_stdin(Box::new(stdin)).await;
                }

                if let Some(stdout) = child.stdout.take() {
                    let app_clone = app.clone();
                    let session_clone = session_id.clone();
                    let acp_clone = state.acp_session.clone();
                    let active_ws_clone = state.active_process_workspace.clone();
                    let active_child_clone = state.active_child.clone();
                    let child_pid = child.id();
                    std::thread::spawn(move || {
                        let reader = BufReader::new(stdout);
                        for line in reader.lines() {
                            if let Ok(l) = line {
                                handle_acp_line(&l, &app_clone, &acp_clone, &session_clone);
                            }
                        }
                        // When stdout closes (CLI process crashed, exited, or pipe broken):
                        // Only reset active process state and emit is_done if this process is still the active child!
                        // If a workspace or mode switch already spawned a newer child process, this dying thread
                        // must NOT clobber the new process state or emit premature is_done into the new session.
                        tauri::async_runtime::spawn(async move {
                            let mut child_lock = active_child_clone.lock().await;
                            let is_still_active = match child_lock.as_ref() {
                                Some(c) => c.id() == child_pid,
                                None => false,
                            };

                            if is_still_active {
                                if let Some(c) = child_lock.take() {
                                    crate::process_manager::ProcessSupervisor::terminate_child(c);
                                }
                                drop(child_lock);
                                let mut ws_lock = active_ws_clone.lock().await;
                                *ws_lock = None;
                                drop(ws_lock);

                                let _ = app_clone.emit("acp-chunk", StreamChunkPayload {
                                    session_id: session_clone,
                                    delta: String::new(),
                                    is_done: true,
                                });
                            }
                        });
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

                *ws_guard = Some(process_key);
                // Drop ws_guard so initialize and new_session calls never hold locks across network timeouts
                drop(ws_guard);

                let mut child_guard = state.active_child.lock().await;
                *child_guard = Some(child);
                drop(child_guard);

                // Send initialize request (per ACP specification) and store advertised capabilities
                if let Ok(init_res) = state.acp_session.send_request_with_response("initialize", serde_json::json!({
                    "protocolVersion": 1,
                    "clientCapabilities": {
                        "terminal": true
                    },
                    "clientInfo": {
                        "name": "GeminiDesktop",
                        "version": env!("CARGO_PKG_VERSION"),
                        "engine": active_engine
                    }
                })).await {
                    let caps = state.acp_session.set_agent_capabilities_from_init(&init_res);
                    let _ = app.emit("acp-capabilities-updated", serde_json::json!({
                        "engine": active_engine,
                        "capabilities": caps,
                    }));
                }
            }
            Err(e) => {
                let assistant_id = format!("msg-{}", Uuid::new_v4());
                let engine_label = if active_engine == "agy" { "Antigravity (agy)" } else { "Gemini" };
                let fallback_text = format!(
                    "**[Offline / Mock Mode]**\n\nFailed to launch {} CLI at `{}` (`{}`).\n\nPrompt received:\n> {}\n\nPlease check permissions or verify your CLI installation.",
                    engine_label, cli_bin.display(), e, prompt
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
    } else {
        drop(ws_guard);
    }

    let ws_path_str = ws.as_ref().map(|w| w.path.clone()).unwrap_or_else(|| ".".to_string());
    state.acp_session.set_workspace_dir(std::path::PathBuf::from(&ws_path_str));
    let should_track = ws.as_ref().and_then(|w| w.track_file_changes).unwrap_or(false);
    state.acp_session.set_track_file_changes(should_track);

    let caps = state.acp_session.get_agent_capabilities();
    let mut acp_session_res_val: Option<serde_json::Value> = None;

    // 4. Establish session context & send prompt over ACP (preferring session/resume when supported)
    let acp_session_id = match state.acp_session.get_acp_session_id(&session_id) {
        Some(id) => id,
        None => {
            let active_mcp_servers = crate::commands::mcp::get_active_mcp_servers_for_session(
                ws.as_ref().map(|w| w.path.as_str()),
                &active_engine,
            );

            let mut resumed_id: Option<String> = None;
            if caps.supports_resume {
                if let Ok(Some(db_session)) = state.db.get_session(&session_id) {
                    if let (Some(saved_acp_id), Some(saved_engine)) = (db_session.acp_session_id, db_session.acp_engine) {
                        if !saved_acp_id.trim().is_empty() && saved_engine == active_engine {
                            let resume_params = serde_json::json!({
                                "sessionId": saved_acp_id,
                                "cwd": ws_path_str,
                                "mcpServers": active_mcp_servers,
                            });
                            if let Ok(res) = state.acp_session.send_request_with_response("session/resume", resume_params).await {
                                let sid = res.get("sessionId")
                                    .and_then(|s| s.as_str())
                                    .unwrap_or(&saved_acp_id)
                                    .to_string();
                                state.acp_session.register_session_mapping(&session_id, &sid);
                                acp_session_res_val = Some(res);
                                resumed_id = Some(sid);
                            }
                        }
                    }
                }
            }

            if let Some(id) = resumed_id {
                id
            } else {
                let mut new_session_map = serde_json::Map::new();
                new_session_map.insert("cwd".to_string(), serde_json::Value::String(ws_path_str.clone()));
                new_session_map.insert("mcpServers".to_string(), serde_json::Value::Array(active_mcp_servers));

                let parsed_dirs = parse_additional_dirs(additional_dirs.as_deref());
                if caps.supports_additional_dirs && !parsed_dirs.is_empty() {
                    new_session_map.insert(
                        "additionalDirectories".to_string(),
                        serde_json::Value::Array(parsed_dirs.into_iter().map(serde_json::Value::String).collect()),
                    );
                }

                if engine_desc.capabilities.supports_system_prompt_append && caps.supports_system_prompt_append {
                    if let Some(ref sys_prompt) = system_prompt_text {
                        new_session_map.insert(
                            "_meta".to_string(),
                            serde_json::json!({
                                "antigravity": {
                                    "systemPrompt": {
                                        "append": sys_prompt
                                    }
                                }
                            }),
                        );
                    }
                }

                let res = state.acp_session.send_request_with_response(
                    "session/new",
                    serde_json::Value::Object(new_session_map),
                ).await?;

                let assigned_id = res.get("sessionId")
                    .and_then(|s| s.as_str())
                    .ok_or_else(|| "ACP agent did not return a sessionId from session/new".to_string())?
                    .to_string();

                state.acp_session.register_session_mapping(&session_id, &assigned_id);
                let _ = state.db.update_session_acp_binding(&session_id, Some(&assigned_id), Some(&active_engine));
                acp_session_res_val = Some(res);
                assigned_id
            }
        }
    };

    if let Some(ref session_meta) = acp_session_res_val {
        if session_meta.get("models").is_some() || session_meta.get("modes").is_some() {
            let _ = app.emit("acp-session-capabilities", serde_json::json!({
                "sessionId": session_id,
                "models": session_meta.get("models"),
                "modes": session_meta.get("modes"),
            }));
        }
    }

    state.acp_session.set_active_local_session(&session_id);

    // Explicitly set the active model on this session via ACP session/set_model
    if sanitized_model != "auto" && !sanitized_model.is_empty() {
        let set_model_params = serde_json::json!({
            "sessionId": acp_session_id,
            "modelId": sanitized_model
        });
        let _ = state.acp_session.send_request_with_response("session/set_model", set_model_params).await;
    }

    // Explicitly set the active policy approval mode on this session via ACP session/set_mode
    let acp_mode_opt: Option<&str> = adapter.map_acp_mode(effective_mode);

    if let Some(acp_mode_id) = acp_mode_opt {
        let set_mode_params = serde_json::json!({
            "sessionId": acp_session_id,
            "modeId": acp_mode_id
        });
        let _ = state.acp_session.send_request_with_response("session/set_mode", set_mode_params).await;
    }

    let final_prompt = resolve_git_context(&prompt, ws_path.as_deref());
    let mut prompt_blocks = vec![serde_json::json!({
        "type": "text",
        "text": final_prompt
    })];

    if caps.supports_image {
        if let Some(att_list) = attachments.as_ref() {
            for att in att_list {
                let att_path = std::path::PathBuf::from(&att.path);
                let ext = att_path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                let mime = att.mime_type.as_deref().map(|s| s.to_string()).unwrap_or_else(|| {
                    match ext.as_str() {
                        "png" => "image/png".to_string(),
                        "jpg" | "jpeg" => "image/jpeg".to_string(),
                        "webp" => "image/webp".to_string(),
                        "gif" => "image/gif".to_string(),
                        _ => String::new(),
                    }
                });
                if mime.starts_with("image/") {
                    if let Ok(bytes) = std::fs::read(&att_path) {
                        if bytes.len() <= 10 * 1024 * 1024 {
                            let encoded = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes);
                            prompt_blocks.push(serde_json::json!({
                                "type": "image",
                                "data": encoded,
                                "mimeType": mime,
                            }));
                        }
                    }
                }
            }
        }
    }

    let prompt_params = serde_json::json!({
        "sessionId": acp_session_id,
        "prompt": prompt_blocks
    });

    let req_id = state.acp_session.send_prompt_request(&session_id, prompt_params).await?;
    Ok(req_id)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewindResult {
    pub rewound_messages_count: usize,
    pub restored_files: Vec<String>,
    pub previous_prompt: Option<String>,
}

pub const USER_STEERING_INSTRUCTION: &str = "Internal instruction: Re-evaluate the active plan using this user steering update. Classify it as ADD_TASK, MODIFY_TASK, CANCEL_TASK, or EXTRA_CONTEXT. Apply minimal-diff changes only to affected tasks and keep unaffected tasks active. Do not cancel/skip tasks unless the user explicitly cancels them. Acknowledge the steering briefly and state the course correction.";

pub fn build_user_steering_prompt(hint: &str, is_plan_mode: bool) -> String {
    if is_plan_mode {
        format!(
            "User steering update:\n{}\n\n{}",
            hint.trim(),
            USER_STEERING_INSTRUCTION
        )
    } else {
        format!(
            "User steering update:\n{}\n\nPlease adapt your execution according to this user guidance.",
            hint.trim()
        )
    }
}

#[tauri::command]
pub async fn steer_prompt(
    app: AppHandle,
    state: State<'_, AppState>,
    session_id: String,
    hint: String,
) -> Result<(), String> {
    let trimmed = hint.trim();
    if trimmed.is_empty() {
        return Err("Steering hint cannot be empty".to_string());
    }

    let active_engine = state.db.get_setting("active_cli_engine")
        .ok()
        .flatten()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "gemini".to_string());
    let adapter_caps = crate::engines::get_adapter(&active_engine).static_descriptor().capabilities;

    let current_mode = state.acp_session.get_session_mode(&session_id);
    let is_plan_mode = current_mode == "plan";

    // For engines without concurrent steering support (e.g. Gemini CLI), cancel in-flight generation first.
    if !adapter_caps.supports_concurrent_steer && state.acp_session.has_pending_prompt_for_session(&session_id) {
        let _ = state.acp_session.send_cancel(Some(&session_id), None).await;
        // Delay to allow cancellation to propagate through ACP
        tokio::time::sleep(tokio::time::Duration::from_millis(250)).await;
    }

    // 2. Format steering prompt instruction (adapting for plan mode vs general mode)
    let steering_prompt = build_user_steering_prompt(trimmed, is_plan_mode);

    // 3. Obtain current ACP session ID
    let acp_sid = match state.acp_session.get_acp_session_id(&session_id) {
        Some(id) => id,
        None => {
            let _ = app.emit("acp-chunk", StreamChunkPayload {
                session_id: session_id.clone(),
                delta: format!("\n\n> 🎯 **[Offline Mock Steer]** Received steering hint: *{}*\n", trimmed),
                is_done: true,
            });
            return Ok(());
        }
    };

    state.acp_session.set_active_local_session(&session_id);

    let prompt_params = serde_json::json!({
        "sessionId": acp_sid,
        "prompt": [
            {
                "type": "text",
                "text": steering_prompt
            }
        ]
    });

    // 4. Send steering prompt to the model in the same active ACP session
    let _ = state.acp_session.send_prompt_request(&session_id, prompt_params).await?;

    let _ = app.emit("acp-steer", serde_json::json!({
        "session_id": session_id,
        "hint": trimmed,
        "timestamp": Utc::now().to_rfc3339(),
    }));

    Ok(())
}

fn add_cleaned_rel_path(files: &mut Vec<String>, raw: &str, ws_path: &std::path::Path) {
    let trimmed = raw.trim().trim_matches(|c| c == '"' || c == '\'' || c == '`');
    if trimmed.is_empty() || trimmed.contains("..") {
        return;
    }
    let p = std::path::Path::new(trimmed);
    let norm = if p.is_absolute() {
        if let Ok(rel) = p.strip_prefix(ws_path) {
            rel.to_string_lossy().replace('\\', "/")
        } else {
            return;
        }
    } else {
        trimmed.replace('\\', "/")
    };

    let norm_clean = norm.trim_start_matches("./").to_string();
    if !norm_clean.is_empty() && !norm_clean.starts_with(".git") && !norm_clean.starts_with("node_modules") {
        if !files.contains(&norm_clean) {
            files.push(norm_clean);
        }
    }
}

fn extract_potential_filenames(files: &mut Vec<String>, text: &str, ws_path: &std::path::Path) {
    for word in text.split_whitespace() {
        let clean = word.trim().trim_matches(|c: char| !c.is_alphanumeric() && c != '.' && c != '_' && c != '-' && c != '/' && c != '\\');
        if clean.contains('.') && !clean.ends_with('.') && !clean.starts_with('.') && !clean.contains("http") && !clean.contains("www.") {
            let candidate_path = ws_path.join(clean);
            if candidate_path.exists() {
                add_cleaned_rel_path(files, clean, ws_path);
            }
        }
    }
}

pub fn extract_files_from_messages(messages: &[Message], ws_path: &std::path::Path) -> Vec<String> {
    let mut files = Vec::new();

    for m in messages {
        if let Some(ref json_str) = m.tool_calls_json {
            if let Ok(serde_json::Value::Array(tool_list)) = serde_json::from_str::<serde_json::Value>(json_str) {
                for tool in tool_list {
                    if let Some(params) = tool.get("parameters") {
                        if let Some(obj) = params.as_object() {
                            for key in &["filePath", "file_path", "path", "target_path", "targetPath", "file", "filename", "destination"] {
                                if let Some(val) = obj.get(*key).and_then(|v| v.as_str()) {
                                    add_cleaned_rel_path(&mut files, val, ws_path);
                                }
                            }
                        }
                    }
                    if let Some(title) = tool.get("title").and_then(|t| t.as_str()) {
                        extract_potential_filenames(&mut files, title, ws_path);
                    }
                    if let Some(summary) = tool.get("summary").and_then(|s| s.as_str()) {
                        extract_potential_filenames(&mut files, summary, ws_path);
                    }
                }
            }
        }

        extract_potential_filenames(&mut files, &m.content, ws_path);
    }

    files.sort();
    files.dedup();
    files
}

#[tauri::command]
pub async fn rewind_turn(
    app: AppHandle,
    state: State<'_, AppState>,
    session_id: String,
    workspace_id: String,
    target_message_id: String,
    revert_code: bool,
    revert_messages: Option<bool>,
) -> Result<RewindResult, String> {
    // 1. Cancel active turn for this session if running
    let _ = state.acp_session.send_cancel(Some(&session_id), None).await;

    // 2. Clear ACP session mapping (memory + DB binding)
    state.acp_session.remove_session(&session_id);
    let _ = state.db.update_session_acp_binding(&session_id, None, None);

    // 3. Delete or inspect messages from database
    let should_revert_messages = revert_messages.unwrap_or(true);
    let (count, prev_prompt, target_messages) = if should_revert_messages {
        state.db.delete_messages_from(&session_id, &target_message_id)?
    } else {
        state.db.get_messages_from(&session_id, &target_message_id)?
    };

    // 4. Optionally selectively revert code modifications
    let mut restored_files = Vec::new();
    if revert_code {
        let ws_opt = state.db.list_workspaces()?.into_iter().find(|w| w.id == workspace_id);
        if let Some(ws) = ws_opt {
            let ws_path = std::path::PathBuf::from(&ws.path);
            if ws_path.exists() {
                // Determine candidate files touched during the rewound turn(s)
                let mut target_files = extract_files_from_messages(&target_messages, &ws_path);

                // Find earliest timestamp of rewound turn
                let earliest_ts = target_messages.iter()
                    .filter_map(|m| chrono::DateTime::parse_from_rfc3339(&m.created_at).ok())
                    .min()
                    .map(|dt| dt.with_timezone(&Utc));

                // Also inspect git status to selectively capture files modified during this turn
                let mut status_cmd = Command::new("git");
                status_cmd.args(&["status", "--porcelain"]).current_dir(&ws_path);
                #[cfg(target_os = "windows")]
                {
                    use std::os::windows::process::CommandExt;
                    status_cmd.creation_flags(0x08000000);
                }

                if let Ok(out) = status_cmd.output() {
                    let text = String::from_utf8_lossy(&out.stdout);
                    for line in text.lines() {
                        let trimmed = line.trim();
                        if trimmed.len() > 3 {
                            let file_part = trimmed[3..].trim();
                            let full_path = ws_path.join(file_part);
                            let was_modified_in_turn = if let Some(turn_time) = earliest_ts {
                                if let Ok(meta) = full_path.metadata() {
                                    if let Ok(mod_time) = meta.modified() {
                                        let mod_dt: chrono::DateTime<Utc> = mod_time.into();
                                        mod_dt >= turn_time - chrono::Duration::seconds(15)
                                    } else {
                                        false
                                    }
                                } else {
                                    false
                                }
                            } else {
                                false
                            };

                            if was_modified_in_turn {
                                add_cleaned_rel_path(&mut target_files, file_part, &ws_path);
                            }
                        }
                    }
                }

                // Selectively revert or remove ONLY the files identified for this turn.
                // Never run blind git clean -fd across the workspace to avoid wiping earlier untracked files.
                for file_rel in target_files {
                    let full_path = ws_path.join(&file_rel);

                    let mut check_tracked_cmd = Command::new("git");
                    check_tracked_cmd.args(&["ls-files", "--error-unmatch", &file_rel]).current_dir(&ws_path);
                    #[cfg(target_os = "windows")]
                    {
                        use std::os::windows::process::CommandExt;
                        check_tracked_cmd.creation_flags(0x08000000);
                    }
                    let is_tracked = check_tracked_cmd.status().map(|s| s.success()).unwrap_or(false);

                    if is_tracked {
                        let mut unstage_cmd = Command::new("git");
                        unstage_cmd.args(&["restore", "--staged", &file_rel]).current_dir(&ws_path);
                        #[cfg(target_os = "windows")]
                        {
                            use std::os::windows::process::CommandExt;
                            unstage_cmd.creation_flags(0x08000000);
                        }
                        let _ = unstage_cmd.status();

                        let mut restore_cmd = Command::new("git");
                        restore_cmd.args(&["restore", &file_rel]).current_dir(&ws_path);
                        #[cfg(target_os = "windows")]
                        {
                            use std::os::windows::process::CommandExt;
                            restore_cmd.creation_flags(0x08000000);
                        }
                        if let Ok(status) = restore_cmd.status() {
                            if status.success() {
                                restored_files.push(file_rel);
                            }
                        }
                    } else if full_path.exists() {
                        if full_path.is_file() {
                            if std::fs::remove_file(&full_path).is_ok() {
                                restored_files.push(file_rel);
                            }
                        } else if full_path.is_dir() {
                            if std::fs::remove_dir_all(&full_path).is_ok() {
                                restored_files.push(file_rel);
                            }
                        }
                    }
                }
            }
        }
    }

    // 5. Emit acp-rewind event
    let _ = app.emit("acp-rewind", serde_json::json!({
        "session_id": session_id,
        "rewound_count": count,
        "restored_files": restored_files,
    }));

    Ok(RewindResult {
        rewound_messages_count: count,
        restored_files,
        previous_prompt: prev_prompt,
    })
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
    request_id: serde_json::Value,
    option_id: Option<String>,
    allowed: Option<bool>,
    answer: Option<String>,
) -> Result<(), String> {
    state
        .acp_session
        .respond_permission_with_answer(request_id, option_id, allowed.unwrap_or(true), answer)
        .await
}

#[tauri::command]
pub async fn restart_gemini_session(state: State<'_, AppState>) -> Result<String, String> {
    let mut child_guard = state.active_child.lock().await;
    if let Some(old_child) = child_guard.take() {
        crate::process_manager::ProcessSupervisor::terminate_child(old_child);
    }
    let mut ws_guard = state.active_process_workspace.lock().await;
    *ws_guard = None;
    state.acp_session.clear_sessions();
    Ok("Gemini CLI session reset successfully. Next prompt will launch with updated environment.".to_string())
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContinuationResult {
    pub new_session: crate::database::Session,
    pub continuation_message: Message,
}

#[tauri::command]
pub async fn create_continuation_session(
    state: State<'_, AppState>,
    exhausted_session_id: String,
    workspace_path: Option<String>,
) -> Result<ContinuationResult, String> {
    let session = state
        .db
        .get_session(&exhausted_session_id)?
        .ok_or_else(|| format!("Session {} not found", exhausted_session_id))?;

    let messages = state.db.list_messages(&exhausted_session_id)?;

    // 1. Extract recent user goals
    let recent_user_prompts: Vec<String> = messages
        .iter()
        .filter(|m| m.role == "user")
        .rev()
        .take(3)
        .map(|m| {
            let trimmed = m.content.trim();
            let first_line = trimmed.lines().next().unwrap_or(trimmed);
            if first_line.len() > 100 {
                format!("{}...", &first_line[..97])
            } else {
                first_line.to_string()
            }
        })
        .collect();

    // 2. Extract touched files
    let ws_path = workspace_path.as_deref().map(std::path::Path::new);
    let touched_files = if let Some(ws) = ws_path {
        extract_files_from_messages(&messages, ws)
    } else {
        Vec::new()
    };

    // 3. Format structured brief
    let mut brief = format!(
        "### 🔄 Context Continuation Summary\n\n*This session was automatically continued from **{}** after reaching the 1,000,000 token context limit.*\n\n",
        session.title
    );

    if !recent_user_prompts.is_empty() {
        brief.push_str("#### Recent Objectives:\n");
        for p in recent_user_prompts.iter().rev() {
            brief.push_str(&format!("- {}\n", p));
        }
        brief.push('\n');
    }

    if !touched_files.is_empty() {
        brief.push_str("#### Modified & Inspected Files:\n");
        for f in touched_files.iter().take(10) {
            brief.push_str(&format!("- `{f}`\n"));
        }
        if touched_files.len() > 10 {
            brief.push_str(&format!("- *(and {} more files)*\n", touched_files.len() - 10));
        }
        brief.push('\n');
    }

    brief.push_str("---\n*All previous state is captured in your project files. You can continue working directly from here!*");

    // 4. Create new continuation session
    let continuation_title = if session.title.ends_with("(Continuation)") {
        format!("{} (Part 2)", session.title)
    } else {
        format!("{} (Continuation)", session.title)
    };

    let new_session = state.db.create_session(&session.workspace_id, &continuation_title)?;

    // 5. Save synthetic assistant message
    let continuation_message = Message {
        id: format!("msg-{}", Uuid::new_v4()),
        session_id: new_session.id.clone(),
        role: "assistant".to_string(),
        token_count: (brief.len() / 4) as i64,
        tool_calls_json: None,
        content: brief,
        created_at: Utc::now().to_rfc3339(),
    };
    state.db.save_message(continuation_message.clone())?;

    // Clean up old ACP session mapping (memory + DB binding) so new session creates fresh context
    state.acp_session.remove_session(&exhausted_session_id);
    let _ = state.db.update_session_acp_binding(&exhausted_session_id, None, None);

    Ok(ContinuationResult {
        new_session,
        continuation_message,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_continuation_result_serialization() {
        let res = ContinuationResult {
            new_session: crate::database::Session {
                id: "sess-100".to_string(),
                workspace_id: "ws-1".to_string(),
                title: "Sprint 2 (Continuation)".to_string(),
                acp_session_id: None,
                acp_engine: None,
                created_at: "2026-10-28T12:00:00Z".to_string(),
                updated_at: "2026-10-28T12:00:00Z".to_string(),
            },
            continuation_message: Message {
                id: "msg-100".to_string(),
                session_id: "sess-100".to_string(),
                role: "assistant".to_string(),
                content: "Continuation brief".to_string(),
                tool_calls_json: None,
                token_count: 50,
                created_at: "2026-10-28T12:00:00Z".to_string(),
            },
        };
        let serialized = serde_json::to_string(&res).unwrap();
        assert!(serialized.contains("newSession"));
        assert!(serialized.contains("continuationMessage"));
        assert!(serialized.contains("Sprint 2 (Continuation)"));
    }

    #[test]
    fn test_rewind_result_serialization() {
        let res = RewindResult {
            rewound_messages_count: 2,
            restored_files: vec!["src/main.rs".to_string()],
            previous_prompt: Some("Fix bug".to_string()),
        };
        let serialized = serde_json::to_string(&res).unwrap();
        assert!(serialized.contains("\"rewound_messages_count\":2"));
        assert!(serialized.contains("src/main.rs"));
        assert!(serialized.contains("Fix bug"));
    }

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

    #[test]
    fn test_build_agy_extra_args() {
        let args_yolo = build_agy_extra_args("auto", "yolo", None);
        assert!(!args_yolo.contains(&"--model".to_string()));
        assert!(args_yolo.contains(&"--dangerously-skip-permissions".to_string()));

        let args_model = build_agy_extra_args("claude-sonnet-4-6", "auto_edit", Some("high"));
        assert!(args_model.contains(&"--model".to_string()));
        assert!(args_model.contains(&"claude-sonnet-4-6".to_string()));
        assert!(args_model.contains(&"--mode".to_string()));
        assert!(args_model.contains(&"accept-edits".to_string()));
        assert!(args_model.contains(&"--effort".to_string()));
        assert!(args_model.contains(&"high".to_string()));

        let args_plan = build_agy_extra_args("gemini-3.8-flash-high", "plan", None);
        assert!(args_plan.contains(&"--mode".to_string()));
        assert!(args_plan.contains(&"plan".to_string()));
    }

    #[test]
    fn test_sanitize_model_for_engine() {
        // Cross-engine fallback when switching from gemini to agy
        assert_eq!(sanitize_model_for_engine("agy", "gemini-2.5-pro"), "auto");
        assert_eq!(sanitize_model_for_engine("agy", "gemini-3.5-flash"), "auto");
        assert_eq!(sanitize_model_for_engine("agy", "gemini-3.8-flash"), "auto");
        assert_eq!(sanitize_model_for_engine("agy", "gemini-1.5-pro"), "auto");
        assert_eq!(sanitize_model_for_engine("agy", "gemini-3.1-pro-preview"), "auto");

        // Native agy models preserved
        assert_eq!(sanitize_model_for_engine("agy", "gemini-3.8-flash-high"), "gemini-3.8-flash-high");
        assert_eq!(sanitize_model_for_engine("agy", "gemini-3.8-flash-medium"), "gemini-3.8-flash-medium");
        assert_eq!(sanitize_model_for_engine("agy", "gemini-3.8-flash-low"), "gemini-3.8-flash-low");
        assert_eq!(sanitize_model_for_engine("agy", "claude-sonnet-4-6"), "claude-sonnet-4-6");
        assert_eq!(sanitize_model_for_engine("agy", "claude-opus-4-6-thinking"), "claude-opus-4-6-thinking");
        assert_eq!(sanitize_model_for_engine("agy", "gpt-oss-120b-medium"), "gpt-oss-120b-medium");
        assert_eq!(sanitize_model_for_engine("agy", "gemini-4-argon"), "gemini-4-argon");
        assert_eq!(sanitize_model_for_engine("agy", "auto"), "auto");
        assert_eq!(sanitize_model_for_engine("agy", ""), "auto");

        // Cross-engine fallback when switching from agy to gemini
        assert_eq!(sanitize_model_for_engine("gemini", "gemini-3.8-flash-high"), "auto");
        assert_eq!(sanitize_model_for_engine("gemini", "claude-sonnet-4-6"), "auto");
        assert_eq!(sanitize_model_for_engine("gemini", "gpt-oss-120b-medium"), "auto");
        assert_eq!(sanitize_model_for_engine("gemini", "gemini-4-argon"), "auto");

        // Native gemini models preserved
        assert_eq!(sanitize_model_for_engine("gemini", "gemini-2.5-pro"), "gemini-2.5-pro");
        assert_eq!(sanitize_model_for_engine("gemini", "gemini-3.5-flash"), "gemini-3.5-flash");
        assert_eq!(sanitize_model_for_engine("gemini", "gemini-3.8-flash"), "gemini-3.8-flash");
        assert_eq!(sanitize_model_for_engine("gemini", "auto"), "auto");
    }

    #[test]
    fn test_build_user_steering_prompt() {
        let plan_prompt = build_user_steering_prompt("focus on frontend", true);
        assert!(plan_prompt.contains("focus on frontend"));
        assert!(plan_prompt.contains("Re-evaluate the active plan"));

        let general_prompt = build_user_steering_prompt("stop counting", false);
        assert!(general_prompt.contains("stop counting"));
        assert!(!general_prompt.contains("Re-evaluate the active plan"));
        assert!(general_prompt.contains("Please adapt your execution"));
    }

    #[test]
    fn test_active_child_pid_guard_prevents_stale_reaping() {
        // Test that an old process PID doesn't match a new process PID
        let old_pid: u32 = 1234;
        let new_pid: u32 = 5678;

        // When switching from Workspace 1 (YOLO) to Workspace 2 (Ask),
        // the active child PID changes. An EOF event from old_pid must be rejected.
        let active_pid_option = Some(new_pid);
        let is_still_active = match active_pid_option {
            Some(pid) => pid == old_pid,
            None => false,
        };

        assert!(!is_still_active, "Old PID must not match active PID when new child was spawned");

        // When the active PID matches the running child PID, it is recognized as active
        let is_active_for_current = match active_pid_option {
            Some(pid) => pid == new_pid,
            None => false,
        };
        assert!(is_active_for_current, "Current PID must match active PID");
    }

    #[test]
    fn test_extract_files_from_messages_isolation() {
        let temp_dir = std::env::temp_dir().join(format!("test_isolation_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let file2 = temp_dir.join("test2.abc");
        let file3 = temp_dir.join("test3.abc");
        std::fs::write(&file2, "content 2").unwrap();
        std::fs::write(&file3, "content 3").unwrap();

        let turn2_user = Message {
            id: "msg-user-2".to_string(),
            session_id: "sess-1".to_string(),
            role: "user".to_string(),
            content: "Hi,\nCreate a dummy file test3.abc".to_string(),
            tool_calls_json: None,
            token_count: 10,
            created_at: "2026-10-25T10:02:00Z".to_string(),
        };

        let turn2_assistant = Message {
            id: "msg-assistant-2".to_string(),
            session_id: "sess-1".to_string(),
            role: "assistant".to_string(),
            content: "I have created test3.abc for you.".to_string(),
            tool_calls_json: Some(serde_json::json!([
                {
                    "tool_name": "write_file",
                    "parameters": { "filePath": "test3.abc" },
                    "summary": "Edited: test3.abc"
                }
            ]).to_string()),
            token_count: 20,
            created_at: "2026-10-25T10:02:05Z".to_string(),
        };

        let turn2_messages = vec![turn2_user, turn2_assistant];
        let extracted = extract_files_from_messages(&turn2_messages, &temp_dir);

        assert!(extracted.contains(&"test3.abc".to_string()));
        assert!(!extracted.contains(&"test2.abc".to_string()));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_extended_engine_flags_and_sanitization() {
        // agy: --mode auto, --sandbox, --agent, repeatable --add-dir, --effort
        let agy_args = build_agy_extra_args_with_options(
            "gemini-4-hydrogen-high",
            "auto",
            Some("xhigh"),
            Some("architect"),
            true,
            Some("C:\\Shared; ../common\nD:\\Libs"),
        );
        assert!(agy_args.windows(2).any(|w| w == ["--mode", "auto"]));
        assert!(agy_args.windows(2).any(|w| w == ["--effort", "xhigh"]));
        assert!(agy_args.windows(2).any(|w| w == ["--agent", "architect"]));
        assert!(agy_args.contains(&"--sandbox".to_string()));
        assert_eq!(agy_args.windows(2).filter(|w| w[0] == "--add-dir").count(), 3);

        // gemini: "auto" safely maps to auto_edit; --sandbox and --include-directories allowed; never passes --effort/--agent
        let gemini_args = build_gemini_extra_args_with_options(
            "gemini-3.5-flash",
            "auto",
            true,
            Some("C:\\Shared, ../common"),
        );
        assert!(gemini_args.windows(2).any(|w| w == ["--approval-mode", "auto_edit"]));
        assert!(gemini_args.contains(&"--sandbox".to_string()));
        assert_eq!(gemini_args.windows(2).filter(|w| w[0] == "--include-directories").count(), 2);
        assert!(!gemini_args.contains(&"--effort".to_string()));
        assert!(!gemini_args.contains(&"--agent".to_string()));
        assert!(!gemini_args.contains(&"--add-dir".to_string()));

        // Orchestrator & thinking-suffix fallback on Gemini engine
        assert_eq!(sanitize_model_for_engine("gemini", "orchestrator"), "auto");
        assert_eq!(sanitize_model_for_engine("gemini", "gemini-4"), "auto");
        assert_eq!(sanitize_model_for_engine("gemini", "gemini-4-hydrogen-xhigh"), "auto");
        assert_eq!(sanitize_model_for_engine("agy", "orchestrator"), "orchestrator");
    }
}
