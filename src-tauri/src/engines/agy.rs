use super::{
    parse_additional_dirs, resolve_home_dir, CliEngineAdapter, EngineCapabilities, EngineDescriptor,
    EngineSlashCommandMeta, ModelOptionDto, ModelOptionGroupDto,
};
use crate::process_manager::find_agy_executable;
use crate::transports::TransportProtocol;
use std::path::PathBuf;

pub struct AgyEngineAdapter;

impl CliEngineAdapter for AgyEngineAdapter {
    fn id(&self) -> &'static str {
        "agy"
    }

    fn transport(&self) -> TransportProtocol {
        TransportProtocol::Acp
    }

    fn static_descriptor(&self) -> EngineDescriptor {
        EngineDescriptor {
            id: "agy".to_string(),
            display_name: "Antigravity CLI (agy)".to_string(),
            short_name: "Antigravity (agy)".to_string(),
            badge_icon: "🚀".to_string(),
            description: "Enterprise Antigravity agent runtime with native Agent Client Protocol (ACP) support.".to_string(),
            default_model: "auto".to_string(),
            fallback_test_model: "auto".to_string(),
            global_settings_hint: "~/.gemini/antigravity-cli/settings.json".to_string(),
            workspace_instructions_hint: "Antigravity CLI will run inside this directory and auto-read GEMINI.md or AGENTS.md.".to_string(),
            install_hint: "agy update or irm https://antigravity.google/cli/install.ps1 | iex".to_string(),
            installed: false,
            binary_path: None,
            binary_dir: None,
            in_user_path: false,
            cli_version: None,
            capabilities: EngineCapabilities {
                transport: "acp".to_string(),
                supports_plugins: true,
                supports_live_model_discovery: true,
                supports_mcp_config: true,
                supports_mcp_import_from_gemini: true,
                supports_skill_sync: true,
                supports_reasoning_effort: true,
                supports_agent_persona: true,
                supports_orchestrator_settings: true,
                supports_native_compress: false,
                supports_cli_install: true,
                supports_login_launch: true,
                supports_system_prompt_append: true,
                supports_concurrent_steer: true,
            },
            model_groups: vec![
                ModelOptionGroupDto {
                    group: "Antigravity Default & Orchestrator".to_string(),
                    options: vec![
                        ModelOptionDto {
                            value: "auto".to_string(),
                            label: "Auto — Default Antigravity model".to_string(),
                        },
                        ModelOptionDto {
                            value: "orchestrator".to_string(),
                            label: "orchestrator — Multi-Model Coordinator (Planner + Executor)".to_string(),
                        },
                    ],
                },
                ModelOptionGroupDto {
                    group: "Gemini 4 Frontier (Hydrogen / Carbon / Neon)".to_string(),
                    options: vec![
                        ModelOptionDto {
                            value: "gemini-4-hydrogen-high".to_string(),
                            label: "gemini-4-hydrogen-high (Gemini 4 Hydrogen High)".to_string(),
                        },
                        ModelOptionDto {
                            value: "gemini-4-hydrogen-xhigh".to_string(),
                            label: "gemini-4-hydrogen-xhigh (Gemini 4 Hydrogen Extra High)".to_string(),
                        },
                        ModelOptionDto {
                            value: "gemini-4-hydrogen-medium".to_string(),
                            label: "gemini-4-hydrogen-medium (Gemini 4 Hydrogen Medium)".to_string(),
                        },
                        ModelOptionDto {
                            value: "gemini-4-hydrogen-low".to_string(),
                            label: "gemini-4-hydrogen-low (Gemini 4 Hydrogen Low)".to_string(),
                        },
                        ModelOptionDto {
                            value: "gemini-4-carbon".to_string(),
                            label: "gemini-4-carbon (Gemini 4 Carbon — Fast Frontier)".to_string(),
                        },
                        ModelOptionDto {
                            value: "gemini-4-neon".to_string(),
                            label: "gemini-4-neon (Gemini 4 Neon — Balanced Frontier)".to_string(),
                        },
                        ModelOptionDto {
                            value: "gemini-4-argon".to_string(),
                            label: "gemini-4-argon (Gemini 4 Argon — Legacy Frontier)".to_string(),
                        },
                    ],
                },
                ModelOptionGroupDto {
                    group: "Gemini 3.8 Flash (Reasoning & Speed)".to_string(),
                    options: vec![
                        ModelOptionDto {
                            value: "gemini-3.8-flash-high".to_string(),
                            label: "gemini-3.8-flash-high (Gemini 3.8 Flash High Reasoning)".to_string(),
                        },
                        ModelOptionDto {
                            value: "gemini-3.8-flash-medium".to_string(),
                            label: "gemini-3.8-flash-medium (Gemini 3.8 Flash Medium Reasoning)".to_string(),
                        },
                        ModelOptionDto {
                            value: "gemini-3.8-flash-low".to_string(),
                            label: "gemini-3.8-flash-low (Gemini 3.8 Flash Low Reasoning)".to_string(),
                        },
                    ],
                },
                ModelOptionGroupDto {
                    group: "Anthropic Claude (Extended Thinking)".to_string(),
                    options: vec![
                        ModelOptionDto {
                            value: "claude-sonnet-4-6".to_string(),
                            label: "claude-sonnet-4-6 (Claude Sonnet 4.6 Thinking)".to_string(),
                        },
                        ModelOptionDto {
                            value: "claude-opus-4-6-thinking".to_string(),
                            label: "claude-opus-4-6-thinking (Claude Opus 4.6 Thinking)".to_string(),
                        },
                    ],
                },
                ModelOptionGroupDto {
                    group: "Open Models".to_string(),
                    options: vec![ModelOptionDto {
                        value: "gpt-oss-120b-medium".to_string(),
                        label: "gpt-oss-120b-medium (GPT-OSS 120B Medium Reasoning)".to_string(),
                    }],
                },
            ],
            extra_slash_commands: vec![EngineSlashCommandMeta {
                id: "plugins".to_string(),
                label: "/plugins".to_string(),
                title: "Antigravity Plugins & Skills".to_string(),
                description: "Open Antigravity plugins hub, install extensions, or import from Gemini/Claude".to_string(),
                category: "tool".to_string(),
            }],
        }
    }

    fn find_executable(&self) -> Option<PathBuf> {
        find_agy_executable()
    }

    fn build_acp_args(
        &self,
        model: &str,
        approval_mode: &str,
        effort: Option<&str>,
        agent_name: Option<&str>,
        sandbox_enabled: bool,
        additional_dirs: Option<&str>,
    ) -> Vec<String> {
        let mut extra_args = Vec::new();
        let trimmed_model = model.trim();
        if !trimmed_model.is_empty() && trimmed_model != "auto" {
            extra_args.push("--model".to_string());
            extra_args.push(trimmed_model.to_string());
        }

        if let Some(eff) = effort.map(str::trim).filter(|s| !s.is_empty()) {
            extra_args.push("--effort".to_string());
            extra_args.push(eff.to_string());
        }

        if let Some(agent) = agent_name.map(str::trim).filter(|s| !s.is_empty()) {
            extra_args.push("--agent".to_string());
            extra_args.push(agent.to_string());
        }

        match approval_mode {
            "yolo" => {
                extra_args.push("--dangerously-skip-permissions".to_string());
            }
            "auto_edit" => {
                extra_args.push("--mode".to_string());
                extra_args.push("accept-edits".to_string());
            }
            "auto" => {
                extra_args.push("--mode".to_string());
                extra_args.push("auto".to_string());
            }
            "plan" => {
                extra_args.push("--mode".to_string());
                extra_args.push("plan".to_string());
            }
            _ => {}
        }

        if sandbox_enabled {
            extra_args.push("--sandbox".to_string());
        }

        for dir in parse_additional_dirs(additional_dirs) {
            extra_args.push("--add-dir".to_string());
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
        let mut args = vec!["-p".to_string(), prompt_arg.to_string()];
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
            "auto_edit" => "auto_edit",
            "auto" => "auto",
            "plan" => "plan",
            _ => "default",
        }
    }

    fn map_acp_mode(&self, effective_mode: &str) -> Option<&'static str> {
        match effective_mode {
            "auto_edit" => Some("accept-edits"),
            "auto" => Some("auto"),
            "plan" => Some("plan"),
            // For agy in yolo mode, --dangerously-skip-permissions was passed at launch.
            "yolo" => None,
            _ => Some("default"),
        }
    }

    fn sanitize_model(&self, model: &str) -> String {
        let trimmed = model.trim();
        if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("auto") {
            return "auto".to_string();
        }
        let lower = trimmed.to_lowercase();
        let is_gemini_cli_specific = lower.starts_with("gemini-2.5")
            || lower.starts_with("gemini-1.5")
            || lower.starts_with("gemini-3.5")
            || lower.starts_with("gemini-3.1")
            || lower.starts_with("gemini-3-")
            || lower == "gemini-3.8-flash";

        if is_gemini_cli_specific {
            "auto".to_string()
        } else {
            trimmed.to_string()
        }
    }

    fn resolve_settings_path(&self) -> Result<PathBuf, String> {
        Ok(resolve_home_dir()?
            .join(".gemini")
            .join("antigravity-cli")
            .join("settings.json"))
    }

    fn resolve_mcp_config_path(&self, workspace_path: Option<&str>) -> Result<PathBuf, String> {
        if let Some(ws) = workspace_path {
            let trimmed = ws.trim();
            if !trimmed.is_empty() {
                return Ok(PathBuf::from(trimmed).join(".agents").join("mcp_config.json"));
            }
        }
        Ok(resolve_home_dir()?
            .join(".gemini")
            .join("config")
            .join("mcp_config.json"))
    }

    fn default_global_settings_json(&self) -> &'static str {
        "{\n  \"agentMode\": \"auto\",\n  \"showThinking\": true\n}"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agy_adapter_headless_args_modes_and_paths() {
        let adapter = AgyEngineAdapter;
        assert_eq!(adapter.id(), "agy");
        assert_eq!(adapter.transport(), TransportProtocol::Acp);

        let auto_args = adapter.build_headless_prompt_args(Some("auto"), "hi", "json");
        assert!(!auto_args.contains(&"--model".to_string()));
        assert_eq!(auto_args, vec!["-p", "hi", "--output-format", "json"]);

        let model_args = adapter.build_headless_prompt_args(Some("gemini-4-carbon"), "hi", "text");
        assert_eq!(
            model_args,
            vec!["-p", "hi", "--model", "gemini-4-carbon", "--output-format", "text"]
        );

        assert_eq!(adapter.normalize_approval_mode("auto_edit"), "auto_edit");
        assert_eq!(adapter.normalize_approval_mode("auto"), "auto");
        assert_eq!(adapter.map_acp_mode("auto_edit"), Some("accept-edits"));
        assert_eq!(adapter.map_acp_mode("yolo"), None);

        assert!(adapter.default_global_settings_json().contains("agentMode"));

        let ws_mcp = adapter.resolve_mcp_config_path(Some("C:\\Work\\Proj")).unwrap();
        assert!(ws_mcp.ends_with(".agents\\mcp_config.json") || ws_mcp.ends_with(".agents/mcp_config.json"));
    }
}
