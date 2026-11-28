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
    UserEnvVarStatus,
    UserEnvSyncResult,
  } from "$lib/types";
  import EngineSwitcher from "$lib/components/EngineSwitcher.svelte";
  import CliPathStatusBanner from "$lib/components/CliPathStatusBanner.svelte";
  import { engineStore } from "$lib/stores/engineStore.svelte";
  import {
    type AuthDraftSnapshot,
    normalizeAuthDraft,
    isAuthDraftDirty,
    buildUserEnvComparison,
    evaluateConnectionDiagnostics,
  } from "$lib/utils/cliSetup";
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
    FolderOpen,
    Trash2,
    LogIn,
    ArrowRight,
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
  let agyAdcAuth = $state(false);
  let applyToWorkspace = $state(false);
  let syncToUserEnvOnSave = $state(true);
  let includeUseVertexFlag = $state(false);

  let baselineDraft = $state<AuthDraftSnapshot>(normalizeAuthDraft({}));

  // Installer State
  let isInstalling = $state(false);
  let installResult = $state<CliInstallResult | null>(null);

  // GCloud & Windows User Env state
  let gcloudStatus = $state<GCloudStatus | null>(null);
  let isCheckingGCloud = $state(false);
  let userEnvVars = $state<UserEnvVarStatus[]>([]);
  let isCheckingUserEnv = $state(false);
  let isSyncingUserEnv = $state(false);
  let userEnvFeedback = $state<{ type: "success" | "error"; text: string } | null>(null);

  // Diagnostic Test State
  let isTesting = $state(false);
  let testResult = $state<CliTestResult | null>(null);
  let testModel = $state("gemini-3.8-flash");

  // General feedback & guard state
  let saveMessage = $state<{ type: "success" | "error"; text: string } | null>(null);
  let isSaving = $state(false);
  let hasCopiedPath = $state(false);
  let hasCopiedError = $state(false);
  let backdropPointerDown = $state(false);
  let showDiscardConfirm = $state(false);

  const VERTEX_LOCATIONS = [
    { id: "us-central1", label: "us-central1 (Iowa — Recommended)" },
    { id: "global", label: "global (Multi-Region Vertex Routing)" },
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

  // Agy Updater & Importer State
  let isUpdatingAgy = $state(false);
  let agyInstallResult = $state<CliInstallResult | null>(null);
  let isImportingPlugins = $state(false);
  let pluginImportResult = $state<{ success: boolean; message: string; output: string } | null>(null);
  let isLaunchingLogin = $state(false);
  let loginResult = $state<CliInstallResult | null>(null);
  let verifyAgyLiveAuth = $state(true);

  let effectiveEnvStatus = $derived(engineStore.envStatus ?? envStatus);
  let isActiveInstalled = $derived(
    activeEngine === "agy"
      ? Boolean(enginesStatus?.agyInstalled || effectiveEnvStatus?.installed)
      : Boolean(effectiveEnvStatus?.installed),
  );
  let activeBinaryPath = $derived(
    activeEngine === "agy"
      ? (enginesStatus?.agyPath || effectiveEnvStatus?.path || null)
      : (effectiveEnvStatus?.path || enginesStatus?.geminiPath || null),
  );
  let activeVersionLabel = $derived.by(() => {
    const ver = effectiveEnvStatus?.version?.trim();
    if (!ver || ver === "installed") {
      return activeEngine === "agy" ? "ACP Ready" : "Ready";
    }
    return ver.startsWith("v") ? ver : `v${ver}`;
  });

  let currentDraft = $derived<AuthDraftSnapshot>({
    authMode,
    apiKey,
    googleCloudProject,
    googleCloudLocation,
    googleAppCredentials,
    agyAdcAuth,
  });

  let isDirty = $derived(isAuthDraftDirty(currentDraft, baselineDraft));

  let userEnvComparison = $derived(
    buildUserEnvComparison(
      {
        activeEngine,
        authMode,
        googleCloudProject,
        googleCloudLocation,
        agyAdcAuth,
        includeUseVertexFlag,
      },
      userEnvVars,
    ),
  );

  let diagnostics = $derived(
    evaluateConnectionDiagnostics({
      activeEngine,
      effectiveEnvStatus,
      enginesStatus,
      authMode,
      apiKey,
      googleCloudProject,
      googleAppCredentials,
      agyAdcAuth,
      gcloudStatus,
      testModel,
      testResult,
    }),
  );

  $effect(() => {
    if (isOpen) {
      initializeModal();
    }
  });

  async function initializeModal() {
    showDiscardConfirm = false;
    saveMessage = null;
    userEnvFeedback = null;
    loginResult = null;
    await loadSavedConfig();
    await Promise.all([
      loadEngineStatus(),
      loadUserEnvVarsStatus(),
      loadGCloudStatus(),
    ]);
  }

  async function loadEngineStatus() {
    try {
      await engineStore.refresh();
      enginesStatus = engineStore.enginesStatus;
      activeEngine = engineStore.activeEngine;
      if (engineStore.envStatus && onEnvStatusUpdated) {
        onEnvStatusUpdated(engineStore.envStatus);
      }
    } catch (e) {
      console.warn("Failed to load CLI engines status:", e);
    }
  }

  async function loadUserEnvVarsStatus() {
    isCheckingUserEnv = true;
    try {
      userEnvVars = await invoke<UserEnvVarStatus[]>("get_user_env_vars_status");
    } catch (e) {
      console.warn("Failed to read Windows User Env status:", e);
    } finally {
      isCheckingUserEnv = false;
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
      agyAdcAuth = Boolean(config.agy_adc_auth);

      baselineDraft = normalizeAuthDraft({
        authMode,
        apiKey,
        googleCloudProject,
        googleCloudLocation,
        googleAppCredentials,
        agyAdcAuth,
      });
    } catch (e) {
      console.warn("Failed to load auth configuration:", e);
    }
  }

  async function loadGCloudStatus() {
    isCheckingGCloud = true;
    try {
      gcloudStatus = await invoke<GCloudStatus>("detect_gcloud_status");
    } catch (e) {
      console.warn("Failed to detect gcloud status:", e);
    } finally {
      isCheckingGCloud = false;
    }
  }

  async function handleEngineSwitched(eng: CliEngine) {
    activeEngine = eng;
    enginesStatus = engineStore.enginesStatus;
    if (engineStore.envStatus && onEnvStatusUpdated) {
      onEnvStatusUpdated(engineStore.envStatus);
    }
    testResult = null;
    loginResult = null;
    saveMessage = null;
    installResult = null;
    agyInstallResult = null;
  }

  async function handleUpdateOrInstallAgy() {
    isUpdatingAgy = true;
    agyInstallResult = null;
    try {
      const res = await invoke<CliInstallResult>("update_or_install_agy_cli");
      agyInstallResult = res;
      await loadEngineStatus();
    } catch (e: any) {
      agyInstallResult = {
        success: false,
        message: e?.toString() || "Antigravity CLI update failed.",
      };
    } finally {
      isUpdatingAgy = false;
    }
  }

  async function handleInstallCli() {
    isInstalling = true;
    installResult = null;
    try {
      const res = await invoke<CliInstallResult>("install_gemini_cli");
      installResult = res;
      await loadEngineStatus();
    } catch (e: any) {
      installResult = {
        success: false,
        message: e?.toString() || "Installation failed.",
      };
    } finally {
      isInstalling = false;
    }
  }

  async function handleBrowseCredentials() {
    try {
      const picked = await invoke<string | null>("pick_credentials_file", {
        initialPath: googleAppCredentials.trim() || undefined,
      });
      if (picked) {
        googleAppCredentials = picked;
      }
    } catch (e: any) {
      saveMessage = {
        type: "error",
        text: `Could not open file picker: ${e?.toString()}`,
      };
    }
  }

  async function handleApplyUserEnv(clearAll = false) {
    isSyncingUserEnv = true;
    userEnvFeedback = null;
    try {
      const request = clearAll
        ? {
            googleCloudProject: undefined,
            googleCloudLocation: undefined,
            googleGenaiUseVertexai: false,
            agyAdcAuth: false,
            clearUnspecified: true,
          }
        : userEnvComparison.syncRequest;

      const res = await invoke<UserEnvSyncResult>("sync_user_env_vars", { request });
      userEnvVars = res.vars;
      userEnvFeedback = {
        type: res.success ? "success" : "error",
        text: res.message,
      };
    } catch (e: any) {
      userEnvFeedback = {
        type: "error",
        text: e?.toString() || "Failed to update Windows User Environment variables.",
      };
    } finally {
      isSyncingUserEnv = false;
    }
  }

  async function handleSaveAuth(goToDiagnostics = false) {
    isSaving = true;
    saveMessage = null;
    try {
      const config: GeminiAuthConfig = {
        auth_mode: authMode,
        api_key: apiKey.trim() || undefined,
        google_cloud_project: googleCloudProject.trim() || undefined,
        google_cloud_location: googleCloudLocation.trim() || undefined,
        google_app_credentials: googleAppCredentials.trim() || undefined,
        agy_adc_auth: agyAdcAuth,
      };

      const wsPath = applyToWorkspace && activeWorkspace ? activeWorkspace.path : undefined;
      await invoke("save_gemini_auth_config", {
        config,
        applyToWorkspace: wsPath,
      });

      let envSyncNote = "";
      if (
        syncToUserEnvOnSave &&
        (authMode === "vertex_ai" || activeEngine === "agy") &&
        (googleCloudProject.trim().length > 0 || (activeEngine === "agy" && agyAdcAuth))
      ) {
        const res = await invoke<UserEnvSyncResult>("sync_user_env_vars", {
          request: userEnvComparison.syncRequest,
        });
        userEnvVars = res.vars;
        envSyncNote = res.success
          ? " + synced Windows User Env."
          : ` (Windows User Env warning: ${res.message})`;
      }

      baselineDraft = normalizeAuthDraft(currentDraft);
      showDiscardConfirm = false;

      saveMessage = {
        type: "success",
        text: `Saved settings for Gemini Desktop${wsPath ? ", workspace .env.local" : ""}${envSyncNote}`,
      };

      if (goToDiagnostics) {
        activeTab = "diagnostics";
      }
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
      const draftConfig: GeminiAuthConfig = {
        auth_mode: authMode,
        api_key: apiKey.trim() || undefined,
        google_cloud_project: googleCloudProject.trim() || undefined,
        google_cloud_location: googleCloudLocation.trim() || undefined,
        google_app_credentials: googleAppCredentials.trim() || undefined,
        agy_adc_auth: agyAdcAuth,
      };

      if (activeEngine === "agy") {
        testResult = await invoke<CliTestResult>("test_agy_cli", {
          config: draftConfig,
          workspacePath: wsPath,
          verifyAuth: verifyAgyLiveAuth,
        });
      } else {
        testResult = await invoke<CliTestResult>("test_gemini_cli", {
          config: draftConfig,
          workspacePath: wsPath,
          model: testModel,
        });
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

  async function handleLaunchEngineLogin(targetEngine: CliEngine) {
    isLaunchingLogin = true;
    loginResult = null;
    try {
      const cmd = targetEngine === "agy" ? "launch_agy_login" : "launch_gemini_login";
      loginResult = await invoke<CliInstallResult>(cmd);
    } catch (e: any) {
      loginResult = {
        success: false,
        message: e?.toString() || "Failed to launch CLI login window.",
      };
    } finally {
      isLaunchingLogin = false;
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
      pluginImportResult = await invoke<{ success: boolean; message: string; output: string }>(
        "import_gemini_plugins_via_agy",
      );
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

  function handleBackdropMouseDown(e: MouseEvent) {
    backdropPointerDown = e.target === e.currentTarget;
  }

  function handleBackdropClick(e: MouseEvent) {
    if (!backdropPointerDown || e.target !== e.currentTarget) {
      backdropPointerDown = false;
      return;
    }
    backdropPointerDown = false;
    handleAttemptClose();
  }

  function handleAttemptClose() {
    if (isDirty) {
      showDiscardConfirm = true;
      return;
    }
    onClose();
  }

  function formatEnvBadge(state: string): { label: string; cls: string } {
    switch (state) {
      case "synced":
        return { label: "Synced", cls: "bg-emerald-500/15 text-emerald-300 border-emerald-500/30" };
      case "differs":
        return { label: "Out of sync", cls: "bg-amber-500/15 text-amber-300 border-amber-500/30" };
      case "unset_in_windows":
        return { label: "Not in User Env", cls: "bg-sky-500/15 text-sky-300 border-sky-500/30" };
      case "only_in_windows":
        return { label: "Set in Windows only", cls: "bg-purple-500/15 text-purple-300 border-purple-500/30" };
      default:
        return { label: "Not set", cls: "bg-surface text-muted-theme border-subtle" };
    }
  }
</script>

{#if isOpen}
  <!-- Modal Backdrop (Protected against accidental mid-drag or dirty dismissal) -->
  <div
    class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-xs animate-in fade-in duration-200"
    onmousedown={handleBackdropMouseDown}
    onclick={handleBackdropClick}
    onkeydown={(e) => {
      if (e.key === "Escape") handleAttemptClose();
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
      <div class="flex items-center justify-between px-5 py-4 border-b border-subtle bg-surface/50 gap-3">
        <div class="flex items-center gap-3 min-w-0">
          <div
            class="w-9 h-9 rounded-lg flex items-center justify-center text-white shadow-md shrink-0"
            style="background: var(--accent-gradient);"
          >
            <Cpu size={20} />
          </div>
          <div class="min-w-0">
            <div class="flex items-center gap-2 flex-wrap">
              <h2 id="cli-setup-title" class="text-sm font-semibold text-primary-theme">
                {activeEngine === "agy"
                  ? "Antigravity CLI (agy) & Auth Setup"
                  : "Gemini CLI & Auth Setup"}
              </h2>
              <span class="px-2 py-0.5 rounded-full text-[10px] font-medium border border-subtle bg-surface text-secondary-theme shrink-0">
                {activeEngine === "agy" ? "🚀 Antigravity (agy)" : "♊ Gemini CLI"}
              </span>
              {#if isDirty}
                <span class="px-2 py-0.5 rounded-full text-[10px] font-semibold bg-amber-500/15 text-amber-300 border border-amber-500/30 shrink-0">
                  Unsaved changes
                </span>
              {/if}
            </div>
            <p class="text-xs text-muted-theme mt-0.5 truncate">
              {activeEngine === "agy"
                ? "Configure Antigravity runtime, User PATH, OAuth/ADC auth, and GCP User Environment vars."
                : "Install Gemini CLI, configure Vertex AI or API Key auth, and sync GCP User Environment vars."}
            </p>
          </div>
        </div>
        <button
          type="button"
          onclick={handleAttemptClose}
          class="p-1.5 rounded-lg text-secondary-theme hover:text-primary-theme hover:bg-surface-hover transition-colors cursor-pointer shrink-0"
          aria-label="Close dialog"
        >
          <X size={18} />
        </button>
      </div>

      <!-- Unsaved Discard Guard Banner -->
      {#if showDiscardConfirm}
        <div class="px-5 py-2.5 bg-amber-500/10 border-b border-amber-500/30 flex items-center justify-between gap-3 text-xs">
          <span class="text-amber-200 font-medium">
            You have unsaved authentication or Vertex changes.
          </span>
          <div class="flex items-center gap-2 shrink-0">
            <button
              type="button"
              onclick={() => (showDiscardConfirm = false)}
              class="px-2.5 py-1 rounded bg-surface border border-subtle text-secondary-theme hover:text-primary-theme cursor-pointer"
            >
              Keep Editing
            </button>
            <button
              type="button"
              onclick={() => {
                showDiscardConfirm = false;
                onClose();
              }}
              class="px-2.5 py-1 rounded bg-rose-500/20 border border-rose-500/40 text-rose-300 hover:bg-rose-500/30 cursor-pointer font-medium"
            >
              Discard &amp; Close
            </button>
          </div>
        </div>
      {/if}

      <!-- Navigation Tabs -->
      <div class="flex items-center border-b border-subtle bg-surface px-5 gap-2 pt-2">
        <button
          type="button"
          onclick={() => (activeTab = "cli")}
          class="px-3 py-2 text-xs font-medium border-b-2 transition-colors flex items-center gap-2 cursor-pointer {activeTab === 'cli' ? 'border-accent-theme text-accent-theme font-semibold' : 'border-transparent text-secondary-theme hover:text-primary-theme'}"
        >
          <Terminal size={14} />
          <span>1. CLI Engine</span>
          <span class="inline-block w-1.5 h-1.5 rounded-full {isActiveInstalled ? 'bg-emerald-400' : 'bg-amber-400'}"></span>
        </button>

        <button
          type="button"
          onclick={() => (activeTab = "auth")}
          class="px-3 py-2 text-xs font-medium border-b-2 transition-colors flex items-center gap-2 cursor-pointer {activeTab === 'auth' ? 'border-accent-theme text-accent-theme font-semibold' : 'border-transparent text-secondary-theme hover:text-primary-theme'}"
        >
          <Key size={14} />
          <span>2. Authentication</span>
          {#if diagnostics.auth.status === "ok"}
            <span class="inline-block w-1.5 h-1.5 rounded-full bg-emerald-400"></span>
          {/if}
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
            <EngineSwitcher variant="toggle-bar" onSwitch={handleEngineSwitched} />

            {#if activeEngine === "agy"}
              <!-- Unified Antigravity CLI Status + Install/Update Card -->
              <div class="p-4 rounded-lg border {isActiveInstalled ? 'border-emerald-500/30 bg-emerald-500/5' : 'border-amber-500/30 bg-amber-500/5'}">
                <div class="flex items-start justify-between gap-3">
                  <div class="flex items-center gap-2.5 min-w-0">
                    {#if isActiveInstalled}
                      <CheckCircle2 size={18} class="text-emerald-400 shrink-0" />
                      <div>
                        <div class="text-xs font-semibold text-primary-theme flex items-center gap-2 flex-wrap">
                          <span>Antigravity CLI (agy) Detected</span>
                          <span class="px-1.5 py-0.5 rounded text-[10px] bg-emerald-500/20 text-emerald-400 font-mono font-medium">
                            {activeVersionLabel}
                          </span>
                        </div>
                        <p class="text-[11px] text-muted-theme mt-0.5">
                          Runs <code class="font-mono text-secondary-theme">agy update</code> or the official installer to keep agent capabilities current.
                        </p>
                      </div>
                    {:else}
                      <AlertCircle size={18} class="text-amber-400 shrink-0" />
                      <div>
                        <div class="text-xs font-semibold text-primary-theme">
                          Antigravity CLI (agy) Not Found
                        </div>
                        <p class="text-[11px] text-muted-theme mt-0.5">
                          Install <code class="font-mono text-secondary-theme">agy</code> via the official release script or ensure it is available on your system.
                        </p>
                      </div>
                    {/if}
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
                      <span>{isActiveInstalled ? "Update agy" : "Install agy"}</span>
                    {/if}
                  </button>
                </div>

                {#if activeBinaryPath}
                  <div class="mt-3 pt-3 border-t border-subtle/60 flex items-center justify-between text-[11px] font-mono text-muted-theme">
                    <span class="truncate mr-2">{activeBinaryPath}</span>
                    <button
                      type="button"
                      onclick={() => activeBinaryPath && handleCopyPath(activeBinaryPath)}
                      class="p-1 hover:text-accent-theme text-secondary-theme rounded transition-colors shrink-0 cursor-pointer flex items-center gap-1"
                      title="Copy binary path"
                    >
                      {#if hasCopiedPath}
                        <Check size={12} class="text-emerald-400" />
                        <span class="text-emerald-400 text-[10px]">Copied</span>
                      {:else}
                        <Copy size={12} />
                      {/if}
                    </button>
                  </div>
                {/if}
              </div>

              {#if agyInstallResult}
                <div class="p-3 rounded-lg text-xs flex items-start gap-2 {agyInstallResult.success ? 'bg-emerald-500/10 border border-emerald-500/30 text-emerald-300' : 'bg-rose-500/10 border border-rose-500/30 text-rose-300'}">
                  {#if agyInstallResult.success}
                    <CheckCircle2 size={15} class="text-emerald-400 shrink-0 mt-0.5" />
                  {:else}
                    <XCircle size={15} class="text-rose-400 shrink-0 mt-0.5" />
                  {/if}
                  <div>
                    <p class="font-medium">{agyInstallResult.message}</p>
                    {#if agyInstallResult.version}
                      <p class="text-[11px] opacity-80 mt-0.5 font-mono">Installed Version: {agyInstallResult.version}</p>
                    {/if}
                  </div>
                </div>
              {/if}
            {:else}
              <!-- Unified Gemini CLI Status + Install/Update Card -->
              <div class="p-4 rounded-lg border {isActiveInstalled ? 'border-emerald-500/30 bg-emerald-500/5' : 'border-amber-500/30 bg-amber-500/5'}">
                <div class="flex items-start justify-between gap-3">
                  <div class="flex items-center gap-2.5 min-w-0">
                    {#if isActiveInstalled}
                      <CheckCircle2 size={18} class="text-emerald-400 shrink-0" />
                      <div>
                        <div class="text-xs font-semibold text-primary-theme flex items-center gap-2 flex-wrap">
                          <span>Gemini CLI Installed</span>
                          <span class="px-1.5 py-0.5 rounded text-[10px] bg-emerald-500/20 text-emerald-400 font-mono font-medium">
                            {activeVersionLabel}
                          </span>
                        </div>
                        <p class="text-[11px] text-muted-theme mt-0.5">
                          Installs or updates <code class="font-mono text-secondary-theme">@google/gemini-cli@latest</code> via npm or WinGet.
                        </p>
                      </div>
                    {:else}
                      <AlertCircle size={18} class="text-amber-400 shrink-0" />
                      <div>
                        <div class="text-xs font-semibold text-primary-theme">
                          Gemini CLI Not Detected
                        </div>
                        <p class="text-[11px] text-muted-theme mt-0.5">
                          Install the official <code class="font-mono text-secondary-theme">@google/gemini-cli</code> package via npm or WinGet.
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
                    <span>{isInstalling ? "Installing..." : isActiveInstalled ? "Update to Latest" : "Install Gemini CLI"}</span>
                  </button>
                </div>

                {#if activeBinaryPath}
                  <div class="mt-3 pt-3 border-t border-subtle/60 flex items-center justify-between text-[11px] text-muted-theme font-mono">
                    <span class="truncate pr-2">{activeBinaryPath}</span>
                    <button
                      type="button"
                      onclick={() => activeBinaryPath && handleCopyPath(activeBinaryPath)}
                      class="p-1 hover:text-accent-theme text-secondary-theme rounded transition-colors shrink-0 cursor-pointer flex items-center gap-1"
                      title="Copy binary path"
                    >
                      {#if hasCopiedPath}
                        <Check size={12} class="text-emerald-400" />
                        <span class="text-emerald-400 text-[10px]">Copied</span>
                      {:else}
                        <Copy size={12} />
                      {/if}
                    </button>
                  </div>
                {/if}
              </div>

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
            {/if}

            <!-- Strictly Active-Engine Global PATH Card -->
            <div class="p-4 rounded-lg border border-subtle bg-surface space-y-3">
              <div class="flex items-start justify-between gap-2">
                <div>
                  <div class="text-xs font-semibold text-primary-theme flex items-center gap-2">
                    <Globe size={14} class="text-accent-theme" />
                    <span>Windows User PATH ({activeEngine === "agy" ? "agy" : "gemini"})</span>
                  </div>
                  <p class="text-[11px] text-muted-theme mt-0.5">
                    Add the <code class="font-mono text-secondary-theme">{activeEngine === "agy" ? "agy" : "gemini"}</code> binary folder to your Windows User PATH so it works directly in external terminals.
                  </p>
                </div>
                <button
                  type="button"
                  onclick={() => engineStore.refresh()}
                  disabled={engineStore.isCheckingPath}
                  class="p-1.5 rounded text-secondary-theme hover:text-primary-theme hover:bg-surface-elevated transition-colors cursor-pointer text-xs shrink-0"
                  title="Refresh PATH Status"
                >
                  <RefreshCw size={13} class={engineStore.isCheckingPath ? "animate-spin" : ""} />
                </button>
              </div>

              <CliPathStatusBanner engineId={activeEngine} />
            </div>

            <!-- Antigravity Extension/Config Info (only when agy selected) -->
            {#if activeEngine === "agy"}
              <div class="p-4 rounded-lg border border-subtle bg-surface space-y-3">
                <div class="flex items-start justify-between gap-3">
                  <div>
                    <div class="text-xs font-semibold text-primary-theme flex items-center gap-2">
                      <Sparkles size={14} class="text-purple-400" />
                      <span>Import Gemini CLI Extensions &amp; Skills</span>
                    </div>
                    <p class="text-[11px] text-muted-theme mt-0.5 leading-relaxed">
                      Runs <code class="font-mono text-secondary-theme">agy plugin import gemini</code>. Global config lives in <code class="font-mono text-secondary-theme">~/.gemini/antigravity-cli/settings.json</code> and MCP/plugins in <code class="font-mono text-secondary-theme">~/.gemini/config/</code>.
                    </p>
                  </div>
                  {#if isActiveInstalled}
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
                        <span>Import Plugins</span>
                      {/if}
                    </button>
                  {/if}
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

            <div class="flex justify-end pt-1">
              <button
                type="button"
                onclick={() => (activeTab = "auth")}
                class="px-3.5 py-2 rounded-lg text-xs font-medium bg-accent-theme/15 hover:bg-accent-theme/25 text-accent-theme border border-accent-theme/30 transition-colors cursor-pointer flex items-center gap-1.5"
              >
                <span>Continue to 2. Authentication</span>
                <ArrowRight size={13} />
              </button>
            </div>
          </div>
        {/if}

        <!-- ================= TAB 2: AUTHENTICATION & USER ENV VARS ================= -->
        {#if activeTab === "auth"}
          <div class="space-y-4">
            {#if activeEngine === "agy"}
              <!-- Antigravity Interactive OAuth Primary Card -->
              <div class="p-4 rounded-lg border border-purple-500/30 bg-purple-500/5 space-y-3">
                <div class="flex items-start justify-between gap-3">
                  <div class="space-y-1">
                    <div class="text-xs font-semibold text-primary-theme flex items-center gap-2">
                      <LogIn size={15} class="text-purple-400" />
                      <span>Antigravity Account Sign-In (<code class="font-mono text-purple-300">agy acp --login</code>)</span>
                    </div>
                    <p class="text-[11px] text-secondary-theme leading-relaxed">
                      Recommended for Antigravity CLI: signs in via OAuth keyring and handles token refresh automatically. For corporate Vertex ADC or a personal API key, configure the overrides below.
                    </p>
                  </div>
                  <button
                    type="button"
                    onclick={() => handleLaunchEngineLogin("agy")}
                    disabled={isLaunchingLogin || !isActiveInstalled}
                    class="px-3 py-1.5 rounded-lg bg-purple-600 hover:bg-purple-500 text-white font-medium text-xs transition-colors cursor-pointer disabled:opacity-50 flex items-center gap-1.5 shrink-0"
                  >
                    <LogIn size={13} />
                    <span>{isLaunchingLogin ? "Launching..." : "Sign In (agy)"}</span>
                  </button>
                </div>

                {#if loginResult}
                  <div class="p-2.5 rounded-lg text-[11px] {loginResult.success ? 'bg-emerald-500/10 border border-emerald-500/30 text-emerald-300' : 'bg-rose-500/10 border border-rose-500/30 text-rose-300'}">
                    {loginResult.message}
                  </div>
                {/if}
              </div>
            {/if}

            <!-- Credential Provider Mode Selector -->
            <div>
              <span class="block text-xs font-semibold text-primary-theme mb-2">
                {activeEngine === "agy"
                  ? "Google Cloud / BYOK Override Mode"
                  : "Select Authentication Mode"}
              </span>
              <div class="grid grid-cols-2 gap-3">
                <button
                  type="button"
                  onclick={() => (authMode = "vertex_ai")}
                  class="p-3 rounded-lg border text-left transition-all cursor-pointer {authMode === 'vertex_ai' ? 'border-accent-theme bg-accent-theme/10 ring-1 ring-accent-theme/30' : 'border-subtle bg-surface hover:bg-surface-hover'}"
                >
                  <div class="flex items-center justify-between">
                    <div class="flex items-center gap-2 text-xs font-semibold {authMode === 'vertex_ai' ? 'text-accent-theme' : 'text-primary-theme'}">
                      <Building2 size={14} />
                      <span>{activeEngine === "agy" ? "Google Cloud / ADC" : "Google Cloud Vertex AI"}</span>
                    </div>
                    <span class="text-[10px] px-1.5 py-0.5 rounded bg-accent-theme/20 text-accent-theme font-medium">Default</span>
                  </div>
                  <p class="text-[11px] text-muted-theme mt-1 leading-relaxed">
                    {activeEngine === "agy"
                      ? "Set GCP Project & Region (and optional AGY_ADC_AUTH) for corporate Vertex or ADC workflows."
                      : "Vertex AI with GCP Project, native Gemini OAuth, ADC, or Service Account key."}
                  </p>
                </button>

                <button
                  type="button"
                  onclick={() => (authMode = "api_key")}
                  class="p-3 rounded-lg border text-left transition-all cursor-pointer {authMode === 'api_key' ? 'border-accent-theme bg-accent-theme/10 ring-1 ring-accent-theme/30' : 'border-subtle bg-surface hover:bg-surface-hover'}"
                >
                  <div class="flex items-center gap-2 text-xs font-semibold {authMode === 'api_key' ? 'text-accent-theme' : 'text-primary-theme'}">
                    <Key size={14} />
                    <span>Google AI Studio (API Key)</span>
                  </div>
                  <p class="text-[11px] text-muted-theme mt-1 leading-relaxed">
                    Uses <code class="font-mono text-[10px]">GEMINI_API_KEY</code> for direct developer API billing or free tier.
                  </p>
                </button>
              </div>
            </div>

            <!-- Mode A Inputs: Google Cloud / Vertex AI -->
            {#if authMode === "vertex_ai"}
              <div class="space-y-3.5 p-4 rounded-lg bg-surface border border-subtle">
                {#if activeEngine === "agy"}
                  <label class="flex items-start gap-2.5 p-2.5 rounded-lg bg-surface-elevated border border-subtle cursor-pointer">
                    <input
                      id="agy-adc-auth"
                      type="checkbox"
                      bind:checked={agyAdcAuth}
                      class="w-4 h-4 mt-0.5 rounded border-subtle accent-accent-theme cursor-pointer"
                    />
                    <div class="text-xs">
                      <span class="font-medium text-primary-theme">
                        Enable Enterprise ADC (<code class="font-mono text-[11px] text-accent-theme">AGY_ADC_AUTH=true</code>)
                      </span>
                      <p class="text-[11px] text-muted-theme mt-0.5">
                        Enable this if your organization configures Antigravity CLI to authenticate through Google Application Default Credentials instead of interactive login.
                      </p>
                    </div>
                  </label>
                {/if}

                <!-- GCP Project ID -->
                <div>
                  <div class="flex items-center justify-between mb-1.5 gap-2 flex-wrap">
                    <label for="gcp-project" class="text-xs font-medium text-primary-theme flex items-center gap-1.5">
                      <span>Google Cloud Project ID</span>
                      {#if activeEngine === "gemini"}
                        <span class="text-rose-400">*</span>
                      {/if}
                    </label>
                    <div class="flex items-center gap-2.5">
                      {#if userEnvComparison.detectedUserProject && googleCloudProject.trim() !== userEnvComparison.detectedUserProject}
                        <button
                          type="button"
                          onclick={() => (googleCloudProject = userEnvComparison.detectedUserProject || "")}
                          class="text-[11px] text-sky-400 hover:underline cursor-pointer"
                        >
                          Use User Env ({userEnvComparison.detectedUserProject})
                        </button>
                      {/if}
                      {#if gcloudStatus?.active_project && googleCloudProject.trim() !== gcloudStatus.active_project}
                        <button
                          type="button"
                          onclick={() => (googleCloudProject = gcloudStatus?.active_project || "")}
                          class="text-[11px] text-accent-theme hover:underline cursor-pointer"
                        >
                          Use gcloud ({gcloudStatus.active_project})
                        </button>
                      {/if}
                    </div>
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

                <!-- Google Account / OAuth & ADC Status Banner (for Gemini CLI or ADC verification) -->
                <div class="p-3 rounded-lg border border-subtle bg-surface-elevated flex items-start justify-between gap-3">
                  <div class="flex items-start gap-2.5 min-w-0">
                    <Cloud size={16} class="text-sky-400 shrink-0 mt-0.5" />
                    <div class="min-w-0">
                      <div class="text-xs font-semibold text-primary-theme flex items-center gap-2 flex-wrap">
                        <span>Google Account / ADC Status</span>
                        {#if gcloudStatus?.has_gemini_oauth && activeEngine === "gemini"}
                          <span class="px-1.5 py-0.5 rounded text-[10px] bg-emerald-500/20 text-emerald-400 font-medium">
                            Gemini OAuth Connected
                          </span>
                        {:else if gcloudStatus?.has_adc}
                          <span class="px-1.5 py-0.5 rounded text-[10px] bg-emerald-500/20 text-emerald-400 font-medium">
                            ADC Found
                          </span>
                        {:else}
                          <span class="px-1.5 py-0.5 rounded text-[10px] bg-amber-500/20 text-amber-400 font-medium">
                            {activeEngine === "gemini" ? "Browser Login Ready" : "ADC Not Found"}
                          </span>
                        {/if}
                      </div>
                      <p class="text-[11px] text-muted-theme mt-1 leading-relaxed">
                        {#if activeEngine === "gemini"}
                          {#if gcloudStatus?.has_gemini_oauth}
                            Native Gemini CLI OAuth is active for <strong class="text-secondary-theme font-medium">{gcloudStatus?.active_account || "your Google account"}</strong> (<code class="font-mono text-[10px]">~/.gemini/</code>). <code class="font-mono text-[10px]">gcloud</code> is optional.
                          {:else if gcloudStatus?.has_adc}
                            Application Default Credentials detected for <strong class="text-secondary-theme font-medium">{gcloudStatus?.active_account || "active account"}</strong>.
                          {:else}
                            Launch Gemini CLI to complete browser OAuth sign-in automatically, or pick a Service Account JSON key below. <code class="font-mono text-[10px]">gcloud</code> is not required.
                          {/if}
                        {:else}
                          {#if gcloudStatus?.has_adc}
                            Google Application Default Credentials found for <strong class="text-secondary-theme font-medium">{gcloudStatus?.active_account || "active account"}</strong>.
                          {:else}
                            No local <code class="font-mono text-[10px]">gcloud</code> ADC file detected — normal if using <code class="font-mono text-[10px]">agy acp --login</code> or a Service Account key below.
                          {/if}
                        {/if}
                      </p>
                      {#if activeEngine === "gemini" && loginResult}
                        <div class="mt-2 p-2 rounded text-[11px] {loginResult.success ? 'bg-emerald-500/10 border border-emerald-500/30 text-emerald-300' : 'bg-rose-500/10 border border-rose-500/30 text-rose-300'}">
                          {loginResult.message}
                        </div>
                      {/if}
                    </div>
                  </div>
                  <div class="flex items-center gap-1.5 shrink-0">
                    {#if activeEngine === "gemini"}
                      <button
                        type="button"
                        onclick={() => handleLaunchEngineLogin("gemini")}
                        disabled={isLaunchingLogin || !isActiveInstalled}
                        class="px-2.5 py-1.5 rounded-lg bg-accent-theme/15 hover:bg-accent-theme/25 text-accent-theme border border-accent-theme/30 font-medium text-[11px] transition-colors cursor-pointer disabled:opacity-50 flex items-center gap-1"
                        title="Open interactive Gemini CLI terminal for OAuth login"
                      >
                        <LogIn size={12} />
                        <span>Launch Login</span>
                      </button>
                    {/if}
                    <button
                      type="button"
                      onclick={loadGCloudStatus}
                      disabled={isCheckingGCloud}
                      class="p-1.5 rounded text-secondary-theme hover:text-primary-theme hover:bg-surface transition-colors cursor-pointer"
                      title="Re-detect OAuth & ADC status"
                    >
                      <RefreshCw size={12} class={isCheckingGCloud ? "animate-spin" : ""} />
                    </button>
                  </div>
                </div>

                <!-- Optional Service Account Key with Native Browse Picker -->
                <div>
                  <label for="gcp-creds" class="block text-xs font-medium text-primary-theme mb-1.5">
                    Service Account Key File (Optional)
                  </label>
                  <div class="flex items-center gap-2">
                    <input
                      id="gcp-creds"
                      type="text"
                      bind:value={googleAppCredentials}
                      placeholder="C:\keys\service-account-key.json"
                      class="flex-1 min-w-0 px-3 py-2 rounded-lg bg-surface-elevated border border-theme-default text-xs text-primary-theme focus:outline-none focus:border-accent-theme font-mono"
                    />
                    <button
                      type="button"
                      onclick={handleBrowseCredentials}
                      class="px-3 py-2 rounded-lg border border-theme-default bg-surface-elevated hover:bg-surface-hover text-xs text-secondary-theme hover:text-primary-theme transition-colors cursor-pointer flex items-center gap-1.5 shrink-0"
                    >
                      <FolderOpen size={13} />
                      <span>Browse...</span>
                    </button>
                  </div>
                  <p class="text-[10px] text-muted-theme mt-1">
                    Environment variable: <code class="font-mono text-secondary-theme">GOOGLE_APPLICATION_CREDENTIALS</code> (injected securely by Gemini Desktop / workspace <code class="font-mono">.env.local</code>).
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
                      <span>Get API Key</span>
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
                    Stored in Gemini Desktop SQLite (and optional workspace <code class="font-mono">.env.local</code>). Never written to Windows User Environment Registry.
                  </p>
                </div>
              </div>
            {/if}

            <!-- Windows User Environment Variables Card (Non-Secret GCP / Vertex Settings) -->
            {#if authMode === "vertex_ai" || userEnvComparison.hasAnyWindowsVarSet}
              <div class="p-4 rounded-lg bg-surface border border-subtle space-y-3">
                <div class="flex items-start justify-between gap-2">
                  <div>
                    <div class="text-xs font-semibold text-primary-theme flex items-center gap-2 flex-wrap">
                      <Globe size={14} class="text-sky-400" />
                      <span>Windows User Environment Variables</span>
                      <span class="px-1.5 py-0.5 rounded text-[10px] bg-sky-500/15 text-sky-300 border border-sky-500/30 font-mono">
                        Non-Secret GCP Vars
                      </span>
                    </div>
                    <p class="text-[11px] text-muted-theme mt-0.5 leading-relaxed">
                      Sync <code class="font-mono text-secondary-theme">GOOGLE_CLOUD_PROJECT</code> &amp; <code class="font-mono text-secondary-theme">GOOGLE_CLOUD_LOCATION</code> to <code class="font-mono text-secondary-theme">HKCU\Environment</code> so external terminals (PowerShell, VS Code) pick them up automatically.
                    </p>
                  </div>
                  <button
                    type="button"
                    onclick={loadUserEnvVarsStatus}
                    disabled={isCheckingUserEnv}
                    class="p-1.5 rounded text-secondary-theme hover:text-primary-theme hover:bg-surface-elevated transition-colors cursor-pointer shrink-0"
                    title="Refresh Windows User Environment status"
                  >
                    <RefreshCw size={12} class={isCheckingUserEnv ? "animate-spin" : ""} />
                  </button>
                </div>

                <!-- Live Variable Comparison Grid -->
                <div class="space-y-1.5">
                  {#each userEnvComparison.rows as row (row.name)}
                    {@const badge = formatEnvBadge(row.state)}
                    <div class="px-2.5 py-1.5 rounded bg-surface-elevated border border-subtle/80 flex items-center justify-between gap-2 text-[11px]">
                      <div class="flex items-center gap-2 min-w-0">
                        <code class="font-mono font-medium text-primary-theme shrink-0">{row.name}</code>
                        <span class="font-mono text-muted-theme truncate">
                          {row.windowsValue ? `= ${row.windowsValue}` : "(not set in Windows)"}
                        </span>
                      </div>
                      <span class="px-1.5 py-0.5 rounded text-[10px] font-medium border shrink-0 {badge.cls}">
                        {badge.label}
                      </span>
                    </div>
                  {/each}
                </div>

                {#if authMode === "vertex_ai"}
                  <div class="space-y-1.5 pt-0.5">
                    <label class="flex items-center gap-2 text-xs text-secondary-theme cursor-pointer">
                      <input
                        id="include-use-vertex-flag"
                        type="checkbox"
                        bind:checked={includeUseVertexFlag}
                        class="w-3.5 h-3.5 rounded border-subtle accent-accent-theme cursor-pointer"
                      />
                      <span>Also export <code class="font-mono text-[11px] text-primary-theme">GOOGLE_GENAI_USE_VERTEXAI=true</code></span>
                    </label>
                    <label class="flex items-center gap-2 text-xs text-secondary-theme cursor-pointer">
                      <input
                        id="sync-user-env-on-save"
                        type="checkbox"
                        bind:checked={syncToUserEnvOnSave}
                        class="w-3.5 h-3.5 rounded border-subtle accent-accent-theme cursor-pointer"
                      />
                      <span>Apply to Windows User Environment when clicking <strong>Save Configuration</strong></span>
                    </label>
                  </div>
                {/if}

                <div class="flex items-center justify-between gap-2 flex-wrap pt-1 border-t border-subtle/60">
                  <span class="text-[10px] text-muted-theme">
                    Already-open external terminals require a restart after User Env updates.
                  </span>
                  <div class="flex items-center gap-2 shrink-0">
                    {#if userEnvComparison.hasAnyWindowsVarSet}
                      <button
                        type="button"
                        onclick={() => handleApplyUserEnv(true)}
                        disabled={isSyncingUserEnv}
                        class="px-2.5 py-1 rounded-md text-[11px] font-medium border border-rose-500/30 bg-rose-500/10 hover:bg-rose-500/20 text-rose-300 transition-colors cursor-pointer disabled:opacity-50 flex items-center gap-1"
                      >
                        <Trash2 size={11} />
                        <span>Clear GCP Vars</span>
                      </button>
                    {/if}
                    {#if authMode === "vertex_ai"}
                      <button
                        type="button"
                        onclick={() => handleApplyUserEnv(false)}
                        disabled={isSyncingUserEnv || (!googleCloudProject.trim() && !(activeEngine === "agy" && agyAdcAuth))}
                        class="px-2.5 py-1 rounded-md text-[11px] font-medium border border-sky-500/30 bg-sky-500/15 hover:bg-sky-500/25 text-sky-300 transition-colors cursor-pointer disabled:opacity-50 flex items-center gap-1"
                      >
                        {#if isSyncingUserEnv}
                          <RefreshCw size={11} class="animate-spin" />
                          <span>Applying...</span>
                        {:else}
                          <Globe size={11} />
                          <span>Apply Now to User Env</span>
                        {/if}
                      </button>
                    {/if}
                  </div>
                </div>

                {#if userEnvFeedback}
                  <div class="p-2.5 rounded-lg text-[11px] flex items-start gap-2 {userEnvFeedback.type === 'success' ? 'bg-emerald-500/10 border border-emerald-500/30 text-emerald-300' : 'bg-rose-500/10 border border-rose-500/30 text-rose-300'}">
                    {#if userEnvFeedback.type === "success"}
                      <CheckCircle2 size={14} class="text-emerald-400 shrink-0 mt-0.5" />
                    {:else}
                      <AlertCircle size={14} class="text-rose-400 shrink-0 mt-0.5" />
                    {/if}
                    <span>{userEnvFeedback.text}</span>
                  </div>
                {/if}
              </div>
            {/if}

            <!-- Scope Selection (Workspace .env.local) -->
            {#if activeWorkspace}
              <div class="flex items-center gap-2 px-1">
                <input
                  id="apply-workspace-env"
                  type="checkbox"
                  bind:checked={applyToWorkspace}
                  class="w-4 h-4 rounded border-subtle accent-accent-theme cursor-pointer"
                />
                <label for="apply-workspace-env" class="text-xs text-secondary-theme cursor-pointer flex items-center gap-1.5">
                  <FolderKanban size={13} class="text-accent-theme" />
                  <span>Also sync into active workspace <code class="font-mono text-[11px] text-primary-theme">.env.local</code> ({activeWorkspace.name})</span>
                </label>
              </div>
            {/if}

            <!-- Save Feedback Message -->
            {#if saveMessage}
              <div class="p-3 rounded-lg text-xs border {saveMessage.type === 'success' ? 'border-emerald-500/30 bg-emerald-500/10 text-emerald-300' : 'border-rose-500/30 bg-rose-500/10 text-rose-300'} flex items-center gap-2">
                {#if saveMessage.type === "success"}
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
                onclick={() => (isDirty ? handleSaveAuth(true) : (activeTab = "diagnostics"))}
                disabled={isSaving}
                class="px-3.5 py-2 rounded-lg text-xs font-medium text-secondary-theme hover:text-primary-theme hover:bg-surface transition-colors cursor-pointer disabled:opacity-50"
              >
                {isDirty ? "Save & Test Connection →" : "Go to Connection Test →"}
              </button>
              <button
                type="button"
                onclick={() => handleSaveAuth(false)}
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
              <div class="flex items-center justify-between gap-3">
                <div>
                  <h3 class="text-xs font-semibold text-primary-theme flex items-center gap-1.5">
                    <ShieldCheck size={15} class="text-accent-theme" />
                    Live Verification Suite ({activeEngine === "agy" ? "Antigravity CLI" : "Gemini CLI"})
                  </h3>
                  <p class="text-[11px] text-muted-theme mt-0.5">
                    Tests the selected CLI binary, active credentials, and live model response using your current settings.
                  </p>
                </div>
                <button
                  type="button"
                  onclick={handleRunTest}
                  disabled={isTesting || !isActiveInstalled}
                  class="px-4 py-2 rounded-lg text-xs font-medium bg-accent-theme text-white hover:opacity-90 disabled:opacity-50 transition-all flex items-center gap-1.5 cursor-pointer shadow-xs shrink-0"
                >
                  <RefreshCw size={13} class={isTesting ? "animate-spin" : ""} />
                  <span>{isTesting ? "Testing..." : `Run Test (${activeEngine === "agy" ? "agy" : "gemini"})`}</span>
                </button>
              </div>

              {#if activeEngine === "gemini"}
                <div class="flex items-center gap-2 pt-2.5 border-t border-subtle text-xs">
                  <label for="test-model-select" class="text-secondary-theme font-medium shrink-0">
                    Verification Model:
                  </label>
                  <select
                    id="test-model-select"
                    bind:value={testModel}
                    class="flex-1 px-2.5 py-1.5 rounded-lg bg-surface-elevated border border-theme-default text-xs text-primary-theme focus:outline-none focus:border-accent-theme font-mono"
                  >
                    <option value="gemini-3.8-flash">gemini-3.8-flash (Latest Preview Flash — Recommended)</option>
                    <option value="auto">auto (Let Gemini CLI Auto-Route Dynamically)</option>
                    <option value="gemini-3.5-flash">gemini-3.5-flash (Fast &amp; Stable 3.5)</option>
                    <option value="gemini-3.5-flash-lite">gemini-3.5-flash-lite (Ultra-fast &amp; Lightweight 3.5)</option>
                    <option value="gemini-3.1-pro-preview">gemini-3.1-pro-preview (Gemini 3.1 Pro)</option>
                    <option value="gemini-2.5-pro">gemini-2.5-pro (Gemini 2.5 Pro)</option>
                  </select>
                </div>
              {:else}
                <div class="flex flex-wrap items-center justify-between gap-2 pt-2.5 border-t border-subtle text-xs text-secondary-theme">
                  <span class="font-mono">🚀 Engine: <strong>Antigravity CLI (agy)</strong></span>
                  <label class="flex items-center gap-1.5 cursor-pointer text-[11px] text-primary-theme">
                    <input
                      type="checkbox"
                      bind:checked={verifyAgyLiveAuth}
                      class="rounded accent-accent-theme cursor-pointer"
                    />
                    <span>Verify live auth &amp; model response (<code class="font-mono text-[10px]">-p</code>)</span>
                  </label>
                </div>
              {/if}
            </div>

            <!-- Engine-Accurate Diagnostics Checklist Cards -->
            <div class="grid grid-cols-2 gap-3">
              {#each [diagnostics.binary, diagnostics.auth, diagnostics.latency, diagnostics.model] as item}
                <div class="p-3 rounded-lg border border-subtle bg-surface-elevated">
                  <div class="flex items-center justify-between">
                    <span class="text-xs font-medium text-secondary-theme">{item.label}</span>
                    {#if item.status === "ok"}
                      <CheckCircle2 size={15} class="text-emerald-400 shrink-0" />
                    {:else if item.status === "error"}
                      <XCircle size={15} class="text-rose-400 shrink-0" />
                    {:else if item.status === "warn"}
                      <AlertCircle size={15} class="text-amber-400 shrink-0" />
                    {:else}
                      <span class="text-[10px] text-muted-theme font-mono">Ready</span>
                    {/if}
                  </div>
                  <div class="text-[11px] text-muted-theme mt-1 truncate">
                    {item.detail}
                  </div>
                </div>
              {/each}
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
          Settings saved locally in SQLite; non-secret GCP variables optionally synced to Windows User Env.
        </span>
        <button
          type="button"
          onclick={handleAttemptClose}
          class="px-4 py-1.5 rounded-lg bg-surface hover:bg-surface-hover border border-subtle text-primary-theme transition-colors cursor-pointer"
        >
          {isDirty ? "Discard & Close" : "Close"}
        </button>
      </div>
    </div>
  </div>
{/if}
