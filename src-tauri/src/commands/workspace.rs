use crate::commands::AppState;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
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
}
