use crate::commands::AppState;
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tauri::State;

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
    let q_norm = q_lower.replace('\\', "/");
    let match_rel_path = q_norm.contains('/');
    let mut entries = Vec::new();
    walk_search_dir(root, root, &q_norm, match_rel_path, 0, 10, max_results, cancellation, &mut entries);

    entries.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(entries)
}

fn walk_search_dir(
    root: &std::path::Path,
    current: &std::path::Path,
    query_lower: &str,
    match_rel_path: bool,
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

            // Plain queries match strictly against file_name to avoid folder-name noise;
            // path-qualified queries containing '/' match against relative_path.
            let is_match = if match_rel_path {
                relative.to_lowercase().contains(query_lower)
            } else {
                file_name.to_lowercase().contains(query_lower)
            };

            if is_match {
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
        walk_search_dir(root, &subdir, query_lower, match_rel_path, depth + 1, max_depth, max_results, cancellation, out);
        if out.len() >= max_results {
            return;
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspacePathValidation {
    pub is_valid: bool,
    pub exists: bool,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceContextProbe {
    pub validation: WorkspacePathValidation,
    pub suggested_name: String,
    pub has_git: bool,
    pub has_gemini_md: bool,
    pub has_agents_md: bool,
    pub has_gemini_skills: bool,
    pub has_agents_skills: bool,
    pub detected_stack: Vec<String>,
}

/// Checks if a local directory path falls inside root drives or EDR-restricted system locations.
fn check_restricted_workspace_path(p: &Path) -> Option<String> {
    let path_str = p.to_string_lossy().to_string();
    let norm = path_str.replace('/', "\\").to_lowercase();
    let clean_norm = norm.trim_end_matches('\\');

    // 1. Root drives (e.g. "c:", "c:\", "d:\")
    if (clean_norm.len() <= 2 && clean_norm.ends_with(':')) || norm == "c:\\" || norm == "d:\\" || norm == "/" {
        return Some("Cannot use root drive as a workspace. Please select a specific project or subfolder to prevent enterprise security and filesystem crawl alerts.".to_string());
    }

    // 2. Windows / System directories
    if let Ok(windir) = std::env::var("WINDIR") {
        let win_norm = windir.replace('/', "\\").to_lowercase();
        if clean_norm == win_norm || clean_norm.starts_with(&(win_norm + "\\")) {
            return Some("Access to Windows system directories is restricted for security and EDR threat prevention.".to_string());
        }
    } else if clean_norm.starts_with("c:\\windows") {
        return Some("Access to Windows system directories is restricted for security and EDR threat prevention.".to_string());
    }

    // 3. Program Files
    if clean_norm.starts_with("c:\\program files") {
        return Some("Program Files is read-only for standard users and monitored by enterprise EDR. Please select a development folder.".to_string());
    }

    // 4. Temporary volatile folders
    if let Ok(temp) = std::env::var("TEMP") {
        let temp_norm = temp.replace('/', "\\").to_lowercase();
        if clean_norm == temp_norm || clean_norm.starts_with(&(temp_norm + "\\")) {
            return Some("Temporary directories are monitored by CrowdStrike / EDR software for volatile script execution. Please choose a permanent project folder.".to_string());
        }
    }

    None
}

/// Returns true when a path targets a Windows UNC network share (`\\server\share` / `//server/share`),
/// Win32 device namespace (`\\.\...`, `\\?\...`), or NT object namespace (`\??\...`).
/// Rejecting these before touching the filesystem prevents SMB/NTLM credential leaks.
pub fn is_unsafe_windows_unc_or_device_path(raw: &str) -> bool {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return false;
    }
    let lower = trimmed.to_lowercase();
    let without_file = lower.strip_prefix("file://").unwrap_or(&lower);
    // file://server/share (not file:///C:/) has a non-empty authority host before '/'
    if lower.starts_with("file://") && !lower.starts_with("file:///") {
        let after = &lower["file://".len()..];
        let host = after.split(['/', '\\']).next().unwrap_or("");
        if !host.is_empty()
            && !host.eq_ignore_ascii_case("localhost")
            && !(host.len() == 2 && host.ends_with(':') && host.chars().next().map_or(false, |c| c.is_ascii_alphabetic()))
        {
            return true;
        }
    }
    trimmed.starts_with("\\\\")
        || trimmed.starts_with("//")
        || trimmed.starts_with("\\??\\")
        || trimmed.starts_with("/??/")
        || without_file.starts_with("\\\\")
        || without_file.starts_with("\\??\\")
}

#[tauri::command]
pub fn validate_workspace_path(path: String) -> Result<WorkspacePathValidation, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Ok(WorkspacePathValidation {
            is_valid: false,
            exists: false,
            error_message: Some("Workspace folder path cannot be empty.".to_string()),
        });
    }

    if is_unsafe_windows_unc_or_device_path(trimmed) {
        return Ok(WorkspacePathValidation {
            is_valid: false,
            exists: false,
            error_message: Some("Network (UNC), device, and NT object paths are blocked for Windows credential security. Please select a local directory.".to_string()),
        });
    }

    let p = PathBuf::from(trimmed);
    if !p.exists() || !p.is_dir() {
        if let Some(restricted) = check_restricted_workspace_path(&p) {
            return Ok(WorkspacePathValidation {
                is_valid: false,
                exists: false,
                error_message: Some(restricted),
            });
        }
        return Ok(WorkspacePathValidation {
            is_valid: false,
            exists: false,
            error_message: Some("Directory does not exist on disk.".to_string()),
        });
    }

    if let Some(restricted) = check_restricted_workspace_path(&p) {
        return Ok(WorkspacePathValidation {
            is_valid: false,
            exists: true,
            error_message: Some(restricted),
        });
    }

    Ok(WorkspacePathValidation {
        is_valid: true,
        exists: true,
        error_message: None,
    })
}

#[tauri::command]
pub fn create_workspace_directory(path: String) -> Result<WorkspacePathValidation, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("Workspace folder path cannot be empty.".to_string());
    }
    if is_unsafe_windows_unc_or_device_path(trimmed) {
        return Err("Network (UNC), device, and NT object paths cannot be created.".to_string());
    }

    let p = PathBuf::from(trimmed);
    if let Some(restricted) = check_restricted_workspace_path(&p) {
        return Err(restricted);
    }
    if p.exists() && !p.is_dir() {
        return Err("A file already exists at that path.".to_string());
    }

    std::fs::create_dir_all(&p)
        .map_err(|e| format!("Failed to create directory: {}", e))?;

    validate_workspace_path(trimmed.to_string())
}

#[tauri::command]
pub fn probe_workspace_context(path: String) -> Result<WorkspaceContextProbe, String> {
    let validation = validate_workspace_path(path.clone())?;
    let trimmed = path.trim();
    let p = PathBuf::from(trimmed);
    let suggested_name = p
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && !s.ends_with(':'))
        .unwrap_or_default();

    if !validation.is_valid || !validation.exists {
        return Ok(WorkspaceContextProbe {
            validation,
            suggested_name,
            has_git: false,
            has_gemini_md: false,
            has_agents_md: false,
            has_gemini_skills: false,
            has_agents_skills: false,
            detected_stack: Vec::new(),
        });
    }

    let has_git = p.join(".git").exists();
    let has_gemini_md = p.join("GEMINI.md").is_file() || p.join(".gemini").join("GEMINI.md").is_file();
    let has_agents_md = p.join("AGENTS.md").is_file();
    let has_gemini_skills = p.join(".gemini").join("skills").is_dir();
    let has_agents_skills = p.join(".agents").join("skills").is_dir();

    let mut detected_stack = Vec::new();
    if p.join("package.json").is_file() {
        detected_stack.push("Node / TS".to_string());
    }
    if p.join("Cargo.toml").is_file() || p.join("src-tauri").join("Cargo.toml").is_file() {
        detected_stack.push("Rust".to_string());
    }
    if p.join("pyproject.toml").is_file() || p.join("requirements.txt").is_file() {
        detected_stack.push("Python".to_string());
    }
    if p.join("go.mod").is_file() {
        detected_stack.push("Go".to_string());
    }
    if let Ok(entries) = std::fs::read_dir(&p) {
        let has_dotnet = entries
            .flatten()
            .take(60)
            .any(|e| {
                let name = e.file_name().to_string_lossy().to_lowercase();
                name.ends_with(".sln") || name.ends_with(".csproj")
            });
        if has_dotnet {
            detected_stack.push(".NET".to_string());
        }
    }

    Ok(WorkspaceContextProbe {
        validation,
        suggested_name,
        has_git,
        has_gemini_md,
        has_agents_md,
        has_gemini_skills,
        has_agents_skills,
        detected_stack,
    })
}

#[tauri::command]
pub async fn pick_workspace_directory(initial_path: Option<String>) -> Result<Option<String>, String> {
    #[cfg(target_os = "windows")]
    {
        let init_dir = initial_path
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty() && !is_unsafe_windows_unc_or_device_path(s))
            .and_then(|s| {
                let p = PathBuf::from(&s);
                if p.is_dir() {
                    Some(s)
                } else {
                    p.parent().filter(|par| par.is_dir()).map(|par| par.to_string_lossy().to_string())
                }
            })
            .unwrap_or_else(|| get_default_workspace_dir().unwrap_or_default());

        tokio::task::spawn_blocking(move || {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            let script = r#"
Add-Type -AssemblyName System.Windows.Forms
$owner = New-Object System.Windows.Forms.Form
$owner.TopMost = $true
$owner.StartPosition = 'Manual'
$owner.Size = New-Object System.Drawing.Size(0, 0)
$dialog = New-Object System.Windows.Forms.FolderBrowserDialog
$dialog.Description = 'Select Project Workspace Folder'
$dialog.UseDescriptionForTitle = $true
$dialog.ShowNewFolderButton = $true
if ($env:GEMINI_PICKER_INIT_DIR -and (Test-Path -LiteralPath $env:GEMINI_PICKER_INIT_DIR)) {
  $dialog.SelectedPath = $env:GEMINI_PICKER_INIT_DIR
}
$result = $dialog.ShowDialog($owner)
if ($result -eq [System.Windows.Forms.DialogResult]::OK -and $dialog.SelectedPath) {
  [Console]::OutputEncoding = [System.Text.Encoding]::UTF8
  Write-Output $dialog.SelectedPath
}
$dialog.Dispose()
$owner.Dispose()
"#;
            let mut cmd = std::process::Command::new("powershell.exe");
            cmd.args(["-NoProfile", "-NonInteractive", "-STA", "-Command", script]);
            cmd.env("GEMINI_PICKER_INIT_DIR", &init_dir);
            cmd.creation_flags(CREATE_NO_WINDOW);
            let output = cmd.output().map_err(|e| format!("Failed to launch folder picker: {}", e))?;
            if !output.status.success() {
                let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
                return Err(if err.is_empty() {
                    "Folder picker exited unexpectedly.".to_string()
                } else {
                    err
                });
            }
            let picked = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if picked.is_empty() {
                Ok(None)
            } else {
                Ok(Some(picked))
            }
        })
        .await
        .map_err(|e| format!("Folder picker task failed: {}", e))?
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = initial_path;
        Ok(None)
    }
}

#[tauri::command]
pub fn get_default_workspace_dir() -> Result<String, String> {
    if let Ok(home) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
        let projects = PathBuf::from(&home).join("Projects");
        if projects.is_dir() {
            return Ok(projects.to_string_lossy().to_string());
        }
        let dev = PathBuf::from(&home).join("Development");
        if dev.is_dir() {
            return Ok(dev.to_string_lossy().to_string());
        }
        return Ok(home);
    }
    Ok(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")).to_string_lossy().to_string())
}

#[tauri::command]
pub fn check_directory_exists(path: String) -> Result<bool, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Ok(false);
    }
    let p = PathBuf::from(trimmed);
    Ok(p.exists() && p.is_dir())
}

/// Percent-decodes a URI string (e.g. `%20` -> ` `).
fn percent_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '%' {
            let h1 = chars.next();
            let h2 = chars.next();
            if let (Some(c1), Some(c2)) = (h1, h2) {
                if let Ok(byte) = u8::from_str_radix(&format!("{}{}", c1, c2), 16) {
                    result.push(byte as char);
                    continue;
                }
                result.push('%');
                result.push(c1);
                result.push(c2);
                continue;
            }
        }
        result.push(ch);
    }
    result
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedTarget {
    pub path: PathBuf,
    pub is_url: bool,
    pub line: Option<usize>,
    pub col: Option<usize>,
}

/// Extracts base target path, line number, and column number from strings like:
/// - `api/main.py:179` -> ("api/main.py", Some(179), None)
/// - `api/main.py:179:15` -> ("api/main.py", Some(179), Some(15))
/// - `api/main.py:179-185` -> ("api/main.py", Some(179), None)
/// - `api/main.py#L179` -> ("api/main.py", Some(179), None)
/// - `api/main.py#L179-L185` -> ("api/main.py", Some(179), None)
/// - `api/main.py(179)` -> ("api/main.py", Some(179), None)
/// - `C:\path\api\main.py:179` -> ("C:\path\api\main.py", Some(179), None)
pub fn parse_target_line_col(input: &str) -> (String, Option<usize>, Option<usize>) {
    let mut s = input.trim();
    let mut parsed_line = None;
    let mut parsed_col = None;

    // 1. Fragment #L10-L20, #L15, #line-15
    if let Some(pos) = s.find('#') {
        let frag = &s[pos + 1..];
        s = &s[..pos];
        let digits: String = frag.chars().skip_while(|c| !c.is_ascii_digit()).take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(l) = digits.parse::<usize>() {
            parsed_line = Some(l);
        }
    }

    // 2. Query parameters (?...)
    if let Some(pos) = s.find('?') {
        s = &s[..pos];
    }

    // 3. Parentheses like (179) or (179:10) at end of string
    if s.ends_with(')') {
        if let Some(open_pos) = s.rfind('(') {
            let inside = s[open_pos + 1..s.len() - 1].trim();
            let parts: Vec<&str> = inside.split(':').collect();
            if parts.len() == 1 {
                if let Ok(l) = parts[0].trim().parse::<usize>() {
                    parsed_line = Some(l);
                    s = s[..open_pos].trim_end();
                } else {
                    let trimmed_before = s[..open_pos].trim_end();
                    if trimmed_before.contains('.') {
                        s = trimmed_before;
                    }
                }
            } else if parts.len() == 2 {
                if let (Ok(l), Ok(c)) = (parts[0].trim().parse::<usize>(), parts[1].trim().parse::<usize>()) {
                    parsed_line = Some(l);
                    parsed_col = Some(c);
                    s = s[..open_pos].trim_end();
                }
            } else {
                let trimmed_before = s[..open_pos].trim_end();
                if trimmed_before.contains('.') {
                    s = trimmed_before;
                }
            }
        }
    }

    // 4. Colon suffixes: :179, :179:15, :179-185
    if parsed_line.is_none() {
        let chars: Vec<char> = s.chars().collect();
        if let Some(last_colon) = s.rfind(':') {
            // Guard against Windows drive colon (e.g. C:\... at index 1, or /C:\... at index 2)
            let is_drive = (last_colon == 1 && chars.first().map_or(false, |c| c.is_ascii_alphabetic()))
                || (last_colon == 2 && chars.first() == Some(&'/') && chars.get(1).map_or(false, |c| c.is_ascii_alphabetic()));

            if !is_drive {
                let tail = &s[last_colon + 1..];
                if !tail.is_empty() && tail.chars().all(|c| c.is_ascii_digit()) {
                    let num = tail.parse::<usize>().unwrap();
                    let prefix = &s[..last_colon];
                    // Check if prefix also has a second line/col colon
                    if let Some(sec_colon) = prefix.rfind(':') {
                        let is_sec_drive = (sec_colon == 1 && chars.first().map_or(false, |c| c.is_ascii_alphabetic()))
                            || (sec_colon == 2 && chars.first() == Some(&'/') && chars.get(1).map_or(false, |c| c.is_ascii_alphabetic()));
                        if !is_sec_drive {
                            let mid = &prefix[sec_colon + 1..];
                            if !mid.is_empty() && mid.chars().all(|c| c.is_ascii_digit()) {
                                if let Ok(line_num) = mid.parse::<usize>() {
                                    parsed_line = Some(line_num);
                                    parsed_col = Some(num);
                                    s = &prefix[..sec_colon];
                                }
                            }
                        }
                    }
                    if parsed_line.is_none() {
                        parsed_line = Some(num);
                        s = prefix;
                    }
                } else if tail.contains('-') {
                    // Line range e.g. 179-185
                    let parts: Vec<&str> = tail.split('-').collect();
                    if parts.len() == 2 && parts[0].chars().all(|c| c.is_ascii_digit()) {
                        if let Ok(l) = parts[0].parse::<usize>() {
                            parsed_line = Some(l);
                            s = &s[..last_colon];
                        }
                    }
                }
            }
        }
    }

    (s.trim().to_string(), parsed_line, parsed_col)
}

/// Recursively searches for a file in the workspace by matching its relative path or filename.
pub fn find_matching_file_in_workspace(ws: &Path, rel_path: &Path) -> Option<PathBuf> {
    let target_name = rel_path.file_name()?.to_str()?;
    let rel_normalized = rel_path.to_string_lossy().replace('\\', "/").to_lowercase();
    let mut best_match: Option<PathBuf> = None;
    let mut stack = vec![(ws.to_path_buf(), 0)];
    let ignored = [".git", "node_modules", "target", "build", "dist", ".svelte-kit", "__pycache__", ".venv", "venv"];

    while let Some((dir, depth)) = stack.pop() {
        if depth > 6 { continue; }
        let rd = match std::fs::read_dir(&dir) {
            Ok(rd) => rd,
            Err(_) => continue,
        };
        for entry in rd.flatten() {
            let path = entry.path();
            let name = match entry.file_name().into_string() {
                Ok(s) => s,
                Err(_) => continue,
            };
            if ignored.iter().any(|&ign| ign.eq_ignore_ascii_case(&name)) {
                continue;
            }
            if path.is_dir() {
                stack.push((path, depth + 1));
            } else if path.is_file() {
                let path_norm = path.to_string_lossy().replace('\\', "/").to_lowercase();
                // Exact relative suffix match: e.g. path ends with "/api/main.py"
                if path_norm.ends_with(&format!("/{}", rel_normalized)) || path_norm.ends_with(&rel_normalized) {
                    return Some(path);
                }
                // If filename matches, keep as fallback
                if name.eq_ignore_ascii_case(target_name) && best_match.is_none() {
                    best_match = Some(path);
                }
            }
        }
    }
    best_match
}

/// Normalizes an incoming resource target string (which may be a `file:///` URI,
/// an HTTP/HTTPS web link, a relative workspace path, or an Antigravity Brain artifact).
pub fn normalize_open_target(target: &str, workspace_dir: Option<&Path>) -> Result<NormalizedTarget, String> {
    let trimmed = target.trim();
    if trimmed.is_empty() {
        return Err("Target path or URL is empty".to_string());
    }

    if is_unsafe_windows_unc_or_device_path(trimmed) {
        return Err("Blocked unsafe network (UNC), device, or NT object path".to_string());
    }

    // 1. Web URLs: http, https, mailto
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") || trimmed.starts_with("mailto:") {
        return Ok(NormalizedTarget {
            path: PathBuf::from(trimmed),
            is_url: true,
            line: None,
            col: None,
        });
    }

    // 2. Strip file:/// or file:// scheme
    let cleaned = if trimmed.starts_with("file:///") {
        &trimmed["file:///".len()..]
    } else if trimmed.starts_with("file://") {
        &trimmed["file://".len()..]
    } else {
        trimmed
    };

    // 3. Extract line and column numbers before decoding percent or resolving
    let (path_part, line, col) = parse_target_line_col(cleaned);

    // 4. URL percent-decode (e.g. %20 -> space)
    let mut decoded = percent_decode(&path_part);

    // 5. On Windows, if path starts with `/C:` or `/c:`, strip the leading slash
    #[cfg(target_os = "windows")]
    if decoded.starts_with('/') && decoded.len() > 2 && decoded.chars().nth(2) == Some(':') {
        decoded = decoded[1..].to_string();
    }

    let path = PathBuf::from(&decoded);

    // 6. If path directly exists on disk (e.g. absolute path or direct artifact file)
    if path.exists() {
        return Ok(NormalizedTarget {
            path,
            is_url: false,
            line,
            col,
        });
    }

    // 7. Expand ~/... or .gemini/... relative to USERPROFILE / HOME
    if decoded.starts_with('~') || decoded.starts_with(".gemini") {
        if let Some(home) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) {
            let home_path = PathBuf::from(home);
            let sub = if decoded.starts_with("~/") || decoded.starts_with("~\\") {
                &decoded[2..]
            } else {
                &decoded
            };
            let candidate = home_path.join(sub);
            if candidate.exists() {
                return Ok(NormalizedTarget {
                    path: candidate,
                    is_url: false,
                    line,
                    col,
                });
            }
        }
    }

    // 8. If relative, try resolving against workspace directory
    if path.is_relative() {
        if let Some(ws) = workspace_dir {
            let candidate = ws.join(&path);
            if candidate.exists() {
                return Ok(NormalizedTarget {
                    path: candidate,
                    is_url: false,
                    line,
                    col,
                });
            }

            // 8b. Fallback: Search workspace for matching relative path or filename
            if let Some(matched) = find_matching_file_in_workspace(ws, &path) {
                return Ok(NormalizedTarget {
                    path: matched,
                    is_url: false,
                    line,
                    col,
                });
            }

            return Ok(NormalizedTarget {
                path: candidate,
                is_url: false,
                line,
                col,
            });
        }
    }

    Ok(NormalizedTarget {
        path,
        is_url: false,
        line,
        col,
    })
}

/// Tries to launch an external editor (code, cursor, windsurf) with `--goto <path:line:col>`.
fn try_launch_editor_goto(editor: &str, goto_arg: &str) -> bool {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let mut cmd = std::process::Command::new("cmd.exe");
        cmd.args(["/c", editor, "-r", "-g", goto_arg]);
        cmd.creation_flags(CREATE_NO_WINDOW);
        if let Ok(mut child) = cmd.spawn() {
            if let Ok(status) = child.wait() {
                if status.success() {
                    return true;
                }
            }
        }
        false
    }
    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(mut child) = std::process::Command::new(editor)
            .arg("-r")
            .arg("-g")
            .arg(goto_arg)
            .spawn()
        {
            if let Ok(status) = child.wait() {
                if status.success() {
                    return true;
                }
            }
        }
        false
    }
}

#[tauri::command]
pub fn open_workspace_file(
    path: String,
    with_app: Option<String>,
    workspace_path: Option<String>,
    create_if_missing: Option<bool>,
) -> Result<(), String> {
    let ws_fallback = if workspace_path.is_none() {
        get_default_workspace_dir().ok().map(PathBuf::from)
    } else {
        None
    };
    let effective_ws = workspace_path.as_deref().map(Path::new).or(ws_fallback.as_deref());
    let target = normalize_open_target(&path, effective_ws)?;

    // 1. Web URL handling: open with system browser
    if target.is_url {
        let url_str = target.path.to_string_lossy().to_string();
        return open::that_detached(&url_str)
            .map_err(|e| format!("Failed to open URL {}: {}", url_str, e));
    }

    // 2. Local File / Directory handling
    if !target.path.exists() {
        if create_if_missing == Some(true) {
            if let Some(parent) = target.path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Err(e) = std::fs::write(&target.path, "") {
                return Err(format!("Failed to create missing file: {}", e));
            }
        } else {
            return Err(format!("File does not exist: {}", target.path.display()));
        }
    }

    let path_str = target.path.to_string_lossy().to_string();

    // Strategy A: Explicit with_app (e.g. notepad)
    if let Some(ref app) = with_app {
        if app.eq_ignore_ascii_case("notepad") {
            #[cfg(target_os = "windows")]
            {
                std::process::Command::new("notepad.exe")
                    .arg(&path_str)
                    .spawn()
                    .map(|_| ())
                    .map_err(|e| format!("Failed to launch Notepad: {}", e))?;
                return Ok(());
            }
        }
        return open::with_detached(&path_str, app).map_err(|e| format!("Failed to open with {}: {}", app, e));
    }

    // Strategy B: If line number is present, try code / cursor / windsurf with -g goto line
    if let Some(line) = target.line {
        let goto_arg = if let Some(col) = target.col {
            format!("{}:{}:{}", path_str, line, col)
        } else {
            format!("{}:{}", path_str, line)
        };

        if try_launch_editor_goto("code", &goto_arg) {
            return Ok(());
        }
        if try_launch_editor_goto("cursor", &goto_arg) {
            return Ok(());
        }
        if try_launch_editor_goto("windsurf", &goto_arg) {
            return Ok(());
        }
    }

    // Strategy C: Try system default application (file association)
    let res = open::that_detached(&path_str);
    if res.is_ok() {
        return Ok(());
    }

    // On Windows: try cmd.exe /c start "" "<path_str>"
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let mut cmd = std::process::Command::new("cmd.exe");
        cmd.args(["/c", "start", "", &path_str]);
        cmd.creation_flags(CREATE_NO_WINDOW);
        if let Ok(mut child) = cmd.spawn() {
            if let Ok(status) = child.wait() {
                if status.success() {
                    return Ok(());
                }
            }
        }
    }

    // Strategy D: Fallback to Notepad on Windows
    #[cfg(target_os = "windows")]
    {
        return std::process::Command::new("notepad.exe")
            .arg(&path_str)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("Failed to open file with default app and Notepad: {}", e));
    }
    #[cfg(not(target_os = "windows"))]
    {
        return res.map_err(|e| e.to_string());
    }
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

#[tauri::command]
pub async fn save_clipboard_image(
    workspace_path: Option<String>,
    image_base64: String,
    file_name: Option<String>,
) -> Result<String, String> {
    save_clipboard_image_internal(workspace_path.as_deref(), &image_base64, file_name.as_deref())
}

pub fn save_clipboard_image_internal(
    workspace_path: Option<&str>,
    image_base64: &str,
    file_name: Option<&str>,
) -> Result<String, String> {
    let raw_base64 = if let Some(idx) = image_base64.find(',') {
        &image_base64[idx + 1..]
    } else {
        image_base64
    };

    let bytes = base64::engine::general_purpose::STANDARD
        .decode(raw_base64.trim())
        .map_err(|e| format!("Failed to decode base64 image data: {}", e))?;

    if bytes.is_empty() {
        return Err("Decoded image data is empty".to_string());
    }

    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let clean_name = match file_name {
        Some(f) if !f.trim().is_empty() => {
            let sanitized: String = f.chars()
                .filter(|c| c.is_alphanumeric() || *c == '.' || *c == '_' || *c == '-')
                .collect();
            if sanitized.is_empty() {
                format!("screenshot_{}.png", timestamp)
            } else if !sanitized.contains('.') {
                format!("{}.png", sanitized)
            } else {
                sanitized
            }
        }
        _ => format!("screenshot_{}.png", timestamp),
    };

    let (target_file, return_path) = if let Some(ws) = workspace_path {
        let ws_buf = PathBuf::from(ws);
        if ws_buf.exists() && ws_buf.is_dir() {
            let attach_dir = ws_buf.join(".gemini").join("attachments");
            std::fs::create_dir_all(&attach_dir)
                .map_err(|e| format!("Failed to create attachments directory: {}", e))?;
            let file_path = attach_dir.join(&clean_name);
            let rel_path = format!(".gemini/attachments/{}", clean_name);
            (file_path, rel_path)
        } else {
            let fallback_dir = dirs_fallback_attachments()?;
            std::fs::create_dir_all(&fallback_dir)
                .map_err(|e| format!("Failed to create fallback attachments directory: {}", e))?;
            let file_path = fallback_dir.join(&clean_name);
            let ret = file_path.to_string_lossy().replace('\\', "/");
            (file_path, ret)
        }
    } else {
        let fallback_dir = dirs_fallback_attachments()?;
        std::fs::create_dir_all(&fallback_dir)
            .map_err(|e| format!("Failed to create fallback attachments directory: {}", e))?;
        let file_path = fallback_dir.join(&clean_name);
        let ret = file_path.to_string_lossy().replace('\\', "/");
        (file_path, ret)
    };

    std::fs::write(&target_file, &bytes)
        .map_err(|e| format!("Failed to write image file: {}", e))?;

    Ok(return_path)
}

fn dirs_fallback_attachments() -> Result<PathBuf, String> {
    if let Ok(appdata) = std::env::var("APPDATA") {
        Ok(PathBuf::from(appdata).join("gemini_desktop").join("attachments"))
    } else if let Ok(home) = std::env::var("USERPROFILE") {
        Ok(PathBuf::from(home).join(".gemini").join("attachments"))
    } else {
        Ok(std::env::temp_dir().join("gemini_desktop_attachments"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

        // 3b. Path-qualified search with '/' or '\' matches relative_path cleanly
        let qualified_res = search_workspace_files_internal(&temp_dir, "src/app", 100, None).expect("search failed");
        assert_eq!(qualified_res.len(), 1);
        assert_eq!(qualified_res[0].relative_path, "src/app.svelte");

        let qualified_win_res = search_workspace_files_internal(&temp_dir, "src\\app", 100, None).expect("search failed");
        assert_eq!(qualified_win_res.len(), 1);
        assert_eq!(qualified_win_res[0].relative_path, "src/app.svelte");

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
        let res = open_workspace_file(non_existent.to_string(), None, None, None);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("File does not exist"));
    }

    #[test]
    fn test_normalize_open_target_web_urls() {
        let target = normalize_open_target("https://github.com/google/gemini", None).unwrap();
        assert!(target.is_url);
        assert_eq!(target.path.to_str().unwrap(), "https://github.com/google/gemini");

        let target_http = normalize_open_target("http://localhost:5173", None).unwrap();
        assert!(target_http.is_url);
        assert_eq!(target_http.path.to_str().unwrap(), "http://localhost:5173");
    }

    #[test]
    fn test_normalize_open_target_file_uri_and_anchors() {
        // file:/// with line range anchor #L10-L20 and percent encoding
        let target = normalize_open_target("file:///C:/My%20Projects/doc.md#L10-L20", None).unwrap();
        assert!(!target.is_url);
        let path_str = target.path.to_str().unwrap().replace('\\', "/");
        assert_eq!(path_str, "C:/My Projects/doc.md");
        assert_eq!(target.line, Some(10));

        // file:// prefix
        let target2 = normalize_open_target("file://C:/notes.txt?v=1", None).unwrap();
        let path2_str = target2.path.to_str().unwrap().replace('\\', "/");
        assert_eq!(path2_str, "C:/notes.txt");
    }

    #[test]
    fn test_normalize_open_target_workspace_relative() {
        let temp_dir = std::env::temp_dir().join(format!("gemini_test_open_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let target_file = temp_dir.join("PROJECT_ANALYSIS.md");
        std::fs::write(&target_file, "# Analysis").unwrap();

        // Resolving relative "PROJECT_ANALYSIS.md" against workspace directory
        let target = normalize_open_target("PROJECT_ANALYSIS.md", Some(&temp_dir)).unwrap();
        assert!(!target.is_url);
        assert_eq!(target.path, target_file);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_normalize_open_target_line_and_col_numbers() {
        // api/main.py:179
        let (p1, l1, c1) = parse_target_line_col("api/main.py:179");
        assert_eq!(p1, "api/main.py");
        assert_eq!(l1, Some(179));
        assert_eq!(c1, None);

        // api/main.py:179:15
        let (p2, l2, c2) = parse_target_line_col("api/main.py:179:15");
        assert_eq!(p2, "api/main.py");
        assert_eq!(l2, Some(179));
        assert_eq!(c2, Some(15));

        // api/main.py:179-185
        let (p3, l3, c3) = parse_target_line_col("api/main.py:179-185");
        assert_eq!(p3, "api/main.py");
        assert_eq!(l3, Some(179));
        assert_eq!(c3, None);

        // Windows drive letter with line number: C:\project\api\main.py:673
        let (p4, l4, c4) = parse_target_line_col("C:\\project\\api\\main.py:673");
        assert_eq!(p4, "C:\\project\\api\\main.py");
        assert_eq!(l4, Some(673));
        assert_eq!(c4, None);

        // Parentheses: api/main.py(946)
        let (p5, l5, _) = parse_target_line_col("api/main.py(946)");
        assert_eq!(p5, "api/main.py");
        assert_eq!(l5, Some(946));

        // Function annotation: api/main.py (upload_file)
        let (p6, _, _) = parse_target_line_col("api/main.py (upload_file)");
        assert_eq!(p6, "api/main.py");
    }

    #[test]
    fn test_fuzzy_workspace_search_and_line_stripping() {
        let temp_dir = std::env::temp_dir().join(format!("gemini_test_fuzzy_{}", uuid::Uuid::new_v4()));
        let sub_dir = temp_dir.join("backend").join("api");
        std::fs::create_dir_all(&sub_dir).unwrap();
        let target_file = sub_dir.join("main.py");
        std::fs::write(&target_file, "print('hello')").unwrap();

        // Target specifies relative "api/main.py:179" which lives in backend/api/main.py
        let target = normalize_open_target("api/main.py:179", Some(&temp_dir)).unwrap();
        assert_eq!(target.line, Some(179));
        assert_eq!(target.path, target_file);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_open_workspace_file_create_if_missing() {
        let temp_dir = std::env::temp_dir().join(format!("gemini_test_create_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let missing_path = temp_dir.join("new_dir").join("new_file.py");

        assert!(!missing_path.exists());
        let res = open_workspace_file(
            missing_path.to_string_lossy().to_string(),
            None,
            Some(temp_dir.to_string_lossy().to_string()),
            Some(true),
        );
        // It should have created the file on disk
        assert!(missing_path.exists());
        let _ = res; // Opening might succeed or fallback to notepad

        let _ = std::fs::remove_dir_all(&temp_dir);
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
    fn test_check_directory_exists() {
        let temp_dir = std::env::temp_dir().join(format!("gemini_test_check_dir_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let file_path = temp_dir.join("some_file.txt");
        std::fs::write(&file_path, "hello").unwrap();

        // Existing directory → true
        let result = check_directory_exists(temp_dir.to_string_lossy().to_string()).unwrap();
        assert!(result);

        // Non-existent path → false
        let result = check_directory_exists("C:/definitely/nonexistent/path".to_string()).unwrap();
        assert!(!result);

        // File (not a directory) → false
        let result = check_directory_exists(file_path.to_string_lossy().to_string()).unwrap();
        assert!(!result);

        // Empty string → false
        let result = check_directory_exists("".to_string()).unwrap();
        assert!(!result);

        // Whitespace-only → false
        let result = check_directory_exists("   ".to_string()).unwrap();
        assert!(!result);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_validate_workspace_path() {
        // Empty path
        let empty_res = validate_workspace_path("".to_string()).unwrap();
        assert!(!empty_res.is_valid);
        assert!(!empty_res.exists);

        // Non-existent path
        let non_exist = validate_workspace_path("C:/definitely/not_a_real_folder_xyz_123".to_string()).unwrap();
        assert!(!non_exist.is_valid);
        assert!(!non_exist.exists);

        // Root drive rejection
        let root_c = validate_workspace_path("C:\\".to_string()).unwrap();
        assert!(!root_c.is_valid);
        assert!(root_c.error_message.unwrap().contains("root drive"));

        // Valid temp subfolder
        let temp_dir = std::env::temp_dir().join(format!("gemini_test_ws_val_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let val_temp = validate_workspace_path(temp_dir.to_string_lossy().to_string()).unwrap();
        // Since it's inside %TEMP%, it should be flagged as temporary directory
        assert!(!val_temp.is_valid);
        assert!(val_temp.error_message.unwrap().contains("Temporary"));

        let _ = std::fs::remove_dir_all(&temp_dir);

        // Valid user folder
        if let Ok(home) = std::env::var("USERPROFILE") {
            let home_val = validate_workspace_path(home).unwrap();
            assert!(home_val.is_valid);
            assert!(home_val.exists);
        }

        // UNC / device / NT object paths must be rejected without touching network
        let unc_res = validate_workspace_path(r"\\evil-server\share\repo".to_string()).unwrap();
        assert!(!unc_res.is_valid);
        assert!(unc_res.error_message.unwrap().contains("UNC"));

        let dev_res = validate_workspace_path(r"\\.\PhysicalDrive0".to_string()).unwrap();
        assert!(!dev_res.is_valid);

        let nt_res = validate_workspace_path(r"\??\C:\Users".to_string()).unwrap();
        assert!(!nt_res.is_valid);

        assert!(normalize_open_target(r"\\10.0.0.5\share\secret.txt", None).is_err());
        assert!(normalize_open_target("file://evil-host/share/doc.md", None).is_err());
    }

    #[test]
    fn test_get_default_workspace_dir() {
        let dir = get_default_workspace_dir().unwrap();
        assert!(!dir.is_empty());
    }

    #[test]
    fn test_save_clipboard_image_internal() {
        let temp_dir = std::env::temp_dir().join(format!("gemini_test_img_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        // 1. Valid base64 with data URL prefix
        let sample_bytes = b"fake-png-content";
        let encoded = format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(sample_bytes));

        let res = save_clipboard_image_internal(Some(&temp_dir.to_string_lossy()), &encoded, Some("test_shot.png")).unwrap();
        assert_eq!(res, ".gemini/attachments/test_shot.png");
        assert!(temp_dir.join(".gemini").join("attachments").join("test_shot.png").is_file());

        // 2. Empty data error
        let empty_res = save_clipboard_image_internal(Some(&temp_dir.to_string_lossy()), "", None);
        assert!(empty_res.is_err());

        // 3. Fallback without workspace
        let res_no_ws = save_clipboard_image_internal(None, &encoded, Some("fallback.png")).unwrap();
        assert!(res_no_ws.ends_with("fallback.png"));

        // 4. Invalid base64 payload returns error
        let invalid_res = save_clipboard_image_internal(Some(&temp_dir.to_string_lossy()), "data:image/png;base64,!!!invalid-base64!!!", None);
        assert!(invalid_res.is_err());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_probe_workspace_context_and_create_directory() {
        // Test safe non-temp project path under USERPROFILE if available, otherwise validate EDR guard rejects %TEMP%
        let temp_sub = std::env::temp_dir().join(format!("gemini_test_probe_{}", uuid::Uuid::new_v4()));
        let err_create = create_workspace_directory(temp_sub.to_string_lossy().to_string());
        assert!(err_create.is_err(), "Creating workspace directory inside %TEMP% must be rejected by EDR guard");

        // Probe a real directory containing git, AGENTS.md, package.json, Cargo.toml, and .gemini/skills
        let probe_dir = std::env::temp_dir().join(format!("gemini_probe_fixture_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(probe_dir.join(".git")).unwrap();
        std::fs::create_dir_all(probe_dir.join(".gemini").join("skills")).unwrap();
        std::fs::write(probe_dir.join("AGENTS.md"), "# Rules").unwrap();
        std::fs::write(probe_dir.join("package.json"), "{}").unwrap();

        let probe = probe_workspace_context(probe_dir.to_string_lossy().to_string()).unwrap();
        assert!(!probe.validation.is_valid); // %TEMP% flagged by EDR guard
        assert!(!probe.suggested_name.is_empty());

        // Probe the actual repository root (not in %TEMP%) to verify git/stack/instructions detection
        if let Ok(cwd) = std::env::current_dir() {
            let repo_root = if cwd.ends_with("src-tauri") {
                cwd.parent().unwrap_or(&cwd).to_path_buf()
            } else {
                cwd
            };
            if repo_root.join("package.json").is_file() {
                let repo_probe = probe_workspace_context(repo_root.to_string_lossy().to_string()).unwrap();
                assert!(repo_probe.validation.is_valid);
                assert!(repo_probe.validation.exists);
                assert!(repo_probe.has_git);
                assert!(repo_probe.has_agents_md);
                assert!(repo_probe.detected_stack.contains(&"Node / TS".to_string()));
                assert!(repo_probe.detected_stack.contains(&"Rust".to_string()));
            }
        }

        let _ = std::fs::remove_dir_all(&probe_dir);
    }
}
