use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::RwLock;

#[cfg(target_os = "windows")]
use std::os::windows::io::AsRawHandle;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::HANDLE;
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
    SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
};

static CLI_PATH_CACHE: RwLock<Option<HashMap<String, Option<PathBuf>>>> = RwLock::new(None);

/// Clears the in-memory CLI executable path cache so changes (e.g. installs, updates, engine switches) are immediately detected.
pub fn invalidate_cli_path_cache() {
    if let Ok(mut cache) = CLI_PATH_CACHE.write() {
        *cache = None;
    }
}

fn cache_binary_result(name: &str, result: &Option<PathBuf>) {
    if let Ok(mut cache) = CLI_PATH_CACHE.write() {
        let map = cache.get_or_insert_with(HashMap::new);
        map.insert(name.to_string(), result.clone());
    }
}

/// Locates an executable by name on the system without spawning external processes.
/// Fast in-memory check with thread-safe caching: inspects well-known locations and system PATH.
/// Handles .exe, .cmd, and .bat files on Windows.
pub fn find_binary_by_name(name: &str) -> Option<PathBuf> {
    // 1. Check in-memory cache first
    if let Ok(cache) = CLI_PATH_CACHE.read() {
        if let Some(map) = cache.as_ref() {
            if let Some(cached) = map.get(name) {
                match cached {
                    Some(p) if p.is_file() => return Some(p.clone()),
                    None => return None,
                    _ => {} // Stale entry: re-detect
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        // 2. Check well-known global installation directories on Windows (fastest path, 0 subprocesses)
        let mut candidates = Vec::new();

        if let Ok(localappdata) = std::env::var("LOCALAPPDATA") {
            candidates.push(PathBuf::from(&localappdata).join(name).join("bin").join(format!("{}.exe", name)));
            candidates.push(PathBuf::from(&localappdata).join(name).join("bin").join(format!("{}.cmd", name)));
            candidates.push(PathBuf::from(&localappdata).join(name).join("bin").join(format!("{}.bat", name)));
            candidates.push(PathBuf::from(&localappdata).join("Microsoft").join("WinGet").join("Links").join(format!("{}.exe", name)));
            candidates.push(PathBuf::from(&localappdata).join("Programs").join(name).join(format!("{}.exe", name)));
            candidates.push(PathBuf::from(&localappdata).join("Programs").join(name).join("bin").join(format!("{}.exe", name)));
            candidates.push(PathBuf::from(&localappdata).join("pnpm").join(format!("{}.cmd", name)));
            candidates.push(PathBuf::from(&localappdata).join("Yarn").join("bin").join(format!("{}.cmd", name)));
        }

        if let Ok(appdata) = std::env::var("APPDATA") {
            candidates.push(PathBuf::from(&appdata).join("npm").join(format!("{}.cmd", name)));
            candidates.push(PathBuf::from(&appdata).join("npm").join(format!("{}.exe", name)));
            candidates.push(PathBuf::from(&appdata).join("npm").join(format!("{}.bat", name)));
        }

        if let Ok(userprofile) = std::env::var("USERPROFILE") {
            candidates.push(PathBuf::from(&userprofile).join(".gemini").join("bin").join(format!("{}.exe", name)));
            candidates.push(PathBuf::from(&userprofile).join(".gemini").join("bin").join(format!("{}.cmd", name)));
            candidates.push(PathBuf::from(&userprofile).join(".gemini").join("antigravity").join("bin").join(format!("{}.exe", name)));
            candidates.push(PathBuf::from(&userprofile).join(".gemini").join("antigravity").join("bin").join(format!("{}.cmd", name)));
            candidates.push(PathBuf::from(&userprofile).join(".gemini").join("antigravity").join("bin").join(format!("{}.bat", name)));
            candidates.push(PathBuf::from(&userprofile).join(".cargo").join("bin").join(format!("{}.exe", name)));
        }

        if let Ok(pf) = std::env::var("ProgramFiles") {
            candidates.push(PathBuf::from(&pf).join("nodejs").join(format!("{}.cmd", name)));
            candidates.push(PathBuf::from(&pf).join(name).join(format!("{}.exe", name)));
            candidates.push(PathBuf::from(&pf).join("Google").join(name).join(format!("{}.exe", name)));
            candidates.push(PathBuf::from(&pf).join("Google").join("Cloud SDK").join("google-cloud-sdk").join("bin").join(format!("{}.cmd", name)));
        }

        if let Ok(pf86) = std::env::var("ProgramFiles(x86)") {
            candidates.push(PathBuf::from(&pf86).join("nodejs").join(format!("{}.cmd", name)));
            candidates.push(PathBuf::from(&pf86).join("Google").join("Cloud SDK").join("google-cloud-sdk").join("bin").join(format!("{}.cmd", name)));
        }

        for candidate in candidates {
            if candidate.is_file() {
                let res = Some(candidate);
                cache_binary_result(name, &res);
                return res;
            }
        }

        // 3. Scan system PATH in memory without spawning where.exe (0 subprocesses)
        if let Some(path_var) = std::env::var_os("PATH") {
            let dirs: Vec<PathBuf> = std::env::split_paths(&path_var).collect();

            // Pass 1: Prioritize .exe binaries
            for dir in &dirs {
                let p = dir.join(format!("{}.exe", name));
                if p.is_file() {
                    let res = Some(p);
                    cache_binary_result(name, &res);
                    return res;
                }
            }

            // Pass 2: Prioritize .cmd scripts (standard npm/pnpm wrapper on Windows)
            for dir in &dirs {
                let p = dir.join(format!("{}.cmd", name));
                if p.is_file() {
                    let res = Some(p);
                    cache_binary_result(name, &res);
                    return res;
                }
            }

            // Pass 3: Prioritize .bat scripts
            for dir in &dirs {
                let p = dir.join(format!("{}.bat", name));
                if p.is_file() {
                    let res = Some(p);
                    cache_binary_result(name, &res);
                    return res;
                }
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Some(path_var) = std::env::var_os("PATH") {
            for dir in std::env::split_paths(&path_var) {
                let p = dir.join(name);
                if p.is_file() {
                    let res = Some(p);
                    cache_binary_result(name, &res);
                    return res;
                }
            }
        }
    }

    cache_binary_result(name, &None);
    None
}

/// Locates the real Gemini CLI executable on the system.
/// Handles .cmd, .bat, and .exe files on Windows and skips extensionless bash scripts.
pub fn find_gemini_executable() -> Option<PathBuf> {
    find_binary_by_name("gemini")
}

/// Locates the real Antigravity CLI (agy) executable on the system.
pub fn find_agy_executable() -> Option<PathBuf> {
    find_binary_by_name("agy")
}

/// Locates the appropriate CLI executable for the given engine ("agy" | "gemini").
#[allow(dead_code)]
pub fn find_cli_executable(engine: &str) -> Option<PathBuf> {
    if engine.trim().eq_ignore_ascii_case("agy") {
        find_agy_executable()
    } else {
        find_gemini_executable()
    }
}

pub struct ProcessSupervisor {
    #[cfg(target_os = "windows")]
    job_handle: Option<HANDLE>,
}

#[cfg(target_os = "windows")]
unsafe impl Send for ProcessSupervisor {}
#[cfg(target_os = "windows")]
unsafe impl Sync for ProcessSupervisor {}

impl ProcessSupervisor {
    pub fn new() -> Self {
        #[cfg(target_os = "windows")]
        {
            let job = unsafe {
                let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
                if !job.is_null() {
                    let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
                    info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
                    let result = SetInformationJobObject(
                        job,
                        JobObjectExtendedLimitInformation,
                        &info as *const _ as *const _,
                        std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                    );
                    if result != 0 {
                        Some(job)
                    } else {
                        None
                    }
                } else {
                    None
                }
            };
            Self { job_handle: job }
        }

        #[cfg(not(target_os = "windows"))]
        {
            Self {}
        }
    }

    /// Resolves a safe working directory complying with enterprise EDR standards.
    pub fn fallback_safe_dir() -> PathBuf {
        if let Ok(home) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
            let p = PathBuf::from(home);
            if p.is_dir() {
                return p;
            }
        }
        if let Ok(local_app) = std::env::var("LOCALAPPDATA") {
            let p = PathBuf::from(local_app).join("GeminiDesktop");
            let _ = std::fs::create_dir_all(&p);
            if p.is_dir() {
                return p;
            }
        }
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    }

    /// Reads and parses KEY=VALUE environment variables from a .env file.
    pub fn parse_dotenv_file(path: &Path) -> std::collections::HashMap<String, String> {
        let mut map = std::collections::HashMap::new();
        if let Ok(content) = std::fs::read_to_string(path) {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }
                let line_str = if let Some(stripped) = trimmed.strip_prefix("export ") {
                    stripped.trim()
                } else {
                    trimmed
                };
                if let Some((k, v)) = line_str.split_once('=') {
                    let key = k.trim().to_string();
                    let mut val = v.trim().to_string();
                    if (val.starts_with('"') && val.ends_with('"')) || (val.starts_with('\'') && val.ends_with('\'')) {
                        if val.len() >= 2 {
                            val = val[1..val.len() - 1].to_string();
                        }
                    }
                    if !key.is_empty() {
                        map.insert(key, val);
                    }
                }
            }
        }
        map
    }

    /// Partitions CLI flags for Antigravity (`agy [root-options] acp [acp-options]`).
    /// Root flags (`--model`, `--mode`, `--effort`) must precede the `acp` subcommand,
    /// whereas session/sandbox flags (`--agent`, `--add-dir`, `--sandbox`,
    /// `--dangerously-skip-permissions`) follow `acp`.
    pub fn partition_agy_cli_args(extra_args: &[String]) -> (Vec<String>, Vec<String>) {
        let mut root_args = Vec::new();
        let mut acp_args = Vec::new();
        let mut i = 0;
        while i < extra_args.len() {
            let arg = &extra_args[i];
            if arg == "--model" || arg == "--mode" || arg == "--effort" {
                root_args.push(arg.clone());
                if i + 1 < extra_args.len() {
                    i += 1;
                    root_args.push(extra_args[i].clone());
                }
            } else {
                acp_args.push(arg.clone());
            }
            i += 1;
        }
        (root_args, acp_args)
    }

    /// Builds a `std::process::Command` for invoking a CLI executable across platforms.
    /// Automatically wraps Windows `.cmd` / `.bat` shims via `cmd.exe /c` and sets `CREATE_NO_WINDOW`.
    pub fn build_cli_command<S: AsRef<OsStr>>(binary: &Path, args: &[S]) -> Command {
        let path_str = binary.to_string_lossy().to_string();
        let lower = path_str.to_lowercase();
        let is_batch = lower.ends_with(".cmd") || lower.ends_with(".bat");

        #[cfg(target_os = "windows")]
        let mut cmd = if is_batch {
            let mut c = Command::new("cmd.exe");
            c.arg("/c");
            c.arg(&path_str);
            for arg in args {
                c.arg(arg.as_ref());
            }
            c
        } else {
            let mut c = Command::new(&path_str);
            for arg in args {
                c.arg(arg.as_ref());
            }
            c
        };

        #[cfg(not(target_os = "windows"))]
        let mut cmd = {
            let mut c = Command::new(&path_str);
            for arg in args {
                c.arg(arg.as_ref());
            }
            c
        };

        #[cfg(target_os = "windows")]
        {
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }

        cmd
    }

    pub fn spawn_agent(
        &self,
        engine: &str,
        binary: &Path,
        working_dir: Option<PathBuf>,
        extra_args: &[String],
        extra_envs: &[(String, String)],
    ) -> Result<Child, String> {
        let is_agy = engine.trim().eq_ignore_ascii_case("agy");
        let path_str = binary.to_string_lossy().to_string();

        let mut spawn_args: Vec<String> = Vec::new();
        if is_agy {
            // Antigravity CLI ACP server signature: `agy [root-options] acp [acp-options]`
            // Root options (--model, --mode, --effort) must precede the `acp` subcommand.
            let (root_args, acp_args) = Self::partition_agy_cli_args(extra_args);
            spawn_args.extend(root_args);
            spawn_args.push("acp".to_string());
            spawn_args.extend(acp_args);
        } else {
            // Gemini CLI ACP signature: `gemini --skip-trust --acp [options]`
            spawn_args.push("--skip-trust".to_string());
            spawn_args.push("--acp".to_string());
            spawn_args.extend(extra_args.iter().cloned());
        }

        let mut cmd = Self::build_cli_command(binary, &spawn_args);

        // Inject global credentials and extra environment variables
        for (k, v) in extra_envs {
            cmd.env(k, v);
        }

        let safe_cwd = if let Some(dir) = working_dir {
            if dir.exists() {
                if !is_agy {
                    ensure_folder_trusted(&dir);
                }

                // Auto-inject workspace .env and .env.local into CLI agent and its child tools
                let dot_env = dir.join(".env");
                if dot_env.is_file() {
                    let envs = Self::parse_dotenv_file(&dot_env);
                    cmd.envs(&envs);
                }
                let dot_env_local = dir.join(".env.local");
                if dot_env_local.is_file() {
                    let envs = Self::parse_dotenv_file(&dot_env_local);
                    cmd.envs(&envs);
                }
                dir
            } else {
                Self::fallback_safe_dir()
            }
        } else {
            Self::fallback_safe_dir()
        };
        cmd.current_dir(&safe_cwd);

        // Ensure model environment variable matches CLI --model argument if provided
        if let Some(model_arg) = extra_args.windows(2).find(|w| w[0] == "--model").map(|w| &w[1]) {
            if is_agy {
                cmd.env("AGY_MODEL", model_arg);
            } else {
                cmd.env("GEMINI_MODEL", model_arg);
            }
        }

        // Ensure GEMINI_APPROVAL_MODE environment variable matches CLI --approval-mode argument if provided
        if !is_agy {
            if let Some(mode_arg) = extra_args.windows(2).find(|w| w[0] == "--approval-mode").map(|w| &w[1]) {
                cmd.env("GEMINI_APPROVAL_MODE", mode_arg);
            }
        }

        // Enforce non-interactive pagers and CI environment to prevent blocking child tools in autonomous mode
        cmd.env("CI", "true");
        cmd.env("PAGER", "cat");
        cmd.env("GIT_PAGER", "cat");

        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(target_os = "windows")]
        {
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }

        let engine_name = if is_agy { "agy" } else { "gemini" };
        let child = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn {} CLI ({}): {}", engine_name, path_str, e))?;

        #[cfg(target_os = "windows")]
        {
            if let Some(job) = self.job_handle {
                let process_handle = child.as_raw_handle();
                unsafe {
                    AssignProcessToJobObject(job, process_handle as _);
                }
            }
        }

        Ok(child)
    }

    #[allow(dead_code)]
    pub fn spawn_gemini(
        &self,
        gemini_binary: &Path,
        working_dir: Option<PathBuf>,
        extra_args: &[String],
        extra_envs: &[(String, String)],
    ) -> Result<Child, String> {
        self.spawn_agent("gemini", gemini_binary, working_dir, extra_args, extra_envs)
    }

    /// Gracefully kills a child process and its child processes on Windows or Unix without hanging the runtime.
    pub fn terminate_child(mut child: Child) {
        let pid = child.id();
        #[cfg(target_os = "windows")]
        {
            // On Windows, child processes spawned via cmd.exe /c need taskkill /F /T to terminate the entire process tree.
            let _ = Command::new("taskkill")
                .arg("/F")
                .arg("/T")
                .arg("/PID")
                .arg(pid.to_string())
                .creation_flags(0x08000000) // CREATE_NO_WINDOW
                .output();
        }
        let _ = child.kill();
        // Wait in a separate thread so it never blocks the async Tokio runtime if IO pipes are draining
        std::thread::spawn(move || {
            let _ = child.wait();
        });
    }
}

/// Adds a workspace directory to ~/.gemini/trustedFolders.json so Gemini CLI treats it as trusted.
pub fn ensure_folder_trusted(path: &Path) {
    let home = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")).ok();
    if let Some(h) = home {
        let gemini_dir = PathBuf::from(h).join(".gemini");
        let tf_path = gemini_dir.join("trustedFolders.json");
        let mut map: serde_json::Map<String, serde_json::Value> = if tf_path.is_file() {
            std::fs::read_to_string(&tf_path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .and_then(|v: serde_json::Value| v.as_object().cloned())
                .unwrap_or_default()
        } else {
            serde_json::Map::new()
        };

        let norm_path = path.to_string_lossy().replace('\\', "/").to_lowercase();
        let trimmed_path = norm_path.trim_end_matches('/').to_string();
        if !trimmed_path.is_empty() {
            map.insert(trimmed_path, serde_json::Value::String("TRUST_FOLDER".to_string()));
            if let Ok(json_str) = serde_json::to_string_pretty(&map) {
                let _ = std::fs::create_dir_all(&gemini_dir);
                let _ = std::fs::write(&tf_path, json_str);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_dotenv_file() {
        let temp_file = std::env::temp_dir().join(format!("test_dotenv_{}.env", uuid::Uuid::new_v4()));
        let sample = "
# Comment line
AZURE_DEVOPS_EXT_PAT=secret_pat_value_123
GEMINI_API_KEY=\"ai_key_456\"
EMPTY_LINE=

DEBUG='true'
# Another comment
";
        std::fs::write(&temp_file, sample).unwrap();
        let parsed = ProcessSupervisor::parse_dotenv_file(&temp_file);
        let _ = std::fs::remove_file(&temp_file);

        assert_eq!(parsed.get("AZURE_DEVOPS_EXT_PAT"), Some(&"secret_pat_value_123".to_string()));
        assert_eq!(parsed.get("GEMINI_API_KEY"), Some(&"ai_key_456".to_string()));
        assert_eq!(parsed.get("DEBUG"), Some(&"true".to_string()));
        assert_eq!(parsed.get("EMPTY_LINE"), Some(&"".to_string()));
        assert_eq!(parsed.get("# Comment line"), None);
    }

    #[test]
    fn test_model_arg_extraction() {
        let extra_args = vec!["--model".to_string(), "gemini-3.5-flash-lite".to_string()];
        let model = extra_args.windows(2).find(|w| w[0] == "--model").map(|w| &w[1]);
        assert_eq!(model, Some(&"gemini-3.5-flash-lite".to_string()));

        let preview_args = vec!["--model".to_string(), "gemini-3.8-flash".to_string()];
        let preview_model = preview_args.windows(2).find(|w| w[0] == "--model").map(|w| &w[1]);
        assert_eq!(preview_model, Some(&"gemini-3.8-flash".to_string()));

        let empty_args: Vec<String> = vec![];
        let no_model = empty_args.windows(2).find(|w| w[0] == "--model").map(|w| &w[1]);
        assert_eq!(no_model, None);
    }

    #[test]
    fn test_parse_dotenv_complex_cases() {
        let temp_file = std::env::temp_dir().join(format!("test_complex_dotenv_{}.env", uuid::Uuid::new_v4()));
        let sample = "export API_URL=https://api.example.com/v1\r\nexport DB_CONN=\"postgres://user:p=w@localhost:5432/mydb\"\r\nFLAG_MULTIPLE_EQUALS=alpha=beta=gamma\r\nSPACED_KEY = spaced_val\r\n";
        std::fs::write(&temp_file, sample).unwrap();
        let parsed = ProcessSupervisor::parse_dotenv_file(&temp_file);
        let _ = std::fs::remove_file(&temp_file);

        assert_eq!(parsed.get("API_URL"), Some(&"https://api.example.com/v1".to_string()));
        assert_eq!(parsed.get("DB_CONN"), Some(&"postgres://user:p=w@localhost:5432/mydb".to_string()));
        assert_eq!(parsed.get("FLAG_MULTIPLE_EQUALS"), Some(&"alpha=beta=gamma".to_string()));
        assert_eq!(parsed.get("SPACED_KEY"), Some(&"spaced_val".to_string()));
    }

    #[test]
    fn test_model_arg_extraction_edge_cases() {
        // Trailing --model with no value
        let trailing_model = vec!["--verbose".to_string(), "--model".to_string()];
        let model = trailing_model.windows(2).find(|w| w[0] == "--model").map(|w| &w[1]);
        assert_eq!(model, None);

        // Custom fine-tuned model path
        let custom_args = vec!["--model".to_string(), "custom/org-gemini-3.5-flash:nightly".to_string()];
        let custom_model = custom_args.windows(2).find(|w| w[0] == "--model").map(|w| &w[1]);
        assert_eq!(custom_model, Some(&"custom/org-gemini-3.5-flash:nightly".to_string()));
    }

    #[test]
    fn test_find_cli_executable_routing() {
        // Test that routing correctly delegates without crashing
        let _ = find_cli_executable("gemini");
        let _ = find_cli_executable("agy");
        let _ = find_cli_executable("unknown_fallback");
    }

    #[test]
    fn test_cli_path_cache_invalidation() {
        invalidate_cli_path_cache();
        let first = find_binary_by_name("nonexistent_binary_xyz_123");
        assert_eq!(first, None);

        // Subsequent lookup should return from cache
        let second = find_binary_by_name("nonexistent_binary_xyz_123");
        assert_eq!(second, None);

        // Cache invalidation should reset cache without panic
        invalidate_cli_path_cache();
    }

    #[test]
    fn test_partition_agy_cli_args_ordering() {
        let input = vec![
            "--model".to_string(),
            "gemini-4-hydrogen-high".to_string(),
            "--effort".to_string(),
            "xhigh".to_string(),
            "--mode".to_string(),
            "auto".to_string(),
            "--sandbox".to_string(),
            "--agent".to_string(),
            "reviewer".to_string(),
            "--add-dir".to_string(),
            "C:\\Shared".to_string(),
        ];
        let (root_args, acp_args) = ProcessSupervisor::partition_agy_cli_args(&input);
        assert_eq!(
            root_args,
            vec![
                "--model",
                "gemini-4-hydrogen-high",
                "--effort",
                "xhigh",
                "--mode",
                "auto"
            ]
        );
        assert_eq!(
            acp_args,
            vec!["--sandbox", "--agent", "reviewer", "--add-dir", "C:\\Shared"]
        );
    }

    #[test]
    fn test_build_cli_command_handles_batch_and_binary() {
        let exe_cmd = ProcessSupervisor::build_cli_command(
            Path::new("C:\\Users\\test\\agy.exe"),
            &["--version"],
        );
        assert_eq!(exe_cmd.get_program(), "C:\\Users\\test\\agy.exe");
        let exe_args: Vec<_> = exe_cmd.get_args().collect();
        assert_eq!(exe_args, vec!["--version"]);

        let cmd_shim = ProcessSupervisor::build_cli_command(
            Path::new("C:\\Users\\test\\AppData\\Roaming\\npm\\gemini.cmd"),
            &["models", "--output-format", "json"],
        );
        #[cfg(target_os = "windows")]
        {
            assert_eq!(cmd_shim.get_program(), "cmd.exe");
            let shim_args: Vec<_> = cmd_shim.get_args().collect();
            assert_eq!(
                shim_args,
                vec![
                    "/c",
                    "C:\\Users\\test\\AppData\\Roaming\\npm\\gemini.cmd",
                    "models",
                    "--output-format",
                    "json"
                ]
            );
        }
    }
}

