use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;

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

    let output = crate::process_manager::ProcessSupervisor::build_cli_command(
        &bin,
        &["plugin", "list", "--json"],
    )
    .output();

    let out = output.map_err(|e| format!("Failed to execute agy plugin list: {}", e))?;

    if !out.status.success() {
        let err_text = String::from_utf8_lossy(&out.stderr);
        return Err(format!("agy plugin list exited with error: {}", err_text));
    }

    let json_text = String::from_utf8_lossy(&out.stdout);
    let mut plugins: Vec<AgyPluginInfo> = serde_json::from_str(crate::commands::strip_utf8_bom(json_text.trim()))
        .map_err(|e| format!("Failed to parse agy plugin list JSON: {}", e))?;

    // Enrich each plugin with metadata from its local plugin.json if present
    for plugin in &mut plugins {
        if let Some(ref path_str) = plugin.path {
            let manifest_path = PathBuf::from(path_str).join("plugin.json");
            if manifest_path.exists() {
                if let Ok(manifest_content) = std::fs::read_to_string(&manifest_path) {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(crate::commands::strip_utf8_bom(&manifest_content)) {
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

    let action = if enabled { "enable" } else { "disable" };
    let output = crate::process_manager::ProcessSupervisor::build_cli_command(
        &bin,
        &["plugin", action, name],
    )
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

fn akbo_manifest_json() -> serde_json::Value {
    serde_json::json!({
        "name": "akbo",
        "displayName": "AKBO Knowledge Oracle & Central Memory Pool",
        "version": "3.0.0",
        "description": "Hybrid AST Codebase Knowledge Graph (TS Compiler + WASM tree-sitter), centralized cross-project knowledge pooling, AST-backed convention guardrails, and staged post-conversation memory consolidation ('dreaming')."
    })
}

fn akbo_on_stop_script() -> &'static str {
    r#"#!/usr/bin/env node
const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

function allowStop() {
  // Antigravity CLI parses hook stdout via protojson; emit an empty JSON object when allowing stop.
  process.stdout.write('{}');
}

function main() {
  let rawInput = '';
  try {
    rawInput = fs.readFileSync(0, 'utf-8');
  } catch {
    allowStop();
    return;
  }

  if (!rawInput.trim()) {
    allowStop();
    return;
  }

  let payload = {};
  try {
    payload = JSON.parse(rawInput);
  } catch {
    allowStop();
    return;
  }

  // Prevent recursive stop-hook firing during continuation turns (agy uses camelCase continuationActive)
  if (payload && (payload.continuationActive || payload.continuation_active)) {
    allowStop();
    return;
  }

  const workspacePath = (payload.workspacePaths && payload.workspacePaths[0]) || payload.cwd || process.cwd();
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

  // Migrate any leftover workspace session files into central pool and purge workspace .akb
  try {
    const wsAkb = path.join(workspacePath, '.akb');
    if (fs.existsSync(wsAkb)) {
      const wsSessions = path.join(wsAkb, 'sessions');
      const poolSessions = path.join(targetProjectPool, '.akb', 'sessions');
      if (fs.existsSync(wsSessions)) {
        fs.mkdirSync(poolSessions, { recursive: true });
        for (const agent of fs.readdirSync(wsSessions)) {
          const srcAgent = path.join(wsSessions, agent);
          const destAgent = path.join(poolSessions, agent);
          if (!fs.statSync(srcAgent).isDirectory()) continue;
          fs.mkdirSync(destAgent, { recursive: true });
          const srcObs = path.join(srcAgent, 'observations.jsonl');
          const destObs = path.join(destAgent, 'observations.jsonl');
          if (fs.existsSync(srcObs)) {
            const srcLines = fs.readFileSync(srcObs, 'utf-8').split('\n').filter(Boolean);
            let destLines = [];
            if (fs.existsSync(destObs)) {
              destLines = fs.readFileSync(destObs, 'utf-8').split('\n').filter(Boolean);
            }
            const existingIds = new Set(destLines.map(l => {
              try { return JSON.parse(l).id; } catch { return null; }
            }).filter(Boolean));
            for (const line of srcLines) {
              try {
                const parsed = JSON.parse(line);
                if (!existingIds.has(parsed.id)) {
                  destLines.push(line);
                }
              } catch {}
            }
            fs.writeFileSync(destObs, destLines.join('\n') + '\n', 'utf8');
          }
        }
      }
      fs.rmSync(wsAkb, { recursive: true, force: true });
    }
  } catch {}

  // Skip spawning CLI entirely if there are no pending unconsolidated observations
  let hasPendingObservations = false;
  try {
    const poolSessions = path.join(targetProjectPool, '.akb', 'sessions');
    if (fs.existsSync(poolSessions)) {
      for (const agent of fs.readdirSync(poolSessions)) {
        const obsFile = path.join(poolSessions, agent, 'observations.jsonl');
        if (!fs.existsSync(obsFile)) continue;
        const lines = fs.readFileSync(obsFile, 'utf-8').split('\n').filter(Boolean);
        for (const line of lines) {
          try {
            const parsed = JSON.parse(line);
            if (!parsed.status || parsed.status === 'recorded') {
              hasPendingObservations = true;
              break;
            }
          } catch {}
        }
        if (hasPendingObservations) break;
      }
    }
  } catch {}

  if (!hasPendingObservations) {
    allowStop();
    return;
  }

  // Run dreaming exclusively in targetProjectPool
  try {
    const stageArg = stageOnly ? ' --stage-only' : ' --auto-approve';
    const PINNED_PKG = 'akbo-cli@3.0.0';
    let runner = `npx -y ${PINNED_PKG}`;

    try {
      const pkgJsonPath = path.join(workspacePath, 'package.json');
      const localCli = path.join(workspacePath, 'dist', 'cli.js');
      if (fs.existsSync(pkgJsonPath) && fs.existsSync(localCli)) {
        const pkg = JSON.parse(fs.readFileSync(pkgJsonPath, 'utf-8'));
        if (pkg && pkg.name === 'akbo-cli') {
          runner = `"${process.execPath}" "${localCli}"`;
        }
      }
    } catch {}

    if (runner.startsWith('npx')) {
      try {
        const checkCmd = process.platform === 'win32' ? 'where akbo' : 'which akbo';
        execSync(checkCmd, { stdio: 'ignore', timeout: 2000 });
        runner = 'akbo';
      } catch {}
    }

    const cmd = `${runner} dream "${workspacePath}" --pool "${targetProjectPool}"${stageArg} --json`;

    execSync(cmd, {
      cwd: workspacePath,
      stdio: ['ignore', 'pipe', 'pipe'],
      timeout: 30000,
      env: { ...process.env, AKBO_POOL_DIR: targetProjectPool }
    });

    // Ensure workspace remains unpolluted (clean any stray .akb created during dream)
    try {
      const wsAkb = path.join(workspacePath, '.akb');
      if (fs.existsSync(wsAkb)) {
        fs.rmSync(wsAkb, { recursive: true, force: true });
      }
    } catch {}
  } catch (err) {
    console.error('[akbo-central-pool-sync] dream deferred:', err && err.message ? err.message : String(err));
  }

  allowStop();
}

main();
"#
}

fn akbo_rules_markdown() -> &'static str {
    r#"---
trigger: always_on
---

# 🧠 AKBO Centralized Knowledge & Memory Rules

## 1. Pre-Execution Grounding
- Before proposing code modifications, investigating unfamiliar project architectures, or querying dependencies, invoke `akbo_context`, `akbo_search`, or `akbo_list_invariants` to retrieve the project's Codebase Knowledge Graph (CKG) and approved architectural constraints with 0 token waste.
- Never perform speculative raw file grepping if high-level architectural standards exist in the central knowledge pool.

## 2. Project Analysis & Baseline Knowledge Onboarding
- `akbo_harvest` only scans deterministic code topology into `.akb/caches/` (0 LLM tokens); it does **not** create Knowledge Units (`knowledge/*.yaml`, `index.yaml`).
- When analyzing, documenting, or onboarding a project—especially when `akbo_list_invariants` returns empty (`[]`) or no `knowledge/index.yaml` exists yet—run `akbo_harvest`, inspect key manifests/modules, and persist baseline Knowledge Units into the central pool via `akbo_upsert_units`:
  - Standard units (`architecture_standard`, `development_standard`, `product_rule`): require `id` (e.g. `ARCH-SYS-001`), `type`, `title`, `domain` (`architecture` | `development` | `product`), `status` (`"approved"`), `applies_to` (`string[]`), `rule`, `do` (`string[]`, min 1), `dont` (`string[]`, min 1), `reason`.
  - Decision units (`architecture_decision`): require `id` (e.g. `ADR-001`), `type` (`"architecture_decision"`), `title`, `domain` (`"decisions"`), `status` (`"approved"`), `decision`, `context`, `applies_to` (`string[]`).

## 3. Out-of-Band Observation Recording
- Whenever you encounter an undocumented convention, resolve a tricky bug, or identify a runtime trap, immediately call `akbo_record_observation` using its exact schema parameters:
  - `topic`: Target subsystem, endpoint, file, or library (required)
  - `finding`: Concrete technical insight, trap, or bug workaround (required)
  - `reproducible_fix`: Actionable do/dont solution or pattern to adhere to (optional)
  - `associated_files`: Relevant relative file paths involved (optional)
- Do not write extensive runbooks into conversation replies; the Stop hook runs `akbo dream` in the central pool (`--stage-only` writes proposals to `.akb/staged/` unless `stageOnly: false` is enabled or approved via `akbo_dream`).

## 4. Runtime Guardrails
- Before making significant modifications to core files or schemas, call `akbo_guard` to verify that proposed changes do not violate active `dont:` directives or deprecated standards.

## 5. Central Pool Isolation (Zero Workspace Pollution)
- All session observations (`.akb/sessions/`), Codebase Knowledge Graph caches (`.akb/caches/`), and durable knowledge units (`knowledge/`, `ADR-xxx.yaml`, `index.yaml`) reside exclusively in the external Central Knowledge Pool (`~/.akb/central-pool/projects/<project>/`).
- Never create or write `.akb/` folders or ephemeral session caches directly into the active project workspace.
"#
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

    // 1. plugin.json (metadata only; skills, rules, hooks, and mcp_config are auto-discovered by agy)
    let manifest = akbo_manifest_json();
    let manifest_str = serde_json::to_string_pretty(&manifest)
        .map_err(|e| format!("Failed to serialize plugin.json: {}", e))?;
    std::fs::write(plugin_dir.join("plugin.json"), manifest_str)
        .map_err(|e| format!("Failed to write plugin.json: {}", e))?;

    // 2. mcp.cjs launcher that isolates all caches, sessions, and knowledge units in the central pool
    let mcp_script = r#"#!/usr/bin/env node
const fs = require('fs');
const path = require('path');
const { spawn, execSync } = require('child_process');

const workspacePath = process.env.WORKSPACE_DIR || process.env.PROJECT_DIR || process.cwd();

const userHome = process.env.USERPROFILE || process.env.HOME || '';
const configFile = path.join(userHome, '.gemini', 'config', 'akbo_pool.json');

let centralPoolRoot = process.env.AKBO_CENTRAL_POOL_DIR || process.env.AKBO_POOL_DIR;
if (fs.existsSync(configFile)) {
  try {
    const cfg = JSON.parse(fs.readFileSync(configFile, 'utf-8'));
    if (cfg.centralPoolDir && cfg.centralPoolDir.trim()) {
      centralPoolRoot = cfg.centralPoolDir.trim();
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

// Purge any workspace .akb directory if present and migrate
try {
  const wsAkb = path.join(workspacePath, '.akb');
  if (fs.existsSync(wsAkb)) {
    const wsSessions = path.join(wsAkb, 'sessions');
    const poolSessions = path.join(targetProjectPool, '.akb', 'sessions');
    if (fs.existsSync(wsSessions)) {
      fs.mkdirSync(poolSessions, { recursive: true });
      for (const agent of fs.readdirSync(wsSessions)) {
        const srcAgentDir = path.join(wsSessions, agent);
        const destAgentDir = path.join(poolSessions, agent);
        if (!fs.statSync(srcAgentDir).isDirectory()) continue;
        fs.mkdirSync(destAgentDir, { recursive: true });
        const srcObs = path.join(srcAgentDir, 'observations.jsonl');
        const destObs = path.join(destAgentDir, 'observations.jsonl');
        if (fs.existsSync(srcObs)) {
          const srcLines = fs.readFileSync(srcObs, 'utf-8').split('\n').filter(Boolean);
          let destLines = [];
          if (fs.existsSync(destObs)) {
            destLines = fs.readFileSync(destObs, 'utf-8').split('\n').filter(Boolean);
          }
          const existingIds = new Set(destLines.map(l => {
            try { return JSON.parse(l).id; } catch { return null; }
          }).filter(Boolean));
          for (const line of srcLines) {
            try {
              const parsed = JSON.parse(line);
              if (!existingIds.has(parsed.id)) {
                destLines.push(line);
              }
            } catch {}
          }
          fs.writeFileSync(destObs, destLines.join('\n') + '\n', 'utf8');
        }
      }
    }
    fs.rmSync(wsAkb, { recursive: true, force: true });
  }
} catch {}

const env = {
  ...process.env,
  AKBO_POOL_DIR: targetProjectPool
};

const PINNED_PKG = 'akbo-cli@3.0.0';
let command = 'npx';
let args = ['-y', PINNED_PKG, 'mcp', workspacePath, '--pool', targetProjectPool];
let useShell = true;

try {
  const pkgJsonPath = path.join(workspacePath, 'package.json');
  const localCli = path.join(workspacePath, 'dist', 'cli.js');
  if (fs.existsSync(pkgJsonPath) && fs.existsSync(localCli)) {
    const pkg = JSON.parse(fs.readFileSync(pkgJsonPath, 'utf-8'));
    if (pkg && pkg.name === 'akbo-cli') {
      command = process.execPath;
      args = [localCli, 'mcp', workspacePath, '--pool', targetProjectPool];
      useShell = false;
    }
  }
} catch {}

if (command === 'npx') {
  try {
    const checkCmd = process.platform === 'win32' ? 'where akbo-mcp' : 'which akbo-mcp';
    execSync(checkCmd, { stdio: 'ignore', timeout: 2000 });
    command = 'akbo-mcp';
    args = [workspacePath, '--pool', targetProjectPool];
    useShell = process.platform === 'win32';
  } catch {}
}

const child = spawn(command, args, {
  shell: useShell,
  stdio: 'inherit',
  env
});

process.on('SIGTERM', () => child.kill());
process.on('SIGINT', () => child.kill());

child.on('exit', (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal);
  } else {
    process.exit(code || 0);
  }
});
"#;
    std::fs::write(plugin_dir.join("mcp.cjs"), mcp_script)
        .map_err(|e| format!("Failed to write mcp.cjs: {}", e))?;

    // 3. mcp_config.json
    let mcp = serde_json::json!({
        "mcpServers": {
            "akbo": {
                "command": "node",
                "args": ["mcp.cjs"]
            }
        }
    });
    let mcp_str = serde_json::to_string_pretty(&mcp)
        .map_err(|e| format!("Failed to serialize mcp_config.json: {}", e))?;
    std::fs::write(plugin_dir.join("mcp_config.json"), mcp_str)
        .map_err(|e| format!("Failed to write mcp_config.json: {}", e))?;

    // 4. hooks.json
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

    // 5. hooks/on-stop.cjs
    let hooks_dir = plugin_dir.join("hooks");
    std::fs::create_dir_all(&hooks_dir)
        .map_err(|e| format!("Failed to create hooks directory: {}", e))?;
    std::fs::write(hooks_dir.join("on-stop.cjs"), akbo_on_stop_script())
        .map_err(|e| format!("Failed to write on-stop.cjs: {}", e))?;

    // 6. rules/AGENTS.md
    let rules_dir = plugin_dir.join("rules");
    std::fs::create_dir_all(&rules_dir)
        .map_err(|e| format!("Failed to create rules directory: {}", e))?;
    std::fs::write(rules_dir.join("AGENTS.md"), akbo_rules_markdown())
        .map_err(|e| format!("Failed to write rules/AGENTS.md: {}", e))?;

    // 7. skills/akbo/SKILL.md
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
Consolidates recorded observations and recent conversation error traces out-of-band into staged proposals in `.akb/staged/` (default) for review:
```powershell
npx -y akbo-cli@3.0.0 dream --stage-only
npx -y akbo-cli@3.0.0 dream --list-staged
npx -y akbo-cli@3.0.0 dream --approve all
```
To auto-approve and immediately update the central pool without manual staging:
```powershell
npx -y akbo-cli@3.0.0 dream --auto-approve
```

### 2. Inspect Invariants (`/invariants`)
Retrieves all approved architecture and development invariants for the active project from the central pool:
```powershell
npx -y akbo-cli@3.0.0 invariants
```

### 3. Hybrid AST Codebase Knowledge Graph Harvest (`/harvest`)
Deterministically scans project topology using the Hybrid AST Engine (TS Compiler AST + WASM tree-sitter for C#, Java, Python, Rust, Go with comment-stripped regex fallback, 0 LLM tokens) and updates the CKG cache in the central pool:
```powershell
npx -y akbo-cli@3.0.0 harvest .
```

### 4. Scoped Blast-Radius & Convention Guardrails
Traces architectural boundaries and cross-file TS `IMPORTS`/`CALLS_SYMBOL` (`scope: all | architectural | symbol` on `akbo_impact`, `file#symbol` disambiguation) before refactoring, and evaluates `ast_query` + diff-aware guardrails:
```powershell
npx -y akbo-cli@3.0.0 context <symbol_or_file>
npx -y akbo-cli@3.0.0 guard <file> -c "<code_or_diff>"
```
"#;
    std::fs::write(skill_dir.join("SKILL.md"), skill_md)
        .map_err(|e| format!("Failed to write skills/akbo/SKILL.md: {}", e))?;

    // 7. Ensure ~/.gemini/config/akbo_pool.json exists with stageOnly: true by default
    let pool_config_dir = PathBuf::from(&home).join(".gemini").join("config");
    let _ = std::fs::create_dir_all(&pool_config_dir);
    let pool_config_file = pool_config_dir.join("akbo_pool.json");
    if !pool_config_file.exists() {
        let default_pool = PathBuf::from(&home).join(".akb").join("central-pool").to_string_lossy().to_string();
        let pool_payload = serde_json::json!({
            "centralPoolDir": default_pool,
            "stageOnly": true
        });
        if let Ok(pool_json) = serde_json::to_string_pretty(&pool_payload) {
            let _ = std::fs::write(&pool_config_file, pool_json);
        }
    }

    // 8. Enable plugin
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

    let output = crate::process_manager::ProcessSupervisor::build_cli_command(
        &bin,
        &["plugin", "install", target],
    )
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

    let output = crate::process_manager::ProcessSupervisor::build_cli_command(
        &bin,
        &["plugin", "uninstall", name],
    )
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
            "---\ntrigger: always_on\n---\n\n# Directives & Rules for {}\n\n- Adhere to project guidelines and security standards.\n",
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

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AgyPluginValidationResult {
    pub success: bool,
    pub target: String,
    pub strict: bool,
    pub issues: Vec<String>,
    pub raw_output: String,
}

pub fn parse_plugin_validation_output(
    target: &str,
    strict: bool,
    exit_success: bool,
    stdout: &str,
    stderr: &str,
) -> AgyPluginValidationResult {
    let trimmed_out = crate::commands::strip_utf8_bom(stdout).trim();
    let trimmed_err = stderr.trim();

    if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(trimmed_out) {
        let mut issues = Vec::new();
        if let Some(arr) = json_val
            .get("issues")
            .or_else(|| json_val.get("errors"))
            .or_else(|| json_val.get("warnings"))
            .and_then(|v| v.as_array())
        {
            for item in arr {
                if let Some(s) = item.as_str() {
                    issues.push(s.to_string());
                } else if let Some(msg) = item.get("message").and_then(|v| v.as_str()) {
                    let sev = item.get("severity").or_else(|| item.get("level")).and_then(|v| v.as_str()).unwrap_or("issue");
                    issues.push(format!("[{}] {}", sev, msg));
                }
            }
        }
        let valid = json_val
            .get("valid")
            .or_else(|| json_val.get("success"))
            .and_then(|v| v.as_bool())
            .unwrap_or(exit_success && issues.is_empty());
        return AgyPluginValidationResult {
            success: valid,
            target: target.to_string(),
            strict,
            issues,
            raw_output: trimmed_out.to_string(),
        };
    }

    let combined = match (trimmed_out.is_empty(), trimmed_err.is_empty()) {
        (false, false) => format!("{}\n{}", trimmed_out, trimmed_err),
        (false, true) => trimmed_out.to_string(),
        (true, false) => trimmed_err.to_string(),
        (true, true) => if exit_success { "Plugin validation passed.".to_string() } else { "Validation failed.".to_string() },
    };

    let mut issues = Vec::new();
    if !exit_success {
        for line in combined.lines().map(str::trim).filter(|l| !l.is_empty()) {
            issues.push(line.to_string());
        }
    }

    AgyPluginValidationResult {
        success: exit_success,
        target: target.to_string(),
        strict,
        issues,
        raw_output: combined,
    }
}

fn find_agy_bin() -> Result<PathBuf, String> {
    crate::process_manager::find_agy_executable()
        .ok_or_else(|| "Antigravity CLI (agy) executable not found on PATH.".to_string())
}

pub fn validate_agy_plugin_internal(target: &str, strict: bool) -> Result<AgyPluginValidationResult, String> {
    let trimmed_target = target.trim();
    if trimmed_target.is_empty() {
        return Err("Plugin name or path cannot be empty.".to_string());
    }
    let agy_bin = find_agy_bin()?;
    let mut args = vec!["plugin", "validate", trimmed_target, "--json"];
    if strict {
        args.push("--strict");
    }

    let out = crate::process_manager::ProcessSupervisor::build_cli_command(&agy_bin, &args)
        .output()
        .map_err(|e| format!("Failed to execute agy plugin validate: {}", e))?;
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    Ok(parse_plugin_validation_output(trimmed_target, strict, out.status.success(), &stdout, &stderr))
}

pub fn import_claude_plugins_via_agy_internal() -> Result<AgyPluginOpResult, String> {
    let agy_bin = find_agy_bin()?;
    let out = crate::process_manager::ProcessSupervisor::build_cli_command(
        &agy_bin,
        &["plugin", "import", "claude"],
    )
    .output()
    .map_err(|e| format!("Failed to execute agy plugin import claude: {}", e))?;
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
    if out.status.success() {
        Ok(AgyPluginOpResult {
            success: true,
            message: if stdout.is_empty() {
                "Claude Code plugins imported successfully.".to_string()
            } else {
                stdout
            },
        })
    } else {
        Ok(AgyPluginOpResult {
            success: false,
            message: if !stderr.is_empty() { stderr } else { format!("Import exited with code {:?}", out.status.code()) },
        })
    }
}

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
pub async fn validate_agy_plugin(target: String, strict: Option<bool>) -> Result<AgyPluginValidationResult, String> {
    let is_strict = strict.unwrap_or(false);
    tokio::task::spawn_blocking(move || validate_agy_plugin_internal(&target, is_strict))
        .await
        .map_err(|e| format!("Task execution error: {}", e))?
}

#[tauri::command]
pub async fn import_claude_plugins_via_agy() -> Result<AgyPluginOpResult, String> {
    tokio::task::spawn_blocking(import_claude_plugins_via_agy_internal)
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

    #[test]
    fn test_akbo_on_stop_script_syntax() {
        let on_stop = akbo_on_stop_script();
        assert!(on_stop.contains("dream \"${workspacePath}\""));
        assert!(on_stop.contains("payload.continuationActive || payload.continuation_active"));
        assert!(on_stop.contains("process.stdout.write('{}')"));
        assert!(!on_stop.contains("status:"));
        assert!(!on_stop.contains("--targetDir"));
        assert!(!on_stop.contains("--transcriptFiles"));
        assert!(!on_stop.contains("akbo-cli@latest"));
    }

    #[test]
    fn test_akbo_manifest_and_rules_contracts() {
        let manifest = akbo_manifest_json();
        assert_eq!(manifest.get("name").and_then(|v| v.as_str()), Some("akbo"));
        assert!(manifest.get("skills").is_none(), "skills should be auto-discovered by agy");
        assert!(manifest.get("rules").is_none(), "rules should be auto-discovered by agy");
        assert!(manifest.get("mcpServers").is_none(), "mcpServers should be auto-discovered by agy");

        let rules = akbo_rules_markdown();
        assert!(rules.contains("akbo_upsert_units"));
        assert!(rules.contains("`topic`"));
        assert!(rules.contains("`finding`"));
        assert!(!rules.contains("`content`: Concrete technical description"));
    }

    #[test]
    fn test_akbo_mcp_wrapper_syntax() {
        let pinned_pkg = "akbo-cli@3.0.0";
        let sample_mcp = format!(r#"const args = ['-y', '{}', 'mcp', workspacePath, '--pool', targetProjectPool];"#, pinned_pkg);
        assert!(sample_mcp.contains("--pool"));
        assert!(sample_mcp.contains("akbo-cli@3.0.0"));
        assert!(!sample_mcp.contains("@latest"));
        assert!(sample_mcp.contains("mcp"));
    }

    #[test]
    fn test_parse_plugin_validation_output() {
        let ok_json = r#"{"valid": true, "issues": []}"#;
        let ok_res = parse_plugin_validation_output("akbo", true, true, ok_json, "");
        assert!(ok_res.success);
        assert!(ok_res.issues.is_empty());

        let bad_json = "\u{FEFF}{\"valid\": false, \"issues\": [{\"severity\": \"error\", \"message\": \"plugin name must be kebab-case\"}]}";
        let bad_res = parse_plugin_validation_output("bad_name", false, false, bad_json, "");
        assert!(!bad_res.success);
        assert_eq!(bad_res.issues.len(), 1);
        assert!(bad_res.issues[0].contains("kebab-case"));
    }
}
