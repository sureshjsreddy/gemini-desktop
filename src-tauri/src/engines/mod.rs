pub mod agy;
pub mod gemini;

use crate::commands::setup::{get_cli_binary_version, get_user_path_dirs, is_dir_in_path_list};
use crate::transports::TransportProtocol;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub use agy::AgyEngineAdapter;
pub use gemini::GeminiEngineAdapter;

static GEMINI_ADAPTER: GeminiEngineAdapter = GeminiEngineAdapter;
static AGY_ADAPTER: AgyEngineAdapter = AgyEngineAdapter;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelOptionDto {
    pub value: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelOptionGroupDto {
    pub group: String,
    pub options: Vec<ModelOptionDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineSlashCommandMeta {
    pub id: String,
    pub label: String,
    pub title: String,
    pub description: String,
    pub category: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineCapabilities {
    pub transport: String,
    pub supports_plugins: bool,
    pub supports_live_model_discovery: bool,
    pub supports_mcp_config: bool,
    pub supports_mcp_import_from_gemini: bool,
    pub supports_skill_sync: bool,
    pub supports_reasoning_effort: bool,
    pub supports_agent_persona: bool,
    pub supports_orchestrator_settings: bool,
    pub supports_native_compress: bool,
    pub supports_cli_install: bool,
    pub supports_login_launch: bool,
    pub supports_system_prompt_append: bool,
    pub supports_concurrent_steer: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineDescriptor {
    pub id: String,
    pub display_name: String,
    pub short_name: String,
    pub badge_icon: String,
    pub description: String,
    pub default_model: String,
    pub fallback_test_model: String,
    pub global_settings_hint: String,
    pub workspace_instructions_hint: String,
    pub install_hint: String,
    pub installed: bool,
    pub binary_path: Option<String>,
    pub binary_dir: Option<String>,
    pub in_user_path: bool,
    pub cli_version: Option<String>,
    pub capabilities: EngineCapabilities,
    pub model_groups: Vec<ModelOptionGroupDto>,
    pub extra_slash_commands: Vec<EngineSlashCommandMeta>,
}

/// Extension point for each supported CLI agent engine (`gemini`, `agy`, and future CLIs like `claude` or `codex`).
pub trait CliEngineAdapter: Send + Sync {
    fn id(&self) -> &'static str;
    fn transport(&self) -> TransportProtocol;
    fn static_descriptor(&self) -> EngineDescriptor;
    fn find_executable(&self) -> Option<PathBuf>;

    fn build_acp_args(
        &self,
        model: &str,
        approval_mode: &str,
        effort: Option<&str>,
        agent_name: Option<&str>,
        sandbox_enabled: bool,
        additional_dirs: Option<&str>,
    ) -> Vec<String>;

    fn build_headless_prompt_args(
        &self,
        model: Option<&str>,
        prompt_arg: &str,
        output_format: &str,
    ) -> Vec<String>;

    fn normalize_approval_mode(&self, mode: &str) -> &'static str;
    fn map_acp_mode(&self, effective_mode: &str) -> Option<&'static str>;
    fn sanitize_model(&self, model: &str) -> String;
    fn resolve_settings_path(&self) -> Result<PathBuf, String>;
    fn resolve_mcp_config_path(&self, workspace_path: Option<&str>) -> Result<PathBuf, String>;
    fn default_global_settings_json(&self) -> &'static str;
}

/// Returns all registered CLI engine adapters in display order.
/// To add a new CLI (e.g. Claude Code or OpenAI Codex), implement `CliEngineAdapter` and register it here.
pub fn all_adapters() -> Vec<&'static dyn CliEngineAdapter> {
    vec![&GEMINI_ADAPTER, &AGY_ADAPTER]
}

/// Normalizes an arbitrary engine identifier to a registered adapter ID, defaulting to `"gemini"`.
pub fn normalize_engine_id(engine_id: &str) -> &'static str {
    let trimmed = engine_id.trim();
    for adapter in all_adapters() {
        if adapter.id().eq_ignore_ascii_case(trimmed) {
            return adapter.id();
        }
    }
    GEMINI_ADAPTER.id()
}

/// Looks up the engine adapter matching `engine_id`, defaulting cleanly to `gemini`.
pub fn get_adapter(engine_id: &str) -> &'static dyn CliEngineAdapter {
    let trimmed = engine_id.trim();
    for adapter in all_adapters() {
        if adapter.id().eq_ignore_ascii_case(trimmed) {
            return adapter;
        }
    }
    &GEMINI_ADAPTER
}

/// Resolves the user home directory across Windows (`USERPROFILE`) and Unix (`HOME`).
pub fn resolve_home_dir() -> Result<PathBuf, String> {
    std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map(PathBuf::from)
        .map_err(|_| "Could not determine user home directory".to_string())
}

/// Parses comma/newline-separated additional directories into clean trimmed tokens.
pub fn parse_additional_dirs(raw: Option<&str>) -> Vec<String> {
    raw.unwrap_or("")
        .split([',', '\n', ';'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

/// Builds live descriptors (binary detection, PATH membership, optional version probe) for all registered engines.
pub fn build_engines_catalog(check_path: bool, include_versions: bool) -> Vec<EngineDescriptor> {
    let user_dirs = if check_path {
        get_user_path_dirs()
    } else {
        Vec::new()
    };

    all_adapters()
        .into_iter()
        .map(|adapter| {
            let mut desc = adapter.static_descriptor();
            desc.capabilities.transport = adapter.transport().as_str().to_string();

            if let Some(bin) = adapter.find_executable() {
                desc.installed = true;
                desc.binary_path = Some(bin.to_string_lossy().to_string());
                if let Some(dir) = bin.parent() {
                    desc.binary_dir = Some(dir.to_string_lossy().to_string());
                    if check_path {
                        desc.in_user_path = is_dir_in_path_list(dir, &user_dirs);
                    }
                }
                if include_versions {
                    desc.cli_version = get_cli_binary_version(&bin);
                }
            }
            desc
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_registered_adapters_have_unique_ids_and_valid_descriptors() {
        let adapters = all_adapters();
        assert!(adapters.len() >= 2);
        let mut seen = HashSet::new();
        for adapter in adapters {
            assert!(seen.insert(adapter.id()));
            let desc = adapter.static_descriptor();
            assert_eq!(desc.id, adapter.id());
            assert!(!desc.model_groups.is_empty());
        }
    }

    #[test]
    fn test_normalize_and_lookup_adapters() {
        assert_eq!(normalize_engine_id("agy"), "agy");
        assert_eq!(normalize_engine_id(" AGY "), "agy");
        assert_eq!(normalize_engine_id("gemini"), "gemini");
        assert_eq!(normalize_engine_id("unknown_future_cli"), "gemini");
        assert_eq!(get_adapter("agy").id(), "agy");
        assert_eq!(get_adapter("anything_else").id(), "gemini");
    }

    #[test]
    fn test_build_engines_catalog_contains_gemini_and_agy() {
        let catalog = build_engines_catalog(false, false);
        assert!(catalog.iter().any(|e| e.id == "gemini" && !e.capabilities.supports_plugins));
        assert!(catalog.iter().any(|e| e.id == "agy" && e.capabilities.supports_plugins));
    }
}
