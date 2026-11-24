<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import type { AgyPluginInfo, AgyPluginOpResult, CreateCustomPluginRequest, AkboPoolConfig } from "$lib/types";
  import { dialogManager } from "$lib/dialog.svelte";
  import {
    X,
    Plus,
    Trash2,
    Check,
    Folder,
    RefreshCw,
    Package,
    Search,
    Sliders,
    Sparkles,
    BookOpen,
    Server,
    Shield,
    Bot,
    ExternalLink,
    Download,
    CheckCircle2,
    AlertCircle,
  } from "lucide-svelte";
  import Tooltip from "$lib/components/Tooltip.svelte";

  let {
    isOpen = false,
    activeEngine = "gemini",
    onClose,
  }: {
    isOpen: boolean;
    activeEngine?: string;
    onClose: () => void;
  } = $props();

  let activeTab: "installed" | "catalog" | "create" = $state("installed");
  let isLoading = $state(false);
  let isSubmitting = $state(false);
  let statusMessage = $state<{ text: string; type: "success" | "error" | "info" } | null>(null);
  let plugins = $state<AgyPluginInfo[]>([]);
  let searchQuery = $state("");

  // Catalog custom install input
  let customInstallTarget = $state("");

  // AKBO Central Knowledge Pool Configuration state
  let isPoolConfigOpen = $state(false);
  let akboPoolDir = $state("");
  let akboStageOnly = $state(true);
  let isSavingPoolConfig = $state(false);

  // Create plugin form state
  let formName = $state("");
  let formDisplayName = $state("");
  let formDescription = $state("");
  let formIncludeSkill = $state(true);
  let formSkillName = $state("workflow");
  let formIncludeMcp = $state(true);
  let formIncludeRules = $state(true);
  let formIncludeAgent = $state(false);
  let formAgentName = $state("specialist");

  // Curated catalog recommendations
  const CURATED_CATALOG = [
    {
      id: "akbo",
      name: "AKBO Knowledge Oracle & Central Memory Pool",
      category: "Knowledge & Memory",
      description: "AST Codebase Knowledge Graph (CKG), centralized cross-project memory pooling, and autonomous post-turn consolidation ('dreaming').",
      skills: ["akbo"],
    },
    {
      id: "android-cli-plugin",
      name: "Android CLI & Device Hub",
      category: "Mobile",
      description: "Manage Android Virtual Devices, mirror screens, render Jetpack Compose previews, and interact via ADB.",
      skills: ["android-cli", "android-device-hub", "compose-preview"],
    },
    {
      id: "chrome-devtools-plugin",
      name: "Chrome DevTools Automation",
      category: "Web & QA",
      description: "Reliable browser automation, in-depth debugging, screenshot capture, and performance inspection via Puppeteer.",
      skills: ["chrome-devtools"],
    },
    {
      id: "firebase",
      name: "Firebase Cloud & AI",
      category: "Cloud Backend",
      description: "Manage Firebase Auth, Firestore, Cloud Functions, and prototype cloud-native agent applications.",
      skills: ["firebase-core"],
    },
    {
      id: "science",
      name: "Science & Computational Biology",
      category: "Research",
      description: "Comprehensive suite for AlphaFold 3, ChEMBL, PubChem, PubMed, PyMOL, and biomolecular analysis.",
      skills: ["alphafold", "chembl", "pymol", "pubmed"],
    },
    {
      id: "google-antigravity-sdk",
      name: "Google Antigravity SDK",
      category: "Developer Tools",
      description: "Core Python SDK tools, sidecar management, and environment utilities for building agent extensions.",
      skills: ["antigravity-sdk"],
    },
  ];

  let filteredPlugins = $derived.by(() => {
    const q = searchQuery.trim().toLowerCase();
    if (!q) return plugins;
    return plugins.filter((p) => {
      const matchName = p.name.toLowerCase().includes(q) || (p.displayName && p.displayName.toLowerCase().includes(q));
      const matchDesc = p.description && p.description.toLowerCase().includes(q);
      const matchSkill = p.skills && p.skills.some((s) => s.toLowerCase().includes(q));
      return matchName || matchDesc || matchSkill;
    });
  });

  async function loadPlugins() {
    if (activeEngine !== "agy") return;
    isLoading = true;
    statusMessage = null;
    try {
      plugins = await invoke<AgyPluginInfo[]>("list_agy_plugins");
    } catch (e: any) {
      console.error("Failed to list plugins:", e);
      statusMessage = { text: `Failed to load plugins: ${e}`, type: "error" };
    } finally {
      isLoading = false;
    }
  }

  async function loadAkboPoolConfig() {
    try {
      const cfg = await invoke<AkboPoolConfig>("get_akbo_pool_config");
      akboPoolDir = cfg.centralPoolDir;
      akboStageOnly = cfg.stageOnly;
    } catch (e: any) {
      console.error("Failed to load AKBO pool config:", e);
    }
  }

  async function handleSaveAkboPoolConfig() {
    if (!akboPoolDir.trim()) {
      statusMessage = { text: "Central Knowledge Pool directory path cannot be empty", type: "error" };
      return;
    }
    isSavingPoolConfig = true;
    try {
      const res = await invoke<AgyPluginOpResult>("save_akbo_pool_config", {
        config: {
          centralPoolDir: akboPoolDir.trim(),
          stageOnly: akboStageOnly,
        },
      });
      if (res.success) {
        statusMessage = { text: res.message, type: "success" };
        isPoolConfigOpen = false;
      } else {
        statusMessage = { text: res.message, type: "error" };
      }
    } catch (e: any) {
      statusMessage = { text: `Failed to save AKBO pool config: ${e}`, type: "error" };
    } finally {
      isSavingPoolConfig = false;
    }
  }

  $effect(() => {
    if (isOpen) {
      loadPlugins();
      loadAkboPoolConfig();
    }
  });

  async function handleTogglePlugin(plugin: AgyPluginInfo) {
    isSubmitting = true;
    statusMessage = null;
    const targetState = !plugin.enabled;
    try {
      const res = await invoke<AgyPluginOpResult>("toggle_agy_plugin", {
        name: plugin.name,
        enabled: targetState,
      });
      if (res.success) {
        statusMessage = { text: res.message, type: "success" };
        plugin.enabled = targetState;
      } else {
        statusMessage = { text: res.message, type: "error" };
      }
    } catch (e: any) {
      statusMessage = { text: `Error updating plugin: ${e}`, type: "error" };
    } finally {
      isSubmitting = false;
    }
  }

  async function handleUninstall(plugin: AgyPluginInfo) {
    const confirmed = await dialogManager.confirm(
      `Are you sure you want to uninstall plugin "${plugin.displayName || plugin.name}"? This removes the plugin files from disk.`,
      { title: "Uninstall Plugin", isDestructive: true, confirmText: "Uninstall" }
    );
    if (!confirmed) return;

    isSubmitting = true;
    statusMessage = null;
    try {
      const res = await invoke<AgyPluginOpResult>("uninstall_agy_plugin", {
        name: plugin.name,
      });
      if (res.success) {
        statusMessage = { text: res.message, type: "success" };
        await loadPlugins();
      } else {
        statusMessage = { text: res.message, type: "error" };
      }
    } catch (e: any) {
      statusMessage = { text: `Error uninstalling plugin: ${e}`, type: "error" };
    } finally {
      isSubmitting = false;
    }
  }

  async function handleInstallTarget(target: string) {
    if (!target.trim()) return;
    isSubmitting = true;
    statusMessage = { text: `Installing plugin "${target}"...`, type: "info" };
    try {
      const res = await invoke<AgyPluginOpResult>("install_agy_plugin", {
        target: target.trim(),
      });
      if (res.success) {
        statusMessage = { text: res.message, type: "success" };
        customInstallTarget = "";
        await loadPlugins();
        activeTab = "installed";
      } else {
        statusMessage = { text: res.message, type: "error" };
      }
    } catch (e: any) {
      statusMessage = { text: `Error installing plugin: ${e}`, type: "error" };
    } finally {
      isSubmitting = false;
    }
  }

  async function handleOpenDir(path: string | undefined) {
    if (!path) return;
    try {
      await invoke("open_plugin_directory", { pluginPath: path });
    } catch (e: any) {
      statusMessage = { text: `Failed to open directory: ${e}`, type: "error" };
    }
  }

  async function handleCreatePlugin() {
    const name = formName.trim();
    if (!name) {
      statusMessage = { text: "Plugin ID cannot be empty", type: "error" };
      return;
    }

    isSubmitting = true;
    statusMessage = { text: "Scaffolding custom plugin...", type: "info" };

    const payload: CreateCustomPluginRequest = {
      name,
      displayName: formDisplayName.trim() || undefined,
      description: formDescription.trim() || undefined,
      includeSkill: formIncludeSkill,
      skillName: formIncludeSkill ? formSkillName.trim() || "workflow" : undefined,
      includeMcp: formIncludeMcp,
      includeRules: formIncludeRules,
      includeAgent: formIncludeAgent,
      agentName: formIncludeAgent ? formAgentName.trim() || "assistant" : undefined,
    };

    try {
      const res = await invoke<AgyPluginOpResult>("create_custom_plugin", { req: payload });
      if (res.success) {
        statusMessage = { text: res.message, type: "success" };
        formName = "";
        formDisplayName = "";
        formDescription = "";
        await loadPlugins();
        activeTab = "installed";
      } else {
        statusMessage = { text: res.message, type: "error" };
      }
    } catch (e: any) {
      statusMessage = { text: `Failed to create plugin: ${e}`, type: "error" };
    } finally {
      isSubmitting = false;
    }
  }
</script>

{#if isOpen}
  <!-- Backdrop -->
  <div
    class="fixed inset-0 bg-black/60 backdrop-blur-xs z-50 flex items-center justify-center p-4 animate-in fade-in duration-200"
    role="presentation"
    onclick={onClose}
    onkeydown={(e) => e.key === "Escape" && onClose()}
  >
    <!-- Modal Card -->
    <div
      role="dialog"
      aria-modal="true"
      tabindex="-1"
      class="w-full max-w-4xl bg-surface-elevated border border-theme-default rounded-2xl shadow-2xl flex flex-col max-h-[88vh] overflow-hidden animate-in zoom-in-95 duration-150"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
    >
      <!-- Modal Header -->
      <div class="px-6 py-4 border-b border-subtle flex items-center justify-between bg-surface/50">
        <div class="flex items-center gap-3">
          <div
            class="w-9 h-9 rounded-xl flex items-center justify-center text-white shadow-md shadow-black/20 shrink-0"
            style="background: var(--accent-gradient);"
          >
            <Package size={18} />
          </div>
          <div>
            <div class="flex items-center gap-2">
              <h2 class="text-sm font-semibold text-primary-theme">Plugins & Extensions Hub</h2>
              {#if activeEngine === "agy"}
                <span class="text-[10px] uppercase font-mono font-bold px-2 py-0.5 rounded bg-purple-500/15 text-purple-400 border border-purple-500/30">
                  Antigravity (agy)
                </span>
              {:else}
                <span class="text-[10px] uppercase font-mono font-bold px-2 py-0.5 rounded bg-blue-500/15 text-blue-400 border border-blue-500/30">
                  Gemini CLI
                </span>
              {/if}
            </div>
            <p class="text-xs text-secondary-theme">Modular bundles of skills, MCP tools, agent rules, and specialized subagents.</p>
          </div>
        </div>

        <div class="flex items-center gap-2">
          {#if activeEngine === "agy"}
            <Tooltip text="Refresh Plugins" position="bottom">
              <button
                onclick={loadPlugins}
                disabled={isLoading}
                class="p-1.5 text-secondary-theme hover:text-primary-theme hover:bg-surface rounded-lg transition-colors cursor-pointer disabled:opacity-50"
                aria-label="Refresh plugins"
              >
                <RefreshCw size={15} class={isLoading ? "animate-spin" : ""} />
              </button>
            </Tooltip>
          {/if}
          <Tooltip text="Close Modal" position="bottom">
            <button
              onclick={onClose}
              class="p-1.5 text-secondary-theme hover:text-primary-theme hover:bg-surface rounded-lg transition-colors cursor-pointer"
              aria-label="Close modal"
            >
              <X size={16} />
            </button>
          </Tooltip>
        </div>
      </div>

      <!-- Gemini CLI Mode Informative Banner -->
      {#if activeEngine === "gemini"}
        <div class="p-6 overflow-y-auto">
          <div class="p-5 rounded-2xl border border-sky-500/30 bg-sky-500/10 flex items-start gap-3.5">
            <Sparkles size={22} class="text-sky-400 shrink-0 mt-0.5" />
            <div class="text-xs space-y-2">
              <div class="font-semibold text-sm text-sky-200">Antigravity CLI Agentic Feature</div>
              <p class="text-secondary-theme leading-relaxed">
                Plugins and extension bundles are a native capability of the Antigravity CLI (<code class="text-sky-300 font-mono">agy</code>). The standard Gemini CLI engine does not support this modular plugin architecture.
              </p>
              <p class="text-muted-theme text-[11px] leading-relaxed">
                To explore, install, or create custom plugins, switch your active CLI agent engine to <span class="text-accent-theme font-semibold">Antigravity CLI (agy)</span> using the engine selector in the bottom left footer or in CLI Settings.
              </p>
            </div>
          </div>
        </div>
      {:else}
        <!-- Navigation Tab Strip (agy mode) -->
        <div class="px-6 py-2.5 border-b border-subtle bg-surface-elevated/40 flex items-center justify-between gap-4">
          <div class="flex items-center gap-1.5 p-0.5 bg-app rounded-lg border border-subtle">
            <button
              onclick={() => (activeTab = "installed")}
              class="flex items-center gap-1.5 px-3.5 py-1.5 rounded-md text-xs font-medium transition-all cursor-pointer {activeTab === 'installed' ? 'bg-surface-elevated text-accent-theme shadow-xs border border-subtle' : 'text-secondary-theme hover:text-primary-theme'}"
            >
              <Package size={13} />
              <span>Installed ({plugins.length})</span>
            </button>
            <button
              onclick={() => (activeTab = "catalog")}
              class="flex items-center gap-1.5 px-3.5 py-1.5 rounded-md text-xs font-medium transition-all cursor-pointer {activeTab === 'catalog' ? 'bg-surface-elevated text-accent-theme shadow-xs border border-subtle' : 'text-secondary-theme hover:text-primary-theme'}"
            >
              <Download size={13} />
              <span>Explore Catalog</span>
            </button>
            <button
              onclick={() => (activeTab = "create")}
              class="flex items-center gap-1.5 px-3.5 py-1.5 rounded-md text-xs font-medium transition-all cursor-pointer {activeTab === 'create' ? 'bg-surface-elevated text-accent-theme shadow-xs border border-subtle' : 'text-secondary-theme hover:text-primary-theme'}"
            >
              <Plus size={13} />
              <span>+ Create Custom</span>
            </button>
          </div>

          {#if activeTab === "installed"}
            <div class="relative w-64">
              <Search size={13} class="absolute left-2.5 top-1/2 -translate-y-1/2 text-muted-theme" />
              <input
                type="text"
                bind:value={searchQuery}
                placeholder="Search plugins & skills..."
                class="w-full pl-8 pr-3 py-1 rounded-lg bg-surface border border-theme-default text-xs text-primary-theme placeholder:text-muted-theme focus:outline-hidden focus:border-accent-theme/60 transition-colors"
              />
            </div>
          {/if}
        </div>

        <!-- Status Message Banner -->
        {#if statusMessage}
          <div
            class="px-6 py-2 text-xs flex items-center justify-between border-b {statusMessage.type === 'success' ? 'bg-emerald-500/10 text-emerald-300 border-emerald-500/20' : statusMessage.type === 'error' ? 'bg-rose-500/10 text-rose-300 border-rose-500/20' : 'bg-sky-500/10 text-sky-300 border-sky-500/20'}"
          >
            <span>{statusMessage.text}</span>
            <button onclick={() => (statusMessage = null)} class="opacity-70 hover:opacity-100 cursor-pointer">
              <X size={12} />
            </button>
          </div>
        {/if}

        <!-- Tab Body Content -->
        <div class="flex-1 overflow-y-auto p-6 space-y-4">
          <!-- TAB 1: INSTALLED PLUGINS -->
          {#if activeTab === "installed"}
            {#if isLoading}
              <div class="py-16 flex flex-col items-center justify-center text-secondary-theme gap-2">
                <RefreshCw size={24} class="animate-spin text-accent-theme" />
                <span class="text-xs">Loading installed plugins...</span>
              </div>
            {:else if filteredPlugins.length === 0}
              <div class="py-16 text-center text-muted-theme space-y-2">
                <Package size={32} class="mx-auto opacity-40 mb-1" />
                <div class="text-sm font-medium text-secondary-theme">No plugins found</div>
                <div class="text-xs max-w-sm mx-auto">
                  {searchQuery ? "No installed plugins match your search query." : "No plugins are currently installed in your Antigravity environment."}
                </div>
              </div>
            {:else}
              <div class="grid grid-cols-1 gap-3">
                {#each filteredPlugins as plugin (plugin.name)}
                  <div
                    class="p-4 rounded-xl border border-theme-default bg-surface hover:border-accent-theme/30 transition-all flex flex-col gap-3"
                  >
                    <div class="flex items-start justify-between gap-3">
                      <div class="flex items-start gap-3 min-w-0">
                        <div class="w-10 h-10 rounded-xl bg-surface-elevated border border-subtle flex items-center justify-center text-accent-theme font-bold font-mono text-sm shrink-0 shadow-2xs">
                          {#if plugin.logo}
                            <img src={plugin.logo} alt={plugin.name} class="w-6 h-6 object-contain" />
                          {:else}
                            {plugin.name.slice(0, 2).toUpperCase()}
                          {/if}
                        </div>
                        <div class="min-w-0">
                          <div class="flex items-center gap-2">
                            <span class="font-semibold text-xs text-primary-theme truncate">
                              {plugin.displayName || plugin.name}
                            </span>
                            <span class="text-[10px] font-mono text-muted-theme">
                              {plugin.name}
                            </span>
                            {#if plugin.version}
                              <span class="text-[10px] font-mono px-1.5 py-0.2 rounded bg-surface-elevated border border-subtle text-secondary-theme">
                                v{plugin.version}
                              </span>
                            {/if}
                            {#if plugin.source}
                              <span class="text-[9px] uppercase tracking-wider font-semibold px-1.5 py-0.2 rounded bg-accent-theme/10 text-accent-theme border border-accent-theme/20">
                                {plugin.source}
                              </span>
                            {/if}
                          </div>
                          <p class="text-xs text-secondary-theme mt-1 leading-relaxed line-clamp-2">
                            {plugin.description || "No description provided."}
                          </p>
                        </div>
                      </div>

                      <!-- Enable / Disable Switch -->
                      <div class="flex items-center gap-2 shrink-0">
                        <span class="text-[11px] font-medium {plugin.enabled ? 'text-emerald-400' : 'text-muted-theme'}">
                          {plugin.enabled ? "Active" : "Disabled"}
                        </span>
                        <button
                          type="button"
                          role="switch"
                          aria-label={plugin.enabled ? `Disable ${plugin.displayName || plugin.name}` : `Enable ${plugin.displayName || plugin.name}`}
                          aria-checked={plugin.enabled}
                          disabled={isSubmitting}
                          onclick={() => handleTogglePlugin(plugin)}
                          class="w-9 h-5 rounded-full transition-colors relative cursor-pointer disabled:opacity-50 {plugin.enabled ? 'bg-accent-theme' : 'bg-surface-elevated border border-subtle'}"
                        >
                          <span
                            class="absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white transition-transform {plugin.enabled ? 'translate-x-4' : 'translate-x-0'}"
                          ></span>
                        </button>
                      </div>
                    </div>

                    <!-- Component Pills & Actions -->
                    <div class="pt-2 border-t border-subtle/60 flex flex-wrap items-center justify-between gap-2 text-[11px]">
                      <div class="flex flex-wrap items-center gap-2">
                        {#if plugin.skills && plugin.skills.length > 0}
                          <Tooltip text={`Skills: ${plugin.skills.join(', ')}`} position="top">
                            <span class="flex items-center gap-1 px-2 py-0.5 rounded-md bg-surface-elevated text-secondary-theme border border-subtle">
                              <BookOpen size={11} class="text-accent-theme" />
                              <span>{plugin.skills.length} {plugin.skills.length === 1 ? 'Skill' : 'Skills'}</span>
                            </span>
                          </Tooltip>
                        {/if}
                        {#if plugin.mcpServers && plugin.mcpServers.length > 0}
                          <Tooltip text={`MCP: ${plugin.mcpServers.join(', ')}`} position="top">
                            <span class="flex items-center gap-1 px-2 py-0.5 rounded-md bg-surface-elevated text-secondary-theme border border-subtle">
                              <Server size={11} class="text-emerald-400" />
                              <span>{plugin.mcpServers.length} MCP</span>
                            </span>
                          </Tooltip>
                        {/if}
                        {#if plugin.rules && plugin.rules.length > 0}
                          <span class="flex items-center gap-1 px-2 py-0.5 rounded-md bg-surface-elevated text-secondary-theme border border-subtle">
                            <Shield size={11} class="text-sky-400" />
                            <span>Rules</span>
                          </span>
                        {/if}
                      </div>

                      <div class="flex items-center gap-1.5 ml-auto">
                        {#if plugin.name === "akbo"}
                          <Tooltip text="Configure Central Knowledge Pool directory & dreaming" position="top">
                            <button
                              onclick={() => {
                                loadAkboPoolConfig();
                                isPoolConfigOpen = !isPoolConfigOpen;
                              }}
                              class="flex items-center gap-1 px-2 py-1 rounded bg-accent-theme/10 hover:bg-accent-theme/20 text-accent-theme border border-accent-theme/30 transition-colors cursor-pointer text-xs"
                            >
                              <Sliders size={12} />
                              <span>Configure Pool</span>
                            </button>
                          </Tooltip>
                        {/if}
                        {#if plugin.path}
                          <Tooltip text="Reveal folder in Windows File Explorer" position="top">
                            <button
                              onclick={() => handleOpenDir(plugin.path)}
                              class="flex items-center gap-1 px-2 py-1 rounded hover:bg-surface-elevated text-secondary-theme hover:text-primary-theme transition-colors cursor-pointer"
                            >
                              <Folder size={12} />
                              <span>Folder</span>
                            </button>
                          </Tooltip>
                        {/if}
                        {#if plugin.source !== "antigravity-plugins-official"}
                          <Tooltip text="Uninstall plugin" position="top">
                            <button
                              onclick={() => handleUninstall(plugin)}
                              disabled={isSubmitting}
                              class="p-1 hover:text-rose-400 text-muted-theme rounded hover:bg-surface-elevated transition-colors cursor-pointer disabled:opacity-50"
                            >
                              <Trash2 size={13} />
                            </button>
                          </Tooltip>
                        {/if}
                      </div>
                    </div>

                    {#if plugin.name === "akbo" && isPoolConfigOpen}
                      <div class="mt-2 p-3.5 rounded-xl bg-surface-elevated/70 border border-subtle space-y-3">
                        <div class="flex items-center justify-between">
                          <span class="font-semibold text-xs text-primary-theme flex items-center gap-1.5">
                            <Sliders size={13} class="text-accent-theme" />
                            Central Knowledge Pool Configuration
                          </span>
                          <button
                            onclick={() => (isPoolConfigOpen = false)}
                            class="text-muted-theme hover:text-primary-theme cursor-pointer"
                          >
                            <X size={12} />
                          </button>
                        </div>

                        <div>
                          <label for="akbo-pool-dir-input" class="block text-[11px] font-medium text-secondary-theme mb-1">
                            Central Pool Directory (mirrors <code class="text-accent-theme">projects/&lt;project-name&gt;/</code>)
                          </label>
                          <input
                            id="akbo-pool-dir-input"
                            type="text"
                            bind:value={akboPoolDir}
                            placeholder="e.g. C:\Users\user\.akb\central-pool"
                            class="w-full px-2.5 py-1.5 rounded-md bg-surface border border-theme-default text-xs text-primary-theme placeholder:text-muted-theme focus:outline-hidden focus:border-accent-theme/60 font-mono transition-colors"
                          />
                        </div>

                        <div class="flex items-center justify-between pt-1">
                          <div class="text-[11px]">
                            <span class="font-medium text-primary-theme">Review Proposals (<code class="text-xs">--stage-only</code>)</span>
                            <p class="text-muted-theme text-[10px]">Autonomous dreaming stages changes for your manual review before updating ADRs.</p>
                          </div>
                          <button
                            type="button"
                            role="switch"
                            aria-label="Toggle stage-only mode"
                            aria-checked={akboStageOnly}
                            onclick={() => (akboStageOnly = !akboStageOnly)}
                            class="w-8 h-4.5 rounded-full transition-colors relative cursor-pointer {akboStageOnly ? 'bg-accent-theme' : 'bg-surface border border-subtle'}"
                          >
                            <span
                              class="absolute top-0.5 left-0.5 w-3.5 h-3.5 rounded-full bg-white transition-transform {akboStageOnly ? 'translate-x-3.5' : 'translate-x-0'}"
                            ></span>
                          </button>
                        </div>

                        <div class="flex justify-end pt-1">
                          <button
                            onclick={handleSaveAkboPoolConfig}
                            disabled={isSavingPoolConfig}
                            class="px-3 py-1.5 rounded-lg bg-accent-theme text-white text-xs font-medium hover:opacity-90 transition-opacity cursor-pointer disabled:opacity-50 flex items-center gap-1.5"
                          >
                            <Check size={12} />
                            <span>Save Pool Config</span>
                          </button>
                        </div>
                      </div>
                    {/if}
                  </div>
                {/each}
              </div>
            {/if}

          <!-- TAB 2: EXPLORE CATALOG -->
          {:else if activeTab === "catalog"}
            <div class="space-y-4">
              <!-- Custom Install Bar -->
              <div class="p-4 rounded-xl border border-subtle bg-surface/60 flex items-center gap-3">
                <input
                  type="text"
                  bind:value={customInstallTarget}
                  placeholder="Install plugin by ID or GitHub repo (e.g. 'linear' or 'user/plugin-name')..."
                  class="flex-1 px-3 py-2 rounded-lg bg-surface border border-theme-default text-xs text-primary-theme placeholder:text-muted-theme focus:outline-hidden focus:border-accent-theme/60 transition-colors"
                />
                <button
                  onclick={() => handleInstallTarget(customInstallTarget)}
                  disabled={isSubmitting || !customInstallTarget.trim()}
                  class="px-4 py-2 rounded-lg bg-accent-theme text-white text-xs font-semibold hover:opacity-90 transition-opacity cursor-pointer disabled:opacity-50 flex items-center gap-1.5 shrink-0"
                >
                  <Download size={13} />
                  <span>Install</span>
                </button>
              </div>

              <!-- Curated Showcase List -->
              <div class="text-[11px] font-semibold uppercase text-muted-theme tracking-wider px-1">
                Featured Official & Partner Plugins
              </div>

              <div class="grid grid-cols-1 gap-3">
                {#each CURATED_CATALOG as item (item.id)}
                  {@const isInstalled = plugins.some((p) => p.name === item.id)}
                  <div class="p-4 rounded-xl border border-theme-default bg-surface flex items-start justify-between gap-4">
                    <div class="space-y-1 min-w-0">
                      <div class="flex items-center gap-2">
                        <span class="font-semibold text-xs text-primary-theme">{item.name}</span>
                        <span class="text-[10px] font-mono text-muted-theme">({item.id})</span>
                        <span class="text-[9px] uppercase tracking-wider px-1.5 py-0.2 rounded bg-surface-elevated border border-subtle text-secondary-theme">
                          {item.category}
                        </span>
                      </div>
                      <p class="text-xs text-secondary-theme leading-relaxed">
                        {item.description}
                      </p>
                      <div class="flex items-center gap-1.5 text-[10px] text-muted-theme pt-1">
                        <span>Skills included:</span>
                        {#each item.skills as s}
                          <span class="px-1.5 py-0.2 rounded bg-surface-elevated font-mono text-[10px]">{s}</span>
                        {/each}
                      </div>
                    </div>

                    <div class="shrink-0">
                      {#if isInstalled}
                        <span class="flex items-center gap-1 px-3 py-1.5 rounded-lg bg-emerald-500/10 border border-emerald-500/30 text-emerald-300 text-xs font-medium">
                          <CheckCircle2 size={13} />
                          <span>Installed</span>
                        </span>
                      {:else}
                        <button
                          onclick={() => handleInstallTarget(item.id)}
                          disabled={isSubmitting}
                          class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-surface hover:bg-surface-hover border border-subtle hover:border-accent-theme/40 text-xs font-medium text-primary-theme transition-colors cursor-pointer disabled:opacity-50"
                        >
                          <Download size={13} class="text-accent-theme" />
                          <span>Install</span>
                        </button>
                      {/if}
                    </div>
                  </div>
                {/each}
              </div>
            </div>

          <!-- TAB 3: CREATE CUSTOM PLUGIN -->
          {:else if activeTab === "create"}
            <div class="space-y-4 max-w-2xl">
              <div class="p-4 rounded-xl border border-accent-theme/20 bg-accent-theme/5 flex items-start gap-3">
                <Sparkles size={18} class="text-accent-theme shrink-0 mt-0.5" />
                <div class="text-xs space-y-1">
                  <div class="font-semibold text-primary-theme">Create Your Custom Plugin Bundle</div>
                  <p class="text-secondary-theme leading-relaxed">
                    Packages skills, MCP servers, agent rules, and subagents into a unified bundle in <code class="text-accent-theme font-mono">%USERPROFILE%\.gemini\config\plugins\&lt;name&gt;</code>.
                  </p>
                </div>
              </div>

              <div class="space-y-3 text-xs">
                <div>
                  <label for="plugin-name-input" class="block font-medium text-primary-theme mb-1">
                    Plugin Identifier (Directory Name) *
                  </label>
                  <input
                    id="plugin-name-input"
                    type="text"
                    bind:value={formName}
                    placeholder="e.g. database-tools or my-team-assistant"
                    class="w-full px-3 py-2 rounded-lg bg-surface border border-theme-default text-xs text-primary-theme placeholder:text-muted-theme focus:outline-hidden focus:border-accent-theme/60 transition-colors font-mono"
                  />
                  <span class="text-[10px] text-muted-theme">Lower-case alphanumeric with hyphens.</span>
                </div>

                <div>
                  <label for="plugin-display-name-input" class="block font-medium text-primary-theme mb-1">
                    Display Name
                  </label>
                  <input
                    id="plugin-display-name-input"
                    type="text"
                    bind:value={formDisplayName}
                    placeholder="e.g. Database Tools"
                    class="w-full px-3 py-2 rounded-lg bg-surface border border-theme-default text-xs text-primary-theme placeholder:text-muted-theme focus:outline-hidden focus:border-accent-theme/60 transition-colors"
                  />
                </div>

                <div>
                  <label for="plugin-desc-input" class="block font-medium text-primary-theme mb-1">
                    Description
                  </label>
                  <textarea
                    id="plugin-desc-input"
                    bind:value={formDescription}
                    rows="2"
                    placeholder="What does this plugin do? (e.g. Custom database schemas, migration runners, and SQL review rules)"
                    class="w-full px-3 py-2 rounded-lg bg-surface border border-theme-default text-xs text-primary-theme placeholder:text-muted-theme focus:outline-hidden focus:border-accent-theme/60 transition-colors"
                  ></textarea>
                </div>

                <!-- Capabilities Selection -->
                <div class="pt-2 border-t border-subtle space-y-2">
                  <div class="font-semibold text-primary-theme">Select Bundle Components to Scaffold:</div>

                  <label class="flex items-center gap-2 cursor-pointer select-none">
                    <input type="checkbox" bind:checked={formIncludeSkill} class="rounded text-accent-theme" />
                    <span>Include Custom Skill (<code class="font-mono text-[11px]">skills/&lt;skill&gt;/SKILL.md</code>)</span>
                  </label>
                  {#if formIncludeSkill}
                    <div class="ml-6">
                      <input
                        type="text"
                        bind:value={formSkillName}
                        placeholder="Skill Name (e.g. db-migration)"
                        class="px-2.5 py-1.5 rounded-lg bg-surface border border-theme-default text-xs text-primary-theme w-64 font-mono"
                      />
                    </div>
                  {/if}

                  <label class="flex items-center gap-2 cursor-pointer select-none">
                    <input type="checkbox" bind:checked={formIncludeMcp} class="rounded text-accent-theme" />
                    <span>Include Local MCP Server Config (<code class="font-mono text-[11px]">mcp_config.json</code>)</span>
                  </label>

                  <label class="flex items-center gap-2 cursor-pointer select-none">
                    <input type="checkbox" bind:checked={formIncludeRules} class="rounded text-accent-theme" />
                    <span>Include Agent Rules & Directives (<code class="font-mono text-[11px]">rules/AGENTS.md</code>)</span>
                  </label>

                  <label class="flex items-center gap-2 cursor-pointer select-none">
                    <input type="checkbox" bind:checked={formIncludeAgent} class="rounded text-accent-theme" />
                    <span>Include Specialist Subagent (<code class="font-mono text-[11px]">agents/&lt;role&gt;.md</code>)</span>
                  </label>
                  {#if formIncludeAgent}
                    <div class="ml-6">
                      <input
                        type="text"
                        bind:value={formAgentName}
                        placeholder="Agent Name (e.g. reviewer)"
                        class="px-2.5 py-1.5 rounded-lg bg-surface border border-theme-default text-xs text-primary-theme w-64 font-mono"
                      />
                    </div>
                  {/if}
                </div>

                <div class="pt-4 flex items-center justify-end gap-2">
                  <button
                    onclick={handleCreatePlugin}
                    disabled={isSubmitting || !formName.trim()}
                    class="px-5 py-2.5 rounded-xl bg-accent-theme text-white text-xs font-semibold hover:opacity-90 transition-opacity cursor-pointer disabled:opacity-50 flex items-center gap-2 shadow-sm"
                  >
                    <Plus size={14} />
                    <span>Scaffold & Enable Plugin</span>
                  </button>
                </div>
              </div>
            </div>
          {/if}
        </div>
      {/if}
    </div>
  </div>
{/if}
