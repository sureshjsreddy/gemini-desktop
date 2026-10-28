<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import type { GlobalCliSettings, SaveGlobalCliSettingsPayload } from "$lib/types";
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
  let settingsFilePath = $state("");
  let chatCompressionEnabled = $state(false);
  let compressionThresholdPercent = $state(75);
  let maxSessionTurns = $state(-1);
  let respectGitIgnore = $state(true);
  let respectGeminiIgnore = $state(true);
  let retentionEnabled = $state(true);
  let retentionMaxAge = $state("30d");
  let rawJson = $state("");

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
      const res = await invoke<GlobalCliSettings>("get_global_cli_settings");
      settingsFilePath = res.filePath;
      chatCompressionEnabled = res.chatCompression.enabled;
      compressionThresholdPercent = Math.round(res.chatCompression.contextPercentageThreshold * 100);
      maxSessionTurns = res.maxSessionTurns;
      respectGitIgnore = res.fileFiltering.respectGitIgnore;
      respectGeminiIgnore = res.fileFiltering.respectGeminiIgnore;
      retentionEnabled = res.sessionRetention.enabled;
      retentionMaxAge = res.sessionRetention.maxAge;
      rawJson = res.rawJson;
    } catch (e: any) {
      errorMessage = typeof e === "string" ? e : e?.message || "Failed to load global CLI settings.";
    } finally {
      isLoading = false;
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
              Gemini CLI Global Settings
            </h2>
            <p class="text-xs text-muted-theme mt-0.5">
              Configure session compaction, turn limits, and token retention policies in <code class="font-mono text-secondary-theme">~/.gemini/settings.json</code>.
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
