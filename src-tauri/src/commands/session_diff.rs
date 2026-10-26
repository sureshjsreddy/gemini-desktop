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
    let p = Path::new(trimmed);
    let norm = if p.is_absolute() {
        if let Ok(rel) = p.strip_prefix(ws_path) {
            rel.to_string_lossy().replace('\\', "/")
        } else {
            return None;
        }
    } else {
        trimmed.replace('\\', "/")
    };

    let clean = norm.trim_start_matches("./").trim_start_matches('/').to_string();
    if clean.is_empty() || clean.starts_with(".git") || clean.starts_with("node_modules") {
        None
    } else {
        Some(clean)
    }
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
                        if let Some(title) = item.get("title").and_then(|t| t.as_str()) {
                            for prefix in &[
                                "writing to ", "editing ", "creating ", "overwriting ",
                                "modifying ", "updating ",
                            ] {
                                if let Some(idx) = title.to_lowercase().find(prefix) {
                                    let cand = &title[idx + prefix.len()..];
                                    if let Some(norm) = normalize_relative_path(cand, ws_path) {
                                        files.push(norm);
                                    }
                                }
                            }
                        }
                        if let Some(params) = item.get("parameters").and_then(|p| p.as_object()) {
                            for key in &["filePath", "file_path", "path", "file", "target", "filename"] {
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
        if msg.role == "assistant" {
            for part in msg.content.split('`') {
                let trimmed = part.trim();
                if (trimmed.contains('\\') || trimmed.contains('/')) && (trimmed.contains('.') || trimmed.contains(':')) {
                    if let Some(norm) = normalize_relative_path(trimmed, ws_path) {
                        let full = ws_path.join(&norm);
                        if full.exists() {
                            files.push(norm);
                        }
                    }
                }
            }
        }
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
        manifest.files.insert(
            rel_path.clone(),
            SnapshotEntry {
                status_at_snapshot: "did_not_exist".to_string(),
                timestamp: now,
            },
        );
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

    for (rel_path, entry) in manifest.files {
        let current_disk_path = ws_path.join(&rel_path);

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

                if !has_snapshot || original_content != current_content {
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

    // Sync only files specifically touched by tools in this session's conversation messages.
    // This strictly scopes file changes to the current conversation and excludes external files
    // created or edited by other apps or outside the chat.
    if let Ok(messages) = state.db.list_messages(&session_id) {
        let touched_files = extract_files_from_session_messages(&messages, &ws_path);
        let mut manifest = load_manifest(&session_dir);
        let mut updated = false;
        let now = chrono::Utc::now().to_rfc3339();
        for f in touched_files {
            if !manifest.files.contains_key(&f) {
                let current_file = ws_path.join(&f);
                let status_at_snapshot = if current_file.exists() {
                    let snap = session_dir.join("files").join(&f);
                    if snap.exists() {
                        "existed".to_string()
                    } else {
                        "did_not_exist".to_string()
                    }
                } else {
                    "existed".to_string()
                };
                manifest.files.insert(
                    f,
                    SnapshotEntry {
                        status_at_snapshot,
                        timestamp: now.clone(),
                    },
                );
                updated = true;
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

    if entry.status_at_snapshot == "did_not_exist" {
        // File was created in this session, reverting means removing it
        if current_disk_path.exists() {
            fs::remove_file(&current_disk_path)
                .map_err(|e| format!("Failed to delete created file {}: {}", file_path, e))?;
        }
    } else {
        // File existed before, restore from snapshot
        let snapshot_file = session_dir.join("files").join(file_path);
        if snapshot_file.exists() {
            if let Some(parent) = current_disk_path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            fs::copy(&snapshot_file, &current_disk_path)
                .map_err(|e| format!("Failed to restore file {}: {}", file_path, e))?;
            let _ = fs::remove_file(&snapshot_file);
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
}
