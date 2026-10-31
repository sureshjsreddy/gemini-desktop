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
                staged: Vec::new(),
                unstaged: Vec::new(),
            });
        }

        let repo_name = get_git_repo_name(&ws_path);
        let active_branch = get_git_active_branch(&ws_path);

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

/// Generates an AI commit message based on staged (or unstaged) diff
#[tauri::command]
pub async fn git_generate_commit_message(workspace_dir: String) -> Result<String, String> {
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
            return Ok(format!(
                "chore: update workspace files\n\n- Updated:\n{}",
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

        // Synthesize an accurate, conventional commit message
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

        // Determine type: feat, fix, refactor, or chore
        let is_test = changed_files.iter().any(|f| f.contains("test"));
        let is_doc = changed_files.iter().any(|f| f.ends_with(".md") || f.contains("docs"));
        let commit_type = if is_test {
            "test"
        } else if is_doc {
            "docs"
        } else if full_diff_str.contains("fix") || full_diff_str.contains("bug") || full_diff_str.contains("error") {
            "fix"
        } else if changed_files.iter().any(|f| f.contains("component") || f.contains("routes")) {
            "feat"
        } else {
            "chore"
        };

        let file_summary = if changed_files.len() == 1 {
            format!("in {}", changed_files[0])
        } else if changed_files.len() <= 3 {
            format!("in {}", changed_files.join(", "))
        } else {
            format!("across {} files", changed_files.len())
        };

        let commit_msg = format!(
            "{}({}): update implementation {}\n\nSummary of changes:\n{}",
            commit_type,
            scope,
            file_summary,
            changed_files
                .iter()
                .take(8)
                .map(|f| format!("- {}", f))
                .collect::<Vec<_>>()
                .join("\n")
        );

        Ok(commit_msg)
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
}
