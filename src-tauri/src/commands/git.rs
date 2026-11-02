use crate::commands::session_diff::FileChangeSummary;
use serde::{Deserialize, Serialize};
use similar::{ChangeTag, TextDiff};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GitFileItem {
    pub path: String,
    pub status: String, // "modified" | "added" | "deleted" | "untracked" | "renamed"
    pub is_staged: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitStatusResult {
    pub is_git_repo: bool,
    pub repo_name: String,
    pub active_branch: String,
    #[serde(default)]
    pub ahead: usize,
    #[serde(default)]
    pub behind: usize,
    pub staged: Vec<GitFileItem>,
    pub unstaged: Vec<GitFileItem>,
}

pub fn run_git_command(ws_path: &Path, args: &[&str]) -> Result<std::process::Output, String> {
    let mut cmd = Command::new("git");
    cmd.args(args).current_dir(ws_path);

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    cmd.output()
        .map_err(|e| format!("Failed to execute git command {:?}: {}", args, e))
}

/// Checks if directory is inside a git working tree
pub fn is_inside_git_repo(ws_path: &Path) -> bool {
    if let Ok(output) = run_git_command(ws_path, &["rev-parse", "--is-inside-work-tree"]) {
        if output.status.success() {
            let out_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            return out_str == "true";
        }
    }
    false
}

/// Retrieves the repository top-level folder name
pub fn get_git_repo_name(ws_path: &Path) -> String {
    if let Ok(output) = run_git_command(ws_path, &["rev-parse", "--show-toplevel"]) {
        if output.status.success() {
            let top_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let p = Path::new(&top_str);
            if let Some(file_name) = p.file_name() {
                return file_name.to_string_lossy().to_string();
            }
        }
    }
    ws_path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "Workspace".to_string())
}

/// Retrieves the active branch name or short commit hash
pub fn get_git_active_branch(ws_path: &Path) -> String {
    if let Ok(output) = run_git_command(ws_path, &["branch", "--show-current"]) {
        if output.status.success() {
            let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !branch.is_empty() {
                return branch;
            }
        }
    }
    // Fallback if in detached HEAD
    if let Ok(output) = run_git_command(ws_path, &["rev-parse", "--short", "HEAD"]) {
        if output.status.success() {
            let hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !hash.is_empty() {
                return format!("detached ({})", hash);
            }
        }
    }
    "HEAD".to_string()
}

/// Retrieves ahead/behind count against upstream tracking branch if configured
pub fn get_git_ahead_behind(ws_path: &Path) -> (usize, usize) {
    if let Ok(output) = run_git_command(ws_path, &["rev-list", "--left-right", "--count", "HEAD...@{upstream}"]) {
        if output.status.success() {
            let out_str = String::from_utf8_lossy(&output.stdout);
            let parts: Vec<&str> = out_str.trim().split_whitespace().collect();
            if parts.len() == 2 {
                let ahead = parts[0].parse::<usize>().unwrap_or(0);
                let behind = parts[1].parse::<usize>().unwrap_or(0);
                return (ahead, behind);
            }
        }
    }
    (0, 0)
}

/// Parses porcelain v1 status into staged and unstaged GitFileItems
pub fn parse_porcelain_v1(output: &str) -> (Vec<GitFileItem>, Vec<GitFileItem>) {
    let mut staged = Vec::new();
    let mut unstaged = Vec::new();

    for line in output.lines() {
        if line.len() < 3 {
            continue;
        }

        let bytes = line.as_bytes();
        let index_char = bytes[0] as char;
        let worktree_char = bytes[1] as char;
        let raw_path = &line[3..].trim();

        // Handle rename syntax: "orig -> dest"
        let path = if let Some(idx) = raw_path.find(" -> ") {
            raw_path[idx + 4..].trim().trim_matches('"').to_string()
        } else {
            raw_path.trim_matches('"').to_string()
        };

        if path.is_empty() {
            continue;
        }

        // Unmerged / conflict states (UU, AA, DD, AU, UA, etc.)
        if index_char == 'U' || worktree_char == 'U' || (index_char == 'A' && worktree_char == 'A') || (index_char == 'D' && worktree_char == 'D') {
            unstaged.push(GitFileItem {
                path: path.clone(),
                status: "conflict".to_string(),
                is_staged: false,
            });
            continue;
        }

        // Staged changes (Index)
        match index_char {
            'M' => staged.push(GitFileItem {
                path: path.clone(),
                status: "modified".to_string(),
                is_staged: true,
            }),
            'A' => staged.push(GitFileItem {
                path: path.clone(),
                status: "added".to_string(),
                is_staged: true,
            }),
            'D' => staged.push(GitFileItem {
                path: path.clone(),
                status: "deleted".to_string(),
                is_staged: true,
            }),
            'R' => staged.push(GitFileItem {
                path: path.clone(),
                status: "renamed".to_string(),
                is_staged: true,
            }),
            'C' => staged.push(GitFileItem {
                path: path.clone(),
                status: "added".to_string(),
                is_staged: true,
            }),
            _ => {}
        }

        // Unstaged changes (Worktree)
        match worktree_char {
            'M' => unstaged.push(GitFileItem {
                path: path.clone(),
                status: "modified".to_string(),
                is_staged: false,
            }),
            'A' => unstaged.push(GitFileItem {
                path: path.clone(),
                status: "added".to_string(),
                is_staged: false,
            }),
            'D' => unstaged.push(GitFileItem {
                path: path.clone(),
                status: "deleted".to_string(),
                is_staged: false,
            }),
            '?' => unstaged.push(GitFileItem {
                path: path.clone(),
                status: "untracked".to_string(),
                is_staged: false,
            }),
            _ => {}
        }
    }

    (staged, unstaged)
}

#[tauri::command]
pub async fn git_get_status(workspace_dir: String) -> Result<GitStatusResult, String> {
    tokio::task::spawn_blocking(move || {
        let ws_path = PathBuf::from(&workspace_dir);
        if !ws_path.exists() {
            return Err("Workspace directory does not exist".to_string());
        }

        if !is_inside_git_repo(&ws_path) {
            return Ok(GitStatusResult {
                is_git_repo: false,
                repo_name: ws_path
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "Workspace".to_string()),
                active_branch: String::new(),
                ahead: 0,
                behind: 0,
                staged: Vec::new(),
                unstaged: Vec::new(),
            });
        }

        let repo_name = get_git_repo_name(&ws_path);
        let active_branch = get_git_active_branch(&ws_path);
        let (ahead, behind) = get_git_ahead_behind(&ws_path);

        let output = run_git_command(&ws_path, &["status", "--porcelain=v1", "-uall"])?;
        if !output.status.success() {
            let err_msg = String::from_utf8_lossy(&output.stderr);
            return Err(format!("git status failed: {}", err_msg));
        }

        let stdout_str = String::from_utf8_lossy(&output.stdout);
        let (staged, unstaged) = parse_porcelain_v1(&stdout_str);

        Ok(GitStatusResult {
            is_git_repo: true,
            repo_name,
            active_branch,
            ahead,
            behind,
            staged,
            unstaged,
        })
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
pub async fn git_stage_file(workspace_dir: String, path: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let ws_path = PathBuf::from(&workspace_dir);
        let output = run_git_command(&ws_path, &["add", "--", &path])?;
        if !output.status.success() {
            let err_msg = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to stage file {}: {}", path, err_msg));
        }
        Ok(())
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
pub async fn git_unstage_file(workspace_dir: String, path: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let ws_path = PathBuf::from(&workspace_dir);
        let output = run_git_command(&ws_path, &["restore", "--staged", "--", &path])?;
        if !output.status.success() {
            let err_msg = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to unstage file {}: {}", path, err_msg));
        }
        Ok(())
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
pub async fn git_stage_all(workspace_dir: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let ws_path = PathBuf::from(&workspace_dir);
        let output = run_git_command(&ws_path, &["add", "-A"])?;
        if !output.status.success() {
            let err_msg = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to stage all files: {}", err_msg));
        }
        Ok(())
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
pub async fn git_unstage_all(workspace_dir: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let ws_path = PathBuf::from(&workspace_dir);
        let output = run_git_command(&ws_path, &["restore", "--staged", "."])?;
        if !output.status.success() {
            let err_msg = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to unstage all files: {}", err_msg));
        }
        Ok(())
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
pub async fn git_undo_change(
    workspace_dir: String,
    path: String,
    is_untracked: bool,
) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let ws_path = PathBuf::from(&workspace_dir);
        if is_untracked {
            // Clean untracked file or directory
            let output = run_git_command(&ws_path, &["clean", "-fd", "--", &path])?;
            if !output.status.success() {
                let err_msg = String::from_utf8_lossy(&output.stderr);
                return Err(format!("Failed to clean untracked file {}: {}", path, err_msg));
            }
        } else {
            // Unstage first if staged, then restore working tree
            let _ = run_git_command(&ws_path, &["restore", "--staged", "--", &path]);
            let output = run_git_command(&ws_path, &["restore", "--", &path])?;
            if !output.status.success() {
                let err_msg = String::from_utf8_lossy(&output.stderr);
                return Err(format!("Failed to restore file {}: {}", path, err_msg));
            }
        }
        Ok(())
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
pub async fn git_undo_all(workspace_dir: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let ws_path = PathBuf::from(&workspace_dir);
        // Unstage all, restore all tracked files, clean untracked
        let _ = run_git_command(&ws_path, &["restore", "--staged", "."]);
        let output_restore = run_git_command(&ws_path, &["restore", "."])?;
        if !output_restore.status.success() {
            let err_msg = String::from_utf8_lossy(&output_restore.stderr);
            return Err(format!("Failed to restore working directory: {}", err_msg));
        }
        let output_clean = run_git_command(&ws_path, &["clean", "-fd"])?;
        if !output_clean.status.success() {
            let err_msg = String::from_utf8_lossy(&output_clean.stderr);
            return Err(format!("Failed to clean untracked files: {}", err_msg));
        }
        Ok(())
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
pub async fn git_commit(
    workspace_dir: String,
    message: String,
    commit_all: bool,
) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        let ws_path = PathBuf::from(&workspace_dir);
        let trimmed_msg = message.trim();
        if trimmed_msg.is_empty() {
            return Err("Commit message cannot be empty".to_string());
        }

        let output = if commit_all {
            // Priority fallback: if user chose to commit all / unstaged, auto-stage tracked and untracked
            let stage_res = run_git_command(&ws_path, &["add", "-A"])?;
            if !stage_res.status.success() {
                let err = String::from_utf8_lossy(&stage_res.stderr);
                return Err(format!("Failed to stage changes before committing: {}", err));
            }
            run_git_command(&ws_path, &["commit", "-m", trimmed_msg])?
        } else {
            run_git_command(&ws_path, &["commit", "-m", trimmed_msg])?
        };

        if !output.status.success() {
            let err_msg = String::from_utf8_lossy(&output.stderr);
            let out_msg = String::from_utf8_lossy(&output.stdout);
            return Err(format!("Git commit failed: {} {}", err_msg, out_msg).trim().to_string());
        }

        let commit_summary = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(commit_summary)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

pub fn run_git_remote_command(ws_path: &Path, args: &[&str]) -> Result<std::process::Output, String> {
    let mut cmd = Command::new("git");
    cmd.args(args)
        .current_dir(ws_path)
        .env("GIT_TERMINAL_PROMPT", "0");

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    cmd.output()
        .map_err(|e| format!("Failed to execute git command {:?}: {}", args, e))
}

#[tauri::command]
pub async fn git_pull(workspace_dir: String) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        let ws_path = PathBuf::from(&workspace_dir);
        if !ws_path.exists() {
            return Err("Workspace directory does not exist".to_string());
        }
        if !is_inside_git_repo(&ws_path) {
            return Err("Workspace is not inside a Git repository".to_string());
        }

        let output = run_git_remote_command(&ws_path, &["pull"])?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            let out = String::from_utf8_lossy(&output.stdout);
            let full_err = format!("{}\n{}", err.trim(), out.trim()).trim().to_string();

            if full_err.contains("CONFLICT") || full_err.contains("merge failed") {
                return Err(format!("Pull resulted in merge conflicts. Please resolve conflicts: {}", full_err));
            }
            if full_err.contains("Authentication failed")
                || full_err.contains("could not read Username")
                || full_err.contains("Permission denied")
            {
                return Err("Authentication failed. Please verify your Git credentials or SSH keys.".to_string());
            }
            return Err(format!("Git pull failed: {}", full_err));
        }

        let summary = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(if summary.is_empty() {
            "Already up to date.".to_string()
        } else {
            summary
        })
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
pub async fn git_push(workspace_dir: String, set_upstream: bool) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        let ws_path = PathBuf::from(&workspace_dir);
        if !ws_path.exists() {
            return Err("Workspace directory does not exist".to_string());
        }
        if !is_inside_git_repo(&ws_path) {
            return Err("Workspace is not inside a Git repository".to_string());
        }

        let branch = get_git_active_branch(&ws_path);

        let output = if set_upstream {
            run_git_remote_command(&ws_path, &["push", "-u", "origin", &branch])?
        } else {
            let res = run_git_remote_command(&ws_path, &["push"])?;
            // If failed because no upstream tracking branch is set, auto-fallback with -u origin <branch>
            if !res.status.success() {
                let err_str = String::from_utf8_lossy(&res.stderr);
                if err_str.contains("no upstream branch") || err_str.contains("set-upstream") {
                    run_git_remote_command(&ws_path, &["push", "-u", "origin", &branch])?
                } else {
                    res
                }
            } else {
                res
            }
        };

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            let out = String::from_utf8_lossy(&output.stdout);
            let full_err = format!("{}\n{}", err.trim(), out.trim()).trim().to_string();

            if full_err.contains("Authentication failed")
                || full_err.contains("could not read Username")
                || full_err.contains("Permission denied")
            {
                return Err("Authentication failed. Please verify your Git credentials or SSH keys.".to_string());
            }
            if full_err.contains("non-fast-forward") || full_err.contains("[rejected]") {
                return Err("Push rejected: Remote repository has newer changes. Please pull first.".to_string());
            }
            return Err(format!("Git push failed: {}", full_err));
        }

        let err_msg = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let out_msg = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let summary = format!("{}\n{}", out_msg, err_msg).trim().to_string();
        Ok(if summary.is_empty() {
            "Pushed successfully.".to_string()
        } else {
            summary
        })
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
pub async fn git_get_file_diff(
    workspace_dir: String,
    path: String,
    is_staged: bool,
) -> Result<FileChangeSummary, String> {
    tokio::task::spawn_blocking(move || {
        let ws_path = PathBuf::from(&workspace_dir);
        let normalized = path.replace('\\', "/");

        // Read baseline content
        let original_content = if is_staged {
            // Diff against HEAD
            let mut cmd = Command::new("git");
            cmd.args(&["show", &format!("HEAD:{}", normalized)])
                .current_dir(&ws_path);
            #[cfg(target_os = "windows")]
            {
                use std::os::windows::process::CommandExt;
                cmd.creation_flags(0x08000000);
            }
            if let Ok(out) = cmd.output() {
                if out.status.success() {
                    String::from_utf8_lossy(&out.stdout).to_string()
                } else {
                    String::new()
                }
            } else {
                String::new()
            }
        } else {
            // Unstaged: Diff working tree against index (or HEAD if not in index)
            let mut cmd = Command::new("git");
            cmd.args(&["show", &format!(":{}", normalized)])
                .current_dir(&ws_path);
            #[cfg(target_os = "windows")]
            {
                use std::os::windows::process::CommandExt;
                cmd.creation_flags(0x08000000);
            }
            if let Ok(out) = cmd.output() {
                if out.status.success() {
                    String::from_utf8_lossy(&out.stdout).to_string()
                } else {
                    // Try HEAD
                    let mut cmd2 = Command::new("git");
                    cmd2.args(&["show", &format!("HEAD:{}", normalized)])
                        .current_dir(&ws_path);
                    #[cfg(target_os = "windows")]
                    {
                        use std::os::windows::process::CommandExt;
                        cmd2.creation_flags(0x08000000);
                    }
                    if let Ok(out2) = cmd2.output() {
                        if out2.status.success() {
                            String::from_utf8_lossy(&out2.stdout).to_string()
                        } else {
                            String::new()
                        }
                    } else {
                        String::new()
                    }
                }
            } else {
                String::new()
            }
        };

        // Read current content (index if staged, disk if unstaged)
        let current_content = if is_staged {
            let mut cmd = Command::new("git");
            cmd.args(&["show", &format!(":{}", normalized)])
                .current_dir(&ws_path);
            #[cfg(target_os = "windows")]
            {
                use std::os::windows::process::CommandExt;
                cmd.creation_flags(0x08000000);
            }
            if let Ok(out) = cmd.output() {
                if out.status.success() {
                    String::from_utf8_lossy(&out.stdout).to_string()
                } else {
                    String::new()
                }
            } else {
                String::new()
            }
        } else {
            let disk_file = ws_path.join(&path);
            if disk_file.exists() {
                std::fs::read_to_string(&disk_file).unwrap_or_default()
            } else {
                String::new()
            }
        };

        let diff = TextDiff::from_lines(&original_content, &current_content);
        let mut additions = 0;
        let mut deletions = 0;
        let mut diff_text = String::new();

        for (idx, group) in diff.grouped_ops(3).iter().enumerate() {
            if idx > 0 {
                diff_text.push_str("@@ ... @@\n");
            }
            for op in group {
                for change in diff.iter_changes(op) {
                    let sign = match change.tag() {
                        ChangeTag::Delete => {
                            deletions += 1;
                            "-"
                        }
                        ChangeTag::Insert => {
                            additions += 1;
                            "+"
                        }
                        ChangeTag::Equal => " ",
                    };
                    diff_text.push_str(sign);
                    diff_text.push_str(change.value());
                }
            }
        }

        let status = if original_content.is_empty() && !current_content.is_empty() {
            "created".to_string()
        } else if !original_content.is_empty() && current_content.is_empty() {
            "deleted".to_string()
        } else {
            "modified".to_string()
        };

        Ok(FileChangeSummary {
            file_path: normalized,
            status,
            additions,
            deletions,
            diff_text,
            original_content,
            current_content,
        })
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

/// Extracts story/ticket references, developer intent, and commit type from active chat or prompt hints
pub fn extract_commit_context(hint: &str) -> (Option<String>, Option<String>, Option<&'static str>) {
    let trimmed = hint.trim();
    if trimmed.is_empty() {
        return (None, None, None);
    }

    // 1. Look for ticket / story key in words:
    let mut ticket: Option<String> = None;
    let words: Vec<&str> = trimmed.split_whitespace().collect();

    for (i, word) in words.iter().enumerate() {
        let clean = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '#' && c != '-');

        // Matches e.g. #123, #45
        if clean.starts_with('#') && clean.len() > 1 && clean[1..].chars().all(|c| c.is_ascii_digit()) {
            ticket = Some(clean.to_string());
            break;
        }

        // Matches e.g. PROJ-104, JIRA-42, AUTH-1
        if let Some(dash_idx) = clean.find('-') {
            let prefix = &clean[..dash_idx];
            let suffix = &clean[dash_idx + 1..];
            if !prefix.is_empty()
                && prefix.len() <= 10
                && prefix.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
                && !suffix.is_empty()
                && suffix.chars().all(|c| c.is_ascii_digit())
            {
                ticket = Some(clean.to_string());
                break;
            }
        }

        // Matches e.g. "story 104", "issue #45", "ticket 22"
        let lower = word.to_lowercase();
        let stripped_word = lower.trim_matches(|c: char| !c.is_alphanumeric());
        if (stripped_word == "story" || stripped_word == "issue" || stripped_word == "task" || stripped_word == "ticket")
            && i + 1 < words.len()
        {
            let next_word = words[i + 1].trim_matches(|c: char| !c.is_alphanumeric() && c != '#');
            if !next_word.is_empty() {
                ticket = Some(if next_word.starts_with('#') {
                    next_word.to_string()
                } else if next_word.chars().all(|c| c.is_ascii_digit()) {
                    format!("#{}", next_word)
                } else {
                    next_word.to_string()
                });
                break;
            }
        }
    }

    // 2. Extract user intent from first meaningful sentence or line
    let first_line = trimmed.lines().next().unwrap_or(trimmed);

    // Strip @ mentions like @src/... or @git:diff
    let mention_filtered: Vec<&str> = first_line
        .split_whitespace()
        .filter(|w| !w.starts_with('@'))
        .collect();
    let mut clean_line = mention_filtered.join(" ");

    // Remove leading phrases like "Please ", "Can you ", "We need to ", "I want to "
    let lower_line = clean_line.to_lowercase();
    let prefixes = [
        "please ",
        "can you ",
        "could you ",
        "we need to ",
        "i want to ",
        "let's ",
        "task: ",
        "story: ",
        "issue: ",
    ];
    for p in prefixes {
        if lower_line.starts_with(p) {
            clean_line = clean_line[p.len()..].trim().to_string();
            break;
        }
    }

    // If clean_line contains punctuation (. / ! / ?), take up to first sentence
    if let Some(period_idx) = clean_line.find(|c: char| c == '.' || c == '!' || c == '?') {
        clean_line = clean_line[..period_idx].trim().to_string();
    }

    // Truncate to reasonable length for commit header (max 55 chars)
    if clean_line.len() > 55 {
        if let Some(space_idx) = clean_line[..55].rfind(' ') {
            clean_line = clean_line[..space_idx].trim().to_string();
        } else {
            clean_line.truncate(55);
        }
    }

    // Lowercase first letter of intent
    let intent = if !clean_line.is_empty() {
        let mut chars = clean_line.chars();
        let first = chars.next().unwrap();
        let rest: String = chars.collect();
        Some(format!("{}{}", first.to_lowercase(), rest))
    } else {
        None
    };

    // 3. Detect commit type from context
    let lower_all = trimmed.to_lowercase();
    let commit_type = if lower_all.contains("fix") || lower_all.contains("bug") || lower_all.contains("error") || lower_all.contains("crash") || lower_all.contains("issue") {
        Some("fix")
    } else if lower_all.contains("add") || lower_all.contains("create") || lower_all.contains("implement") || lower_all.contains("feature") || lower_all.contains("support") {
        Some("feat")
    } else if lower_all.contains("refactor") || lower_all.contains("clean") || lower_all.contains("simplify") || lower_all.contains("restructure") {
        Some("refactor")
    } else if lower_all.contains("test") || lower_all.contains("spec") || lower_all.contains("coverage") {
        Some("test")
    } else if lower_all.contains("doc") || lower_all.contains("readme") {
        Some("docs")
    } else if lower_all.contains("perf") || lower_all.contains("optimize") {
        Some("perf")
    } else {
        None
    };

    (ticket, intent, commit_type)
}

pub fn clean_commit_output(raw: &str) -> String {
    let mut text = raw.trim();
    if text.starts_with("```") {
        if let Some(first_newline) = text.find('\n') {
            text = text[first_newline + 1..].trim();
        }
    }
    if text.ends_with("```") {
        if let Some(last_backticks) = text.rfind("```") {
            text = text[..last_backticks].trim();
        }
    }
    text.to_string()
}

pub fn generate_commit_message_with_llm(
    ws_path: &Path,
    full_diff: &str,
    context_hint: Option<&str>,
) -> Result<String, String> {
    let bin = crate::process_manager::find_gemini_executable()
        .ok_or_else(|| "Gemini CLI executable not found".to_string())?;

    let bin_path_str = bin.to_string_lossy().to_string();
    let is_batch = bin_path_str.to_lowercase().ends_with(".cmd") || bin_path_str.to_lowercase().ends_with(".bat");

    let context_section = if let Some(hint) = context_hint {
        if !hint.trim().is_empty() {
            format!("\nContext & Task Intent:\n{}\n", hint.trim())
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let prompt = format!(
        "You are an expert Git commit generator. Analyze the following Git diff and context to generate a concise Conventional Commit message following the Conventional Commits specification (e.g. feat, fix, refactor, docs, test, chore).\n\
Rules:\n\
- Output ONLY the raw commit message (one-line subject, blank line, and brief bullet points).\n\
- Do NOT wrap your output in markdown code fences or backticks.\n\
- If an issue or story key (e.g. #123, PROJ-104) is present in the context, reference it in the commit subject or body.\n\
- Keep the subject line under 72 characters.\n\
{}\n\
Git Diff:\n{}",
        context_section,
        full_diff
    );

    #[cfg(target_os = "windows")]
    let mut cmd = if is_batch {
        let mut c = Command::new("cmd.exe");
        c.args([
            "/c",
            &bin_path_str,
            "--skip-trust",
            "-p",
            &prompt,
            "--model",
            "gemini-3.8-flash",
            "--output-format",
            "text",
        ]);
        c
    } else {
        let mut c = Command::new(&bin_path_str);
        c.args([
            "--skip-trust",
            "-p",
            &prompt,
            "--model",
            "gemini-3.8-flash",
            "--output-format",
            "text",
        ]);
        c
    };

    #[cfg(not(target_os = "windows"))]
    let mut cmd = {
        let mut c = Command::new(&bin_path_str);
        c.args([
            "--skip-trust",
            "-p",
            &prompt,
            "--model",
            "gemini-3.8-flash",
            "--output-format",
            "text",
        ]);
        c
    };

    cmd.current_dir(ws_path);
    cmd.env("CI", "true");
    cmd.env("PAGER", "cat");
    cmd.env("GIT_PAGER", "cat");
    cmd.env("GIT_TERMINAL_PROMPT", "0");

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    let output = cmd.output().map_err(|e| format!("Failed to run gemini CLI: {}", e))?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Gemini CLI exited with status {}: {}", output.status, err));
    }

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let cleaned = clean_commit_output(&stdout_str);
    if cleaned.trim().is_empty() {
        return Err("Gemini CLI returned empty output".to_string());
    }

    Ok(cleaned)
}

pub fn synthesize_heuristic_commit_message(
    stat_str: &str,
    full_diff_str: &str,
    context_hint: Option<&str>,
) -> Result<String, String> {
    let (hint_ticket, hint_intent, hint_type) = context_hint
        .map(extract_commit_context)
        .unwrap_or((None, None, None));

    let lines: Vec<&str> = stat_str.lines().collect();
    let changed_files: Vec<String> = lines
        .iter()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split('|').collect();
            if !parts.is_empty() {
                Some(parts[0].trim().to_string())
            } else {
                None
            }
        })
        .filter(|s| !s.is_empty() && !s.contains("changed,"))
        .collect();

    // Heuristic scope identification
    let scope = if let Some(first_file) = changed_files.first() {
        let path_parts: Vec<&str> = first_file.split('/').collect();
        if path_parts.len() > 1 {
            path_parts[path_parts.len() - 2].to_string()
        } else {
            path_parts[0]
                .split('.')
                .next()
                .unwrap_or("core")
                .to_string()
        }
    } else {
        "core".to_string()
    };

    let is_test = changed_files.iter().any(|f| f.contains("test"));
    let is_doc = changed_files.iter().any(|f| f.ends_with(".md") || f.contains("docs"));
    let commit_type = hint_type.unwrap_or_else(|| {
        if is_test {
            "test"
        } else if is_doc {
            "docs"
        } else if full_diff_str.contains("fix") || full_diff_str.contains("bug") || full_diff_str.contains("error") {
            "fix"
        } else if changed_files.iter().any(|f| f.contains("component") || f.contains("routes")) {
            "feat"
        } else {
            "chore"
        }
    });

    let file_summary = if changed_files.len() == 1 {
        format!("in {}", changed_files[0])
    } else if changed_files.len() <= 3 {
        format!("in {}", changed_files.join(", "))
    } else {
        format!("across {} files", changed_files.len())
    };

    let subject_action = if let Some(intent) = hint_intent {
        if let Some(ref t) = hint_ticket {
            if intent.contains(t.as_str()) {
                intent
            } else {
                format!("{} ({})", intent, t)
            }
        } else {
            intent
        }
    } else {
        format!("update implementation {}", file_summary)
    };

    let ticket_body = if let Some(ref t) = hint_ticket {
        format!("Story/Issue: {}\n\n", t)
    } else {
        String::new()
    };

    let commit_msg = format!(
        "{}({}): {}\n\n{}Summary of changes:\n{}",
        commit_type,
        scope,
        subject_action,
        ticket_body,
        changed_files
            .iter()
            .take(8)
            .map(|f| format!("- {}", f))
            .collect::<Vec<_>>()
            .join("\n")
    );

    Ok(commit_msg)
}

/// Generates an AI commit message based on staged (or unstaged) diff, enriched by optional chat/prompt context
#[tauri::command]
pub async fn git_generate_commit_message(
    workspace_dir: String,
    context_hint: Option<String>,
) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        let ws_path = PathBuf::from(&workspace_dir);

        // 1. Check staged diff first
        let staged_diff_out = run_git_command(&ws_path, &["diff", "--cached", "--stat"])?;
        let is_staged = staged_diff_out.status.success()
            && !String::from_utf8_lossy(&staged_diff_out.stdout).trim().is_empty();

        let (diff_args, stat_args): (&[&str], &[&str]) = if is_staged {
            (&["diff", "--cached"], &["diff", "--cached", "--stat"])
        } else {
            (&["diff", "HEAD"], &["diff", "HEAD", "--stat"])
        };

        let stat_out = run_git_command(&ws_path, stat_args)?;
        let stat_str = String::from_utf8_lossy(&stat_out.stdout).trim().to_string();

        let full_diff_out = run_git_command(&ws_path, diff_args)?;
        let mut full_diff_str = String::from_utf8_lossy(&full_diff_out.stdout).to_string();

        // If diff is still empty (e.g. only untracked files), check git status
        if stat_str.is_empty() && full_diff_str.trim().is_empty() {
            let status_out = run_git_command(&ws_path, &["status", "--porcelain=v1"])?;
            let status_str = String::from_utf8_lossy(&status_out.stdout).trim().to_string();
            if status_str.is_empty() {
                return Err("No changes detected in repository to generate commit message".to_string());
            }
            let (hint_ticket, hint_intent, _) = context_hint
                .as_deref()
                .map(extract_commit_context)
                .unwrap_or((None, None, None));
            let subject = if let Some(intent) = hint_intent {
                if let Some(ref t) = hint_ticket {
                    format!("chore: {} ({})", intent, t)
                } else {
                    format!("chore: {}", intent)
                }
            } else {
                "chore: update workspace files".to_string()
            };
            return Ok(format!(
                "{}\n\n- Updated:\n{}",
                subject,
                status_str
                    .lines()
                    .take(10)
                    .map(|l| format!("  - {}", l.trim()))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }

        // Cap diff length to prevent prompt explosion
        if full_diff_str.len() > 6000 {
            full_diff_str.truncate(6000);
            full_diff_str.push_str("\n... [diff truncated]");
        }

        // 2. PRIMARY: Try calling the underlying Gemini CLI LLM for true generative understanding
        if let Ok(llm_msg) = generate_commit_message_with_llm(&ws_path, &full_diff_str, context_hint.as_deref()) {
            if !llm_msg.trim().is_empty() {
                return Ok(llm_msg);
            }
        }

        // 3. FALLBACK: High-speed, resilient deterministic local heuristic engine
        synthesize_heuristic_commit_message(&stat_str, &full_diff_str, context_hint.as_deref())
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_porcelain_v1_empty() {
        let (staged, unstaged) = parse_porcelain_v1("");
        assert!(staged.is_empty());
        assert!(unstaged.is_empty());
    }

    #[test]
    fn test_parse_porcelain_v1_staged_and_unstaged() {
        let sample = "M  src/lib.rs\n M src/main.rs\nMM src/both.rs\nA  src/new.rs\n?? src/untracked.txt\nD  src/deleted.rs\n D src/unstaged_del.rs\nR  src/old.rs -> src/renamed.rs\n";
        let (staged, unstaged) = parse_porcelain_v1(sample);

        // Staged
        assert_eq!(staged.len(), 5); // lib.rs (M), both.rs (M), new.rs (A), deleted.rs (D), renamed.rs (R)
        assert!(staged.iter().any(|i| i.path == "src/lib.rs" && i.status == "modified"));
        assert!(staged.iter().any(|i| i.path == "src/both.rs" && i.status == "modified"));
        assert!(staged.iter().any(|i| i.path == "src/new.rs" && i.status == "added"));
        assert!(staged.iter().any(|i| i.path == "src/deleted.rs" && i.status == "deleted"));
        assert!(staged.iter().any(|i| i.path == "src/renamed.rs" && i.status == "renamed"));

        // Unstaged
        assert_eq!(unstaged.len(), 4); // main.rs (M), both.rs (M), untracked.txt (untracked), unstaged_del.rs (D)
        assert!(unstaged.iter().any(|i| i.path == "src/main.rs" && i.status == "modified"));
        assert!(unstaged.iter().any(|i| i.path == "src/both.rs" && i.status == "modified"));
        assert!(unstaged.iter().any(|i| i.path == "src/untracked.txt" && i.status == "untracked"));
        assert!(unstaged.iter().any(|i| i.path == "src/unstaged_del.rs" && i.status == "deleted"));
    }

    #[test]
    fn test_parse_porcelain_v1_quotes_and_spaces() {
        let sample = "?? \"src/my folder/file with spaces.txt\"\nA  \"src/another space.rs\"\n";
        let (staged, unstaged) = parse_porcelain_v1(sample);

        assert_eq!(staged.len(), 1);
        assert_eq!(staged[0].path, "src/another space.rs");
        assert_eq!(staged[0].status, "added");

        assert_eq!(unstaged.len(), 1);
        assert_eq!(unstaged[0].path, "src/my folder/file with spaces.txt");
        assert_eq!(unstaged[0].status, "untracked");
    }

    #[test]
    fn test_git_status_result_serialization() {
        let res = GitStatusResult {
            is_git_repo: true,
            repo_name: "test-repo".to_string(),
            active_branch: "main".to_string(),
            ahead: 2,
            behind: 1,
            staged: Vec::new(),
            unstaged: Vec::new(),
        };
        let json = serde_json::to_string(&res).unwrap();
        assert!(json.contains("\"ahead\":2"));
        assert!(json.contains("\"behind\":1"));

        // Backward compatibility: when ahead/behind missing from JSON, defaults to 0
        let backward_json = r#"{"is_git_repo":true,"repo_name":"repo","active_branch":"main","staged":[],"unstaged":[]}"#;
        let parsed: GitStatusResult = serde_json::from_str(backward_json).unwrap();
        assert_eq!(parsed.ahead, 0);
        assert_eq!(parsed.behind, 0);
    }

    #[tokio::test]
    async fn test_git_pull_nonexistent_dir() {
        let result = git_pull("C:\\nonexistent_workspace_dir_12345".to_string()).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[tokio::test]
    async fn test_git_push_nonexistent_dir() {
        let result = git_push("C:\\nonexistent_workspace_dir_12345".to_string(), false).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[test]
    fn test_extract_commit_context_jira_ticket_and_intent() {
        let hint = "Please fix JWT token expiration on mobile safari (JIRA-104) @src/auth/jwt.ts";
        let (ticket, intent, commit_type) = extract_commit_context(hint);
        assert_eq!(ticket, Some("JIRA-104".to_string()));
        assert!(intent.is_some());
        let intent_str = intent.unwrap().to_lowercase();
        assert!(intent_str.contains("jwt token expiration"));
        assert_eq!(commit_type, Some("fix"));
    }

    #[test]
    fn test_extract_commit_context_github_issue() {
        let hint = "Story #42: Add pull and push options to git changes window";
        let (ticket, intent, commit_type) = extract_commit_context(hint);
        assert_eq!(ticket, Some("#42".to_string()));
        assert!(intent.is_some());
        assert_eq!(commit_type, Some("feat"));
    }

    #[test]
    fn test_extract_commit_context_empty() {
        let (ticket, intent, commit_type) = extract_commit_context("");
        assert!(ticket.is_none());
        assert!(intent.is_none());
        assert!(commit_type.is_none());
    }

    #[test]
    fn test_clean_commit_output_markdown_fences() {
        let raw = "```markdown\nfeat(git): add commit and push\n\n- Updated git.rs\n```";
        let cleaned = clean_commit_output(raw);
        assert_eq!(cleaned, "feat(git): add commit and push\n\n- Updated git.rs");

        let plain = "fix(auth): fix token validation";
        assert_eq!(clean_commit_output(plain), "fix(auth): fix token validation");
    }

    #[test]
    fn test_synthesize_heuristic_commit_message() {
        let stat = " src/lib/components/ChatView.svelte | 12 ++\n src-tauri/src/commands/git.rs | 45 +++++\n";
        let diff = "+ let isPulling = true;";
        let msg = synthesize_heuristic_commit_message(stat, diff, Some("Fix JIRA-99: add pull action")).unwrap();
        assert!(msg.starts_with("fix("));
        assert!(msg.contains("JIRA-99"));
        assert!(msg.contains("Summary of changes:"));
    }
}
