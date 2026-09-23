use crate::database::{DbManager, Message, PromptTemplate, SearchResult, Session, Workspace};
use crate::process_manager::ProcessSupervisor;
use crate::acp_client::{handle_acp_line, AcpSession};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tauri::{AppHandle, State, Emitter};
use tokio::sync::Mutex;
use uuid::Uuid;
use chrono::Utc;

#[derive(Debug, Serialize, Deserialize)]
pub struct GeminiEnvStatus {
    pub installed: bool,
    pub path: Option<String>,
    pub version: Option<String>,
    pub details: String,
}

pub struct AppState {
    pub db: DbManager,
    pub supervisor: ProcessSupervisor,
    pub acp_session: Arc<AcpSession>,
    pub active_process_workspace: Arc<Mutex<Option<String>>>,
    pub search_generation: Arc<AtomicU64>,
}

#[tauri::command]
pub async fn check_gemini_env() -> Result<GeminiEnvStatus, String> {
    match crate::process_manager::find_gemini_executable() {
        Some(bin_path) => {
            let path_str = bin_path.to_string_lossy().to_string();
            let is_batch = path_str.to_lowercase().ends_with(".cmd")
                        || path_str.to_lowercase().ends_with(".bat");

            #[cfg(target_os = "windows")]
            let ver_check = if is_batch {
                Command::new("cmd.exe")
                    .args(["/c", &path_str, "--version"])
                    .output()
            } else {
                Command::new(&path_str)
                    .arg("--version")
                    .output()
            };

            #[cfg(not(target_os = "windows"))]
            let ver_check = Command::new(&path_str)
                .arg("--version")
                .output();

            let version = if let Ok(ver_out) = ver_check {
                if ver_out.status.success() {
                    let v = String::from_utf8_lossy(&ver_out.stdout).trim().to_string();
                    if v.is_empty() {
                        "installed".to_string()
                    } else {
                        v
                    }
                } else {
                    "installed".to_string()
                }
            } else {
                "installed".to_string()
            };

            Ok(GeminiEnvStatus {
                installed: true,
                path: Some(path_str),
                version: Some(version),
                details: "Gemini CLI found and verified on system.".to_string(),
            })
        }
        None => Ok(GeminiEnvStatus {
            installed: false,
            path: None,
            version: None,
            details: "Gemini CLI not found in PATH or standard npm/pnpm locations.".to_string(),
        }),
    }
}

#[tauri::command]
pub fn get_workspaces(state: State<AppState>) -> Result<Vec<Workspace>, String> {
    state.db.list_workspaces()
}

#[tauri::command]
pub fn save_workspace(state: State<AppState>, workspace: Workspace) -> Result<(), String> {
    state.db.save_workspace(workspace)
}

#[tauri::command]
pub fn delete_workspace(state: State<AppState>, id: String) -> Result<(), String> {
    state.db.delete_workspace(&id)
}

#[tauri::command]
pub fn get_sessions(state: State<AppState>, workspace_id: String) -> Result<Vec<Session>, String> {
    state.db.list_sessions(&workspace_id)
}

#[tauri::command]
pub fn create_session(state: State<AppState>, workspace_id: String, title: String) -> Result<Session, String> {
    state.db.create_session(&workspace_id, &title)
}

#[tauri::command]
pub fn rename_session(state: State<AppState>, session_id: String, title: String) -> Result<(), String> {
    state.db.rename_session(&session_id, &title)
}

#[tauri::command]
pub fn delete_session(state: State<AppState>, session_id: String) -> Result<(), String> {
    state.acp_session.remove_session(&session_id);
    state.db.delete_session(&session_id)
}

#[tauri::command]
pub fn get_session_messages(state: State<AppState>, session_id: String) -> Result<Vec<Message>, String> {
    state.db.list_messages(&session_id)
}

#[tauri::command]
pub fn save_message(state: State<AppState>, msg: Message) -> Result<(), String> {
    state.db.save_message(msg)
}

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

#[tauri::command]
pub async fn send_prompt(
    app: AppHandle,
    state: State<'_, AppState>,
    session_id: String,
    workspace_id: String,
    prompt: String,
    model: String,
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

    // 3. Ensure CLI process is spawned and ACP connected
    let mut ws_guard = state.active_process_workspace.lock().await;
    let needs_spawn = match &*ws_guard {
        Some(current_ws) => current_ws != &workspace_id,
        None => true,
    };

    if needs_spawn {
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

                let _ = app.emit("acp-chunk", crate::acp_client::StreamChunkPayload {
                    session_id: session_id.clone(),
                    delta: fallback_text,
                    is_done: true,
                });
                return Ok(0);
            }
        };

        let mut extra_args = Vec::new();
        let trimmed_model = model.trim();
        if !trimmed_model.is_empty() && trimmed_model != "auto" {
            extra_args.push("--model".to_string());
            extra_args.push(trimmed_model.to_string());
        }

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

                *ws_guard = Some(workspace_id.clone());

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

                let _ = app.emit("acp-chunk", crate::acp_client::StreamChunkPayload {
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

    // Explicitly set the active model on this session via ACP session/set_model
    let trimmed_model = model.trim();
    if !trimmed_model.is_empty() {
        let set_model_params = serde_json::json!({
            "sessionId": acp_session_id,
            "modelId": trimmed_model
        });
        let _ = state.acp_session.send_request_with_response("session/set_model", set_model_params).await;
    }

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

    state.acp_session.send_request("session/prompt", prompt_params).await
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
pub fn get_prompts(state: State<AppState>) -> Result<Vec<PromptTemplate>, String> {
    state.db.list_prompts()
}

#[tauri::command]
pub fn search_history(state: State<AppState>, query: String) -> Result<Vec<SearchResult>, String> {
    state.db.search_fts(&query)
}

pub fn format_session_export(session_id: &str, messages: &[Message], format: &str) -> Result<String, String> {
    match format {
        "json" => serde_json::to_string_pretty(messages).map_err(|e| e.to_string()),
        "txt" => {
            let mut txt = String::new();
            for m in messages {
                txt.push_str(&format!("{}: {}\n\n", m.role.to_uppercase(), m.content));
            }
            Ok(txt)
        }
        _ => {
            // Markdown export
            let mut md = String::new();
            md.push_str(&format!("# Chat Export - Session {}\n\n", session_id));
            for m in messages {
                if m.role == "user" {
                    md.push_str(&format!("### 👤 User\n\n{}\n\n---\n\n", m.content));
                } else {
                    md.push_str(&format!("### ✨ Gemini\n\n{}\n\n---\n\n", m.content));
                }
            }
            Ok(md)
        }
    }
}

#[tauri::command]
pub fn export_session(state: State<AppState>, session_id: String, format: String) -> Result<String, String> {
    let messages = state.db.list_messages(&session_id)?;
    format_session_export(&session_id, &messages, &format)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceFileEntry {
    pub name: String,
    pub relative_path: String,
    pub is_dir: bool,
    pub extension: Option<String>,
}

#[tauri::command]
pub fn read_workspace_dir(
    state: State<AppState>,
    workspace_id: String,
    relative_path: Option<String>,
) -> Result<Vec<WorkspaceFileEntry>, String> {
    let workspaces = state.db.list_workspaces()?;
    let ws = workspaces.into_iter().find(|w| w.id == workspace_id)
        .ok_or_else(|| "Workspace not found".to_string())?;

    let root = PathBuf::from(&ws.path);
    if !root.exists() || !root.is_dir() {
        return Ok(Vec::new());
    }

    read_workspace_dir_internal(&root, relative_path.as_deref())
}

pub fn read_workspace_dir_internal(
    root: &std::path::Path,
    relative_path: Option<&str>,
) -> Result<Vec<WorkspaceFileEntry>, String> {
    if !root.exists() || !root.is_dir() {
        return Ok(Vec::new());
    }

    let target_dir = if let Some(rel) = relative_path {
        let clean_rel = rel.trim().trim_start_matches('/').trim_start_matches('\\');
        if clean_rel.is_empty() {
            root.to_path_buf()
        } else {
            let candidate = root.join(clean_rel);
            if let (Ok(can_cand), Ok(can_root)) = (candidate.canonicalize(), root.canonicalize()) {
                if !can_cand.starts_with(&can_root) {
                    return Err("Access denied: path outside workspace root".to_string());
                }
            }
            candidate
        }
    } else {
        root.to_path_buf()
    };

    if !target_dir.exists() || !target_dir.is_dir() {
        return Ok(Vec::new());
    }

    let read_dir = match std::fs::read_dir(&target_dir) {
        Ok(rd) => rd,
        Err(e) => return Err(format!("Failed to read directory: {}", e)),
    };

    let ignored_names = [
        ".git", "node_modules", "target", "build", "dist", ".svelte-kit",
        ".vscode", ".idea", "__pycache__", ".next", ".turbo", "vendor"
    ];

    let mut entries = Vec::new();

    for entry in read_dir.flatten() {
        let path = entry.path();
        let file_name = match entry.file_name().into_string() {
            Ok(s) => s,
            Err(_) => continue,
        };

        if ignored_names.iter().any(|&ign| ign.eq_ignore_ascii_case(&file_name)) {
            continue;
        }

        let is_dir = path.is_dir();
        let relative = match path.strip_prefix(root) {
            Ok(p) => p.to_string_lossy().replace('\\', "/"),
            Err(_) => continue,
        };

        let extension = if is_dir {
            None
        } else {
            path.extension().and_then(|e| e.to_str()).map(|s| s.to_string())
        };

        entries.push(WorkspaceFileEntry {
            name: file_name,
            relative_path: relative,
            is_dir,
            extension,
        });
    }

    // Sort: directories first (alphabetical case-insensitive), then files (alphabetical case-insensitive)
    entries.sort_by(|a, b| {
        if a.is_dir != b.is_dir {
            if a.is_dir {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Greater
            }
        } else {
            a.name.to_lowercase().cmp(&b.name.to_lowercase())
        }
    });

    Ok(entries)
}

#[tauri::command]
pub fn cancel_workspace_search(state: State<'_, AppState>) {
    state.search_generation.fetch_add(1, Ordering::SeqCst);
}

#[tauri::command]
pub async fn search_workspace_files(
    state: State<'_, AppState>,
    workspace_id: String,
    query: String,
    max_results: Option<usize>,
) -> Result<Vec<WorkspaceFileEntry>, String> {
    let workspaces = state.db.list_workspaces()?;
    let ws = workspaces.into_iter().find(|w| w.id == workspace_id)
        .ok_or_else(|| "Workspace not found".to_string())?;

    let root = PathBuf::from(&ws.path);
    if !root.exists() || !root.is_dir() {
        return Ok(Vec::new());
    }

    let trimmed = query.trim().to_string();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    let search_gen = state.search_generation.clone();
    let current_id = search_gen.fetch_add(1, Ordering::SeqCst) + 1;
    let limit = max_results.unwrap_or(100);

    tokio::task::spawn_blocking(move || {
        search_workspace_files_internal(&root, &trimmed, limit, Some((&search_gen, current_id)))
    })
    .await
    .map_err(|e| format!("Search task failed: {}", e))?
}

pub fn search_workspace_files_internal(
    root: &std::path::Path,
    query: &str,
    max_results: usize,
    cancellation: Option<(&Arc<AtomicU64>, u64)>,
) -> Result<Vec<WorkspaceFileEntry>, String> {
    if !root.exists() || !root.is_dir() {
        return Ok(Vec::new());
    }

    let q_lower = query.to_lowercase();
    let mut entries = Vec::new();
    walk_search_dir(root, root, &q_lower, 0, 10, max_results, cancellation, &mut entries);

    entries.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(entries)
}

fn walk_search_dir(
    root: &std::path::Path,
    current: &std::path::Path,
    query_lower: &str,
    depth: usize,
    max_depth: usize,
    max_results: usize,
    cancellation: Option<(&Arc<AtomicU64>, u64)>,
    out: &mut Vec<WorkspaceFileEntry>,
) {
    if depth > max_depth || out.len() >= max_results {
        return;
    }

    // Abort traversal if cancelled or superseded by newer search
    if let Some((gen, id)) = cancellation {
        if gen.load(Ordering::Relaxed) != id {
            return;
        }
    }

    let read_dir = match std::fs::read_dir(current) {
        Ok(rd) => rd,
        Err(_) => return,
    };

    let ignored_names = [
        ".git", "node_modules", "target", "build", "dist", ".svelte-kit",
        ".vscode", ".idea", "__pycache__", ".next", ".turbo", "vendor"
    ];

    let mut subdirs = Vec::new();

    for entry in read_dir.flatten() {
        if let Some((gen, id)) = cancellation {
            if gen.load(Ordering::Relaxed) != id {
                return;
            }
        }

        let path = entry.path();
        let file_name = match entry.file_name().into_string() {
            Ok(s) => s,
            Err(_) => continue,
        };

        if ignored_names.iter().any(|&ign| ign.eq_ignore_ascii_case(&file_name)) {
            continue;
        }

        let is_dir = path.is_dir();
        if is_dir {
            subdirs.push(path);
        } else {
            let relative = match path.strip_prefix(root) {
                Ok(p) => p.to_string_lossy().replace('\\', "/"),
                Err(_) => continue,
            };

            // Files only: check if file_name contains query (do not check directory path)
            if file_name.to_lowercase().contains(query_lower) {
                let extension = path.extension().and_then(|e| e.to_str()).map(|s| s.to_string());
                out.push(WorkspaceFileEntry {
                    name: file_name,
                    relative_path: relative,
                    is_dir: false,
                    extension,
                });
            }

            if out.len() >= max_results {
                return;
            }
        }
    }

    for subdir in subdirs {
        walk_search_dir(root, &subdir, query_lower, depth + 1, max_depth, max_results, cancellation, out);
        if out.len() >= max_results {
            return;
        }
    }
}

#[tauri::command]
pub fn open_workspace_file(path: String, with_app: Option<String>) -> Result<(), String> {
    let p = PathBuf::from(&path);
    if !p.exists() {
        return Err(format!("File does not exist: {}", path));
    }

    if let Some(ref app) = with_app {
        if app.eq_ignore_ascii_case("notepad") {
            #[cfg(target_os = "windows")]
            {
                std::process::Command::new("notepad.exe")
                    .arg(&path)
                    .spawn()
                    .map(|_| ())
                    .map_err(|e| format!("Failed to launch Notepad: {}", e))?;
                return Ok(());
            }
        }
        return open::with_detached(&path, app).map_err(|e| format!("Failed to open with {}: {}", app, e));
    }

    // Try system default application first
    let res = open::that_detached(&path);
    if res.is_err() {
        // Fallback to Notepad on Windows if no default application is associated with this file type
        #[cfg(target_os = "windows")]
        {
            return std::process::Command::new("notepad.exe")
                .arg(&path)
                .spawn()
                .map(|_| ())
                .map_err(|e| format!("Failed to open file with default app and Notepad: {}", e));
        }
        #[cfg(not(target_os = "windows"))]
        {
            return res.map_err(|e| e.to_string());
        }
    }

    Ok(())
}

#[tauri::command]
pub fn list_workspace_files(state: State<AppState>, workspace_id: String) -> Result<Vec<WorkspaceFileEntry>, String> {
    let workspaces = state.db.list_workspaces()?;
    let ws = workspaces.into_iter().find(|w| w.id == workspace_id)
        .ok_or_else(|| "Workspace not found".to_string())?;

    let root = PathBuf::from(&ws.path);
    if !root.exists() || !root.is_dir() {
        return Ok(Vec::new());
    }

    let mut entries = Vec::new();
    walk_workspace_dir(&root, &root, 0, 7, &mut entries);
    Ok(entries)
}

fn walk_workspace_dir(
    root: &std::path::Path,
    current: &std::path::Path,
    depth: usize,
    max_depth: usize,
    out: &mut Vec<WorkspaceFileEntry>,
) {
    if depth > max_depth || out.len() >= 3000 {
        return;
    }

    let read_dir = match std::fs::read_dir(current) {
        Ok(rd) => rd,
        Err(_) => return,
    };

    let ignored_names = [
        ".git", "node_modules", "target", "build", "dist", ".svelte-kit",
        ".vscode", ".idea", "__pycache__", ".next", ".turbo", "vendor"
    ];

    let mut subdirs = Vec::new();

    for entry in read_dir.flatten() {
        let path = entry.path();
        let file_name = match entry.file_name().into_string() {
            Ok(s) => s,
            Err(_) => continue,
        };

        if ignored_names.iter().any(|&ign| ign.eq_ignore_ascii_case(&file_name)) {
            continue;
        }

        let is_dir = path.is_dir();
        let relative = match path.strip_prefix(root) {
            Ok(p) => p.to_string_lossy().replace('\\', "/"),
            Err(_) => continue,
        };

        let extension = if is_dir {
            None
        } else {
            path.extension().and_then(|e| e.to_str()).map(|s| s.to_string())
        };

        out.push(WorkspaceFileEntry {
            name: file_name,
            relative_path: relative,
            is_dir,
            extension,
        });

        if is_dir {
            subdirs.push(path);
        }

        if out.len() >= 3000 {
            break;
        }
    }

    for subdir in subdirs {
        walk_workspace_dir(root, &subdir, depth + 1, max_depth, out);
        if out.len() >= 3000 {
            break;
        }
    }
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalCommandResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub duration_ms: u64,
}

#[tauri::command]
pub async fn run_terminal_command(
    command: String,
    workspace_path: Option<String>,
) -> Result<TerminalCommandResult, String> {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return Ok(TerminalCommandResult {
            stdout: String::new(),
            stderr: String::new(),
            exit_code: 0,
            duration_ms: 0,
        });
    }

    let start = std::time::Instant::now();
    let mut cmd = if cfg!(target_os = "windows") {
        let mut c = std::process::Command::new("powershell.exe");
        c.arg("-NoProfile")
            .arg("-NonInteractive")
            .arg("-ExecutionPolicy")
            .arg("Bypass")
            .arg("-Command")
            .arg(trimmed);
        c
    } else {
        let mut c = std::process::Command::new("sh");
        c.arg("-c").arg(trimmed);
        c
    };

    if let Some(ref wp) = workspace_path {
        let p = PathBuf::from(wp);
        if p.is_dir() {
            cmd.current_dir(&p);

            // Auto-inject workspace .env and .env.local into terminal commands
            let dot_env = p.join(".env");
            if dot_env.is_file() {
                let envs = ProcessSupervisor::parse_dotenv_file(&dot_env);
                cmd.envs(&envs);
            }
            let dot_env_local = p.join(".env.local");
            if dot_env_local.is_file() {
                let envs = ProcessSupervisor::parse_dotenv_file(&dot_env_local);
                cmd.envs(&envs);
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    let output = cmd
        .output()
        .map_err(|e| format!("Failed to execute command: {}", e))?;

    let duration_ms = start.elapsed().as_millis() as u64;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code().unwrap_or(-1);

    Ok(TerminalCommandResult {
        stdout,
        stderr,
        exit_code,
        duration_ms,
    })
}

#[tauri::command]
pub async fn restart_gemini_session(state: State<'_, AppState>) -> Result<String, String> {
    let mut ws_guard = state.active_process_workspace.lock().await;
    *ws_guard = None;
    state.acp_session.clear_sessions();
    Ok("Gemini CLI session reset successfully. Next prompt will launch with updated environment.".to_string())
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

    #[tokio::test]
    async fn test_terminal_command_dotenv_injection() {
        let temp_dir = std::env::temp_dir().join(format!("gemini_test_term_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let dot_env = temp_dir.join(".env");
        std::fs::write(&dot_env, "CUSTOM_TEST_ENV=SecretAlphaValue123\n").unwrap();

        let ws_str = temp_dir.to_string_lossy().to_string();
        let cmd_str = if cfg!(target_os = "windows") {
            "Write-Output $env:CUSTOM_TEST_ENV".to_string()
        } else {
            "echo $CUSTOM_TEST_ENV".to_string()
        };

        let res = run_terminal_command(cmd_str, Some(ws_str)).await.expect("run_terminal_command failed");
        assert_eq!(res.exit_code, 0);
        assert!(res.stdout.contains("SecretAlphaValue123"));

        let _ = std::fs::remove_dir_all(&temp_dir);
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
    fn test_read_workspace_dir() {
        let temp_dir = std::env::temp_dir().join(format!("gemini_test_read_dir_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        // Create a root file and a root subfolder
        std::fs::write(temp_dir.join("root_file.txt"), "hello").unwrap();
        let sub_dir = temp_dir.join("src");
        std::fs::create_dir_all(&sub_dir).unwrap();
        std::fs::write(sub_dir.join("main.rs"), "fn main() {}").unwrap();

        // Create ignored folder
        let git_dir = temp_dir.join(".git");
        std::fs::create_dir_all(&git_dir).unwrap();
        std::fs::write(git_dir.join("config"), "").unwrap();

        // 1. Read root directory
        let root_entries = read_workspace_dir_internal(&temp_dir, None).expect("failed to read root dir");
        // Should contain "src" (dir) and "root_file.txt" (file), but NOT ".git"
        assert_eq!(root_entries.len(), 2);
        assert!(root_entries[0].is_dir);
        assert_eq!(root_entries[0].name, "src");
        assert_eq!(root_entries[0].relative_path, "src");

        assert!(!root_entries[1].is_dir);
        assert_eq!(root_entries[1].name, "root_file.txt");
        assert_eq!(root_entries[1].relative_path, "root_file.txt");
        assert_eq!(root_entries[1].extension.as_deref(), Some("txt"));

        // 2. Read subfolder dynamically
        let sub_entries = read_workspace_dir_internal(&temp_dir, Some("src")).expect("failed to read sub dir");
        assert_eq!(sub_entries.len(), 1);
        assert_eq!(sub_entries[0].name, "main.rs");
        assert_eq!(sub_entries[0].relative_path, "src/main.rs");
        assert_eq!(sub_entries[0].extension.as_deref(), Some("rs"));

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_search_workspace_files() {
        let temp_dir = std::env::temp_dir().join(format!("gemini_test_search_ws_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        // Structure:
        // temp_dir/
        //   readme.md
        //   src/
        //     app.svelte
        //     SolutionExplorer.svelte
        //   node_modules/
        //     ignored.svelte

        std::fs::write(temp_dir.join("readme.md"), "# Test").unwrap();

        let src_dir = temp_dir.join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::write(src_dir.join("app.svelte"), "<div/>").unwrap();
        std::fs::write(src_dir.join("SolutionExplorer.svelte"), "<script/>").unwrap();

        let nm_dir = temp_dir.join("node_modules");
        std::fs::create_dir_all(&nm_dir).unwrap();
        std::fs::write(nm_dir.join("ignored.svelte"), "").unwrap();

        // 1. Search for "solution" - should match only SolutionExplorer.svelte
        let res = search_workspace_files_internal(&temp_dir, "solution", 100, None).expect("search failed");
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].name, "SolutionExplorer.svelte");
        assert_eq!(res[0].relative_path, "src/SolutionExplorer.svelte");
        assert!(!res[0].is_dir);

        // 2. Search for "svelte" - should match app.svelte and SolutionExplorer.svelte, but NOT node_modules
        let svelte_res = search_workspace_files_internal(&temp_dir, "svelte", 100, None).expect("search failed");
        assert_eq!(svelte_res.len(), 2);
        assert_eq!(svelte_res[0].name, "app.svelte");
        assert_eq!(svelte_res[1].name, "SolutionExplorer.svelte");

        // 3. Search for "src" (folder name) - should return 0 because path is not matched, only filename
        let path_res = search_workspace_files_internal(&temp_dir, "src", 100, None).expect("search failed");
        assert_eq!(path_res.len(), 0);

        // 4. Search with limit = 1
        let limited = search_workspace_files_internal(&temp_dir, "svelte", 1, None).expect("search failed");
        assert_eq!(limited.len(), 1);

        // 5. Test search cancellation token
        let token = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(1));
        // Pass mismatched ID (simulating cancellation) -> should abort and return 0
        let cancelled = search_workspace_files_internal(&temp_dir, "svelte", 100, Some((&token, 999))).expect("search failed");
        assert_eq!(cancelled.len(), 0);

        // Pass matching ID -> should return results
        let valid = search_workspace_files_internal(&temp_dir, "svelte", 100, Some((&token, 1))).expect("search failed");
        assert_eq!(valid.len(), 2);

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_open_workspace_file_not_found() {
        let non_existent = "C:/path/to/definitely/non_existent_file.xyz";
        let res = open_workspace_file(non_existent.to_string(), None);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("File does not exist"));
    }

    #[test]
    fn test_format_session_export() {
        let messages = vec![
            Message {
                id: "msg-1".to_string(),
                session_id: "sess-test".to_string(),
                role: "user".to_string(),
                content: "Explain Rust borrowing".to_string(),
                tool_calls_json: None,
                token_count: 5,
                created_at: "2026-09-23T12:00:00Z".to_string(),
            },
            Message {
                id: "msg-2".to_string(),
                session_id: "sess-test".to_string(),
                role: "assistant".to_string(),
                content: "Borrowing allows references without transfer of ownership.".to_string(),
                tool_calls_json: None,
                token_count: 10,
                created_at: "2026-09-23T12:00:05Z".to_string(),
            },
        ];

        // 1. Markdown
        let md = format_session_export("sess-test", &messages, "md").unwrap();
        assert!(md.contains("# Chat Export - Session sess-test"));
        assert!(md.contains("### 👤 User"));
        assert!(md.contains("Explain Rust borrowing"));
        assert!(md.contains("### ✨ Gemini"));

        // 2. Text
        let txt = format_session_export("sess-test", &messages, "txt").unwrap();
        assert!(txt.contains("USER: Explain Rust borrowing"));
        assert!(txt.contains("ASSISTANT: Borrowing allows references"));

        // 3. JSON
        let json_str = format_session_export("sess-test", &messages, "json").unwrap();
        let parsed: Vec<Message> = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].content, "Explain Rust borrowing");
    }

    #[test]
    fn test_walk_workspace_dir_filtering_and_depth() {
        let temp_dir = std::env::temp_dir().join(format!("gemini_test_walk_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        // Valid files & subdirectories
        std::fs::write(temp_dir.join("root.txt"), "root").unwrap();
        let src_dir = temp_dir.join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::write(src_dir.join("lib.rs"), "pub fn test() {}").unwrap();

        let deep_dir = src_dir.join("deep");
        std::fs::create_dir_all(&deep_dir).unwrap();
        std::fs::write(deep_dir.join("nested.rs"), "// deep").unwrap();

        // Ignored directories
        let git_dir = temp_dir.join(".git");
        std::fs::create_dir_all(&git_dir).unwrap();
        std::fs::write(git_dir.join("config"), "").unwrap();

        let nm_dir = temp_dir.join("node_modules").join("pkg");
        std::fs::create_dir_all(&nm_dir).unwrap();
        std::fs::write(nm_dir.join("index.js"), "").unwrap();

        let mut entries = Vec::new();
        walk_workspace_dir(&temp_dir, &temp_dir, 0, 7, &mut entries);

        let relative_paths: Vec<String> = entries.iter().map(|e| e.relative_path.clone()).collect();
        assert!(relative_paths.contains(&"root.txt".to_string()));
        assert!(relative_paths.contains(&"src".to_string()));
        assert!(relative_paths.contains(&"src/lib.rs".to_string()));
        assert!(relative_paths.contains(&"src/deep/nested.rs".to_string()));

        // Verify ignored paths are NOT present
        assert!(!relative_paths.iter().any(|p| p.starts_with(".git")));
        assert!(!relative_paths.iter().any(|p| p.starts_with("node_modules")));

        let _ = std::fs::remove_dir_all(&temp_dir);
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
}

