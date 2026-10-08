use serde::{Deserialize, Serialize};
use std::process::Command;
use tauri::AppHandle;

pub const WINGET_PACKAGE_ID: &str = "SureshJanakiReddy.GeminiDesktop";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub update_available: bool,
    pub current_version: String,
    pub latest_version: String,
    pub package_id: String,
    pub release_url: Option<String>,
}

pub fn parse_winget_version(output: &str) -> Option<String> {
    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Version:") {
            if let Some((_, ver)) = trimmed.split_once(':') {
                let v = ver.trim();
                if !v.is_empty() {
                    return Some(v.to_string());
                }
            }
        }
    }
    None
}

pub fn is_newer_version(latest: &str, current: &str) -> bool {
    let clean_latest = latest.trim().trim_start_matches('v');
    let clean_current = current.trim().trim_start_matches('v');

    let parse_parts = |s: &str| -> Vec<u32> {
        s.split('.')
            .map(|p| p.chars().take_while(|c| c.is_ascii_digit()).collect::<String>())
            .filter_map(|p| p.parse::<u32>().ok())
            .collect()
    };

    let latest_parts = parse_parts(clean_latest);
    let current_parts = parse_parts(clean_current);

    let max_len = latest_parts.len().max(current_parts.len());
    for i in 0..max_len {
        let l = latest_parts.get(i).copied().unwrap_or(0);
        let c = current_parts.get(i).copied().unwrap_or(0);
        if l > c {
            return true;
        } else if l < c {
            return false;
        }
    }

    false
}

#[tauri::command]
pub async fn get_app_version() -> Result<String, String> {
    Ok(env!("CARGO_PKG_VERSION").to_string())
}

#[tauri::command]
pub async fn check_app_update() -> Result<UpdateInfo, String> {
    tokio::task::spawn_blocking(|| {
        let current_version = env!("CARGO_PKG_VERSION").to_string();

        let mut cmd = Command::new("winget");
        cmd.args(["show", WINGET_PACKAGE_ID, "--accept-source-agreements"]);

        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }

        if let Ok(output) = cmd.output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if let Some(latest) = parse_winget_version(&stdout) {
                    let update_available = is_newer_version(&latest, &current_version);
                    let release_url = Some(format!(
                        "https://github.com/sureshjsreddy/gemini-desktop/releases/tag/v{}",
                        latest
                    ));
                    return Ok(UpdateInfo {
                        update_available,
                        current_version,
                        latest_version: latest,
                        package_id: WINGET_PACKAGE_ID.to_string(),
                        release_url,
                    });
                }
            }
        }

        // Return current version if winget check is unavailable or matches
        Ok(UpdateInfo {
            update_available: false,
            current_version: current_version.clone(),
            latest_version: current_version,
            package_id: WINGET_PACKAGE_ID.to_string(),
            release_url: None,
        })
    })
    .await
    .map_err(|e| format!("Failed to check for updates: {}", e))?
}

/// Generates a standalone Windows updater script that waits for Gemini Desktop
/// to close, performs the WinGet upgrade, and relaunches the app on success.
pub fn generate_updater_batch_script(package_id: &str, exe_path: &str) -> String {
    format!(
        r#"@echo off
title Gemini Desktop Updater
cls
echo ========================================================
echo   Gemini Desktop WinGet Auto-Updater
echo ========================================================
echo.
echo Closing Gemini Desktop to release file locks...
timeout /t 2 /nobreak >nul
taskkill /F /IM gemini-desktop.exe >nul 2>&1

echo.
echo Upgrading package: {pkg_id}
echo Running winget upgrade...
echo.
winget upgrade --id {pkg_id} --accept-source-agreements --accept-package-agreements

if %ERRORLEVEL% EQU 0 (
    echo.
    echo ========================================================
    echo   Update completed successfully! Relaunching app...
    echo ========================================================
    timeout /t 2 /nobreak >nul
    start "" "{exe_path}"
    exit
) else (
    echo.
    echo ========================================================
    echo   WinGet upgrade finished with exit code: %ERRORLEVEL%
    echo ========================================================
    echo If access was denied, please run cmd as Administrator.
    echo Press any key to close this window.
    pause >nul
)
"#,
        pkg_id = package_id,
        exe_path = exe_path.replace('"', "")
    )
}

#[tauri::command]
pub fn launch_winget_upgrade(app: AppHandle, auto_close: Option<bool>) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let current_exe = std::env::current_exe().ok();
        let exe_path_str = current_exe
            .as_ref()
            .and_then(|p| p.to_str())
            .unwrap_or("gemini-desktop.exe");

        let batch_content = generate_updater_batch_script(WINGET_PACKAGE_ID, exe_path_str);

        // Store updater batch script in persistent LocalAppData instead of %TEMP%
        // to avoid CrowdStrike Falcon volatile directory script execution IOAs.
        let updates_dir = if let Ok(local_app) = std::env::var("LOCALAPPDATA") {
            let p = std::path::PathBuf::from(local_app).join("GeminiDesktop").join("updates");
            let _ = std::fs::create_dir_all(&p);
            p
        } else if let Ok(home) = std::env::var("USERPROFILE") {
            let p = std::path::PathBuf::from(home).join(".gemini").join("updates");
            let _ = std::fs::create_dir_all(&p);
            p
        } else {
            std::env::temp_dir()
        };

        let bat_file = updates_dir.join("gemini_desktop_updater.bat");
        std::fs::write(&bat_file, batch_content)
            .map_err(|e| format!("Failed to create updater script: {}", e))?;

        let bat_path = bat_file.to_string_lossy().to_string();

        let mut cmd = Command::new("cmd.exe");
        cmd.args(["/c", "start", "cmd.exe", "/c", &bat_path])
            .current_dir(&updates_dir);
        cmd.spawn()
            .map_err(|e| format!("Failed to launch updater: {}", e))?;

        if auto_close.unwrap_or(true) {
            let app_clone = app.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(600));
                app_clone.exit(0);
            });
        }

        return Ok(());
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = app;
        let _ = auto_close;
        return Err("WinGet is only available on Windows".to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_winget_version() {
        let sample = "
Found Gemini Desktop [SureshJanakiReddy.GeminiDesktop]
Version: 0.2.14
Publisher: Suresh Janaki Reddy
Author: Suresh Janaki Reddy
";
        let ver = parse_winget_version(sample);
        assert_eq!(ver, Some("0.2.14".to_string()));

        let empty = "No package found";
        assert_eq!(parse_winget_version(empty), None);
    }

    #[test]
    fn test_is_newer_version() {
        assert!(is_newer_version("0.2.15", "0.2.14"));
        assert!(is_newer_version("0.3.0", "0.2.14"));
        assert!(is_newer_version("1.0.0", "0.2.14"));
        assert!(is_newer_version("v0.2.15", "0.2.14"));
        assert!(is_newer_version("0.2.15", "v0.2.14"));

        assert!(!is_newer_version("0.2.14", "0.2.14"));
        assert!(!is_newer_version("0.2.13", "0.2.14"));
        assert!(!is_newer_version("0.1.99", "0.2.14"));
    }

    #[test]
    fn test_generate_updater_batch_script() {
        let script = generate_updater_batch_script("MyTest.Package", "C:\\App\\gemini-desktop.exe");
        assert!(script.contains("winget upgrade --id MyTest.Package"));
        assert!(script.contains("taskkill /F /IM gemini-desktop.exe"));
        assert!(script.contains("timeout /t 2"));
        assert!(script.contains("C:\\App\\gemini-desktop.exe"));
    }

    #[tokio::test]
    async fn test_get_app_version() {
        let ver = get_app_version().await.unwrap();
        assert!(!ver.is_empty());
        assert_eq!(ver, env!("CARGO_PKG_VERSION"));
    }
}
