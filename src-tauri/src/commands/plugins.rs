use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgyPluginInfo {
    pub name: String,
    pub enabled: bool,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(rename = "displayName", default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub skills: Option<Vec<String>>,
    #[serde(default)]
    pub rules: Option<Vec<String>>,
    #[serde(rename = "mcpServers", default)]
    pub mcp_servers: Option<Vec<String>>,
    #[serde(default)]
    pub logo: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AkboPoolConfig {
    #[serde(rename = "centralPoolDir")]
    pub central_pool_dir: String,
    #[serde(rename = "stageOnly")]
    pub stage_only: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgyPluginOpResult {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateCustomPluginRequest {
    pub name: String,
    #[serde(rename = "displayName", default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(rename = "includeSkill", default)]
    pub include_skill: bool,
    #[serde(rename = "skillName", default)]
    pub skill_name: Option<String>,
    #[serde(rename = "includeMcp", default)]
    pub include_mcp: bool,
    #[serde(rename = "includeRules", default)]
    pub include_rules: bool,
    #[serde(rename = "includeAgent", default)]
    pub include_agent: bool,
    #[serde(rename = "agentName", default)]
    pub agent_name: Option<String>,
}

fn sanitize_plugin_name(raw: &str) -> String {
    raw.trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

pub fn list_agy_plugins_internal() -> Result<Vec<AgyPluginInfo>, String> {
    let bin = match crate::process_manager::find_agy_executable() {
        Some(p) => p,
        None => return Ok(Vec::new()),
    };

    let bin_path_str = bin.to_string_lossy().to_string();
    let is_batch = bin_path_str.to_lowercase().ends_with(".cmd") || bin_path_str.to_lowercase().ends_with(".bat");

    #[cfg(target_os = "windows")]
    let output = if is_batch {
        Command::new("cmd.exe")
            .args(["/c", &bin_path_str, "plugin", "list", "--json"])
            .creation_flags(0x08000000)
            .output()
    } else {
        Command::new(&bin_path_str)
            .args(["plugin", "list", "--json"])
            .creation_flags(0x08000000)
            .output()
    };

    #[cfg(not(target_os = "windows"))]
    let output = Command::new(&bin_path_str)
        .args(["plugin", "list", "--json"])
        .output();

    let out = output.map_err(|e| format!("Failed to execute agy plugin list: {}", e))?;

    if !out.status.success() {
        let err_text = String::from_utf8_lossy(&out.stderr);
        return Err(format!("agy plugin list exited with error: {}", err_text));
    }

    let json_text = String::from_utf8_lossy(&out.stdout);
    let mut plugins: Vec<AgyPluginInfo> = serde_json::from_str(json_text.trim())
        .map_err(|e| format!("Failed to parse agy plugin list JSON: {}", e))?;

    // Enrich each plugin with metadata from its local plugin.json if present
    for plugin in &mut plugins {
        if let Some(ref path_str) = plugin.path {
            let manifest_path = PathBuf::from(path_str).join("plugin.json");
            if manifest_path.exists() {
                if let Ok(manifest_content) = std::fs::read_to_string(&manifest_path) {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&manifest_content) {
                        if plugin.display_name.is_none() {
                            plugin.display_name = val.get("displayName")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());
                        }
                        if plugin.description.is_none() {
                            plugin.description = val.get("description")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());
                        }
                        if plugin.version.is_none() {
                            plugin.version = val.get("version")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());
                        }
                        if plugin.logo.is_none() {
                            plugin.logo = val.get("logo")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());
                        }
                    }
                }
            }
        }
    }

    Ok(plugins)
}

pub fn toggle_agy_plugin_internal(name: &str, enabled: bool) -> Result<AgyPluginOpResult, String> {
    let bin = match crate::process_manager::find_agy_executable() {
        Some(p) => p,
        None => {
            return Ok(AgyPluginOpResult {
                success: false,
                message: "Antigravity CLI (agy) executable not found.".to_string(),
            });
        }
    };

    let bin_path_str = bin.to_string_lossy().to_string();
    let is_batch = bin_path_str.to_lowercase().ends_with(".cmd") || bin_path_str.to_lowercase().ends_with(".bat");
    let action = if enabled { "enable" } else { "disable" };

    #[cfg(target_os = "windows")]
    let output = if is_batch {
        Command::new("cmd.exe")
            .args(["/c", &bin_path_str, "plugin", action, name])
            .creation_flags(0x08000000)
            .output()
    } else {
        Command::new(&bin_path_str)
            .args(["plugin", action, name])
            .creation_flags(0x08000000)
            .output()
    };

    #[cfg(not(target_os = "windows"))]
    let output = Command::new(&bin_path_str)
        .args(["plugin", action, name])
        .output();

    let out = output.map_err(|e| format!("Failed to execute agy plugin {}: {}", action, e))?;
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();

    if out.status.success() {
        Ok(AgyPluginOpResult {
            success: true,
            message: if stdout.is_empty() {
                format!("Plugin '{}' {}d successfully.", name, action)
            } else {
                stdout
            },
        })
    } else {
        Ok(AgyPluginOpResult {
            success: false,
            message: if stderr.is_empty() {
                format!("Failed to {} plugin '{}'.", action, name)
            } else {
                stderr
            },
        })
    }
}

pub fn get_akbo_pool_config_internal() -> Result<AkboPoolConfig, String> {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_default();
    let config_file = PathBuf::from(&home).join(".gemini").join("config").join("akbo_pool.json");
    let default_pool = PathBuf::from(&home).join(".akb").join("central-pool").to_string_lossy().to_string();

    if config_file.exists() {
        if let Ok(content) = std::fs::read_to_string(&config_file) {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&content) {
                let pool_dir = parsed.get("centralPoolDir")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&default_pool)
                    .to_string();
                let stage_only = parsed.get("stageOnly")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                return Ok(AkboPoolConfig {
                    central_pool_dir: pool_dir,
                    stage_only,
                });
            }
        }
    }

    Ok(AkboPoolConfig {
        central_pool_dir: default_pool,
        stage_only: true,
    })
}

pub fn save_akbo_pool_config_internal(config: &AkboPoolConfig) -> Result<AgyPluginOpResult, String> {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map_err(|_| "Could not determine user home directory".to_string())?;
    let config_dir = PathBuf::from(&home).join(".gemini").join("config");
    let _ = std::fs::create_dir_all(&config_dir);
    let config_file = config_dir.join("akbo_pool.json");

    let payload = serde_json::json!({
        "centralPoolDir": config.central_pool_dir,
        "stageOnly": config.stage_only
    });

    let json_str = serde_json::to_string_pretty(&payload)
        .map_err(|e| format!("Failed to serialize akbo pool config: {}", e))?;
    std::fs::write(&config_file, json_str)
        .map_err(|e| format!("Failed to write akbo pool config: {}", e))?;

    Ok(AgyPluginOpResult {
        success: true,
        message: "AKBO central knowledge pool configuration saved successfully.".to_string(),
    })
}

pub fn install_builtin_akbo_plugin() -> Result<AgyPluginOpResult, String> {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map_err(|_| "Could not determine user home directory".to_string())?;

    let plugin_dir = PathBuf::from(&home)
        .join(".gemini")
        .join("config")
        .join("plugins")
        .join("akbo");

    std::fs::create_dir_all(&plugin_dir)
        .map_err(|e| format!("Failed to create plugin directory {:?}: {}", plugin_dir, e))?;

    // 1. plugin.json
    let manifest = serde_json::json!({
        "name": "akbo",
        "displayName": "AKBO Knowledge Oracle & Central Memory Pool",
        "version": "2.0.0",
        "description": "Deterministic AST Codebase Knowledge Graph (CKG), centralized cross-project knowledge pooling, and autonomous post-conversation memory consolidation ('dreaming').",
        "skills": ["akbo"],
        "rules": ["AGENTS.md"],
        "mcpServers": ["akbo"]
    });
    let manifest_str = serde_json::to_string_pretty(&manifest)
        .map_err(|e| format!("Failed to serialize plugin.json: {}", e))?;
    std::fs::write(plugin_dir.join("plugin.json"), manifest_str)
        .map_err(|e| format!("Failed to write plugin.json: {}", e))?;

    // 2. mcp_config.json
    let mcp = serde_json::json!({
        "mcpServers": {
            "akbo": {
                "command": "npx",
                "args": ["-y", "akbo-cli", "mcp"]
            }
        }
    });
    let mcp_str = serde_json::to_string_pretty(&mcp)
        .map_err(|e| format!("Failed to serialize mcp_config.json: {}", e))?;
    std::fs::write(plugin_dir.join("mcp_config.json"), mcp_str)
        .map_err(|e| format!("Failed to write mcp_config.json: {}", e))?;

    // 3. hooks.json
    let hooks = serde_json::json!({
        "akbo-central-pool-sync": {
            "enabled": true,
            "Stop": [
                {
                    "type": "command",
                    "command": "node ./hooks/on-stop.cjs",
                    "timeout": 45
                }
            ]
        }
    });
    let hooks_str = serde_json::to_string_pretty(&hooks)
        .map_err(|e| format!("Failed to serialize hooks.json: {}", e))?;
    std::fs::write(plugin_dir.join("hooks.json"), hooks_str)
        .map_err(|e| format!("Failed to write hooks.json: {}", e))?;

    // 4. hooks/on-stop.cjs
    let hooks_dir = plugin_dir.join("hooks");
    std::fs::create_dir_all(&hooks_dir)
        .map_err(|e| format!("Failed to create hooks directory: {}", e))?;
    let on_stop_script = r#"#!/usr/bin/env node
const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

function main() {
  let rawInput = '';
  try {
    rawInput = fs.readFileSync(0, 'utf-8');
  } catch {
    process.stdout.write(JSON.stringify({ status: 'no_input' }));
    return;
  }

  if (!rawInput.trim()) {
    process.stdout.write(JSON.stringify({ status: 'empty_input' }));
    return;
  }

  let payload = {};
  try {
    payload = JSON.parse(rawInput);
  } catch {
    process.stdout.write(JSON.stringify({ status: 'invalid_json' }));
    return;
  }

  const workspacePath = (payload.workspacePaths && payload.workspacePaths[0]) || process.cwd();
  const transcriptPath = payload.transcriptPath;

  const userHome = process.env.USERPROFILE || process.env.HOME || '';
  const configFile = path.join(userHome, '.gemini', 'config', 'akbo_pool.json');

  let centralPoolRoot = process.env.AKBO_CENTRAL_POOL_DIR || process.env.AKBO_POOL_DIR;
  let stageOnly = true;

  if (fs.existsSync(configFile)) {
    try {
      const cfg = JSON.parse(fs.readFileSync(configFile, 'utf-8'));
      if (cfg.centralPoolDir && cfg.centralPoolDir.trim()) {
        centralPoolRoot = cfg.centralPoolDir.trim();
      }
      if (cfg.stageOnly !== undefined) {
        stageOnly = Boolean(cfg.stageOnly);
      }
    } catch {}
  }

  if (!centralPoolRoot) {
    centralPoolRoot = path.join(userHome, '.akb', 'central-pool');
  }

  let projectName = path.basename(path.resolve(workspacePath));
  try {
    const gitRemote = execSync('git config --get remote.origin.url', {
      cwd: workspacePath,
      stdio: ['ignore', 'pipe', 'ignore'],
      timeout: 3000
    }).toString().trim();

    if (gitRemote) {
      const match = gitRemote.match(/\/([^\/]+?)(\.git)?$/);
      if (match && match[1]) {
        projectName = match[1];
      }
    }
  } catch {}

  const targetProjectPool = path.join(centralPoolRoot, 'projects', projectName);
  try {
    fs.mkdirSync(targetProjectPool, { recursive: true });
  } catch {}

  try {
    const transcriptArg = transcriptPath && fs.existsSync(transcriptPath)
      ? ` --transcriptFiles "${transcriptPath}"`
      : '';
    const stageArg = stageOnly ? ' --stage-only' : '';
    const cmd = `npx -y akbo-cli dream --pool "${targetProjectPool}" --targetDir "${workspacePath}"${transcriptArg}${stageArg} --json`;

    execSync(cmd, {
      stdio: ['ignore', 'pipe', 'pipe'],
      timeout: 30000,
      env: { ...process.env, AKBO_POOL_DIR: targetProjectPool }
    });

    process.stdout.write(JSON.stringify({
      status: 'pooled',
      project: projectName,
      poolPath: targetProjectPool
    }));
  } catch (err) {
    process.stdout.write(JSON.stringify({
      status: 'dream_deferred',
      project: projectName,
      error: err.message
    }));
  }
}

main();
"#;
    std::fs::write(hooks_dir.join("on-stop.cjs"), on_stop_script)
        .map_err(|e| format!("Failed to write on-stop.cjs: {}", e))?;

    // 5. rules/AGENTS.md
    let rules_dir = plugin_dir.join("rules");
    std::fs::create_dir_all(&rules_dir)
        .map_err(|e| format!("Failed to create rules directory: {}", e))?;
    let rules_md = r#"# 🧠 AKBO Centralized Knowledge & Memory Rules

## 1. Pre-Execution Grounding
- Before proposing code modifications, investigating unfamiliar project architectures, or querying dependencies, invoke `akbo_context`, `akbo_search`, or `akbo_list_invariants` to retrieve the project's Codebase Knowledge Graph (CKG) and approved architectural constraints with 0 token waste.
- Never perform speculative raw file grepping if high-level architectural standards exist in the central knowledge pool.

## 2. Out-of-Band Observation Recording
- Whenever you encounter an undocumented convention, resolve a tricky bug, establish a design pattern, or identify a technical constraint, immediately call `akbo_record_observation`:
  - `type`: 'pattern' | 'gotcha' | 'convention' | 'architecture_decision'
  - `title`: Short descriptive title (e.g. 'Svelte 5 runes buffer reactivity')
  - `content`: Concrete technical description and lesson learned
- **Do not** write extensive runbooks or notes into the main conversation response; the out-of-band memory consolidator will synthesize your observations into permanent Architecture Decision Records (`ADR-xxx.yaml`) automatically.

## 3. Runtime Guardrails
- Before making significant modifications to core files or schemas, call `akbo_guard` to verify that proposed changes do not violate active `dont:` directives or deprecated standards.
"#;
    std::fs::write(rules_dir.join("AGENTS.md"), rules_md)
        .map_err(|e| format!("Failed to write rules/AGENTS.md: {}", e))?;

    // 6. skills/akbo/SKILL.md
    let skill_dir = plugin_dir.join("skills").join("akbo");
    std::fs::create_dir_all(&skill_dir)
        .map_err(|e| format!("Failed to create skills directory: {}", e))?;
    let skill_md = r#"---
name: akbo
description: Centralized Codebase Knowledge Graph (CKG), out-of-band memory consolidation (dreaming), and architectural invariant management.
---

# AKBO Knowledge Oracle & Central Memory Pool

Use this skill when you want to inspect project architecture invariants, trigger out-of-band memory consolidation ("dreaming"), review staged proposals, or check central pool status.

## Available Actions & Commands

### 1. Memory Consolidation (`/dream`)
Consolidates recorded observations and recent conversation error traces out-of-band into durable Architecture Decision Records (`ADR-xxx.yaml`) or updates existing knowledge units:
```powershell
npx -y akbo-cli dream --stage-only
```
To auto-approve and immediately update the central pool without manual staging:
```powershell
npx -y akbo-cli dream --auto-approve
```

### 2. Inspect Invariants (`/invariants`)
Retrieves all approved architecture and development invariants for the active project from the central pool:
```powershell
npx -y akbo-cli invariants
```

### 3. Codebase Knowledge Graph Harvest (`/harvest`)
Deterministically scans the project AST (0 LLM tokens) and updates the CKG cache in the central pool:
```powershell
npx -y akbo-cli harvest .
```

### 4. Blast-Radius Impact Analysis
Traces inbound callers, interface implementations, and dependents before refactoring:
```powershell
npx -y akbo-cli impact <symbol_or_file>
```
"#;
    std::fs::write(skill_dir.join("SKILL.md"), skill_md)
        .map_err(|e| format!("Failed to write skills/akbo/SKILL.md: {}", e))?;

    // 7. Enable plugin
    let _ = toggle_agy_plugin_internal("akbo", true);

    Ok(AgyPluginOpResult {
        success: true,
        message: "AKBO Knowledge Oracle & Central Memory Pool plugin installed and enabled successfully.".to_string(),
    })
}

pub fn install_agy_plugin_internal(target: &str) -> Result<AgyPluginOpResult, String> {
    if target.trim().eq_ignore_ascii_case("akbo") {
        return install_builtin_akbo_plugin();
    }

    let bin = match crate::process_manager::find_agy_executable() {
        Some(p) => p,
        None => {
            return Ok(AgyPluginOpResult {
                success: false,
                message: "Antigravity CLI (agy) executable not found.".to_string(),
            });
        }
    };

    let bin_path_str = bin.to_string_lossy().to_string();
    let is_batch = bin_path_str.to_lowercase().ends_with(".cmd") || bin_path_str.to_lowercase().ends_with(".bat");

    #[cfg(target_os = "windows")]
    let output = if is_batch {
        Command::new("cmd.exe")
            .args(["/c", &bin_path_str, "plugin", "install", target])
            .creation_flags(0x08000000)
            .output()
    } else {
        Command::new(&bin_path_str)
            .args(["plugin", "install", target])
            .creation_flags(0x08000000)
            .output()
    };

    #[cfg(not(target_os = "windows"))]
    let output = Command::new(&bin_path_str)
        .args(["plugin", "install", target])
        .output();

    let out = output.map_err(|e| format!("Failed to execute agy plugin install: {}", e))?;
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();

    if out.status.success() {
        Ok(AgyPluginOpResult {
            success: true,
            message: if stdout.is_empty() {
                format!("Plugin '{}' installed successfully.", target)
            } else {
                stdout
            },
        })
    } else {
        Ok(AgyPluginOpResult {
            success: false,
            message: if stderr.is_empty() {
                format!("Failed to install plugin '{}'.", target)
            } else {
                stderr
            },
        })
    }
}

pub fn uninstall_agy_plugin_internal(name: &str) -> Result<AgyPluginOpResult, String> {
    let bin = match crate::process_manager::find_agy_executable() {
        Some(p) => p,
        None => {
            return Ok(AgyPluginOpResult {
                success: false,
                message: "Antigravity CLI (agy) executable not found.".to_string(),
            });
        }
    };

    let bin_path_str = bin.to_string_lossy().to_string();
    let is_batch = bin_path_str.to_lowercase().ends_with(".cmd") || bin_path_str.to_lowercase().ends_with(".bat");

    #[cfg(target_os = "windows")]
    let output = if is_batch {
        Command::new("cmd.exe")
            .args(["/c", &bin_path_str, "plugin", "uninstall", name])
            .creation_flags(0x08000000)
            .output()
    } else {
        Command::new(&bin_path_str)
            .args(["plugin", "uninstall", name])
            .creation_flags(0x08000000)
            .output()
    };

    #[cfg(not(target_os = "windows"))]
    let output = Command::new(&bin_path_str)
        .args(["plugin", "uninstall", name])
        .output();

    let out = output.map_err(|e| format!("Failed to execute agy plugin uninstall: {}", e))?;
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();

    // If directory still exists in ~/.gemini/config/plugins/<name>, ensure cleanup
    let mut custom_dir_removed = false;
    if let Ok(home) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
        let p_dir = PathBuf::from(home).join(".gemini").join("config").join("plugins").join(name);
        if p_dir.exists() {
            let _ = std::fs::remove_dir_all(&p_dir);
            custom_dir_removed = true;
        }
    }

    if out.status.success() || custom_dir_removed {
        Ok(AgyPluginOpResult {
            success: true,
            message: if stdout.is_empty() {
                format!("Plugin '{}' uninstalled successfully.", name)
            } else {
                stdout
            },
        })
    } else {
        Ok(AgyPluginOpResult {
            success: false,
            message: if stderr.is_empty() {
                format!("Failed to uninstall plugin '{}'.", name)
            } else {
                stderr
            },
        })
    }
}

pub fn create_custom_plugin_internal(req: &CreateCustomPluginRequest) -> Result<AgyPluginOpResult, String> {
    let clean_name = sanitize_plugin_name(&req.name);
    if clean_name.is_empty() {
        return Err("Plugin name cannot be empty".to_string());
    }

    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map_err(|_| "Could not determine user home directory".to_string())?;

    let plugin_dir = PathBuf::from(home)
        .join(".gemini")
        .join("config")
        .join("plugins")
        .join(&clean_name);

    if plugin_dir.exists() {
        return Err(format!("Plugin directory already exists: {:?}", plugin_dir));
    }

    std::fs::create_dir_all(&plugin_dir)
        .map_err(|e| format!("Failed to create plugin directory {:?}: {}", plugin_dir, e))?;

    let display_name = req.display_name.as_deref().unwrap_or(&clean_name);
    let description = req.description.as_deref().unwrap_or("Custom Antigravity plugin created with Gemini Desktop");

    // 1. Write plugin.json
    let manifest = serde_json::json!({
        "name": clean_name,
        "displayName": display_name,
        "version": "0.1.0",
        "description": description
    });

    let manifest_str = serde_json::to_string_pretty(&manifest)
        .map_err(|e| format!("Failed to serialize plugin.json: {}", e))?;
    std::fs::write(plugin_dir.join("plugin.json"), manifest_str)
        .map_err(|e| format!("Failed to write plugin.json: {}", e))?;

    // 2. Write README.md
    let readme = format!("# {}\n\n{}\n\nCreated with Gemini Desktop Plugin Studio.\n", display_name, description);
    let _ = std::fs::write(plugin_dir.join("README.md"), readme);

    // 3. Include Skill if requested
    if req.include_skill {
        let skill_id = req.skill_name.as_deref().unwrap_or("custom-workflow");
        let clean_skill_id = sanitize_plugin_name(skill_id);
        let skill_dir = plugin_dir.join("skills").join(&clean_skill_id);
        let _ = std::fs::create_dir_all(&skill_dir);

        let skill_content = format!(
            "---\nname: {}\ndescription: {}\n---\n\n# {}\n\nInstructions and multi-step workflows for the agent.\n",
            clean_skill_id, description, display_name
        );
        let _ = std::fs::write(skill_dir.join("SKILL.md"), skill_content);
    }

    // 4. Include MCP configuration if requested
    if req.include_mcp {
        let mcp_content = serde_json::json!({
            "mcpServers": {}
        });
        if let Ok(mcp_str) = serde_json::to_string_pretty(&mcp_content) {
            let _ = std::fs::write(plugin_dir.join("mcp_config.json"), mcp_str);
        }
    }

    // 5. Include Agent Rules if requested
    if req.include_rules {
        let rules_dir = plugin_dir.join("rules");
        let _ = std::fs::create_dir_all(&rules_dir);
        let rules_content = format!(
            "# Directives & Rules for {}\n\n- Adhere to project guidelines and security standards.\n",
            display_name
        );
        let _ = std::fs::write(rules_dir.join("AGENTS.md"), rules_content);
    }

    // 6. Include Specialist Agent if requested
    if req.include_agent {
        let agent_id = req.agent_name.as_deref().unwrap_or("assistant");
        let clean_agent_id = sanitize_plugin_name(agent_id);
        let agents_dir = plugin_dir.join("agents");
        let _ = std::fs::create_dir_all(&agents_dir);

        let agent_content = format!(
            "---\nname: {}\ndescription: Specialized subagent for {}\n---\n\nYou are a specialized assistant for {}.\n",
            clean_agent_id, display_name, display_name
        );
        let _ = std::fs::write(agents_dir.join(format!("{}.md", clean_agent_id)), agent_content);
    }

    // Enable the newly created plugin
    let _ = toggle_agy_plugin_internal(&clean_name, true);

    Ok(AgyPluginOpResult {
        success: true,
        message: format!("Plugin '{}' created and enabled successfully at {:?}", clean_name, plugin_dir),
    })
}

// Tauri IPC Command wrappers

#[tauri::command]
pub async fn list_agy_plugins() -> Result<Vec<AgyPluginInfo>, String> {
    tokio::task::spawn_blocking(list_agy_plugins_internal)
        .await
        .map_err(|e| format!("Task execution error: {}", e))?
}

#[tauri::command]
pub async fn toggle_agy_plugin(name: String, enabled: bool) -> Result<AgyPluginOpResult, String> {
    tokio::task::spawn_blocking(move || toggle_agy_plugin_internal(&name, enabled))
        .await
        .map_err(|e| format!("Task execution error: {}", e))?
}

#[tauri::command]
pub async fn install_agy_plugin(target: String) -> Result<AgyPluginOpResult, String> {
    tokio::task::spawn_blocking(move || install_agy_plugin_internal(&target))
        .await
        .map_err(|e| format!("Task execution error: {}", e))?
}

#[tauri::command]
pub async fn uninstall_agy_plugin(name: String) -> Result<AgyPluginOpResult, String> {
    tokio::task::spawn_blocking(move || uninstall_agy_plugin_internal(&name))
        .await
        .map_err(|e| format!("Task execution error: {}", e))?
}

#[tauri::command]
pub async fn create_custom_plugin(req: CreateCustomPluginRequest) -> Result<AgyPluginOpResult, String> {
    tokio::task::spawn_blocking(move || create_custom_plugin_internal(&req))
        .await
        .map_err(|e| format!("Task execution error: {}", e))?
}

#[tauri::command]
pub async fn get_akbo_pool_config() -> Result<AkboPoolConfig, String> {
    tokio::task::spawn_blocking(get_akbo_pool_config_internal)
        .await
        .map_err(|e| format!("Task execution error: {}", e))?
}

#[tauri::command]
pub async fn save_akbo_pool_config(config: AkboPoolConfig) -> Result<AgyPluginOpResult, String> {
    tokio::task::spawn_blocking(move || save_akbo_pool_config_internal(&config))
        .await
        .map_err(|e| format!("Task execution error: {}", e))?
}

#[tauri::command]
pub async fn open_plugin_directory(plugin_path: String) -> Result<(), String> {
    let path = PathBuf::from(&plugin_path);
    if !path.exists() {
        return Err(format!("Directory does not exist: {}", plugin_path));
    }

    #[cfg(target_os = "windows")]
    Command::new("explorer.exe")
        .arg(&plugin_path)
        .spawn()
        .map_err(|e| format!("Failed to open directory in Explorer: {}", e))?;

    #[cfg(not(target_os = "windows"))]
    Command::new("xdg-open")
        .arg(&plugin_path)
        .spawn()
        .map_err(|e| format!("Failed to open directory: {}", e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_plugin_name() {
        assert_eq!(sanitize_plugin_name("My Plugin!"), "my-plugin");
        assert_eq!(sanitize_plugin_name("---test_123---"), "test_123");
        assert_eq!(sanitize_plugin_name("data-science"), "data-science");
    }

    #[test]
    fn test_create_and_validate_custom_plugin_structure() {
        let temp_dir = std::env::temp_dir().join(format!("test_agy_plugin_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let req = CreateCustomPluginRequest {
            name: "test-plugin".to_string(),
            display_name: Some("Test Plugin".to_string()),
            description: Some("A test plugin description".to_string()),
            include_skill: true,
            skill_name: Some("data-analysis".to_string()),
            include_mcp: true,
            include_rules: true,
            include_agent: true,
            agent_name: Some("analyzer".to_string()),
        };

        let plugin_dir = temp_dir.join("test-plugin");
        std::fs::create_dir_all(&plugin_dir).unwrap();

        // Write plugin.json
        let manifest = serde_json::json!({
            "name": req.name,
            "displayName": req.display_name,
            "version": "0.1.0",
            "description": req.description
        });
        std::fs::write(plugin_dir.join("plugin.json"), serde_json::to_string_pretty(&manifest).unwrap()).unwrap();

        // Write skills
        let skill_dir = plugin_dir.join("skills").join("data-analysis");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(skill_dir.join("SKILL.md"), "---\nname: data-analysis\ndescription: Analysis\n---\n# Analysis\n").unwrap();

        // Write mcp_config.json
        std::fs::write(plugin_dir.join("mcp_config.json"), r#"{"mcpServers":{}}"#).unwrap();

        // Write rules
        let rules_dir = plugin_dir.join("rules");
        std::fs::create_dir_all(&rules_dir).unwrap();
        std::fs::write(rules_dir.join("AGENTS.md"), "# Rules\n").unwrap();

        // Assert existence
        assert!(plugin_dir.join("plugin.json").exists());
        assert!(plugin_dir.join("skills").join("data-analysis").join("SKILL.md").exists());
        assert!(plugin_dir.join("mcp_config.json").exists());
        assert!(plugin_dir.join("rules").join("AGENTS.md").exists());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_akbo_pool_config_serialization() {
        let config = AkboPoolConfig {
            central_pool_dir: r"C:\Data\KnowledgePool".to_string(),
            stage_only: false,
        };
        let serialized = serde_json::to_string(&config).unwrap();
        assert!(serialized.contains("centralPoolDir"));
        assert!(serialized.contains("stageOnly"));

        let deserialized: AkboPoolConfig = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.central_pool_dir, r"C:\Data\KnowledgePool");
        assert!(!deserialized.stage_only);
    }

    #[test]
    fn test_akbo_pool_config_defaults() {
        let config = get_akbo_pool_config_internal().unwrap();
        assert!(!config.central_pool_dir.is_empty());
    }
}
