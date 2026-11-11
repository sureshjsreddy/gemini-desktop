<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import type {
    GlobalCliSettings,
    SaveGlobalCliSettingsPayload,
    CliEngine,
    CliEnginesStatus,
    CliPathStatus,
    PathAddResult,
  } from "$lib/types";
  import {
    Sliders,
    Zap,
    History,
    FileCode,
    Filter,
    X,
    Check,
    Save,
    RefreshCw,
    FolderOpen,
    Copy,
    AlertCircle,
    Globe,
    CheckCircle2,
  } from "lucide-svelte";
  import Tooltip from "$lib/components/Tooltip.svelte";

  let {
    isOpen = false,
    onClose,
  }: {
    isOpen: boolean;
    onClose: () => void;
  } = $props();

  type TabType = "general" | "raw";
  let activeTab = $state<TabType>("general");

  let isLoading = $state(false);
  let isSaving = $state(false);
  let errorMessage = $state<string | null>(null);
  let successNotice = $state<string | null>(null);
  let hasCopiedPath = $state(false);

  // Settings State
  let activeEngine = $state<CliEngine>("gemini");
  let engineStatus = $state<CliEnginesStatus | null>(null);
  let isSwitchingEngine = $state(false);
  let settingsFilePath = $state("");
  let chatCompressionEnabled = $state(false);
  let compressionThresholdPercent = $state(75);
  let maxSessionTurns = $state(-1);
  let respectGitIgnore = $state(true);
  let respectGeminiIgnore = $state(true);
  let retentionEnabled = $state(true);
  let retentionMaxAge = $state("30d");
  let rawJson = $state("");

  // PATH State
  let cliPathStatus = $state<CliPathStatus | null>(null);
  let isCheckingPath = $state(false);
  let isAddingToPath = $state(false);
  let pathAddResult = $state<PathAddResult | null>(null);

  $effect(() => {
    if (isOpen) {
      loadSettings();
    }
  });

  async function loadSettings() {
    isLoading = true;
    errorMessage = null;
    successNotice = null;
    try {
      const [res, engines, pathStatus] = await Promise.all([
        invoke<GlobalCliSettings>("get_global_cli_settings"),
        invoke<CliEnginesStatus>("get_cli_engines_status"),
        invoke<CliPathStatus>("check_cli_path_status").catch(() => null),
      ]);
      settingsFilePath = res.filePath;
      chatCompressionEnabled = res.chatCompression.enabled;
      compressionThresholdPercent = Math.round(res.chatCompression.contextPercentageThreshold * 100);
      maxSessionTurns = res.maxSessionTurns;
      respectGitIgnore = res.fileFiltering.respectGitIgnore;
      respectGeminiIgnore = res.fileFiltering.respectGeminiIgnore;
      retentionEnabled = res.sessionRetention.enabled;
      retentionMaxAge = res.sessionRetention.maxAge;
      rawJson = res.rawJson;

      engineStatus = engines;
      activeEngine = engines.activeEngine;
      cliPathStatus = pathStatus;
    } catch (e: any) {
      errorMessage = typeof e === "string" ? e : e?.message || "Failed to load global CLI settings.";
    } finally {
      isLoading = false;
    }
  }

  async function handleSwitchEngine(engine: CliEngine) {
    if (activeEngine === engine || isSwitchingEngine) return;
    isSwitchingEngine = true;
    errorMessage = null;
    successNotice = null;
    try {
      const updated = await invoke<CliEngine>("set_active_cli_engine", { engine });
      activeEngine = updated;
      if (typeof window !== "undefined") {
        localStorage.setItem("gemini_desktop_active_engine", updated);
      }
      if (engineStatus) {
        engineStatus.activeEngine = updated;
      }
      await loadSettings();
      cliPathStatus = await invoke<CliPathStatus>("check_cli_path_status").catch(() => null);
      successNotice = `Switched agent engine to ${updated === "agy" ? "Antigravity CLI (agy)" : "Gemini CLI"}! All workspaces will now use this engine.`;
      setTimeout(() => {
        if (successNotice) successNotice = null;
      }, 4000);
    } catch (e: any) {
      errorMessage = typeof e === "string" ? e : e?.message || "Failed to switch engine.";
    } finally {
      isSwitchingEngine = false;
    }
  }

  async function handleAddToPath(engine: CliEngine) {
    isAddingToPath = true;
    pathAddResult = null;
    try {
      const res = await invoke<PathAddResult>("add_cli_to_user_path", { engine });
      pathAddResult = res;
      cliPathStatus = await invoke<CliPathStatus>("check_cli_path_status").catch(() => null);
      const engines = await invoke<CliEnginesStatus>("get_cli_engines_status").catch(() => null);
      if (engines) engineStatus = engines;
    } catch (e: any) {
      pathAddResult = {
        success: false,
        message: e?.toString() || "Failed to add to User PATH.",
        targetDir: "",
        alreadyInPath: false,
      };
    } finally {
      isAddingToPath = false;
    }
  }

  async function handleSaveStructured() {
    isSaving = true;
    errorMessage = null;
    successNotice = null;
    try {
      const payload: SaveGlobalCliSettingsPayload = {
        chatCompression: {
          enabled: chatCompressionEnabled,
          contextPercentageThreshold: compressionThresholdPercent / 100,
        },
        maxSessionTurns: maxSessionTurns,
        fileFiltering: {
          respectGitIgnore: respectGitIgnore,
          respectGeminiIgnore: respectGeminiIgnore,
        },
        sessionRetention: {
          enabled: retentionEnabled,
          maxAge: retentionMaxAge,
        },
      };

      const updated = await invoke<GlobalCliSettings>("save_global_cli_settings", { payload });
      rawJson = updated.rawJson;
      successNotice = "Global CLI settings saved successfully!";
      setTimeout(() => {
        if (successNotice) successNotice = null;
      }, 3000);
    } catch (e: any) {
      errorMessage = typeof e === "string" ? e : e?.message || "Failed to save settings.";
    } finally {
      isSaving = false;
    }
  }

  async function handleSaveRawJson() {
    isSaving = true;
    errorMessage = null;
    successNotice = null;
    try {
      // Validate JSON parse in frontend first
      JSON.parse(rawJson);

      const payload: SaveGlobalCliSettingsPayload = {
        rawJson: rawJson,
      };

      const updated = await invoke<GlobalCliSettings>("save_global_cli_settings", { payload });
      chatCompressionEnabled = updated.chatCompression.enabled;
      compressionThresholdPercent = Math.round(updated.chatCompression.contextPercentageThreshold * 100);
      maxSessionTurns = updated.maxSessionTurns;
      respectGitIgnore = updated.fileFiltering.respectGitIgnore;
      respectGeminiIgnore = updated.fileFiltering.respectGeminiIgnore;
      retentionEnabled = updated.sessionRetention.enabled;
      retentionMaxAge = updated.sessionRetention.maxAge;
      successNotice = "Global settings.json updated and formatted successfully!";
      setTimeout(() => {
        if (successNotice) successNotice = null;
      }, 3000);
    } catch (e: any) {
      errorMessage = typeof e === "string" ? e : e?.message || "Invalid JSON syntax.";
    } finally {
      isSaving = false;
    }
  }

  async function handleCopyPath() {
    if (!settingsFilePath) return;
    try {
      await navigator.clipboard.writeText(settingsFilePath);
      hasCopiedPath = true;
      setTimeout(() => (hasCopiedPath = false), 2000);
    } catch (e) {
      console.error("Failed to copy path", e);
    }
  }
</script>

{#if isOpen}
  <!-- Backdrop -->
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
      aria-labelledby="cli-settings-title"
      tabindex="-1"
    >
      <!-- Header -->
      <div class="flex items-center justify-between px-5 py-4 border-b border-subtle bg-surface/50">
        <div class="flex items-center gap-3">
          <div
            class="w-9 h-9 rounded-lg flex items-center justify-center text-white shadow-md shrink-0"
            style="background: var(--accent-gradient);"
          >
            <Sliders size={20} />
          </div>
          <div>
            <h2 id="cli-settings-title" class="text-sm font-semibold text-primary-theme flex items-center gap-2">
              {activeEngine === 'agy' ? 'Antigravity CLI Global Settings' : 'Gemini CLI Global Settings'}
            </h2>
            <p class="text-xs text-muted-theme mt-0.5">
              Configure session compaction, turn limits, and token retention policies in <code class="font-mono text-secondary-theme">{settingsFilePath || (activeEngine === 'agy' ? '~/.gemini/antigravity-cli/settings.json' : '~/.gemini/settings.json')}</code>.
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
          onclick={() => (activeTab = "general")}
          class="px-3 py-2 text-xs font-medium border-b-2 transition-colors flex items-center gap-2 cursor-pointer {activeTab === 'general' ? 'border-accent-theme text-accent-theme font-semibold' : 'border-transparent text-secondary-theme hover:text-primary-theme'}"
        >
          <Zap size={14} />
          <span>General & Compaction</span>
        </button>

        <button
          type="button"
          onclick={() => (activeTab = "raw")}
          class="px-3 py-2 text-xs font-medium border-b-2 transition-colors flex items-center gap-2 cursor-pointer {activeTab === 'raw' ? 'border-accent-theme text-accent-theme font-semibold' : 'border-transparent text-secondary-theme hover:text-primary-theme'}"
        >
          <FileCode size={14} />
          <span>Raw JSON Editor</span>
        </button>
      </div>

      <!-- Content Area -->
      <div class="p-5 flex-1 overflow-y-auto space-y-4">
        {#if errorMessage}
          <div class="p-3 rounded-lg bg-rose-500/10 border border-rose-500/30 text-rose-300 text-xs flex items-center gap-2">
            <AlertCircle size={15} class="shrink-0 text-rose-400" />
            <span>{errorMessage}</span>
          </div>
        {/if}

        {#if successNotice}
          <div class="p-3 rounded-lg bg-emerald-500/10 border border-emerald-500/30 text-emerald-300 text-xs flex items-center gap-2">
            <Check size={15} class="shrink-0 text-emerald-400" />
            <span>{successNotice}</span>
          </div>
        {/if}

        {#if activeTab === "general"}
          <!-- Engine Selection Card -->
          <div class="p-4 rounded-lg border border-subtle bg-surface space-y-3">
            <div class="flex items-start justify-between">
              <div>
                <span class="text-xs font-semibold text-primary-theme flex items-center gap-2">
                  <Sliders size={14} class="text-accent-theme" />
                  Active Agent Engine
                </span>
                <p class="text-[11px] text-muted-theme mt-1 leading-relaxed">
                  Select which underlying CLI agent runs across all your project workspaces and chat turns.
                </p>
              </div>
            </div>

            <div class="grid grid-cols-2 gap-3 pt-1">
              <!-- Antigravity (agy) Option -->
              <button
                type="button"
                onclick={() => handleSwitchEngine("agy")}
                disabled={isSwitchingEngine}
                class="flex flex-col text-left p-3 rounded-lg border transition-all cursor-pointer {activeEngine === 'agy' ? 'border-accent-theme bg-accent-theme/10 shadow-xs ring-1 ring-accent-theme/50' : 'border-subtle bg-surface-elevated hover:border-theme-default'}"
              >
                <div class="flex items-center justify-between w-full mb-1">
                  <span class="text-xs font-semibold text-primary-theme flex items-center gap-1.5">
                    🚀 Antigravity (<code class="font-mono text-[11px]">agy</code>)
                  </span>
                  {#if activeEngine === "agy"}
                    <span class="w-2 h-2 rounded-full bg-emerald-400"></span>
                  {/if}
                </div>
                <p class="text-[10px] text-secondary-theme leading-normal">
                  Enterprise Antigravity agent runtime with native Agent Client Protocol (ACP) support.
                </p>
                <div class="mt-2 text-[10px] flex items-center gap-1.5 font-mono {engineStatus?.agyInstalled ? 'text-emerald-400' : 'text-amber-400'}">
                  <span class="w-1.5 h-1.5 rounded-full {engineStatus?.agyInstalled ? 'bg-emerald-400' : 'bg-amber-400'}"></span>
                  <span>{engineStatus?.agyInstalled ? "Installed & Detected" : "Not found in PATH"}</span>
                </div>
              </button>

              <!-- Gemini CLI Option -->
              <button
                type="button"
                onclick={() => handleSwitchEngine("gemini")}
                disabled={isSwitchingEngine}
                class="flex flex-col text-left p-3 rounded-lg border transition-all cursor-pointer {activeEngine === 'gemini' ? 'border-accent-theme bg-accent-theme/10 shadow-xs ring-1 ring-accent-theme/50' : 'border-subtle bg-surface-elevated hover:border-theme-default'}"
              >
                <div class="flex items-center justify-between w-full mb-1">
                  <span class="text-xs font-semibold text-primary-theme flex items-center gap-1.5">
                    ♊ Gemini CLI (<code class="font-mono text-[11px]">gemini</code>)
                  </span>
                  {#if activeEngine === "gemini"}
                    <span class="w-2 h-2 rounded-full bg-emerald-400"></span>
                  {/if}
                </div>
                <p class="text-[10px] text-secondary-theme leading-normal">
                  Standard Gemini CLI supporting Google AI Studio and Vertex AI dual authentication.
                </p>
                <div class="mt-2 text-[10px] flex items-center gap-1.5 font-mono {engineStatus?.geminiInstalled ? 'text-emerald-400' : 'text-amber-400'}">
                  <span class="w-1.5 h-1.5 rounded-full {engineStatus?.geminiInstalled ? 'bg-emerald-400' : 'bg-amber-400'}"></span>
                  <span>{engineStatus?.geminiInstalled ? "Installed & Detected" : "Not found in PATH"}</span>
                </div>
              </button>
            </div>

            <!-- Active Engine PATH Status / Quick Add -->
            {#if activeEngine === "agy"}
              <div class="mt-2.5 p-2.5 rounded-lg bg-surface-elevated border border-subtle flex items-center justify-between text-xs">
                <div class="min-w-0 mr-2">
                  <div class="flex items-center gap-1.5">
                    <Globe size={13} class="text-accent-theme" />
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
            {:else}
              <div class="mt-2.5 p-2.5 rounded-lg bg-surface-elevated border border-subtle flex items-center justify-between text-xs">
                <div class="min-w-0 mr-2">
                  <div class="flex items-center gap-1.5">
                    <Globe size={13} class="text-accent-theme" />
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
            {/if}

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
          </div>

          <!-- Card 1: Context Compression -->
          <div class="p-4 rounded-lg border border-subtle bg-surface space-y-3">
            <div class="flex items-start justify-between">
              <div>
                <span class="text-xs font-semibold text-primary-theme flex items-center gap-2">
                  <Zap size={14} class="text-accent-theme" />
                  Auto Context Compression
                </span>
                <p class="text-[11px] text-muted-theme mt-1 leading-relaxed">
                  Automatically distill conversation history into a structured summary when tokens approach the context ceiling.
                </p>
              </div>
              <label class="relative inline-flex items-center cursor-pointer shrink-0">
                <input
                  type="checkbox"
                  bind:checked={chatCompressionEnabled}
                  class="sr-only peer"
                />
                <div class="w-9 h-5 bg-surface-elevated border border-theme-default peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-accent-theme"></div>
              </label>
            </div>

            {#if chatCompressionEnabled}
              <div class="pt-2 border-t border-subtle/50 space-y-2">
                <div class="flex items-center justify-between text-xs">
                  <span class="text-secondary-theme">Compression Trigger Threshold:</span>
                  <span class="font-mono font-semibold text-accent-theme">{compressionThresholdPercent}% of 1M Tokens</span>
                </div>
                <input
                  type="range"
                  min="50"
                  max="90"
                  step="5"
                  bind:value={compressionThresholdPercent}
                  class="w-full accent-accent-theme cursor-pointer"
                />
                <div class="flex justify-between text-[10px] text-muted-theme">
                  <span>50% (Conservative)</span>
                  <span>75% (Recommended)</span>
                  <span>90% (Max Capacity)</span>
                </div>
              </div>
            {/if}
          </div>

          <!-- Card 2: Session Turn Limit -->
          <div class="p-4 rounded-lg border border-subtle bg-surface space-y-3">
            <div>
              <span class="text-xs font-semibold text-primary-theme flex items-center gap-2">
                <History size={14} class="text-sky-400" />
                Maximum Session Turns (<code class="font-mono text-secondary-theme">model.maxSessionTurns</code>)
              </span>
              <p class="text-[11px] text-muted-theme mt-1 leading-relaxed">
                Prevents runaway autonomous loops and context bloat by capping the maximum turns allowed per session.
              </p>
            </div>

            <div class="flex items-center gap-2 pt-1">
              <input
                type="number"
                bind:value={maxSessionTurns}
                class="w-32 px-3 py-1.5 rounded-lg border border-theme-default bg-surface-elevated text-primary-theme text-xs font-mono focus:outline-none focus:border-accent-theme"
              />
              <div class="flex items-center gap-1.5">
                <button
                  type="button"
                  onclick={() => (maxSessionTurns = -1)}
                  class="px-2 py-1 rounded text-[11px] border border-theme-default hover:bg-surface-hover cursor-pointer transition-colors {maxSessionTurns === -1 ? 'bg-accent-theme/20 text-accent-theme border-accent-theme' : 'text-secondary-theme'}"
                >
                  Unlimited (-1)
                </button>
                <button
                  type="button"
                  onclick={() => (maxSessionTurns = 50)}
                  class="px-2 py-1 rounded text-[11px] border border-theme-default hover:bg-surface-hover cursor-pointer transition-colors {maxSessionTurns === 50 ? 'bg-accent-theme/20 text-accent-theme border-accent-theme' : 'text-secondary-theme'}"
                >
                  50 Turns
                </button>
                <button
                  type="button"
                  onclick={() => (maxSessionTurns = 100)}
                  class="px-2 py-1 rounded text-[11px] border border-theme-default hover:bg-surface-hover cursor-pointer transition-colors {maxSessionTurns === 100 ? 'bg-accent-theme/20 text-accent-theme border-accent-theme' : 'text-secondary-theme'}"
                >
                  100 Turns
                </button>
              </div>
            </div>
          </div>

          <!-- Card 3: File Filtering & Ignore Policies -->
          <div class="p-4 rounded-lg border border-subtle bg-surface space-y-3">
            <div>
              <span class="text-xs font-semibold text-primary-theme flex items-center gap-2">
                <Filter size={14} class="text-emerald-400" />
                Context File Filtering (<code class="font-mono text-secondary-theme">context.fileFiltering</code>)
              </span>
              <p class="text-[11px] text-muted-theme mt-1 leading-relaxed">
                Exclude repository ignore rules from agent searches to protect token context from build artifacts and huge datasets.
              </p>
            </div>

            <div class="grid grid-cols-2 gap-3 pt-1">
              <label class="flex items-center gap-2 p-2.5 rounded-lg border border-subtle bg-surface-elevated cursor-pointer">
                <input
                  type="checkbox"
                  bind:checked={respectGitIgnore}
                  class="rounded accent-accent-theme"
                />
                <div>
                  <span class="text-xs font-medium text-primary-theme">Respect .gitignore</span>
                  <p class="text-[10px] text-muted-theme">Skips node_modules, build, etc.</p>
                </div>
              </label>

              <label class="flex items-center gap-2 p-2.5 rounded-lg border border-subtle bg-surface-elevated cursor-pointer">
                <input
                  type="checkbox"
                  bind:checked={respectGeminiIgnore}
                  class="rounded accent-accent-theme"
                />
                <div>
                  <span class="text-xs font-medium text-primary-theme">Respect .geminiignore</span>
                  <p class="text-[10px] text-muted-theme">Dedicated token exclusions</p>
                </div>
              </label>
            </div>
          </div>

          <!-- Card 4: Retention Policy -->
          <div class="p-4 rounded-lg border border-subtle bg-surface space-y-3">
            <div class="flex items-start justify-between">
              <div>
                <span class="text-xs font-semibold text-primary-theme flex items-center gap-2">
                  <FolderOpen size={14} class="text-amber-400" />
                  Disk Session Retention (<code class="font-mono text-secondary-theme">general.sessionRetention</code>)
                </span>
                <p class="text-[11px] text-muted-theme mt-1 leading-relaxed">
                  Prune historical session cache files stored under <code class="font-mono text-secondary-theme">~/.gemini/tmp/</code>.
                </p>
              </div>
              <label class="relative inline-flex items-center cursor-pointer shrink-0">
                <input
                  type="checkbox"
                  bind:checked={retentionEnabled}
                  class="sr-only peer"
                />
                <div class="w-9 h-5 bg-surface-elevated border border-theme-default peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-accent-theme"></div>
              </label>
            </div>

            {#if retentionEnabled}
              <div class="flex items-center gap-2 pt-1 text-xs">
                <span class="text-secondary-theme">Retain session files for:</span>
                <select
                  bind:value={retentionMaxAge}
                  class="px-2.5 py-1 rounded-lg border border-theme-default bg-surface-elevated text-primary-theme text-xs font-mono focus:outline-none focus:border-accent-theme"
                >
                  <option value="7d">7 Days</option>
                  <option value="14d">14 Days</option>
                  <option value="30d">30 Days (Default)</option>
                  <option value="90d">90 Days</option>
                </select>
              </div>
            {/if}
          </div>

        {:else}
          <!-- Raw JSON Editor Tab -->
          <div class="space-y-3">
            <div class="flex items-center justify-between text-xs">
              <span class="text-muted-theme font-mono truncate max-w-[400px]">
                {settingsFilePath}
              </span>
              <button
                type="button"
                onclick={handleCopyPath}
                class="flex items-center gap-1 px-2 py-1 text-[11px] text-secondary-theme hover:text-primary-theme border border-theme-default rounded hover:bg-surface-hover cursor-pointer"
              >
                {#if hasCopiedPath}
                  <Check size={12} class="text-emerald-400" />
                  <span>Copied</span>
                {:else}
                  <Copy size={12} />
                  <span>Copy Path</span>
                {/if}
              </button>
            </div>

            <textarea
              bind:value={rawJson}
              rows={16}
              class="w-full p-3 font-mono text-xs rounded-lg border border-theme-default bg-surface-elevated text-primary-theme focus:outline-none focus:border-accent-theme leading-relaxed resize-none"
              placeholder={"{\n  // json settings\n}"}
            ></textarea>
            <p class="text-[11px] text-muted-theme">
              Directly edit the complete settings file. All non-standard keys are safely preserved upon save.
            </p>
          </div>
        {/if}
      </div>

      <!-- Footer -->
      <div class="px-5 py-3 border-t border-subtle bg-surface flex items-center justify-between">
        <button
          type="button"
          onclick={loadSettings}
          disabled={isLoading || isSaving}
          class="flex items-center gap-1.5 px-3 py-1.5 text-xs text-secondary-theme hover:text-primary-theme rounded-lg hover:bg-surface-hover transition-colors border border-theme-default cursor-pointer disabled:opacity-50"
        >
          <RefreshCw size={13} class={isLoading ? "animate-spin" : ""} />
          <span>Reload</span>
        </button>

        <div class="flex items-center gap-2">
          <button
            type="button"
            onclick={onClose}
            class="px-3 py-1.5 text-xs text-secondary-theme hover:text-primary-theme rounded-lg hover:bg-surface-hover transition-colors border border-theme-default cursor-pointer"
          >
            Cancel
          </button>

          <button
            type="button"
            onclick={activeTab === "raw" ? handleSaveRawJson : handleSaveStructured}
            disabled={isLoading || isSaving}
            class="flex items-center gap-1.5 px-4 py-1.5 text-xs font-semibold text-white rounded-lg transition-all shadow-md cursor-pointer disabled:opacity-50"
            style="background: var(--accent-gradient);"
          >
            {#if isSaving}
              <RefreshCw size={13} class="animate-spin" />
              <span>Saving...</span>
            {:else}
              <Save size={13} />
              <span>Save Settings</span>
            {/if}
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}
