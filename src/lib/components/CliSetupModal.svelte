<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import type {
    GeminiEnvStatus,
    GeminiAuthConfig,
    CliInstallResult,
    CliTestResult,
    GCloudStatus,
    Workspace,
    CliEngine,
    CliEnginesStatus,
    CliPathStatus,
    PathAddResult,
  } from "$lib/types";
  import {
    Terminal,
    Key,
    Cloud,
    CheckCircle2,
    XCircle,
    AlertCircle,
    ExternalLink,
    Eye,
    EyeOff,
    RefreshCw,
    X,
    Cpu,
    Copy,
    Check,
    Download,
    ShieldCheck,
    Building2,
    FolderKanban,
    Globe,
    Sparkles,
  } from "lucide-svelte";

  let {
    isOpen = false,
    envStatus = null,
    activeWorkspace = null,
    onClose,
    onEnvStatusUpdated,
  }: {
    isOpen: boolean;
    envStatus: GeminiEnvStatus | null;
    activeWorkspace: Workspace | null;
    onClose: () => void;
    onEnvStatusUpdated?: (status: GeminiEnvStatus) => void;
  } = $props();

  type TabType = "cli" | "auth" | "diagnostics";
  let activeTab = $state<TabType>("cli");

  // Auth Configuration State
  let authMode = $state<"api_key" | "vertex_ai">("vertex_ai");
  let apiKey = $state("");
  let showApiKey = $state(false);
  let googleCloudProject = $state("");
  let googleCloudLocation = $state("us-central1");
  let googleAppCredentials = $state("");
  let applyToWorkspace = $state(true);

  // Installer State
  let isInstalling = $state(false);
  let installResult = $state<CliInstallResult | null>(null);

  // GCloud detection state
  let gcloudStatus = $state<GCloudStatus | null>(null);
  let isCheckingGCloud = $state(false);

  // Diagnostic Test State
  let isTesting = $state(false);
  let testResult = $state<CliTestResult | null>(null);
  let testModel = $state("gemini-3.8-flash");

  // General feedback
  let saveMessage = $state<{ type: "success" | "error"; text: string } | null>(null);
  let isSaving = $state(false);
  let hasCopiedPath = $state(false);
  let hasCopiedError = $state(false);

  const VERTEX_LOCATIONS = [
    { id: "us-central1", label: "us-central1 (Iowa - Recommended)" },
    { id: "us-east4", label: "us-east4 (N. Virginia)" },
    { id: "us-west1", label: "us-west1 (Oregon)" },
    { id: "europe-west1", label: "europe-west1 (Belgium)" },
    { id: "europe-west4", label: "europe-west4 (Netherlands)" },
    { id: "europe-west9", label: "europe-west9 (Paris)" },
    { id: "asia-northeast1", label: "asia-northeast1 (Tokyo)" },
    { id: "asia-southeast1", label: "asia-southeast1 (Singapore)" },
  ];

  let activeEngine = $state<CliEngine>("gemini");
  let enginesStatus = $state<CliEnginesStatus | null>(null);

  // Agy Updater State
  let isUpdatingAgy = $state(false);
  let agyInstallResult = $state<CliInstallResult | null>(null);

  // PATH Management State
  let cliPathStatus = $state<CliPathStatus | null>(null);
  let isCheckingPath = $state(false);
  let isAddingToPath = $state(false);
  let pathAddResult = $state<PathAddResult | null>(null);

  // Gemini Extensions / Skills Importer State
  let isImportingPlugins = $state(false);
  let pluginImportResult = $state<{ success: boolean; message: string; output: string } | null>(null);

  $effect(() => {
    if (isOpen) {
      loadSavedConfig();
      loadGCloudStatus();
      loadEngineStatus();
      loadPathStatus();
    }
  });

  async function loadPathStatus() {
    isCheckingPath = true;
    try {
      cliPathStatus = await invoke<CliPathStatus>("check_cli_path_status");
    } catch (e) {
      console.warn("Failed to check CLI PATH status:", e);
    } finally {
      isCheckingPath = false;
    }
  }

  async function loadEngineStatus() {
    try {
      const status = await invoke<CliEnginesStatus>("get_cli_engines_status");
      enginesStatus = status;
      activeEngine = status.activeEngine;
    } catch (e) {
      console.warn("Failed to load CLI engines status:", e);
    }
  }

  async function handleSwitchEngine(engine: CliEngine) {
    if (activeEngine === engine) return;
    activeEngine = engine;
    testResult = null;
    try {
      await invoke<string>("set_active_cli_engine", { engine });
      if (typeof window !== "undefined") {
        localStorage.setItem("gemini_desktop_active_engine", engine);
      }
      await loadEngineStatus();
      await loadPathStatus();
    } catch (e) {
      console.warn("Failed to switch engine:", e);
    }
  }

  async function handleUpdateOrInstallAgy() {
    isUpdatingAgy = true;
    agyInstallResult = null;
    try {
      const res = await invoke<CliInstallResult>("update_or_install_agy_cli");
      agyInstallResult = res;
      await loadEngineStatus();
      await loadPathStatus();
    } catch (e: any) {
      agyInstallResult = {
        success: false,
        message: e?.toString() || "Antigravity CLI update failed.",
      };
    } finally {
      isUpdatingAgy = false;
    }
  }

  async function handleAddToPath(engine: CliEngine) {
    isAddingToPath = true;
    pathAddResult = null;
    try {
      const res = await invoke<PathAddResult>("add_cli_to_user_path", { engine });
      pathAddResult = res;
      await loadPathStatus();
      await loadEngineStatus();
    } catch (e: any) {
      pathAddResult = {
        success: false,
        message: e?.toString() || "Failed to add directory to User PATH.",
        targetDir: "",
        alreadyInPath: false,
      };
    } finally {
      isAddingToPath = false;
    }
  }

  async function loadSavedConfig() {
    try {
      const config = await invoke<GeminiAuthConfig>("get_gemini_auth_config");
      authMode = config.auth_mode || "vertex_ai";
      apiKey = config.api_key || "";
      googleCloudProject = config.google_cloud_project || "";
      googleCloudLocation = config.google_cloud_location || "us-central1";
      googleAppCredentials = config.google_app_credentials || "";
    } catch (e) {
      console.warn("Failed to load Gemini auth configuration:", e);
    }
  }

  async function loadGCloudStatus() {
    isCheckingGCloud = true;
    try {
      gcloudStatus = await invoke<GCloudStatus>("detect_gcloud_status");
      // If user hasn't entered a GCP project yet and gcloud has one, auto-suggest it
      if (!googleCloudProject && gcloudStatus.active_project) {
        googleCloudProject = gcloudStatus.active_project;
      }
    } catch (e) {
      console.warn("Failed to detect gcloud status:", e);
    } finally {
      isCheckingGCloud = false;
    }
  }

  async function handleInstallCli() {
    isInstalling = true;
    installResult = null;
    try {
      const res = await invoke<CliInstallResult>("install_gemini_cli");
      installResult = res;
      // Refresh global envStatus
      const updatedStatus = await invoke<GeminiEnvStatus>("check_gemini_env");
      if (onEnvStatusUpdated) {
        onEnvStatusUpdated(updatedStatus);
      }
    } catch (e: any) {
      installResult = {
        success: false,
        message: e?.toString() || "Installation failed.",
      };
    } finally {
      isInstalling = false;
    }
  }

  async function handleSaveAuth() {
    isSaving = true;
    saveMessage = null;
    try {
      const config: GeminiAuthConfig = {
        auth_mode: authMode,
        api_key: apiKey.trim() || undefined,
        google_cloud_project: googleCloudProject.trim() || undefined,
        google_cloud_location: googleCloudLocation.trim() || undefined,
        google_app_credentials: googleAppCredentials.trim() || undefined,
      };

      const wsPath = applyToWorkspace && activeWorkspace ? activeWorkspace.path : undefined;
      await invoke("save_gemini_auth_config", {
        config,
        applyToWorkspace: wsPath,
      });

      saveMessage = {
        type: "success",
        text: "Configuration saved successfully. Credentials synchronized with Gemini CLI.",
      };
    } catch (e: any) {
      saveMessage = {
        type: "error",
        text: `Failed to save configuration: ${e?.toString()}`,
      };
    } finally {
      isSaving = false;
    }
  }

  async function handleRunTest() {
    isTesting = true;
    testResult = null;
    try {
      const wsPath = activeWorkspace?.path;
      if (activeEngine === "agy") {
        const res = await invoke<CliTestResult>("test_agy_cli", {
          workspacePath: wsPath,
        });
        testResult = res;
      } else {
        const config: GeminiAuthConfig = {
          auth_mode: authMode,
          api_key: apiKey.trim() || undefined,
          google_cloud_project: googleCloudProject.trim() || undefined,
          google_cloud_location: googleCloudLocation.trim() || undefined,
          google_app_credentials: googleAppCredentials.trim() || undefined,
        };

        const res = await invoke<CliTestResult>("test_gemini_cli", {
          config,
          workspacePath: wsPath,
          model: testModel,
        });
        testResult = res;
      }
    } catch (e: any) {
      testResult = {
        success: false,
        latency_ms: 0,
        message: e?.toString() || "Diagnostic test failed to execute.",
      };
    } finally {
      isTesting = false;
    }
  }

  function handleCopyPath(path: string) {
    if (navigator.clipboard) {
      navigator.clipboard.writeText(path);
      hasCopiedPath = true;
      setTimeout(() => (hasCopiedPath = false), 2000);
    }
  }

  function handleCopyTestError() {
    if (!testResult) return;
    const fullError = testResult.details
      ? `${testResult.message}\n\n${testResult.details}`
      : testResult.message;
    if (navigator.clipboard) {
      navigator.clipboard.writeText(fullError);
      hasCopiedError = true;
      setTimeout(() => (hasCopiedError = false), 2000);
    }
  }

  async function handleImportGeminiPlugins() {
    isImportingPlugins = true;
    pluginImportResult = null;
    try {
      const res = await invoke<{ success: boolean; message: string; output: string }>("import_gemini_plugins_via_agy");
      pluginImportResult = res;
    } catch (e: any) {
      pluginImportResult = {
        success: false,
        message: `Import failed: ${e?.toString()}`,
        output: "",
      };
    } finally {
      isImportingPlugins = false;
    }
  }
</script>

{#if isOpen}
  <!-- Modal Backdrop -->
  <div
    class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-xs animate-in fade-in duration-200"
    onclick={onClose}
    onkeydown={(e) => {
      if (e.key === "Escape") onClose();
    }}
    role="presentation"
  >
    <!-- Modal Dialog Card -->
    <div
      class="bg-surface-elevated border border-theme-default rounded-xl shadow-2xl w-full max-w-2xl max-h-[90vh] flex flex-col overflow-hidden animate-in zoom-in-95 duration-200"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
      role="dialog"
      aria-modal="true"
      aria-labelledby="cli-setup-title"
      tabindex="-1"
    >
      <!-- Header -->
      <div class="flex items-center justify-between px-5 py-4 border-b border-subtle bg-surface/50">
        <div class="flex items-center gap-3">
          <div
            class="w-9 h-9 rounded-lg flex items-center justify-center text-white shadow-md shrink-0"
            style="background: var(--accent-gradient);"
          >
            <Cpu size={20} />
          </div>
          <div>
            <h2 id="cli-setup-title" class="text-sm font-semibold text-primary-theme flex items-center gap-2">
              Gemini CLI & Authentication Setup
            </h2>
            <p class="text-xs text-muted-theme mt-0.5">
              Install the latest Gemini CLI and configure Google AI Studio or Corporate Vertex AI mode.
            </p>
          </div>
        </div>
        <button
          type="button"
          onclick={onClose}
          class="p-1 rounded-lg text-secondary-theme hover:text-primary-theme hover:bg-surface-hover transition-colors cursor-pointer"
          aria-label="Close dialog"
        >
          <X size={18} />
        </button>
      </div>

      <!-- Navigation Tabs -->
      <div class="flex items-center border-b border-subtle bg-surface px-5 gap-2 pt-2">
        <button
          type="button"
          onclick={() => (activeTab = "cli")}
          class="px-3 py-2 text-xs font-medium border-b-2 transition-colors flex items-center gap-2 cursor-pointer {activeTab === 'cli' ? 'border-accent-theme text-accent-theme font-semibold' : 'border-transparent text-secondary-theme hover:text-primary-theme'}"
        >
          <Terminal size={14} />
          <span>1. CLI Engine</span>
          {#if envStatus?.installed}
            <span class="inline-block w-1.5 h-1.5 rounded-full bg-emerald-400"></span>
          {:else}
            <span class="inline-block w-1.5 h-1.5 rounded-full bg-amber-400"></span>
          {/if}
        </button>

        <button
          type="button"
          onclick={() => (activeTab = "auth")}
          class="px-3 py-2 text-xs font-medium border-b-2 transition-colors flex items-center gap-2 cursor-pointer {activeTab === 'auth' ? 'border-accent-theme text-accent-theme font-semibold' : 'border-transparent text-secondary-theme hover:text-primary-theme'}"
        >
          <Key size={14} />
          <span>2. Authentication</span>
        </button>

        <button
          type="button"
          onclick={() => (activeTab = "diagnostics")}
          class="px-3 py-2 text-xs font-medium border-b-2 transition-colors flex items-center gap-2 cursor-pointer {activeTab === 'diagnostics' ? 'border-accent-theme text-accent-theme font-semibold' : 'border-transparent text-secondary-theme hover:text-primary-theme'}"
        >
          <ShieldCheck size={14} />
          <span>3. Connection Test</span>
          {#if testResult?.success}
            <span class="inline-block w-1.5 h-1.5 rounded-full bg-emerald-400"></span>
          {/if}
        </button>
      </div>

      <!-- Tab Content Area -->
      <div class="p-5 flex-1 overflow-y-auto space-y-4">
        <!-- ================= TAB 1: CLI ENGINE ================= -->
        {#if activeTab === "cli"}
          <div class="space-y-4">
            <!-- Engine Toggle Bar -->
            <div class="p-3.5 rounded-lg border border-subtle bg-surface flex items-center justify-between">
              <div>
                <span class="text-xs font-semibold text-primary-theme">Active Agent Engine</span>
                <p class="text-[11px] text-muted-theme mt-0.5">Switch between Antigravity CLI and Gemini CLI.</p>
              </div>
              <div class="flex items-center gap-1.5 p-1 bg-surface-elevated rounded-lg border border-subtle">
                <button
                  type="button"
                  onclick={() => handleSwitchEngine("agy")}
                  class="px-2.5 py-1 text-xs rounded-md font-medium transition-all cursor-pointer {activeEngine === 'agy' ? 'bg-accent-theme text-white shadow-xs' : 'text-secondary-theme hover:text-primary-theme'}"
                >
                  🚀 Antigravity (agy)
                </button>
                <button
                  type="button"
                  onclick={() => handleSwitchEngine("gemini")}
                  class="px-2.5 py-1 text-xs rounded-md font-medium transition-all cursor-pointer {activeEngine === 'gemini' ? 'bg-accent-theme text-white shadow-xs' : 'text-secondary-theme hover:text-primary-theme'}"
                >
                  ♊ Gemini CLI
                </button>
              </div>
            </div>

            {#if activeEngine === "agy"}
              <!-- Antigravity Status Card -->
              <div class="p-4 rounded-lg border {enginesStatus?.agyInstalled ? 'border-emerald-500/30 bg-emerald-500/5' : 'border-amber-500/30 bg-amber-500/5'}">
                <div class="flex items-start justify-between">
                  <div class="flex items-center gap-2.5">
                    {#if enginesStatus?.agyInstalled}
                      <CheckCircle2 size={18} class="text-emerald-400 shrink-0" />
                      <div>
                        <div class="text-xs font-semibold text-primary-theme flex items-center gap-2">
                          <span>Antigravity CLI (agy) Detected</span>
                          <span class="px-1.5 py-0.5 rounded text-[10px] bg-emerald-500/20 text-emerald-400 font-mono font-medium">
                            Ready (ACP Mode)
                          </span>
                        </div>
                        <p class="text-[11px] text-muted-theme mt-0.5">
                          Antigravity ACP server mode is ready for workspace execution.
                        </p>
                      </div>
                    {:else}
                      <AlertCircle size={18} class="text-amber-400 shrink-0" />
                      <div>
                        <div class="text-xs font-semibold text-primary-theme">
                          Antigravity CLI (agy) Not Found
                        </div>
                        <p class="text-[11px] text-muted-theme mt-0.5">
                          Ensure 'agy' is installed by your organization and in your system PATH.
                        </p>
                      </div>
                    {/if}
                  </div>
                </div>

                {#if enginesStatus?.agyPath}
                  <div class="mt-3 pt-3 border-t border-subtle/50 flex items-center justify-between text-xs font-mono text-muted-theme">
                    <span class="truncate mr-2">{enginesStatus.agyPath}</span>
                    <button
                      type="button"
                      onclick={() => handleCopyPath(enginesStatus?.agyPath || "")}
                      class="px-2 py-0.5 rounded bg-surface hover:bg-surface-hover text-secondary-theme hover:text-primary-theme transition-colors cursor-pointer text-[10px] shrink-0"
                    >
                      {hasCopiedPath ? "Copied!" : "Copy Path"}
                    </button>
                  </div>
                {/if}
              </div>

              <!-- Antigravity Update / Installer Card -->
              <div class="p-4 rounded-lg border border-subtle bg-surface space-y-3">
                <div class="flex items-center justify-between">
                  <div>
                    <div class="text-xs font-semibold text-primary-theme flex items-center gap-2">
                      <Download size={14} class="text-accent-theme" />
                      <span>Update Antigravity CLI to Latest</span>
                    </div>
                    <p class="text-[11px] text-muted-theme mt-0.5">
                      Runs <code class="font-mono text-secondary-theme">agy update</code> or the official installer script to ensure your CLI has the latest agent capabilities.
                    </p>
                  </div>
                  <button
                    type="button"
                    onclick={handleUpdateOrInstallAgy}
                    disabled={isUpdatingAgy}
                    class="px-3 py-1.5 rounded-lg bg-accent-theme hover:bg-accent-hover text-white font-medium text-xs transition-colors cursor-pointer disabled:opacity-50 flex items-center gap-1.5 shrink-0"
                  >
                    {#if isUpdatingAgy}
                      <RefreshCw size={13} class="animate-spin" />
                      <span>Updating agy...</span>
                    {:else}
                      <Download size={13} />
                      <span>{enginesStatus?.agyInstalled ? "Update agy to Latest" : "Install Antigravity CLI"}</span>
                    {/if}
                  </button>
                </div>

                {#if agyInstallResult}
                  <div class="p-3 rounded-lg text-xs flex items-start gap-2 {agyInstallResult.success ? 'bg-emerald-500/10 border border-emerald-500/30 text-emerald-300' : 'bg-rose-500/10 border border-rose-500/30 text-rose-300'}">
                    {#if agyInstallResult.success}
                      <CheckCircle2 size={15} class="text-emerald-400 shrink-0 mt-0.5" />
                    {:else}
                      <AlertCircle size={15} class="text-rose-400 shrink-0 mt-0.5" />
                    {/if}
                    <div>
                      <p class="font-medium">{agyInstallResult.message}</p>
                      {#if agyInstallResult.version}
                        <p class="text-[11px] opacity-80 mt-0.5 font-mono">Installed Version: {agyInstallResult.version}</p>
                      {/if}
                    </div>
                  </div>
                {/if}
              </div>

              <!-- Gemini CLI Plugin & Skills Importer for Antigravity -->
              {#if enginesStatus?.agyInstalled}
                <div class="p-4 rounded-lg border border-subtle bg-surface space-y-3">
                  <div class="flex items-center justify-between">
                    <div>
                      <div class="text-xs font-semibold text-primary-theme flex items-center gap-2">
                        <Sparkles size={14} class="text-purple-400" />
                        <span>Import Gemini CLI Extensions & Skills</span>
                      </div>
                      <p class="text-[11px] text-muted-theme mt-0.5">
                        Runs <code class="font-mono text-secondary-theme">agy plugin import gemini</code> to convert your existing Gemini CLI extensions, skills, and TOML commands into native Antigravity plugins.
                      </p>
                    </div>
                    <button
                      type="button"
                      onclick={handleImportGeminiPlugins}
                      disabled={isImportingPlugins}
                      class="px-3 py-1.5 rounded-lg bg-purple-600 hover:bg-purple-500 text-white font-medium text-xs transition-colors cursor-pointer disabled:opacity-50 flex items-center gap-1.5 shrink-0"
                    >
                      {#if isImportingPlugins}
                        <RefreshCw size={13} class="animate-spin" />
                        <span>Importing...</span>
                      {:else}
                        <Sparkles size={13} />
                        <span>Run Plugin Import</span>
                      {/if}
                    </button>
                  </div>

                  {#if pluginImportResult}
                    <div class="p-3 rounded-lg text-xs space-y-1.5 {pluginImportResult.success ? 'bg-purple-500/10 border border-purple-500/30 text-purple-200' : 'bg-rose-500/10 border border-rose-500/30 text-rose-300'}">
                      <div class="font-semibold flex items-center gap-1.5">
                        {#if pluginImportResult.success}
                          <CheckCircle2 size={14} class="text-purple-400 shrink-0" />
                        {:else}
                          <AlertCircle size={14} class="text-rose-400 shrink-0" />
                        {/if}
                        <span>{pluginImportResult.message}</span>
                      </div>
                      {#if pluginImportResult.output}
                        <pre class="font-mono text-[11px] bg-black/30 p-2 rounded max-h-32 overflow-y-auto whitespace-pre-wrap">{pluginImportResult.output}</pre>
                      {/if}
                    </div>
                  {/if}
                </div>
              {/if}

              <!-- Antigravity Info Card -->
              <div class="p-4 rounded-lg border border-subtle bg-surface space-y-2 text-xs">
                <div class="font-semibold text-primary-theme flex items-center gap-2">
                  <Building2 size={15} class="text-accent-theme" />
                  Enterprise Antigravity Workspace Integration
                </div>
                <p class="text-secondary-theme leading-relaxed">
                  Your organization manages Antigravity credentials and agent policies. All ACP sessions leverage local configuration in <code class="px-1 py-0.5 rounded bg-surface-elevated font-mono text-[11px]">~/.gemini/antigravity/</code> and your active Google account.
                </p>
                <div class="pt-2">
                  <button
                    type="button"
                    onclick={() => (activeTab = "diagnostics")}
                    class="px-3 py-1.5 rounded-lg bg-accent-theme hover:bg-accent-hover text-white font-medium text-xs transition-colors cursor-pointer flex items-center gap-1.5"
                  >
                    <ShieldCheck size={14} />
                    <span>Run Antigravity Connection Test</span>
                  </button>
                </div>
              </div>
            {:else}
            <!-- CLI Status Card -->
            <div class="p-4 rounded-lg border {envStatus?.installed ? 'border-emerald-500/30 bg-emerald-500/5' : 'border-amber-500/30 bg-amber-500/5'}">
              <div class="flex items-start justify-between">
                <div class="flex items-center gap-2.5">
                  {#if envStatus?.installed}
                    <CheckCircle2 size={18} class="text-emerald-400 shrink-0" />
                    <div>
                      <div class="text-xs font-semibold text-primary-theme flex items-center gap-2">
                        <span>Gemini CLI Installed</span>
                        <span class="px-1.5 py-0.5 rounded text-[10px] bg-emerald-500/20 text-emerald-400 font-mono font-medium">
                          v{envStatus?.version || "Ready"}
                        </span>
                      </div>
                      <p class="text-[11px] text-muted-theme mt-0.5">
                        Native Agent Client Protocol (ACP) ready for execution.
                      </p>
                    </div>
                  {:else}
                    <AlertCircle size={18} class="text-amber-400 shrink-0" />
                    <div>
                      <div class="text-xs font-semibold text-primary-theme">
                        Gemini CLI Not Detected
                      </div>
                      <p class="text-[11px] text-muted-theme mt-0.5">
                        Install the official @google/gemini-cli package to enable ACP agent streaming and tool execution.
                      </p>
                    </div>
                  {/if}
                </div>

                <button
                  type="button"
                  onclick={handleInstallCli}
                  disabled={isInstalling}
                  class="px-3 py-1.5 rounded-lg text-xs font-medium bg-accent-theme text-white hover:opacity-90 disabled:opacity-50 transition-all flex items-center gap-1.5 cursor-pointer shadow-xs shrink-0"
                >
                  <RefreshCw size={13} class={isInstalling ? "animate-spin" : ""} />
                  <span>{isInstalling ? "Installing..." : envStatus?.installed ? "Update to Latest" : "Install Gemini CLI"}</span>
                </button>
              </div>

              {#if envStatus?.path}
                <div class="mt-3 pt-3 border-t border-subtle flex items-center justify-between text-[11px] text-muted-theme font-mono">
                  <span class="truncate pr-2">{envStatus.path}</span>
                  <button
                    type="button"
                    onclick={() => envStatus?.path && handleCopyPath(envStatus.path)}
                    class="p-1 hover:text-accent-theme text-secondary-theme rounded transition-colors shrink-0 cursor-pointer"
                    title="Copy path"
                  >
                    {#if hasCopiedPath}
                      <Check size={12} class="text-emerald-400" />
                    {:else}
                      <Copy size={12} />
                    {/if}
                  </button>
                </div>
              {/if}
            </div>

            <!-- Installation Result Notification -->
            {#if installResult}
              <div class="p-3 rounded-lg text-xs border {installResult.success ? 'border-emerald-500/30 bg-emerald-500/10 text-emerald-300' : 'border-rose-500/30 bg-rose-500/10 text-rose-300'} flex items-start gap-2">
                {#if installResult.success}
                  <CheckCircle2 size={16} class="shrink-0 text-emerald-400 mt-0.5" />
                {:else}
                  <XCircle size={16} class="shrink-0 text-rose-400 mt-0.5" />
                {/if}
                <div class="flex-1">
                  <div class="font-medium">{installResult.message}</div>
                  {#if installResult.version}
                    <div class="text-[11px] mt-1 font-mono text-muted-theme">
                      Detected version: {installResult.version}
                    </div>
                  {/if}
                </div>
              </div>
            {/if}

            <!-- Distribution Info -->
            <div class="p-3.5 rounded-lg bg-surface border border-subtle space-y-2">
              <h3 class="text-xs font-semibold text-primary-theme flex items-center gap-1.5">
                <Download size={13} class="text-accent-theme" />
                Supported Package Managers
              </h3>
              <p class="text-[11px] text-muted-theme leading-relaxed">
                Gemini Desktop's 1-click installer automatically targets the global Node Package Manager (<code class="font-mono text-secondary-theme">npm install -g @google/gemini-cli@latest</code>) or Windows Package Manager (<code class="font-mono text-secondary-theme">winget install Google.GeminiCLI</code>).
              </p>
            </div>
            {/if}

            <!-- Global PATH Configuration Card -->
            <div class="p-4 rounded-lg border border-subtle bg-surface space-y-3">
              <div class="flex items-start justify-between">
                <div>
                  <div class="text-xs font-semibold text-primary-theme flex items-center gap-2">
                    <Globe size={14} class="text-accent-theme" />
                    <span>User Environment Variables (Global PATH)</span>
                  </div>
                  <p class="text-[11px] text-muted-theme mt-0.5">
                    Add CLI binary directories to your persistent Windows User PATH so you can run them directly in any PowerShell, Command Prompt, or terminal.
                  </p>
                </div>
                <button
                  type="button"
                  onclick={loadPathStatus}
                  disabled={isCheckingPath}
                  class="p-1 rounded text-secondary-theme hover:text-primary-theme hover:bg-surface-elevated transition-colors cursor-pointer text-xs"
                  title="Refresh PATH Status"
                >
                  <RefreshCw size={13} class={isCheckingPath ? 'animate-spin' : ''} />
                </button>
              </div>

              {#if pathAddResult}
                <div class="p-2.5 rounded-lg text-xs flex items-start gap-2 {pathAddResult.success ? 'bg-emerald-500/10 border border-emerald-500/30 text-emerald-300' : 'bg-rose-500/10 border border-rose-500/30 text-rose-300'}">
                  {#if pathAddResult.success}
                    <CheckCircle2 size={14} class="text-emerald-400 shrink-0 mt-0.5" />
                  {:else}
                    <AlertCircle size={14} class="text-rose-400 shrink-0 mt-0.5" />
                  {/if}
                  <span class="leading-relaxed">{pathAddResult.message}</span>
                </div>
              {/if}

              <div class="space-y-2 pt-1">
                <!-- Antigravity PATH Row -->
                <div class="flex items-center justify-between p-2.5 rounded-lg bg-surface-elevated border border-subtle text-xs">
                  <div class="min-w-0 mr-3">
                    <div class="flex items-center gap-2">
                      <span class="font-medium text-primary-theme">Antigravity CLI (agy)</span>
                      {#if cliPathStatus?.agyInUserPath}
                        <span class="px-1.5 py-0.5 rounded text-[10px] bg-emerald-500/20 text-emerald-400 font-mono font-medium flex items-center gap-1">
                          <Check size={10} /> In User PATH
                        </span>
                      {:else}
                        <span class="px-1.5 py-0.5 rounded text-[10px] bg-amber-500/20 text-amber-400 font-mono font-medium">
                          Not in User PATH
                        </span>
                      {/if}
                    </div>
                    <p class="text-[11px] font-mono text-muted-theme truncate mt-0.5">
                      {cliPathStatus?.agyDir || "Directory not located"}
                    </p>
                  </div>
                  {#if !cliPathStatus?.agyInUserPath}
                    <button
                      type="button"
                      onclick={() => handleAddToPath("agy")}
                      disabled={isAddingToPath}
                      class="px-2.5 py-1 rounded bg-accent-theme hover:bg-accent-hover text-white text-[11px] font-medium transition-colors cursor-pointer shrink-0 disabled:opacity-50"
                    >
                      {isAddingToPath ? "Adding..." : "Add to User PATH"}
                    </button>
                  {/if}
                </div>

                <!-- Gemini CLI PATH Row -->
                <div class="flex items-center justify-between p-2.5 rounded-lg bg-surface-elevated border border-subtle text-xs">
                  <div class="min-w-0 mr-3">
                    <div class="flex items-center gap-2">
                      <span class="font-medium text-primary-theme">Gemini CLI (gemini)</span>
                      {#if cliPathStatus?.geminiInUserPath}
                        <span class="px-1.5 py-0.5 rounded text-[10px] bg-emerald-500/20 text-emerald-400 font-mono font-medium flex items-center gap-1">
                          <Check size={10} /> In User PATH
                        </span>
                      {:else}
                        <span class="px-1.5 py-0.5 rounded text-[10px] bg-amber-500/20 text-amber-400 font-mono font-medium">
                          Not in User PATH
                        </span>
                      {/if}
                    </div>
                    <p class="text-[11px] font-mono text-muted-theme truncate mt-0.5">
                      {cliPathStatus?.geminiDir || "Directory not located"}
                    </p>
                  </div>
                  {#if !cliPathStatus?.geminiInUserPath}
                    <button
                      type="button"
                      onclick={() => handleAddToPath("gemini")}
                      disabled={isAddingToPath}
                      class="px-2.5 py-1 rounded bg-accent-theme hover:bg-accent-hover text-white text-[11px] font-medium transition-colors cursor-pointer shrink-0 disabled:opacity-50"
                    >
                      {isAddingToPath ? "Adding..." : "Add to User PATH"}
                    </button>
                  {/if}
                </div>
              </div>
            </div>
          </div>
        {/if}

        <!-- ================= TAB 2: AUTHENTICATION ================= -->
        {#if activeTab === "auth"}
          <div class="space-y-4">
            <!-- Mode Radio Selector -->
            <div>
              <span class="block text-xs font-semibold text-primary-theme mb-2">
                Select Authentication Mode
              </span>
              <div class="grid grid-cols-2 gap-3">
                <!-- Option A: Google Cloud Vertex AI (Default) -->
                <button
                  type="button"
                  onclick={() => (authMode = "vertex_ai")}
                  class="p-3 rounded-lg border text-left transition-all cursor-pointer {authMode === 'vertex_ai' ? 'border-accent-theme bg-accent-theme/10 ring-1 ring-accent-theme/30' : 'border-subtle bg-surface hover:bg-surface-hover'}"
                >
                  <div class="flex items-center justify-between">
                    <div class="flex items-center gap-2 text-xs font-semibold {authMode === 'vertex_ai' ? 'text-accent-theme' : 'text-primary-theme'}">
                      <Building2 size={14} />
                      <span>Google Cloud</span>
                    </div>
                    <span class="text-[10px] px-1.5 py-0.5 rounded bg-accent-theme/20 text-accent-theme font-medium">Default</span>
                  </div>
                  <p class="text-[11px] text-muted-theme mt-1 leading-relaxed">
                    Vertex AI Mode with GCP Project. Zero-dependency corporate OAuth, enterprise billing, and VPCs.
                  </p>
                </button>

                <!-- Option B: Google AI Studio (API Key) -->
                <button
                  type="button"
                  onclick={() => (authMode = "api_key")}
                  class="p-3 rounded-lg border text-left transition-all cursor-pointer {authMode === 'api_key' ? 'border-accent-theme bg-accent-theme/10 ring-1 ring-accent-theme/30' : 'border-subtle bg-surface hover:bg-surface-hover'}"
                >
                  <div class="flex items-center gap-2 text-xs font-semibold {authMode === 'api_key' ? 'text-accent-theme' : 'text-primary-theme'}">
                    <Key size={14} />
                    <span>Google AI Studio</span>
                  </div>
                  <p class="text-[11px] text-muted-theme mt-1 leading-relaxed">
                    Personal API Key. Best for individual developers, quick prototyping, and free tier.
                  </p>
                </button>
              </div>
            </div>

            <!-- Mode A Inputs: Vertex AI / Google Cloud (Default) -->
            {#if authMode === "vertex_ai"}
              <div class="space-y-3.5 p-4 rounded-lg bg-surface border border-subtle">
                <!-- GCP Project ID -->
                <div>
                  <div class="flex items-center justify-between mb-1.5">
                    <label for="gcp-project" class="text-xs font-medium text-primary-theme flex items-center gap-1.5">
                      <span>Google Cloud Project ID</span>
                      <span class="text-rose-400">*</span>
                    </label>
                    {#if gcloudStatus?.active_project && googleCloudProject !== gcloudStatus.active_project}
                      <button
                        type="button"
                        onclick={() => (googleCloudProject = gcloudStatus?.active_project || "")}
                        class="text-[11px] text-accent-theme hover:underline cursor-pointer"
                      >
                        Use detected project ({gcloudStatus.active_project})
                      </button>
                    {/if}
                  </div>
                  <input
                    id="gcp-project"
                    type="text"
                    bind:value={googleCloudProject}
                    placeholder="e.g. my-corp-ai-project-1234"
                    class="w-full px-3 py-2 rounded-lg bg-surface-elevated border border-theme-default text-xs text-primary-theme focus:outline-none focus:border-accent-theme font-mono"
                  />
                  <p class="text-[10px] text-muted-theme mt-1">
                    Environment variable: <code class="font-mono text-secondary-theme">GOOGLE_CLOUD_PROJECT</code>
                  </p>
                </div>

                <!-- Vertex Region / Location -->
                <div>
                  <label for="gcp-location" class="block text-xs font-medium text-primary-theme mb-1.5">
                    Vertex AI Location / Region
                  </label>
                  <select
                    id="gcp-location"
                    bind:value={googleCloudLocation}
                    class="w-full px-3 py-2 rounded-lg bg-surface-elevated border border-theme-default text-xs text-primary-theme focus:outline-none focus:border-accent-theme"
                  >
                    {#each VERTEX_LOCATIONS as loc}
                      <option value={loc.id}>{loc.label}</option>
                    {/each}
                  </select>
                  <p class="text-[10px] text-muted-theme mt-1">
                    Environment variable: <code class="font-mono text-secondary-theme">GOOGLE_CLOUD_LOCATION</code>
                  </p>
                </div>

                <!-- Google Account / OAuth & ADC Status Banner -->
                <div class="p-3 rounded-lg border border-subtle bg-surface-elevated flex items-start justify-between gap-3">
                  <div class="flex items-start gap-2.5">
                    <Cloud size={16} class="text-sky-400 shrink-0 mt-0.5" />
                    <div>
                      <div class="text-xs font-semibold text-primary-theme flex items-center gap-2">
                        <span>Google Account Authentication</span>
                        {#if gcloudStatus?.has_gemini_oauth}
                          <span class="px-1.5 py-0.2 rounded text-[10px] bg-emerald-500/20 text-emerald-400 font-medium">
                            OAuth Connected
                          </span>
                        {:else if gcloudStatus?.has_adc}
                          <span class="px-1.5 py-0.2 rounded text-[10px] bg-emerald-500/20 text-emerald-400 font-medium">
                            ADC Found
                          </span>
                        {:else}
                          <span class="px-1.5 py-0.2 rounded text-[10px] bg-amber-500/20 text-amber-400 font-medium">
                            Browser Login Ready
                          </span>
                        {/if}
                      </div>
                      <p class="text-[11px] text-muted-theme mt-1 leading-relaxed">
                        {#if gcloudStatus?.has_gemini_oauth}
                          Gemini CLI's native browser login is active for <strong class="text-secondary-theme font-medium">{gcloudStatus?.active_account || 'your account'}</strong> (credentials stored in <code class="font-mono text-[10px] text-secondary-theme">~/.gemini/</code>). The <code class="font-mono text-[10px]">gcloud</code> SDK is <strong>not required</strong>.
                        {:else if gcloudStatus?.has_adc}
                          Google Cloud Application Default Credentials detected for <strong class="text-secondary-theme font-medium">{gcloudStatus?.active_account || 'active account'}</strong>.
                        {:else}
                          Gemini CLI has built-in OAuth browser sign-in. Simply run <code class="font-mono text-[10px] text-secondary-theme">gemini</code> in a terminal to launch the sign-in URL, or supply a Service Account key below. The <code class="font-mono text-[10px]">gcloud</code> SDK is <strong>not required</strong>.
                        {/if}
                      </p>
                    </div>
                  </div>
                </div>

                <!-- Optional Service Account Key -->
                <div>
                  <label for="gcp-creds" class="block text-xs font-medium text-primary-theme mb-1.5">
                    Service Account Key File (Optional)
                  </label>
                  <input
                    id="gcp-creds"
                    type="text"
                    bind:value={googleAppCredentials}
                    placeholder="C:\keys\service-account-key.json"
                    class="w-full px-3 py-2 rounded-lg bg-surface-elevated border border-theme-default text-xs text-primary-theme focus:outline-none focus:border-accent-theme font-mono"
                  />
                  <p class="text-[10px] text-muted-theme mt-1">
                    Environment variable: <code class="font-mono text-secondary-theme">GOOGLE_APPLICATION_CREDENTIALS</code>
                  </p>
                </div>
              </div>
            {/if}

            <!-- Mode B Inputs: API Key -->
            {#if authMode === "api_key"}
              <div class="space-y-3 p-4 rounded-lg bg-surface border border-subtle">
                <div>
                  <div class="flex items-center justify-between mb-1.5">
                    <label for="gemini-api-key" class="text-xs font-medium text-primary-theme flex items-center gap-1.5">
                      <span>Gemini API Key</span>
                      <span class="text-rose-400">*</span>
                    </label>
                    <a
                      href="https://aistudio.google.com/app/apikey"
                      target="_blank"
                      rel="noopener noreferrer"
                      class="text-[11px] text-accent-theme hover:underline flex items-center gap-1"
                    >
                      <span>Get Free API Key</span>
                      <ExternalLink size={11} />
                    </a>
                  </div>
                  <div class="relative">
                    <input
                      id="gemini-api-key"
                      type={showApiKey ? "text" : "password"}
                      bind:value={apiKey}
                      placeholder="AIzaSy..."
                      class="w-full px-3 py-2 pr-10 rounded-lg bg-surface-elevated border border-theme-default text-xs text-primary-theme focus:outline-none focus:border-accent-theme font-mono"
                    />
                    <button
                      type="button"
                      onclick={() => (showApiKey = !showApiKey)}
                      class="absolute right-2.5 top-1/2 -translate-y-1/2 text-muted-theme hover:text-primary-theme p-1 rounded transition-colors cursor-pointer"
                      title={showApiKey ? "Hide key" : "Show key"}
                    >
                      {#if showApiKey}
                        <EyeOff size={14} />
                      {:else}
                        <Eye size={14} />
                      {/if}
                    </button>
                  </div>
                  <p class="text-[10px] text-muted-theme mt-1">
                    Environment variable: <code class="font-mono text-secondary-theme">GEMINI_API_KEY</code>
                  </p>
                </div>
              </div>
            {/if}

            <!-- Scope Selection (Workspace .env.local) -->
            {#if activeWorkspace}
              <div class="flex items-center gap-2 px-1">
                <input
                  id="apply-workspace-env"
                  type="checkbox"
                  bind:checked={applyToWorkspace}
                  class="w-4 h-4 rounded border-subtle text-accent-theme focus:ring-accent-theme cursor-pointer"
                />
                <label for="apply-workspace-env" class="text-xs text-secondary-theme cursor-pointer flex items-center gap-1.5">
                  <FolderKanban size={13} class="text-accent-theme" />
                  <span>Also sync credentials into active workspace <code class="font-mono text-[11px] text-primary-theme">.env.local</code></span>
                </label>
              </div>
            {/if}

            <!-- Feedback Message -->
            {#if saveMessage}
              <div class="p-3 rounded-lg text-xs border {saveMessage.type === 'success' ? 'border-emerald-500/30 bg-emerald-500/10 text-emerald-300' : 'border-rose-500/30 bg-rose-500/10 text-rose-300'} flex items-center gap-2">
                {#if saveMessage.type === 'success'}
                  <CheckCircle2 size={15} class="text-emerald-400 shrink-0" />
                {:else}
                  <AlertCircle size={15} class="text-rose-400 shrink-0" />
                {/if}
                <span>{saveMessage.text}</span>
              </div>
            {/if}

            <!-- Save & Next Buttons -->
            <div class="flex items-center justify-end gap-2 pt-2 border-t border-subtle">
              <button
                type="button"
                onclick={() => (activeTab = "diagnostics")}
                class="px-3.5 py-2 rounded-lg text-xs font-medium text-secondary-theme hover:text-primary-theme hover:bg-surface transition-colors cursor-pointer"
              >
                Go to Connection Test →
              </button>
              <button
                type="button"
                onclick={handleSaveAuth}
                disabled={isSaving}
                class="px-4 py-2 rounded-lg text-xs font-medium bg-accent-theme text-white hover:opacity-90 disabled:opacity-50 transition-all flex items-center gap-1.5 cursor-pointer shadow-xs"
              >
                {#if isSaving}
                  <RefreshCw size={13} class="animate-spin" />
                  <span>Saving...</span>
                {:else}
                  <Check size={13} />
                  <span>Save Configuration</span>
                {/if}
              </button>
            </div>
          </div>
        {/if}

        <!-- ================= TAB 3: DIAGNOSTICS & VERIFICATION ================= -->
        {#if activeTab === "diagnostics"}
          <div class="space-y-4">
            <div class="p-4 rounded-lg bg-surface border border-subtle space-y-3">
              <div class="flex items-center justify-between">
                <div>
                  <h3 class="text-xs font-semibold text-primary-theme flex items-center gap-1.5">
                    <ShieldCheck size={15} class="text-accent-theme" />
                    Live Verification Suite
                  </h3>
                  <p class="text-[11px] text-muted-theme mt-0.5">
                    Executes a minimal test prompt to verify CLI executable, authentication credentials, and model API responses.
                  </p>
                </div>
                <button
                  type="button"
                  onclick={handleRunTest}
                  disabled={isTesting || (activeEngine === 'gemini' && !envStatus?.installed) || (activeEngine === 'agy' && !enginesStatus?.agyInstalled)}
                  class="px-4 py-2 rounded-lg text-xs font-medium bg-accent-theme text-white hover:opacity-90 disabled:opacity-50 transition-all flex items-center gap-1.5 cursor-pointer shadow-xs shrink-0"
                >
                  <RefreshCw size={13} class={isTesting ? "animate-spin" : ""} />
                  <span>{isTesting ? "Testing..." : `Run Test (${activeEngine === 'agy' ? 'agy' : 'gemini'})`}</span>
                </button>
              </div>

              {#if activeEngine === "gemini"}
                <!-- Test Model Selector for Gemini -->
                <div class="flex items-center gap-2 pt-2.5 border-t border-subtle text-xs">
                  <label for="test-model-select" class="text-secondary-theme font-medium shrink-0">
                    Verification Model:
                  </label>
                  <select
                    id="test-model-select"
                    bind:value={testModel}
                    class="flex-1 px-2.5 py-1.5 rounded-lg bg-surface-elevated border border-theme-default text-xs text-primary-theme focus:outline-none focus:border-accent-theme font-mono"
                  >
                    <option value="gemini-3.8-flash">gemini-3.8-flash (Latest Preview Flash — Google Recommended)</option>
                    <option value="auto">auto (Let Gemini CLI Auto-Route Dynamically)</option>
                    <option value="gemini-3.5-flash">gemini-3.5-flash (Fast & Stable 3.5)</option>
                    <option value="gemini-3.5-flash-lite">gemini-3.5-flash-lite (Ultra-fast & Lightweight 3.5)</option>
                    <option value="gemini-3.1-pro-preview">gemini-3.1-pro-preview (Gemini 3.1 Pro)</option>
                    <option value="gemini-2.5-pro">gemini-2.5-pro (Gemini 2.5 Pro)</option>
                  </select>
                </div>
              {:else}
                <div class="flex items-center gap-2 pt-2.5 border-t border-subtle text-xs text-secondary-theme font-mono">
                  <span>🚀 Engine: <strong>Antigravity CLI (agy)</strong> &bull; Verifies binary execution and ACP availability</span>
                </div>
              {/if}
            </div>

            <!-- Diagnostics Checklist Cards -->
            <div class="grid grid-cols-2 gap-3">
              <!-- Check 1: Binary -->
              <div class="p-3 rounded-lg border border-subtle bg-surface-elevated">
                <div class="flex items-center justify-between">
                  <span class="text-xs font-medium text-secondary-theme">1. CLI Binary ({activeEngine})</span>
                  {#if (activeEngine === 'agy' ? enginesStatus?.agyInstalled : envStatus?.installed)}
                    <CheckCircle2 size={15} class="text-emerald-400" />
                  {:else}
                    <XCircle size={15} class="text-rose-400" />
                  {/if}
                </div>
                <div class="text-[11px] text-muted-theme mt-1 truncate">
                  {#if activeEngine === "agy"}
                    {enginesStatus?.agyInstalled ? 'Found (agy ready)' : 'Missing in PATH'}
                  {:else}
                    {envStatus?.installed ? `Found (v${envStatus.version || 'installed'})` : 'Missing in PATH'}
                  {/if}
                </div>
              </div>

              <!-- Check 2: Auth Mode -->
              <div class="p-3 rounded-lg border border-subtle bg-surface-elevated">
                <div class="flex items-center justify-between">
                  <span class="text-xs font-medium text-secondary-theme">2. Auth Protocol</span>
                  {#if (authMode === 'api_key' && apiKey) || (authMode === 'vertex_ai' && googleCloudProject)}
                    <CheckCircle2 size={15} class="text-emerald-400" />
                  {:else}
                    <AlertCircle size={15} class="text-amber-400" />
                  {/if}
                </div>
                <div class="text-[11px] text-muted-theme mt-1 truncate">
                  {authMode === 'vertex_ai' ? `Vertex AI (${googleCloudProject || 'Unset'})` : `API Key (${apiKey ? 'Configured' : 'Empty'})`}
                </div>
              </div>

              <!-- Check 3: Latency & Handshake -->
              <div class="p-3 rounded-lg border border-subtle bg-surface-elevated">
                <div class="flex items-center justify-between">
                  <span class="text-xs font-medium text-secondary-theme">3. Latency & Ping</span>
                  {#if testResult?.success}
                    <CheckCircle2 size={15} class="text-emerald-400" />
                  {:else if testResult}
                    <XCircle size={15} class="text-rose-400" />
                  {:else}
                    <span class="text-[10px] text-muted-theme font-mono">Pending</span>
                  {/if}
                </div>
                <div class="text-[11px] text-muted-theme mt-1">
                  {testResult?.latency_ms ? `${testResult.latency_ms} ms` : 'Not tested'}
                </div>
              </div>

              <!-- Check 4: Model Execution -->
              <div class="p-3 rounded-lg border border-subtle bg-surface-elevated">
                <div class="flex items-center justify-between">
                  <span class="text-xs font-medium text-secondary-theme">4. Model Verification</span>
                  {#if testResult?.success}
                    <CheckCircle2 size={15} class="text-emerald-400" />
                  {:else if testResult}
                    <XCircle size={15} class="text-rose-400" />
                  {:else}
                    <span class="text-[10px] text-muted-theme font-mono">Pending</span>
                  {/if}
                </div>
                <div class="text-[11px] text-muted-theme mt-1 truncate">
                  {testResult?.success ? `${testModel} (Operational)` : testResult ? 'Check failed' : `${testModel} (Ready)`}
                </div>
              </div>
            </div>

            <!-- Detailed Test Output Box -->
            {#if testResult}
              <div class="p-3.5 rounded-lg border text-xs {testResult.success ? 'border-emerald-500/30 bg-emerald-500/10' : 'border-rose-500/30 bg-rose-500/10'}">
                <div class="flex items-start gap-2.5">
                  {#if testResult.success}
                    <CheckCircle2 size={16} class="text-emerald-400 shrink-0 mt-0.5" />
                  {:else}
                    <XCircle size={16} class="text-rose-400 shrink-0 mt-0.5" />
                  {/if}
                  <div class="flex-1 space-y-2 min-w-0">
                    <div class="flex items-center justify-between gap-2">
                      <div class="font-semibold {testResult.success ? 'text-emerald-300' : 'text-rose-300'} truncate">
                        {testResult.message}
                      </div>
                      <button
                        type="button"
                        onclick={handleCopyTestError}
                        class="px-2 py-1 rounded text-[11px] font-medium border border-subtle bg-surface/80 hover:bg-surface text-secondary-theme hover:text-primary-theme transition-colors flex items-center gap-1.5 cursor-pointer shrink-0 shadow-xs"
                        title={testResult.success ? "Copy test result" : "Copy error message and details"}
                      >
                        {#if hasCopiedError}
                          <Check size={12} class="text-emerald-400" />
                          <span class="text-emerald-400 font-medium">Copied!</span>
                        {:else}
                          <Copy size={12} />
                          <span>{testResult.success ? "Copy Output" : "Copy Error"}</span>
                        {/if}
                      </button>
                    </div>
                    {#if testResult.details}
                      <pre class="text-[11px] font-mono p-2.5 rounded bg-black/40 border border-subtle/50 overflow-x-auto text-primary-theme whitespace-pre-wrap max-h-36 leading-relaxed select-text">{testResult.details}</pre>
                    {/if}
                  </div>
                </div>
              </div>
            {/if}
          </div>
        {/if}
      </div>

      <!-- Footer -->
      <div class="flex items-center justify-between px-5 py-3 border-t border-subtle bg-surface/50 text-xs">
        <span class="text-[11px] text-muted-theme">
          All settings are saved locally in SQLite and encrypted credentials files.
        </span>
        <button
          type="button"
          onclick={onClose}
          class="px-4 py-1.5 rounded-lg bg-surface hover:bg-surface-hover border border-subtle text-primary-theme transition-colors cursor-pointer"
        >
          Close
        </button>
      </div>
    </div>
  </div>
{/if}
