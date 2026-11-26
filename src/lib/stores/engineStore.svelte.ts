import { invoke } from "@tauri-apps/api/core";
import type {
  CliEngine,
  CliEnginesStatus,
  CliPathStatus,
  EngineDescriptor,
  GeminiEnvStatus,
  PathAddResult,
} from "$lib/types";
import { getEffectiveModel as getEffectiveModelHelper } from "$lib/utils/engineModels";

const ACTIVE_ENGINE_STORAGE_KEY = "gemini_desktop_active_engine";

const FALLBACK_DESCRIPTORS: EngineDescriptor[] = [
  {
    id: "gemini",
    displayName: "Gemini CLI (gemini)",
    shortName: "Gemini CLI",
    badgeIcon: "♊",
    description: "Standard Gemini CLI supporting Google AI Studio and Vertex AI dual authentication.",
    defaultModel: "auto",
    fallbackTestModel: "gemini-3.8-flash",
    globalSettingsHint: "~/.gemini/settings.json",
    workspaceInstructionsHint: "Gemini CLI will run inside this directory and auto-read GEMINI.md.",
    installHint: "npm install -g @google/gemini-cli@latest",
    installed: false,
    binaryPath: null,
    binaryDir: null,
    inUserPath: false,
    cliVersion: null,
    capabilities: {
      transport: "acp",
      supportsPlugins: false,
      supportsLiveModelDiscovery: true,
      supportsMcpConfig: true,
      supportsMcpImportFromGemini: false,
      supportsSkillSync: false,
      supportsReasoningEffort: false,
      supportsAgentPersona: false,
      supportsOrchestratorSettings: false,
      supportsNativeCompress: true,
      supportsCliInstall: true,
      supportsLoginLaunch: false,
      supportsSystemPromptAppend: false,
      supportsConcurrentSteer: false,
    },
    modelGroups: [],
    extraSlashCommands: [],
  },
  {
    id: "agy",
    displayName: "Antigravity CLI (agy)",
    shortName: "Antigravity (agy)",
    badgeIcon: "🚀",
    description: "Enterprise Antigravity agent runtime with native Agent Client Protocol (ACP) support.",
    defaultModel: "auto",
    fallbackTestModel: "auto",
    globalSettingsHint: "~/.gemini/antigravity-cli/settings.json",
    workspaceInstructionsHint: "Antigravity CLI will run inside this directory and auto-read GEMINI.md or AGENTS.md.",
    installHint: "agy update or irm https://antigravity.google/cli/install.ps1 | iex",
    installed: false,
    binaryPath: null,
    binaryDir: null,
    inUserPath: false,
    cliVersion: null,
    capabilities: {
      transport: "acp",
      supportsPlugins: true,
      supportsLiveModelDiscovery: true,
      supportsMcpConfig: true,
      supportsMcpImportFromGemini: true,
      supportsSkillSync: true,
      supportsReasoningEffort: true,
      supportsAgentPersona: true,
      supportsOrchestratorSettings: true,
      supportsNativeCompress: false,
      supportsCliInstall: true,
      supportsLoginLaunch: true,
      supportsSystemPromptAppend: true,
      supportsConcurrentSteer: true,
    },
    modelGroups: [],
    extraSlashCommands: [],
  },
];

function getInitialEngine(): CliEngine {
  if (typeof window !== "undefined") {
    try {
      const saved = localStorage.getItem(ACTIVE_ENGINE_STORAGE_KEY);
      if (saved && saved.trim()) return saved.trim();
    } catch {
      // Ignore storage error
    }
  }
  return "gemini";
}

class EngineStore {
  activeEngine = $state<CliEngine>(getInitialEngine());
  catalog = $state<EngineDescriptor[]>(FALLBACK_DESCRIPTORS);
  enginesStatus = $state<CliEnginesStatus | null>(null);
  cliPathStatus = $state<CliPathStatus | null>(null);
  envStatus = $state<GeminiEnvStatus | null>(null);
  isSwitching = $state(false);
  isCheckingPath = $state(false);
  isAddingToPath = $state(false);
  pathAddResult = $state<PathAddResult | null>(null);

  activeDescriptor = $derived<EngineDescriptor>(
    this.catalog.find((e) => e.id === this.activeEngine) ??
      this.catalog[0] ??
      FALLBACK_DESCRIPTORS[0]
  );

  getDescriptor(engineId?: string | null): EngineDescriptor {
    const target = (engineId || this.activeEngine).trim().toLowerCase();
    return (
      this.catalog.find((e) => e.id.toLowerCase() === target) ??
      this.activeDescriptor
    );
  }

  getEffectiveModel(model: string | undefined, engineId?: string): string {
    return getEffectiveModelHelper(model, engineId || this.activeEngine);
  }

  async refresh(): Promise<void> {
    this.isCheckingPath = true;
    try {
      const [active, catalogRes, statusRes, pathRes, envRes] = await Promise.all([
        invoke<string>("get_active_cli_engine").catch(() => this.activeEngine),
        invoke<EngineDescriptor[]>("get_engines_catalog").catch(() => null),
        invoke<CliEnginesStatus>("get_cli_engines_status").catch(() => null),
        invoke<CliPathStatus>("check_cli_path_status").catch(() => null),
        invoke<GeminiEnvStatus>("check_gemini_env").catch(() => null),
      ]);

      if (active && active.trim()) {
        this.activeEngine = active.trim();
        if (typeof window !== "undefined") {
          try {
            localStorage.setItem(ACTIVE_ENGINE_STORAGE_KEY, this.activeEngine);
          } catch {}
        }
      }
      if (Array.isArray(catalogRes) && catalogRes.length > 0) {
        this.catalog = catalogRes;
      }
      if (statusRes) {
        this.enginesStatus = statusRes;
      }
      if (pathRes) {
        this.cliPathStatus = pathRes;
      }
      if (envRes) {
        this.envStatus = envRes;
      }
    } finally {
      this.isCheckingPath = false;
    }
  }

  async switchEngine(engineId: CliEngine): Promise<CliEngine> {
    const trimmed = engineId.trim();
    if (!trimmed || (trimmed === this.activeEngine && !this.isSwitching)) {
      return this.activeEngine;
    }

    this.isSwitching = true;
    this.pathAddResult = null;
    try {
      const updated = await invoke<string>("set_active_cli_engine", { engine: trimmed });
      this.activeEngine = updated;
      if (typeof window !== "undefined") {
        try {
          localStorage.setItem(ACTIVE_ENGINE_STORAGE_KEY, updated);
        } catch {}
      }
      await this.refresh();
      return updated;
    } finally {
      this.isSwitching = false;
    }
  }

  async addToPath(engineId?: CliEngine): Promise<PathAddResult | null> {
    const target = engineId || this.activeEngine;
    this.isAddingToPath = true;
    this.pathAddResult = null;
    try {
      const res = await invoke<PathAddResult>("add_cli_to_user_path", { engine: target });
      this.pathAddResult = res;
      await this.refresh();
      return res;
    } catch (e: any) {
      this.pathAddResult = {
        success: false,
        message: e?.toString() || "Failed to add directory to User PATH.",
        targetDir: "",
        alreadyInPath: false,
      };
      return this.pathAddResult;
    } finally {
      this.isAddingToPath = false;
    }
  }
}

export const engineStore = new EngineStore();
