use super::{
    parse_additional_dirs, resolve_home_dir, CliEngineAdapter, EngineCapabilities, EngineDescriptor,
    EngineSlashCommandMeta, ModelOptionDto, ModelOptionGroupDto,
};
use crate::process_manager::find_gemini_executable;
use crate::transports::TransportProtocol;
use std::path::PathBuf;

pub struct GeminiEngineAdapter;

impl CliEngineAdapter for GeminiEngineAdapter {
    fn id(&self) -> &'static str {
        "gemini"
    }

    fn transport(&self) -> TransportProtocol {
        TransportProtocol::Acp
    }

    fn static_descriptor(&self) -> EngineDescriptor {
        EngineDescriptor {
            id: "gemini".to_string(),
            display_name: "Gemini CLI (gemini)".to_string(),
            short_name: "Gemini CLI".to_string(),
            badge_icon: "♊".to_string(),
            description: "Standard Gemini CLI supporting Google AI Studio and Vertex AI dual authentication.".to_string(),
            default_model: "auto".to_string(),
            fallback_test_model: "gemini-3.8-flash".to_string(),
            global_settings_hint: "~/.gemini/settings.json".to_string(),
            workspace_instructions_hint: "Gemini CLI will run inside this directory and auto-read GEMINI.md.".to_string(),
            install_hint: "npm install -g @google/gemini-cli@latest or winget install Google.GeminiCLI".to_string(),
            installed: false,
            binary_path: None,
            binary_dir: None,
            in_user_path: false,
            cli_version: None,
            capabilities: EngineCapabilities {
                transport: "acp".to_string(),
                supports_plugins: false,
                supports_live_model_discovery: true,
                supports_mcp_config: true,
                supports_mcp_import_from_gemini: false,
                supports_skill_sync: false,
                supports_reasoning_effort: false,
                supports_agent_persona: false,
                supports_orchestrator_settings: false,
                supports_native_compress: true,
                supports_cli_install: true,
                supports_login_launch: false,
                supports_system_prompt_append: false,
                supports_concurrent_steer: false,
            },
            model_groups: vec![
                ModelOptionGroupDto {
                    group: "Auto & Routing".to_string(),
                    options: vec![
                        ModelOptionDto {
                            value: "auto".to_string(),
                            label: "Auto — Let Gemini CLI decide the best model (Recommended)".to_string(),
                        },
                        ModelOptionDto {
                            value: "auto-gemini-2.5".to_string(),
                            label: "Auto (Gemini 2.5) — Auto-selects 2.5 Pro / 2.5 Flash".to_string(),
                        },
                    ],
                },
                ModelOptionGroupDto {
                    group: "Gemini 3 / 3.5 / 3.8 (Recommended & Latest)".to_string(),
                    options: vec![
                        ModelOptionDto {
                            value: "gemini-3.8-flash".to_string(),
                            label: "gemini-3.8-flash (Latest Preview — Ultra-responsive next-gen Flash)".to_string(),
                        },
                        ModelOptionDto {
                            value: "gemini-3.5-flash".to_string(),
                            label: "gemini-3.5-flash (Fast & powerful default in CLI 0.59+)".to_string(),
                        },
                        ModelOptionDto {
                            value: "gemini-3.5-flash-lite".to_string(),
                            label: "gemini-3.5-flash-lite (Ultra-fast & lightweight 3.5 — Upgrade target)".to_string(),
                        },
                        ModelOptionDto {
                            value: "gemini-3.1-pro-preview".to_string(),
                            label: "gemini-3.1-pro-preview (Deep reasoning, complex coding)".to_string(),
                        },
                        ModelOptionDto {
                            value: "gemini-3.1-flash-lite".to_string(),
                            label: "gemini-3.1-flash-lite (Ultra-fast & cost-effective)".to_string(),
                        },
                        ModelOptionDto {
                            value: "gemini-3-pro-preview".to_string(),
                            label: "gemini-3-pro-preview (Preview reasoning & coding)".to_string(),
                        },
                        ModelOptionDto {
                            value: "gemini-3-flash-preview".to_string(),
                            label: "gemini-3-flash-preview (High speed, fast results)".to_string(),
                        },
                    ],
                },
                ModelOptionGroupDto {
                    group: "Gemini 2.5".to_string(),
                    options: vec![
                        ModelOptionDto {
                            value: "gemini-2.5-pro".to_string(),
                            label: "gemini-2.5-pro (Complex tasks, coding & architecture)".to_string(),
                        },
                        ModelOptionDto {
                            value: "gemini-2.5-flash".to_string(),
                            label: "gemini-2.5-flash (Balanced speed & performance)".to_string(),
                        },
                        ModelOptionDto {
                            value: "gemini-2.5-flash-lite".to_string(),
                            label: "gemini-2.5-flash-lite (Active on AI Studio; Vertex retiring late 2026)".to_string(),
                        },
                    ],
                },
                ModelOptionGroupDto {
                    group: "Gemini 1.5 (Deprecated / Retired Upstream)".to_string(),
                    options: vec![
                        ModelOptionDto {
                            value: "gemini-1.5-pro".to_string(),
                            label: "gemini-1.5-pro (Deprecated / Retired by Google)".to_string(),
                        },
                        ModelOptionDto {
                            value: "gemini-1.5-flash".to_string(),
                            label: "gemini-1.5-flash (Deprecated / Retired by Google)".to_string(),
                        },
                        ModelOptionDto {
                            value: "gemini-1.5-flash-8b".to_string(),
                            label: "gemini-1.5-flash-8b (Deprecated / Retired by Google)".to_string(),
                        },
                    ],
                },
            ],
            extra_slash_commands: vec![EngineSlashCommandMeta {
                id: "compress".to_string(),
                label: "/compress".to_string(),
                title: "Compress Context".to_string(),
                description: "Gemini CLI native conversation context compression".to_string(),
                category: "action".to_string(),
            }],
        }
    }

    fn find_executable(&self) -> Option<PathBuf> {
        find_gemini_executable()
    }

    fn build_acp_args(
        &self,
        model: &str,
        approval_mode: &str,
        _effort: Option<&str>,
        _agent_name: Option<&str>,
        sandbox_enabled: bool,
        additional_dirs: Option<&str>,
    ) -> Vec<String> {
        let mut extra_args = Vec::new();
        let trimmed_model = model.trim();
        if !trimmed_model.is_empty() && trimmed_model != "auto" {
            extra_args.push("--model".to_string());
            extra_args.push(trimmed_model.to_string());
        }

        match approval_mode {
            "yolo" => {
                extra_args.push("--approval-mode".to_string());
                extra_args.push("yolo".to_string());
            }
            // "auto" maps safely to Gemini CLI's auto_edit since Gemini CLI has no "auto" mode choice
            "auto_edit" | "auto" => {
                extra_args.push("--approval-mode".to_string());
                extra_args.push("auto_edit".to_string());
            }
            "plan" => {
                extra_args.push("--approval-mode".to_string());
                extra_args.push("plan".to_string());
            }
            _ => {
                extra_args.push("--approval-mode".to_string());
                extra_args.push("default".to_string());
            }
        }

        if sandbox_enabled {
            extra_args.push("--sandbox".to_string());
        }

        for dir in parse_additional_dirs(additional_dirs) {
            extra_args.push("--include-directories".to_string());
            extra_args.push(dir);
        }

        extra_args
    }

    fn build_headless_prompt_args(
        &self,
        model: Option<&str>,
        prompt_arg: &str,
        output_format: &str,
    ) -> Vec<String> {
        let mut args = vec!["--skip-trust".to_string(), "-p".to_string(), prompt_arg.to_string()];
        if let Some(m) = model.map(str::trim).filter(|s| !s.is_empty() && *s != "auto") {
            args.push("--model".to_string());
            args.push(m.to_string());
        }
        args.push("--output-format".to_string());
        args.push(output_format.to_string());
        args
    }

    fn normalize_approval_mode(&self, mode: &str) -> &'static str {
        match mode {
            "yolo" => "yolo",
            "auto_edit" | "auto" => "auto_edit",
            "plan" => "plan",
            _ => "default",
        }
    }

    fn map_acp_mode(&self, effective_mode: &str) -> Option<&'static str> {
        Some(match effective_mode {
            "yolo" => "yolo",
            "plan" => "plan",
            "auto_edit" | "auto" => "autoEdit",
            _ => "default",
        })
    }

    fn sanitize_model(&self, model: &str) -> String {
        let trimmed = model.trim();
        if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("auto") {
            return "auto".to_string();
        }
        let lower = trimmed.to_lowercase();
        let is_agy_specific = lower.starts_with("claude-")
            || lower.starts_with("gpt-oss-")
            || lower == "gemini-4"
            || lower.starts_with("gemini-4-")
            || lower == "orchestrator"
            || lower.ends_with("-high")
            || lower.ends_with("-xhigh")
            || lower.ends_with("-medium")
            || lower.ends_with("-low")
            || lower.ends_with("-max");

        if is_agy_specific {
            "auto".to_string()
        } else {
            trimmed.to_string()
        }
    }

    fn resolve_settings_path(&self) -> Result<PathBuf, String> {
        Ok(resolve_home_dir()?.join(".gemini").join("settings.json"))
    }

    fn resolve_mcp_config_path(&self, workspace_path: Option<&str>) -> Result<PathBuf, String> {
        if let Some(ws) = workspace_path {
            let trimmed = ws.trim();
            if !trimmed.is_empty() {
                return Ok(PathBuf::from(trimmed).join(".gemini").join("settings.json"));
            }
        }
        self.resolve_settings_path()
    }

    fn default_global_settings_json(&self) -> &'static str {
        "{\n  \"model\": {\n    \"maxSessionTurns\": -1\n  }\n}"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gemini_adapter_headless_args_modes_and_paths() {
        let adapter = GeminiEngineAdapter;
        assert_eq!(adapter.id(), "gemini");
        assert_eq!(adapter.transport(), TransportProtocol::Acp);

        let args = adapter.build_headless_prompt_args(Some("gemini-3.8-flash"), "hello", "text");
        assert_eq!(
            args,
            vec!["--skip-trust", "-p", "hello", "--model", "gemini-3.8-flash", "--output-format", "text"]
        );

        assert_eq!(adapter.normalize_approval_mode("auto"), "auto_edit");
        assert_eq!(adapter.map_acp_mode("auto"), Some("autoEdit"));
        assert_eq!(adapter.map_acp_mode("yolo"), Some("yolo"));
        assert_eq!(adapter.sanitize_model("gemini-4"), "auto");
        assert_eq!(adapter.sanitize_model("gemini-3.8-flash"), "gemini-3.8-flash");

        assert!(adapter.default_global_settings_json().contains("maxSessionTurns"));

        let ws_mcp = adapter.resolve_mcp_config_path(Some("C:\\Work\\Proj")).unwrap();
        assert!(ws_mcp.ends_with(".gemini\\settings.json") || ws_mcp.ends_with(".gemini/settings.json"));
    }
}
