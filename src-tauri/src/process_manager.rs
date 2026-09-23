use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

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

/// Locates the real Gemini CLI executable on the system.
/// Handles .cmd, .bat, and .exe files on Windows and skips extensionless bash scripts.
pub fn find_gemini_executable() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        // 1. Run `where.exe gemini` and inspect all results
        if let Ok(output) = Command::new("where.exe").arg("gemini").output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let lines: Vec<&str> = stdout.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();

                // Pass 1: Prioritize .exe binaries
                for line in &lines {
                    let p = PathBuf::from(line);
                    if p.is_file() && line.to_lowercase().ends_with(".exe") {
                        return Some(p);
                    }
                }

                // Pass 2: Prioritize .cmd scripts (standard npm/pnpm wrapper on Windows)
                for line in &lines {
                    let p = PathBuf::from(line);
                    if p.is_file() && line.to_lowercase().ends_with(".cmd") {
                        return Some(p);
                    }
                }

                // Pass 3: Prioritize .bat scripts
                for line in &lines {
                    let p = PathBuf::from(line);
                    if p.is_file() && line.to_lowercase().ends_with(".bat") {
                        return Some(p);
                    }
                }
            }
        }

        // 2. Check well-known global installation directories on Windows
        let mut candidates = Vec::new();

        if let Ok(appdata) = std::env::var("APPDATA") {
            candidates.push(PathBuf::from(&appdata).join("npm").join("gemini.cmd"));
            candidates.push(PathBuf::from(&appdata).join("npm").join("gemini.exe"));
        }

        if let Ok(localappdata) = std::env::var("LOCALAPPDATA") {
            candidates.push(PathBuf::from(&localappdata).join("Programs").join("gemini").join("gemini.exe"));
            candidates.push(PathBuf::from(&localappdata).join("pnpm").join("gemini.cmd"));
            candidates.push(PathBuf::from(&localappdata).join("Yarn").join("bin").join("gemini.cmd"));
            candidates.push(PathBuf::from(&localappdata).join("Microsoft").join("WinGet").join("Links").join("gemini.exe"));
        }

        if let Ok(userprofile) = std::env::var("USERPROFILE") {
            candidates.push(PathBuf::from(&userprofile).join(".gemini").join("bin").join("gemini.exe"));
            candidates.push(PathBuf::from(&userprofile).join(".gemini").join("bin").join("gemini.cmd"));
            candidates.push(PathBuf::from(&userprofile).join(".cargo").join("bin").join("gemini.exe"));
        }

        if let Ok(pf) = std::env::var("ProgramFiles") {
            candidates.push(PathBuf::from(&pf).join("nodejs").join("gemini.cmd"));
        }

        if let Ok(pf86) = std::env::var("ProgramFiles(x86)") {
            candidates.push(PathBuf::from(&pf86).join("nodejs").join("gemini.cmd"));
        }

        for candidate in candidates {
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(output) = Command::new("which").arg("gemini").output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let first = stdout.lines().next().unwrap_or("").trim();
                if !first.is_empty() {
                    let p = PathBuf::from(first);
                    if p.is_file() {
                        return Some(p);
                    }
                }
            }
        }
    }

    None
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

    pub fn spawn_gemini(
        &self,
        gemini_binary: &Path,
        working_dir: Option<PathBuf>,
        extra_args: &[String],
    ) -> Result<Child, String> {
        let path_str = gemini_binary.to_string_lossy().to_string();
        let is_batch = path_str.to_lowercase().ends_with(".cmd")
                    || path_str.to_lowercase().ends_with(".bat");

        #[cfg(target_os = "windows")]
        let mut cmd = if is_batch {
            let mut c = Command::new("cmd.exe");
            c.arg("/c");
            c.arg(&path_str);
            c.arg("--acp");
            c
        } else {
            let mut c = Command::new(&path_str);
            c.arg("--acp");
            c
        };

        #[cfg(not(target_os = "windows"))]
        let mut cmd = {
            let mut c = Command::new(&path_str);
            c.arg("--acp");
            c
        };

        for arg in extra_args {
            cmd.arg(arg);
        }

        if let Some(dir) = working_dir {
            if dir.exists() {
                cmd.current_dir(&dir);

                // Auto-inject workspace .env and .env.local into Gemini CLI and its child tools
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
            }
        }

        // Ensure GEMINI_MODEL environment variable matches CLI --model argument if provided
        if let Some(model_arg) = extra_args.windows(2).find(|w| w[0] == "--model").map(|w| &w[1]) {
            cmd.env("GEMINI_MODEL", model_arg);
        }

        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(target_os = "windows")]
        {
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }

        let child = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn gemini CLI ({}): {}", path_str, e))?;

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
}

