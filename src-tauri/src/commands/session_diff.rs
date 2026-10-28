use crate::commands::AppState;
use serde::{Deserialize, Serialize};
use similar::{ChangeTag, TextDiff};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::State;

const MAX_SNAPSHOT_FILE_SIZE: u64 = 10 * 1024 * 1024; // 10 MB maximum snapshot limit

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChangeSummary {
    pub file_path: String,
    pub status: String, // "created", "modified", "deleted"
    pub additions: usize,
    pub deletions: usize,
    pub diff_text: String,
    pub original_content: String,
    pub current_content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotEntry {
    pub status_at_snapshot: String, // "existed", "did_not_exist", "too_large"
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SnapshotManifest {
    pub files: HashMap<String, SnapshotEntry>,
}

pub fn get_snapshots_base_dir() -> PathBuf {
    if let Ok(app_data) = std::env::var("LOCALAPPDATA") {
        PathBuf::from(app_data).join("GeminiDesktop").join("snapshots")
    } else if let Ok(home) = std::env::var("USERPROFILE") {
        PathBuf::from(home).join(".geminidesktop").join("snapshots")
    } else {
        PathBuf::from(".").join("data").join("snapshots")
    }
}

pub fn get_session_snapshots_dir(session_id: &str) -> PathBuf {
    get_snapshots_base_dir().join(session_id)
}

pub fn normalize_relative_path(raw: &str, ws_path: &Path) -> Option<String> {
    let trimmed = raw.trim().trim_matches(|c| c == '"' || c == '\'' || c == '`');
    if trimmed.is_empty() || trimmed.contains("..") {
        return None;
    }

    let p_str = trimmed.replace('\\', "/");
    let ws_str = ws_path.to_string_lossy().replace('\\', "/");

    // Case-insensitive drive letter / prefix handling on Windows
    let norm = if p_str.to_lowercase().starts_with(&ws_str.to_lowercase()) {
        p_str[ws_str.len()..].trim_start_matches('/').to_string()
    } else {
        let p = Path::new(trimmed);
        if p.is_absolute() {
            if let Ok(rel) = p.strip_prefix(ws_path) {
                rel.to_string_lossy().replace('\\', "/")
            } else {
                return None;
            }
        } else {
            p_str
        }
    };

    let clean = norm.trim_start_matches("./").trim_start_matches('/').to_string();
    if clean.is_empty() || clean.starts_with(".git") || clean.starts_with("node_modules") {
        None
    } else {
        Some(clean)
    }
}

/// Determines whether a tool call represents a mutating file operation (write, edit, delete, patch).
/// Explicitly rejects read-only operations (reading, viewing, searching, listing).
pub fn is_mutating_tool(tool_name: Option<&str>, title: Option<&str>) -> bool {
    let name_lower = tool_name.unwrap_or("").to_lowercase();
    let title_lower = title.unwrap_or("").to_lowercase();

    // Explicit read-only tools
    for ro in &[
        "read_file", "read_text_file", "view_file", "grep_search", "search_files",
        "find_by_name", "list_directory", "list_dir", "dir_list", "fs/read_text_file",
        "fs/list_directory", "glob", "search",
    ] {
        if name_lower == *ro || name_lower.starts_with(ro) {
            return false;
        }
    }
    for ro_prefix in &["reading ", "viewing ", "searching ", "listing "] {
        if title_lower.starts_with(ro_prefix) {
            return false;
        }
    }

    // Mutating tool indicators
    for mut_name in &[
        "write_file", "edit_file", "replace_file_content", "apply_diff", "patch_file",
        "create_file", "delete_file", "remove_file", "append_to_file", "save_file",
        "fs/write_text_file",
    ] {
        if name_lower.contains(mut_name) {
            return true;
        }
    }

    for prefix in &[
        "writing to ", "editing ", "creating ", "overwriting ",
        "modifying ", "updating ", "patching ", "deleting ",
    ] {
        if title_lower.contains(prefix) {
            return true;
        }
    }

    false
}

/// Reads committed baseline of a file from Git HEAD if available.
pub fn get_git_head_content(ws_path: &Path, rel_path: &str) -> Option<Vec<u8>> {
    let mut cmd = std::process::Command::new("git");
    let git_target = format!("HEAD:{}", rel_path.replace('\\', "/"));
    cmd.args(&["show", &git_target])
        .current_dir(ws_path)
        .stderr(std::process::Stdio::null());
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    if let Ok(output) = cmd.output() {
        if output.status.success() {
            return Some(output.stdout);
        }
    }
    None
}

/// Checks if a file is tracked in Git.
pub fn is_file_tracked_by_git(ws_path: &Path, rel_path: &str) -> bool {
    let mut cmd = std::process::Command::new("git");
    let git_target = rel_path.replace('\\', "/");
    cmd.args(&["ls-files", "--error-unmatch", &git_target])
        .current_dir(ws_path)
        .stderr(std::process::Stdio::null());
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    cmd.status().map(|s| s.success()).unwrap_or(false)
}

pub fn load_manifest(session_dir: &Path) -> SnapshotManifest {
    let manifest_path = session_dir.join("manifest.json");
    if let Ok(data) = fs::read_to_string(&manifest_path) {
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        SnapshotManifest::default()
    }
}

pub fn save_manifest(session_dir: &Path, manifest: &SnapshotManifest) -> Result<(), String> {
    let manifest_path = session_dir.join("manifest.json");
    let json = serde_json::to_string_pretty(manifest).map_err(|e| e.to_string())?;
    fs::write(manifest_path, json).map_err(|e| format!("Failed to write snapshot manifest: {}", e))
}

pub fn is_binary_content(data: &[u8]) -> bool {
    data.iter().take(1024).any(|&b| b == 0)
}

pub fn extract_files_from_session_messages(
    messages: &[crate::database::Message],
    ws_path: &Path,
) -> Vec<String> {
    let mut files = Vec::new();
    for msg in messages {
        if let Some(ref tc_json) = msg.tool_calls_json {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(tc_json) {
                if let Some(arr) = val.as_array() {
                    for item in arr {
                        let title = item.get("title").and_then(|t| t.as_str());
                        let tool_name = item.get("tool_name")
                            .or_else(|| item.get("name"))
                            .or_else(|| item.get("tool"))
                            .and_then(|n| n.as_str());

                        // ONLY extract files if this is a mutating tool call!
                        // Read-only tools (read_file, grep_search, list_dir) must NEVER be tracked as changed files.
                        if !is_mutating_tool(tool_name, title) {
                            continue;
                        }

                        if let Some(t) = title {
                            for prefix in &[
                                "writing to ", "editing ", "creating ", "overwriting ",
                                "modifying ", "updating ", "patching ", "deleting ",
                            ] {
                                if let Some(idx) = t.to_lowercase().find(prefix) {
                                    let cand = &t[idx + prefix.len()..];
                                    if let Some(norm) = normalize_relative_path(cand, ws_path) {
                                        files.push(norm);
                                    }
                                }
                            }
                        }

                        if let Some(params) = item.get("parameters")
                            .or_else(|| item.get("input"))
                            .or_else(|| item.get("rawInput"))
                            .and_then(|p| p.as_object())
                        {
                            for key in &["filePath", "file_path", "path", "file", "target", "filename", "destination"] {
                                if let Some(p) = params.get(*key).and_then(|v| v.as_str()) {
                                    if let Some(norm) = normalize_relative_path(p, ws_path) {
                                        files.push(norm);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        // NOTE: We strictly DO NOT extract files from msg.role == "assistant" markdown backticks!
        // Mere discussion or mention of a file in an explanation must NEVER mark it as modified or created.
    }
    files.sort();
    files.dedup();
    files
}

/// Snapshots a file before it is modified or created by a Gemini tool call.
/// If already snapshotted during this session, the original baseline is strictly preserved.
/// Files exceeding MAX_SNAPSHOT_FILE_SIZE (10 MB) are tracked without heavy byte copying to prevent disk exhaustion.
pub fn snapshot_file_before_edit_internal(
    session_snapshots_dir: &Path,
    ws_path: &Path,
    raw_path: &str,
) -> Result<Option<String>, String> {
    let rel_path = match normalize_relative_path(raw_path, ws_path) {
        Some(p) => p,
        None => return Ok(None),
    };

    let files_dir = session_snapshots_dir.join("files");
    let _ = fs::create_dir_all(&files_dir);

    let mut manifest = load_manifest(session_snapshots_dir);

    // If already tracked in this session, keep existing baseline
    if manifest.files.contains_key(&rel_path) {
        return Ok(Some(rel_path));
    }

    let source_file = ws_path.join(&rel_path);
    let now = chrono::Utc::now().to_rfc3339();

    if source_file.exists() && source_file.is_file() {
        let file_size = source_file.metadata().map(|m| m.len()).unwrap_or(0);
        if file_size > MAX_SNAPSHOT_FILE_SIZE {
            // Guard against heavy files: do not clone 10MB+ files to disk
            manifest.files.insert(
                rel_path.clone(),
                SnapshotEntry {
                    status_at_snapshot: "too_large".to_string(),
                    timestamp: now,
                },
            );
        } else {
            let dest_file = files_dir.join(&rel_path);
            if let Some(parent) = dest_file.parent() {
                let _ = fs::create_dir_all(parent);
            }
            fs::copy(&source_file, &dest_file)
                .map_err(|e| format!("Failed to snapshot file {}: {}", rel_path, e))?;

            manifest.files.insert(
                rel_path.clone(),
                SnapshotEntry {
                    status_at_snapshot: "existed".to_string(),
                    timestamp: now,
                },
            );
        }
    } else {
        // File does not currently exist on disk. Check Git HEAD if in a git repository.
        if let Some(git_bytes) = get_git_head_content(ws_path, &rel_path) {
            let dest_file = files_dir.join(&rel_path);
            if let Some(parent) = dest_file.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::write(&dest_file, &git_bytes);
            manifest.files.insert(
                rel_path.clone(),
                SnapshotEntry {
                    status_at_snapshot: "existed".to_string(),
                    timestamp: now,
                },
            );
        } else {
            manifest.files.insert(
                rel_path.clone(),
                SnapshotEntry {
                    status_at_snapshot: "did_not_exist".to_string(),
                    timestamp: now,
                },
            );
        }
    }

    save_manifest(session_snapshots_dir, &manifest)?;
    Ok(Some(rel_path))
}

pub fn snapshot_file_before_edit(
    session_id: &str,
    ws_path: &Path,
    raw_path: &str,
) -> Result<Option<String>, String> {
    let session_dir = get_session_snapshots_dir(session_id);
    snapshot_file_before_edit_internal(&session_dir, ws_path, raw_path)
}

pub fn compute_file_changes_internal(
    session_snapshots_dir: &Path,
    ws_path: &Path,
) -> Result<Vec<FileChangeSummary>, String> {
    let manifest = load_manifest(session_snapshots_dir);
    let files_dir = session_snapshots_dir.join("files");

    let mut changes = Vec::new();

    for (rel_path, mut entry) in manifest.files {
        let current_disk_path = ws_path.join(&rel_path);

        // Safeguard: If entry claims "did_not_exist", check if Git HEAD has this file.
        // If it was already committed in Git HEAD, it was never a newly created file.
        if entry.status_at_snapshot == "did_not_exist" {
            if let Some(git_bytes) = get_git_head_content(ws_path, &rel_path) {
                let snap = files_dir.join(&rel_path);
                if let Some(parent) = snap.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                let _ = fs::write(&snap, &git_bytes);
                entry.status_at_snapshot = "existed".to_string();
            }
        }

        if entry.status_at_snapshot == "too_large" {
            changes.push(FileChangeSummary {
                file_path: rel_path.clone(),
                status: "modified".to_string(),
                additions: 0,
                deletions: 0,
                diff_text: format!("File exceeds 10 MB limit ({}). Full unified diff and automated snapshots disabled for heavy files.", rel_path),
                original_content: String::new(),
                current_content: String::new(),
            });
            continue;
        }

        if entry.status_at_snapshot == "did_not_exist" {
            // File was newly created in this session by the agent
            if current_disk_path.exists() && current_disk_path.is_file() {
                let bytes = fs::read(&current_disk_path).unwrap_or_default();
                if bytes.len() as u64 > MAX_SNAPSHOT_FILE_SIZE {
                    changes.push(FileChangeSummary {
                        file_path: rel_path.clone(),
                        status: "created".to_string(),
                        additions: 0,
                        deletions: 0,
                        diff_text: format!("Newly created heavy file (> 10 MB): {}", rel_path),
                        original_content: String::new(),
                        current_content: String::new(),
                    });
                    continue;
                }

                if is_binary_content(&bytes) {
                    changes.push(FileChangeSummary {
                        file_path: rel_path.clone(),
                        status: "created".to_string(),
                        additions: 0,
                        deletions: 0,
                        diff_text: format!("Binary file created ({} bytes)", bytes.len()),
                        original_content: String::new(),
                        current_content: String::new(),
                    });
                    continue;
                }

                let current_content = String::from_utf8_lossy(&bytes).to_string();
                let diff = TextDiff::from_lines("", current_content.as_str());

                let mut additions = 0;
                for change in diff.iter_all_changes() {
                    if change.tag() == ChangeTag::Insert {
                        additions += 1;
                    }
                }

                let diff_text = if current_content.is_empty() {
                    format!("--- /dev/null\n+++ b/{}\n@@ -0,0 +0,0 @@\n [Empty file created]\n", rel_path)
                } else {
                    diff
                        .unified_diff()
                        .context_radius(3)
                        .header(&format!("a/{}", rel_path), &format!("b/{}", rel_path))
                        .to_string()
                };

                changes.push(FileChangeSummary {
                    file_path: rel_path,
                    status: "created".to_string(),
                    additions,
                    deletions: 0,
                    diff_text,
                    original_content: String::new(),
                    current_content,
                });
            }
        } else {
            // File existed prior to the session
            let snapshot_file = files_dir.join(&rel_path);
            let has_snapshot = snapshot_file.exists();
            let original_bytes = if has_snapshot {
                fs::read(&snapshot_file).unwrap_or_default()
            } else if let Some(git_bytes) = get_git_head_content(ws_path, &rel_path) {
                git_bytes
            } else {
                Vec::new()
            };

            if current_disk_path.exists() && current_disk_path.is_file() {
                let current_bytes = fs::read(&current_disk_path).unwrap_or_default();

                if current_bytes.len() as u64 > MAX_SNAPSHOT_FILE_SIZE {
                    changes.push(FileChangeSummary {
                        file_path: rel_path.clone(),
                        status: "modified".to_string(),
                        additions: 0,
                        deletions: 0,
                        diff_text: format!("File exceeds 10 MB limit ({}).", rel_path),
                        original_content: String::new(),
                        current_content: String::new(),
                    });
                    continue;
                }

                if is_binary_content(&current_bytes) || is_binary_content(&original_bytes) {
                    if original_bytes != current_bytes {
                        changes.push(FileChangeSummary {
                            file_path: rel_path.clone(),
                            status: "modified".to_string(),
                            additions: 0,
                            deletions: 0,
                            diff_text: format!("Binary file modified ({} bytes -> {} bytes)", original_bytes.len(), current_bytes.len()),
                            original_content: String::new(),
                            current_content: String::new(),
                        });
                    }
                    continue;
                }

                let original_content = String::from_utf8_lossy(&original_bytes).to_string();
                let current_content = String::from_utf8_lossy(&current_bytes).to_string();

                // Only report as modified if contents actually differ!
                if original_content != current_content {
                    let diff = TextDiff::from_lines(original_content.as_str(), current_content.as_str());

                    let mut additions = 0;
                    let mut deletions = 0;
                    for change in diff.iter_all_changes() {
                        match change.tag() {
                            ChangeTag::Insert => additions += 1,
                            ChangeTag::Delete => deletions += 1,
                            _ => {}
                        }
                    }

                    let diff_text = diff
                        .unified_diff()
                        .context_radius(3)
                        .header(&format!("a/{}", rel_path), &format!("b/{}", rel_path))
                        .to_string();

                    changes.push(FileChangeSummary {
                        file_path: rel_path,
                        status: "modified".to_string(),
                        additions,
                        deletions,
                        diff_text,
                        original_content,
                        current_content,
                    });
                }
            } else {
                // File existed in snapshot, but was deleted from disk
                let original_content = String::from_utf8_lossy(&original_bytes).to_string();
                let diff = TextDiff::from_lines(original_content.as_str(), "");
                let mut deletions = 0;
                for change in diff.iter_all_changes() {
                    if change.tag() == ChangeTag::Delete {
                        deletions += 1;
                    }
                }

                let diff_text = diff
                    .unified_diff()
                    .context_radius(3)
                    .header(&format!("a/{}", rel_path), &format!("b/{}", rel_path))
                    .to_string();

                changes.push(FileChangeSummary {
                    file_path: rel_path,
                    status: "deleted".to_string(),
                    additions: 0,
                    deletions,
                    diff_text,
                    original_content,
                    current_content: String::new(),
                });
            }
        }
    }

    changes.sort_by(|a, b| a.file_path.cmp(&b.file_path));
    Ok(changes)
}

#[tauri::command]
pub fn get_session_file_changes(
    state: State<AppState>,
    session_id: String,
    workspace_path: String,
) -> Result<Vec<FileChangeSummary>, String> {
    let session_dir = get_session_snapshots_dir(&session_id);
    let ws_path = PathBuf::from(&workspace_path);
    if !ws_path.exists() {
        return Ok(Vec::new());
    }
    let _ = fs::create_dir_all(&session_dir);
    let files_dir = session_dir.join("files");

    // Sync only files specifically touched by mutating tools in this session's conversation messages.
    // This strictly scopes file changes to the current conversation and excludes external files
    // created or edited by other apps or outside the chat.
    if let Ok(messages) = state.db.list_messages(&session_id) {
        let touched_files = extract_files_from_session_messages(&messages, &ws_path);
        let mut manifest = load_manifest(&session_dir);
        let mut updated = false;
        let now = chrono::Utc::now().to_rfc3339();

        // 1. Audit existing manifest entries to repair false "did_not_exist" classifications
        let current_keys: Vec<String> = manifest.files.keys().cloned().collect();
        for f in current_keys {
            if let Some(entry) = manifest.files.get(&f).cloned() {
                if entry.status_at_snapshot == "did_not_exist" {
                    if let Some(git_bytes) = get_git_head_content(&ws_path, &f) {
                        let snap = files_dir.join(&f);
                        if let Some(parent) = snap.parent() {
                            let _ = fs::create_dir_all(parent);
                        }
                        let _ = fs::write(&snap, &git_bytes);
                        manifest.files.insert(
                            f.clone(),
                            SnapshotEntry {
                                status_at_snapshot: "existed".to_string(),
                                timestamp: entry.timestamp,
                            },
                        );
                        updated = true;
                    }
                }
            }
        }

        // 2. Add files touched by mutating tools that are not yet in manifest
        for f in &touched_files {
            if !manifest.files.contains_key(f) {
                let current_file = ws_path.join(f);
                let snap = files_dir.join(f);

                if let Some(git_bytes) = get_git_head_content(&ws_path, f) {
                    if let Some(parent) = snap.parent() {
                        let _ = fs::create_dir_all(parent);
                    }
                    let _ = fs::write(&snap, &git_bytes);
                    manifest.files.insert(
                        f.clone(),
                        SnapshotEntry {
                            status_at_snapshot: "existed".to_string(),
                            timestamp: now.clone(),
                        },
                    );
                    updated = true;
                } else if snap.exists() {
                    manifest.files.insert(
                        f.clone(),
                        SnapshotEntry {
                            status_at_snapshot: "existed".to_string(),
                            timestamp: now.clone(),
                        },
                    );
                    updated = true;
                } else if current_file.exists() && is_file_tracked_by_git(&ws_path, f) {
                    manifest.files.insert(
                        f.clone(),
                        SnapshotEntry {
                            status_at_snapshot: "existed".to_string(),
                            timestamp: now.clone(),
                        },
                    );
                    updated = true;
                } else if current_file.exists() {
                    manifest.files.insert(
                        f.clone(),
                        SnapshotEntry {
                            status_at_snapshot: "did_not_exist".to_string(),
                            timestamp: now.clone(),
                        },
                    );
                    updated = true;
                } else {
                    manifest.files.insert(
                        f.clone(),
                        SnapshotEntry {
                            status_at_snapshot: "existed".to_string(),
                            timestamp: now.clone(),
                        },
                    );
                    updated = true;
                }
            }
        }

        // 3. Purge manifest entries for files that were NOT touched by mutating tools
        // AND are clean/unmodified (e.g. from previous bugs where read tools or assistant
        // messages inserted entries into manifest.json).
        let manifest_keys_after: Vec<String> = manifest.files.keys().cloned().collect();
        for f in manifest_keys_after {
            if !touched_files.contains(&f) {
                let snap = files_dir.join(&f);
                let current_file = ws_path.join(&f);

                let is_clean = if let Some(git_bytes) = get_git_head_content(&ws_path, &f) {
                    if current_file.exists() {
                        fs::read(&current_file).map(|b| b == git_bytes).unwrap_or(false)
                    } else {
                        false
                    }
                } else if snap.exists() && current_file.exists() {
                    let snap_bytes = fs::read(&snap).unwrap_or_default();
                    let cur_bytes = fs::read(&current_file).unwrap_or_default();
                    snap_bytes == cur_bytes
                } else {
                    false
                };

                if is_clean {
                    manifest.files.remove(&f);
                    if snap.exists() {
                        let _ = fs::remove_file(&snap);
                    }
                    updated = true;
                }
            }
        }

        if updated {
            let _ = save_manifest(&session_dir, &manifest);
        }
    }

    compute_file_changes_internal(&session_dir, &ws_path)
}

#[tauri::command]
pub fn revert_session_file(
    session_id: String,
    workspace_path: String,
    file_path: String,
) -> Result<(), String> {
    let session_dir = get_session_snapshots_dir(&session_id);
    let ws_path = PathBuf::from(&workspace_path);
    if !session_dir.exists() || !ws_path.exists() {
        return Err("Snapshot directory or workspace not found".to_string());
    }

    revert_session_file_internal(&session_dir, &ws_path, &file_path)
}

pub fn revert_session_file_internal(
    session_dir: &Path,
    ws_path: &Path,
    file_path: &str,
) -> Result<(), String> {
    let mut manifest = load_manifest(session_dir);
    let entry = manifest
        .files
        .get(file_path)
        .cloned()
        .ok_or_else(|| format!("File {} not tracked in session snapshot", file_path))?;

    let current_disk_path = ws_path.join(file_path);

    // Safeguard: Check Git before deleting any file! Never remove tracked Git files.
    let git_tracked = is_file_tracked_by_git(ws_path, file_path)
        || get_git_head_content(ws_path, file_path).is_some();

    if entry.status_at_snapshot == "did_not_exist" && !git_tracked {
        // Truly newly created file outside of Git, safe to remove
        if current_disk_path.exists() {
            fs::remove_file(&current_disk_path)
                .map_err(|e| format!("Failed to delete created file {}: {}", file_path, e))?;
        }
    } else {
        // File existed before, restore from snapshot or Git HEAD
        let snapshot_file = session_dir.join("files").join(file_path);
        if snapshot_file.exists() {
            if let Some(parent) = current_disk_path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            fs::copy(&snapshot_file, &current_disk_path)
                .map_err(|e| format!("Failed to restore file {}: {}", file_path, e))?;
            let _ = fs::remove_file(&snapshot_file);
        } else if let Some(git_bytes) = get_git_head_content(ws_path, file_path) {
            if let Some(parent) = current_disk_path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            fs::write(&current_disk_path, git_bytes)
                .map_err(|e| format!("Failed to restore file from Git HEAD {}: {}", file_path, e))?;
        }
    }

    // Remove from manifest
    manifest.files.remove(file_path);
    save_manifest(session_dir, &manifest)?;

    Ok(())
}

#[tauri::command]
pub fn revert_all_session_files(
    session_id: String,
    workspace_path: String,
) -> Result<usize, String> {
    let session_dir = get_session_snapshots_dir(&session_id);
    let ws_path = PathBuf::from(&workspace_path);
    if !session_dir.exists() || !ws_path.exists() {
        return Ok(0);
    }

    let manifest = load_manifest(&session_dir);
    let mut count = 0;

    for (file_path, _) in manifest.files.clone() {
        if revert_session_file_internal(&session_dir, &ws_path, &file_path).is_ok() {
            count += 1;
        }
    }

    let _ = fs::remove_dir_all(&session_dir);
    Ok(count)
}

#[tauri::command]
pub fn clear_session_snapshots(session_id: String) -> Result<(), String> {
    let session_dir = get_session_snapshots_dir(&session_id);
    if session_dir.exists() {
        let _ = fs::remove_dir_all(&session_dir);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_and_diff_workflow() {
        let temp_dir = std::env::temp_dir().join("gemini_diff_test_suite_v2");
        let ws_dir = temp_dir.join("ws");
        let session_dir = temp_dir.join("snapshots");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&ws_dir).unwrap();
        fs::create_dir_all(&session_dir).unwrap();

        // 1. Existing file
        let existing_file = ws_dir.join("file1.txt");
        fs::write(&existing_file, "line 1\nline 2\n").unwrap();

        // Snapshot before edit
        let res = snapshot_file_before_edit_internal(&session_dir, &ws_dir, "file1.txt").unwrap();
        assert_eq!(res, Some("file1.txt".to_string()));

        // Modify file
        fs::write(&existing_file, "line 1\nline 2 modified\nline 3\n").unwrap();

        // 2. New file (did not exist at snapshot)
        let new_file = ws_dir.join("file2.txt");
        let res2 = snapshot_file_before_edit_internal(&session_dir, &ws_dir, "file2.txt").unwrap();
        assert_eq!(res2, Some("file2.txt".to_string()));
        fs::write(&new_file, "new line A\nnew line B\n").unwrap();

        // 3. Compute diff
        let changes = compute_file_changes_internal(&session_dir, &ws_dir).unwrap();
        assert_eq!(changes.len(), 2);

        let file1_change = changes.iter().find(|c| c.file_path == "file1.txt").unwrap();
        assert_eq!(file1_change.status, "modified");
        assert!(file1_change.additions >= 1);

        let file2_change = changes.iter().find(|c| c.file_path == "file2.txt").unwrap();
        assert_eq!(file2_change.status, "created");
        assert_eq!(file2_change.additions, 2);

        // 4. Revert file2 (created file should be deleted)
        revert_session_file_internal(&session_dir, &ws_dir, "file2.txt").unwrap();
        assert!(!new_file.exists());

        // 5. Revert file1 (modified file should be restored)
        revert_session_file_internal(&session_dir, &ws_dir, "file1.txt").unwrap();
        let restored = fs::read_to_string(&existing_file).unwrap();
        assert_eq!(restored, "line 1\nline 2\n");

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_extract_files_from_session_messages() {
        let temp_dir = std::env::temp_dir().join("gemini_diff_msg_test_v2");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();
        let dummy = temp_dir.join("dummy1.txt");
        fs::write(&dummy, "hello").unwrap();

        let msgs = vec![
            crate::database::Message {
                id: "m1".to_string(),
                session_id: "s1".to_string(),
                role: "assistant".to_string(),
                content: "Created empty file `dummy1.txt`.".to_string(),
                tool_calls_json: Some(serde_json::json!([
                    {
                        "id": "tc1",
                        "title": "Writing to dummy1.txt",
                        "status": "completed"
                    }
                ]).to_string()),
                token_count: 10,
                created_at: "2026-10-26T00:00:00Z".to_string(),
            }
        ];

        let extracted = extract_files_from_session_messages(&msgs, &temp_dir);
        assert_eq!(extracted, vec!["dummy1.txt".to_string()]);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_read_tools_and_assistant_backticks_not_extracted() {
        let temp_dir = std::env::temp_dir().join("gemini_diff_read_guard_test");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();
        let existing = temp_dir.join("api").join("credits.py");
        fs::create_dir_all(existing.parent().unwrap()).unwrap();
        fs::write(&existing, "# credits").unwrap();

        let msgs = vec![
            crate::database::Message {
                id: "m1".to_string(),
                session_id: "s1".to_string(),
                role: "assistant".to_string(),
                content: "Here is the summary of `api/credits.py` and `README.md`.".to_string(),
                tool_calls_json: Some(serde_json::json!([
                    {
                        "id": "tc1",
                        "title": "Reading api/credits.py",
                        "name": "read_file",
                        "parameters": {
                            "path": "api/credits.py"
                        },
                        "status": "completed"
                    },
                    {
                        "id": "tc2",
                        "title": "Searching repo",
                        "name": "grep_search",
                        "parameters": {
                            "query": "credits"
                        },
                        "status": "completed"
                    }
                ]).to_string()),
                token_count: 50,
                created_at: "2026-10-28T00:00:00Z".to_string(),
            }
        ];

        let extracted = extract_files_from_session_messages(&msgs, &temp_dir);
        assert!(extracted.is_empty(), "Read-only tools and assistant markdown must NEVER be extracted as touched files");

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_is_mutating_tool_detection() {
        // Read-only tools
        assert!(!is_mutating_tool(Some("read_file"), Some("Reading file")));
        assert!(!is_mutating_tool(Some("view_file"), None));
        assert!(!is_mutating_tool(Some("grep_search"), Some("Searching code")));
        assert!(!is_mutating_tool(Some("list_directory"), Some("Listing files")));
        assert!(!is_mutating_tool(Some("fs/read_text_file"), None));

        // Mutating tools
        assert!(is_mutating_tool(Some("write_file"), None));
        assert!(is_mutating_tool(Some("edit_file"), None));
        assert!(is_mutating_tool(Some("replace_file_content"), None));
        assert!(is_mutating_tool(Some("apply_diff"), None));
        assert!(is_mutating_tool(Some("patch_file"), None));
        assert!(is_mutating_tool(Some("fs/write_text_file"), None));
        assert!(is_mutating_tool(None, Some("Writing to src/main.rs")));
        assert!(is_mutating_tool(None, Some("Editing config.json")));
        assert!(is_mutating_tool(None, Some("Patching build.gradle")));
    }

    #[test]
    fn test_normalize_relative_path_case_insensitivity() {
        let ws = Path::new("C:/AIFiles/project");
        // Lowercase drive letter in raw, uppercase in ws
        let res1 = normalize_relative_path("c:/AIFiles/project/api/credits.py", ws);
        assert_eq!(res1, Some("api/credits.py".to_string()));

        // Backslashes
        let res2 = normalize_relative_path("C:\\AIFiles\\project\\src\\app.rs", ws);
        assert_eq!(res2, Some("src/app.rs".to_string()));

        // Relative path
        let res3 = normalize_relative_path("./api/credits.py", ws);
        assert_eq!(res3, Some("api/credits.py".to_string()));
    }

    #[test]
    fn test_repair_manifest_false_did_not_exist_on_git_file() {
        let temp_dir = std::env::temp_dir().join("gemini_diff_repair_test");
        let _ = fs::remove_dir_all(&temp_dir);
        let session_dir = temp_dir.join("snapshots");
        fs::create_dir_all(&session_dir).unwrap();

        // Use the current repository as ws_path, which is a git repo with README.md
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf();
        if repo_root.join(".git").exists() && repo_root.join("README.md").exists() {
            let mut manifest = SnapshotManifest::default();
            // Simulate a corrupt entry claiming committed README.md was newly created
            manifest.files.insert(
                "README.md".to_string(),
                SnapshotEntry {
                    status_at_snapshot: "did_not_exist".to_string(),
                    timestamp: "2026-10-28T00:00:00Z".to_string(),
                },
            );
            save_manifest(&session_dir, &manifest).unwrap();

            // Run compute_file_changes_internal
            let changes = compute_file_changes_internal(&session_dir, &repo_root).unwrap();

            // Because README.md in git matches disk, it must NOT be reported as created with all additions!
            let readme_change = changes.iter().find(|c| c.file_path == "README.md");
            if let Some(c) = readme_change {
                assert_ne!(c.status, "created", "Committed Git file must never be reported as created");
            }
        }

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
