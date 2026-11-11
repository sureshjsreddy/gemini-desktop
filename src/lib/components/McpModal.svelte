<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import type { Workspace, McpServerConfig, McpConfigResponse } from "$lib/types";
  import { dialogManager } from "$lib/dialog.svelte";
  import {
    X,
    Plus,
    Trash2,
    Check,
    Server,
    Code,
    Sparkles,
    Folder,
    Globe,
    FileText,
    AlertCircle,
    Info,
    RefreshCw,
  } from "lucide-svelte";
  import Tooltip from "$lib/components/Tooltip.svelte";

  let {
    isOpen = false,
    activeWorkspace = null,
    activeEngine = "gemini",
    onClose,
  }: {
    isOpen: boolean;
    activeWorkspace: Workspace | null;
    activeEngine?: string;
    onClose: () => void;
  } = $props();

  // Scopes: "workspace" or "global"
  let activeScope: "workspace" | "global" = $state("workspace");
  // Edit mode: "form" or "json"
  let editMode: "form" | "json" = $state("form");

  let isLoading = $state(false);
  let statusMessage = $state<{ text: string; type: "success" | "error" | "info" } | null>(null);
  let currentFilePath = $state("");
  let importableServers = $state<string[]>([]);
  let isImporting = $state(false);

  // Map of serverId -> McpServerConfig
  let servers = $state<Record<string, McpServerConfig>>({});
  let selectedServerKey = $state<string>("");

  // Form fields for editing
  let formServerKey = $state("");
  let formCommand = $state("");
  let formArgsText = $state("");
  let formEnvList = $state<{ key: string; value: string }[]>([]);
  let formCwd = $state("");
  let formDisabled = $state(false);

  // Raw JSON editor state
  let rawJsonText = $state("");

  interface Preset {
    id: string;
    label: string;
    description: string;
    icon: string;
    config: {
      key: string;
      command: string;
      args: string[];
      env: Record<string, string>;
      cwd?: string;
    };
  }

  let PRESETS: Preset[] = $derived.by(() => {
    const list: Preset[] = [
      {
      id: "github-docker",
      label: "GitHub (Docker)",
      description: "Official GitHub MCP server running inside Docker",
      icon: "🐙",
      config: {
        key: "github",
        command: "docker",
        args: [
          "run",
          "-i",
          "--rm",
          "-e",
          "GITHUB_PERSONAL_ACCESS_TOKEN",
          "ghcr.io/github/github-mcp-server:latest",
        ],
        env: {
          GITHUB_PERSONAL_ACCESS_TOKEN: "${GITHUB_PERSONAL_ACCESS_TOKEN}",
        },
      },
    },
    {
      id: "filesystem",
      label: "Local Filesystem",
      description: "Secure local directory access for Gemini CLI",
      icon: "📁",
      config: {
        key: "filesystem",
        command: "npx",
        args: ["-y", "@modelcontextprotocol/server-filesystem", activeWorkspace?.path || "C:\\projects"],
        env: {},
      },
    },
    {
      id: "memory",
      label: "Knowledge Graph Memory",
      description: "Persistent memory & entities graph across sessions",
      icon: "🧠",
      config: {
        key: "memory",
        command: "npx",
        args: ["-y", "@modelcontextprotocol/server-memory"],
        env: {},
      },
    },
    {
      id: "postgres",
      label: "PostgreSQL Database",
      description: "Inspect schema, run read-only queries on Postgres",
      icon: "🐘",
      config: {
        key: "postgres",
        command: "npx",
        args: ["-y", "@modelcontextprotocol/server-postgres", "postgresql://localhost/mydb"],
        env: {},
      },
    },
    {
      id: "azure-devops",
      label: "Azure DevOps (MCP)",
      description: "Work items, PRs, pipelines, and git repos in Azure DevOps",
      icon: "☁️",
      config: {
        key: "azure-devops",
        command: "npx",
        args: ["-y", "@azure-devops/mcp", "--organization", "https://dev.azure.com/YOUR_ORG"],
        env: {
          AZURE_DEVOPS_EXT_PAT: "${AZURE_DEVOPS_EXT_PAT}",
        },
      },
    },
    {
      id: "brave-search",
      label: "Brave Web Search",
      description: "Real-time web search capabilities via Brave Search API",
      icon: "🔍",
      config: {
        key: "brave-search",
        command: "npx",
        args: ["-y", "@modelcontextprotocol/server-brave-search"],
        env: {
          BRAVE_API_KEY: "${BRAVE_API_KEY}",
        },
      },
    },
  ];
  return list;
});

  // Load config whenever modal opens, scope changes, or active engine changes
  $effect(() => {
    if (isOpen) {
      // Track reactive dependencies
      const _scope = activeScope;
      const _engine = activeEngine;
      loadConfig();
    }
  });

  async function loadConfig() {
    isLoading = true;
    statusMessage = null;
    try {
      const workspacePath = activeScope === "workspace" ? (activeWorkspace?.path || null) : null;
      const res = await invoke<McpConfigResponse>("get_mcp_config", {
        workspacePath,
        engine: activeEngine,
      });

      currentFilePath = res.file_path;
      servers = res.mcp_servers || {};
      rawJsonText = JSON.stringify(servers, null, 2);

      const keys = Object.keys(servers);
      if (keys.length > 0 && (!selectedServerKey || !servers[selectedServerKey])) {
        selectServer(keys[0]);
      } else if (keys.length === 0) {
        startNewServer();
      } else if (selectedServerKey && servers[selectedServerKey]) {
        selectServer(selectedServerKey);
      }

      // Check for importable Gemini CLI servers if Antigravity is active
      if (activeEngine === "agy") {
        try {
          importableServers = await invoke<string[]>("detect_importable_gemini_mcp", {
            workspacePath,
          });
        } catch {
          importableServers = [];
        }
      } else {
        importableServers = [];
      }
    } catch (err: any) {
      statusMessage = { text: `Failed to load settings: ${err}`, type: "error" };
    } finally {
      isLoading = false;
    }
  }

  async function handleImportFromGemini() {
    isImporting = true;
    try {
      const workspacePath = activeScope === "workspace" ? (activeWorkspace?.path || null) : null;
      const res = await invoke<any>("import_mcp_from_gemini", {
        workspacePath,
      });
      if (res.success) {
        statusMessage = { text: res.message, type: "success" };
        await loadConfig();
      } else {
        statusMessage = { text: res.message, type: "error" };
      }
    } catch (err: any) {
      statusMessage = { text: `Import failed: ${err}`, type: "error" };
    } finally {
      isImporting = false;
    }
  }

  function selectServer(key: string) {
    selectedServerKey = key;
    const cfg = servers[key];
    if (!cfg) return;

    formServerKey = key;
    formCommand = cfg.command || "";
    formArgsText = (cfg.args || []).join("\n");
    formCwd = cfg.cwd || "";
    formDisabled = cfg.disabled ?? false;

    const envs: { key: string; value: string }[] = [];
    if (cfg.env) {
      for (const [k, v] of Object.entries(cfg.env)) {
        envs.push({ key: k, value: String(v) });
      }
    }
    formEnvList = envs;
  }

  function startNewServer() {
    selectedServerKey = "";
    formServerKey = "my-mcp-server";
    formCommand = "npx";
    formArgsText = "-y\n@modelcontextprotocol/server-memory";
    formEnvList = [];
    formCwd = "";
    formDisabled = false;
  }

  function applyPreset(preset: Preset) {
    formServerKey = preset.config.key;
    formCommand = preset.config.command;
    formArgsText = preset.config.args.join("\n");
    formCwd = preset.config.cwd || "";
    formDisabled = false;

    const envs: { key: string; value: string }[] = [];
    for (const [k, v] of Object.entries(preset.config.env)) {
      envs.push({ key: k, value: v });
    }
    formEnvList = envs;
    selectedServerKey = "";
    statusMessage = {
      text: `Applied preset "${preset.label}". Review parameters and click Save.`,
      type: "info",
    };
  }

  function addEnvRow() {
    formEnvList = [...formEnvList, { key: "", value: "" }];
  }

  function removeEnvRow(index: number) {
    formEnvList = formEnvList.filter((_, i) => i !== index);
  }

  async function toggleServerDisabled(key: string, e: MouseEvent) {
    e.stopPropagation();
    const updatedServers = { ...servers };
    const current = updatedServers[key];
    if (current) {
      const willDisable = !current.disabled;
      updatedServers[key] = {
        ...current,
        disabled: willDisable ? true : undefined,
      };
      if (selectedServerKey === key) {
        formDisabled = willDisable;
      }
      await persistServers(updatedServers, selectedServerKey);
    }
  }

  async function handleSaveForm() {
    const key = formServerKey.trim();
    if (!key) {
      statusMessage = { text: "Server ID cannot be empty", type: "error" };
      return;
    }
    if (!formCommand.trim()) {
      statusMessage = { text: "Command cannot be empty", type: "error" };
      return;
    }

    const args = formArgsText
      .split("\n")
      .map((a) => a.trim())
      .filter(Boolean);

    const envObj: Record<string, string> = {};
    for (const item of formEnvList) {
      const k = item.key.trim();
      if (k) {
        envObj[k] = item.value;
      }
    }

    const newConfig: McpServerConfig = {
      command: formCommand.trim(),
      args: args.length > 0 ? args : undefined,
      env: Object.keys(envObj).length > 0 ? envObj : undefined,
      cwd: formCwd.trim() ? formCwd.trim() : undefined,
      disabled: formDisabled ? true : undefined,
    };

    // If key was renamed, delete old key
    const updatedServers = { ...servers };
    if (selectedServerKey && selectedServerKey !== key) {
      delete updatedServers[selectedServerKey];
    }
    updatedServers[key] = newConfig;

    await persistServers(updatedServers, key);
  }

  async function handleDeleteServer(key: string) {
    const confirmed = await dialogManager.confirm(`Remove MCP server "${key}" from settings?`, {
      title: "Remove MCP Server",
      confirmText: "Remove",
      isDestructive: true,
    });
    if (!confirmed) return;

    const updatedServers = { ...servers };
    delete updatedServers[key];
    await persistServers(updatedServers, "");
  }

  async function handleSaveJson() {
    try {
      const parsed = JSON.parse(rawJsonText);
      if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) {
        throw new Error("mcpServers configuration must be a JSON object mapping server IDs to configs.");
      }
      await persistServers(parsed, selectedServerKey);
    } catch (err: any) {
      statusMessage = { text: `Invalid JSON: ${err.message}`, type: "error" };
    }
  }

  async function persistServers(newServers: Record<string, McpServerConfig>, selectKeyAfterSave?: string) {
    isLoading = true;
    statusMessage = null;
    try {
      const workspacePath = activeScope === "workspace" ? (activeWorkspace?.path || null) : null;
      const res = await invoke<McpConfigResponse>("save_mcp_config", {
        workspacePath,
        engine: activeEngine,
        mcpServers: newServers,
      });

      servers = res.mcp_servers || {};
      rawJsonText = JSON.stringify(servers, null, 2);
      currentFilePath = res.file_path;

      const keys = Object.keys(servers);
      if (selectKeyAfterSave && servers[selectKeyAfterSave]) {
        selectServer(selectKeyAfterSave);
      } else if (keys.length > 0) {
        selectServer(keys[0]);
      } else {
        startNewServer();
      }

      const fileBase = currentFilePath ? currentFilePath.split(/[\\/]/).pop() : "config";
      statusMessage = {
        text: `Saved to ${activeScope === "workspace" ? "Workspace" : "Global"} ${fileBase} successfully!`,
        type: "success",
      };
    } catch (err: any) {
      statusMessage = { text: `Failed to save: ${err}`, type: "error" };
    } finally {
      isLoading = false;
    }
  }
</script>

{#if isOpen}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-xs animate-in fade-in duration-150"
    onclick={onClose}
    onkeydown={(e) => e.key === "Escape" && onClose()}
    role="presentation"
  >
    <div
      class="w-full max-w-3xl bg-surface border border-theme-default rounded-2xl shadow-2xl overflow-hidden flex flex-col max-h-[88vh]"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
      role="dialog"
      aria-modal="true"
      tabindex="-1"
    >
      <!-- Header -->
      <div class="px-5 py-3.5 border-b border-subtle flex items-center justify-between bg-surface-elevated/50">
        <div class="flex items-center gap-2.5">
          <div class="p-1.5 rounded-lg bg-accent-subtle text-accent-theme">
            <Server size={18} />
          </div>
          <div>
            <h2 class="text-sm font-semibold text-primary-theme flex items-center gap-2">
              Model Context Protocol (MCP) Servers
              {#if activeEngine === "agy"}
                <span class="text-[10px] uppercase font-mono font-bold px-2 py-0.5 rounded bg-purple-500/15 text-purple-400 border border-purple-500/30">
                  Antigravity (agy)
                </span>
              {:else}
                <span class="text-[10px] uppercase font-mono font-bold px-2 py-0.5 rounded bg-blue-500/15 text-blue-400 border border-blue-500/30">
                  Gemini CLI
                </span>
              {/if}
            </h2>
            <p class="text-xs text-secondary-theme">Extend agent capabilities with external databases, APIs, and tools.</p>
          </div>
        </div>

        <div class="flex items-center gap-2">
          <Tooltip text="Reload Config" subtext="Reload settings.json from disk to sync any manual changes" position="bottom">
            <button
              onclick={loadConfig}
              class="p-1.5 text-secondary-theme hover:text-primary-theme hover:bg-surface rounded-lg transition-colors cursor-pointer"
            >
              <RefreshCw size={15} class={isLoading ? "animate-spin" : ""} />
            </button>
          </Tooltip>
          <Tooltip text="Close Modal" position="bottom">
            <button
              onclick={onClose}
              class="p-1 text-secondary-theme hover:text-primary-theme hover:bg-surface rounded-lg transition-colors cursor-pointer"
              aria-label="Close modal"
            >
              <X size={16} />
            </button>
          </Tooltip>
        </div>
      </div>

      <!-- Scope Selector Bar -->
      <div class="px-5 py-2.5 bg-surface-elevated/20 border-b border-subtle flex items-center justify-between text-xs">
        <div class="flex items-center gap-1.5 p-0.5 bg-app rounded-lg border border-subtle">
          <Tooltip
            text="Workspace Scope"
            subtext="Configures servers stored in .gemini/settings.json within this workspace folder only"
            position="bottom"
          >
            <button
              onclick={() => {
                activeScope = "workspace";
                loadConfig();
              }}
              class="flex items-center gap-1.5 px-3 py-1.5 rounded-md font-medium transition-all cursor-pointer {activeScope === 'workspace' ? 'bg-surface-elevated text-accent-theme shadow-xs border border-subtle' : 'text-secondary-theme hover:text-primary-theme'}"
            >
              <Folder size={13} />
              <span>Workspace Scope</span>
              {#if activeWorkspace}
                <span class="text-[10px] text-muted-theme font-mono truncate max-w-[120px]">({activeWorkspace.name})</span>
              {/if}
            </button>
          </Tooltip>

          <Tooltip
            text="Global Scope"
            subtext="Configures servers stored in ~/.gemini/settings.json available across all workspaces"
            position="bottom"
          >
            <button
              onclick={() => {
                activeScope = "global";
                loadConfig();
              }}
              class="flex items-center gap-1.5 px-3 py-1.5 rounded-md font-medium transition-all cursor-pointer {activeScope === 'global' ? 'bg-surface-elevated text-accent-theme shadow-xs border border-subtle' : 'text-secondary-theme hover:text-primary-theme'}"
            >
              <Globe size={13} />
              <span>Global Scope</span>
              <span class="text-[10px] text-muted-theme font-mono">(~/.gemini)</span>
            </button>
          </Tooltip>
        </div>

        <!-- Mode Toggle: Form vs JSON -->
        <div class="flex items-center gap-1 bg-app p-0.5 rounded-lg border border-subtle">
          <button
            onclick={() => (editMode = "form")}
            class="px-2.5 py-1 rounded text-xs font-medium transition-colors {editMode === 'form' ? 'bg-surface-elevated text-primary-theme' : 'text-secondary-theme hover:text-primary-theme'}"
          >
            Guided Form
          </button>
          <button
            onclick={() => {
              editMode = "json";
              rawJsonText = JSON.stringify(servers, null, 2);
            }}
            class="flex items-center gap-1 px-2.5 py-1 rounded text-xs font-medium transition-colors {editMode === 'json' ? 'bg-surface-elevated text-primary-theme' : 'text-secondary-theme hover:text-primary-theme'}"
          >
            <Code size={12} />
            <span>Raw JSON</span>
          </button>
        </div>
      </div>

      <!-- File Path Bar -->
      <div class="px-5 py-1.5 bg-app/50 border-b border-subtle flex items-center justify-between text-[11px] text-muted-theme font-mono">
        <div class="flex items-center gap-1.5 truncate">
          <FileText size={12} class="shrink-0 text-accent-theme" />
          <span class="truncate">{currentFilePath || "Resolving settings..."}</span>
        </div>
        <span class="shrink-0 text-[10px] text-secondary-theme">
          {Object.keys(servers).length} server{Object.keys(servers).length === 1 ? "" : "s"} configured
        </span>
      </div>

      <!-- Import from Gemini CLI Banner (When Antigravity is active & unimported servers detected) -->
      {#if activeEngine === "agy" && importableServers.length > 0}
        <div class="px-5 py-2.5 bg-purple-500/10 border-b border-purple-500/25 flex items-center justify-between gap-3 text-xs">
          <div class="flex items-center gap-2 text-purple-300 min-w-0">
            <Sparkles size={15} class="shrink-0 text-purple-400" />
            <span class="truncate">
              Found <strong>{importableServers.length}</strong> MCP server{importableServers.length > 1 ? "s" : ""} in Gemini CLI:
              <code class="font-mono text-purple-200">{importableServers.join(", ")}</code>
            </span>
          </div>
          <button
            onclick={handleImportFromGemini}
            disabled={isImporting}
            class="px-2.5 py-1 rounded-md bg-purple-600 hover:bg-purple-500 text-white font-medium text-xs flex items-center gap-1.5 transition-colors cursor-pointer shrink-0 disabled:opacity-50"
          >
            <Plus size={13} />
            {isImporting ? "Importing..." : "Import to Antigravity"}
          </button>
        </div>
      {/if}

      <!-- Status Feedback Banner -->
      {#if statusMessage}
        <div class="px-5 py-2 text-xs flex items-center gap-2 {statusMessage.type === 'error' ? 'bg-rose-500/10 text-rose-400 border-b border-rose-500/20' : statusMessage.type === 'success' ? 'bg-emerald-500/10 text-emerald-400 border-b border-emerald-500/20' : 'bg-sky-500/10 text-sky-400 border-b border-sky-500/20'}">
          {#if statusMessage.type === 'error'}
            <AlertCircle size={14} class="shrink-0" />
          {:else if statusMessage.type === 'success'}
            <Check size={14} class="shrink-0" />
          {:else}
            <Info size={14} class="shrink-0" />
          {/if}
          <span class="truncate">{statusMessage.text}</span>
        </div>
      {/if}

      <!-- Main Content Area -->
      {#if editMode === "form"}
        <div class="flex-1 flex overflow-hidden min-h-[380px]">
          <!-- Left Sidebar: Server list & presets -->
          <div class="w-64 border-r border-subtle p-3 flex flex-col justify-between bg-surface/30 overflow-y-auto">
            <div class="space-y-3">
              <button
                onclick={startNewServer}
                class="w-full flex items-center justify-center gap-1.5 px-3 py-2 rounded-lg bg-accent-subtle hover:bg-accent-theme hover:text-white text-accent-theme text-xs font-medium transition-colors shadow-2xs cursor-pointer"
              >
                <Plus size={14} />
                <span>Add MCP Server</span>
              </button>

              <div>
                <div class="text-[10px] font-semibold uppercase tracking-wider text-muted-theme px-1 mb-1.5">
                  Active Servers
                </div>
                {#if Object.keys(servers).length === 0}
                  <div class="text-[11px] text-muted-theme px-2 py-3 text-center border border-dashed border-subtle rounded-lg">
                    No MCP servers configured yet.
                  </div>
                {:else}
                  <div class="space-y-1">
                    {#each Object.keys(servers) as key}
                      <div
                        class="w-full text-left px-2.5 py-1.5 rounded-lg text-xs transition-colors flex items-center justify-between group {selectedServerKey === key ? 'bg-surface-elevated text-accent-theme font-medium border border-subtle shadow-xs' : 'text-secondary-theme hover:bg-surface-elevated/40'}"
                      >
                        <button
                          type="button"
                          onclick={() => selectServer(key)}
                          class="truncate flex-1 text-left cursor-pointer"
                        >
                          <div class="flex items-center gap-1.5 truncate">
                            <span class="inline-block w-1.5 h-1.5 rounded-full shrink-0 {servers[key].disabled ? 'bg-amber-400/80' : 'bg-emerald-400'}"></span>
                            <span class="font-mono text-xs truncate {servers[key].disabled ? 'line-through text-muted-theme' : ''}">{key}</span>
                          </div>
                          <span class="text-[10px] text-muted-theme block truncate font-mono ml-3">
                            {servers[key].command} {(servers[key].args || []).slice(0, 2).join(' ')}
                          </span>
                        </button>

                        <Tooltip text={servers[key].disabled ? "Enable server" : "Disable server"} position="top">
                          <button
                            type="button"
                            onclick={(e) => toggleServerDisabled(key, e)}
                            class="p-1 rounded hover:bg-surface-hover transition-colors shrink-0 ml-1 cursor-pointer"
                            aria-label={servers[key].disabled ? "Enable server" : "Disable server"}
                          >
                            <span class="text-[10px] px-1.5 py-0.5 rounded font-mono font-medium {servers[key].disabled ? 'bg-amber-500/15 text-amber-400 border border-amber-500/30' : 'bg-emerald-500/15 text-emerald-400 border border-emerald-500/30'}">
                              {servers[key].disabled ? 'Off' : 'On'}
                            </span>
                          </button>
                        </Tooltip>
                      </div>
                    {/each}
                  </div>
                {/if}
              </div>

              <!-- Quick Presets -->
              <div class="pt-2 border-t border-subtle">
                <div class="text-[10px] font-semibold uppercase tracking-wider text-muted-theme px-1 mb-1.5 flex items-center gap-1">
                  <Sparkles size={11} class="text-accent-theme" />
                  <span>Popular Presets</span>
                </div>
                <div class="space-y-1">
                  {#each PRESETS as preset}
                    <button
                      onclick={() => applyPreset(preset)}
                      class="w-full text-left px-2.5 py-1.5 rounded-lg text-xs hover:bg-surface-elevated/60 text-secondary-theme hover:text-primary-theme transition-colors border border-transparent hover:border-subtle group cursor-pointer"
                    >
                      <div class="flex items-center gap-1.5 font-medium text-xs">
                        <span>{preset.icon}</span>
                        <span class="truncate">{preset.label}</span>
                      </div>
                      <div class="text-[10px] text-muted-theme truncate mt-0.5">
                        {preset.description}
                      </div>
                    </button>
                  {/each}
                </div>
              </div>
            </div>
          </div>

          <!-- Right Editor: Server details form -->
          <div class="flex-1 p-5 overflow-y-auto space-y-4 text-xs">
            <!-- Server Status Active/Disabled Toggle -->
            <div class="flex items-center justify-between p-3 bg-app rounded-xl border border-theme-default">
              <div class="space-y-0.5">
                <div class="flex items-center gap-2">
                  <span class="text-xs font-semibold text-primary-theme">Server Status</span>
                  <span class="px-1.5 py-0.5 rounded text-[10px] font-mono font-semibold {formDisabled ? 'bg-amber-500/15 text-amber-500 border border-amber-500/30' : 'bg-emerald-500/15 text-emerald-400 border border-emerald-500/30'}">
                    {formDisabled ? 'Disabled' : 'Active'}
                  </span>
                </div>
                <p class="text-[11px] text-muted-theme">
                  {formDisabled ? "Server is deactivated. Gemini CLI will ignore it until re-enabled." : "Server is active and tools will be exposed to Gemini CLI via ACP."}
                </p>
              </div>
              <button
                type="button"
                role="switch"
                aria-checked={!formDisabled}
                onclick={() => (formDisabled = !formDisabled)}
                class="relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none {formDisabled ? 'bg-surface-elevated border border-subtle' : 'bg-accent-theme'}"
                aria-label="Toggle server status"
              >
                <span
                  class="pointer-events-none inline-block h-4 w-4 transform rounded-full bg-white shadow-md ring-0 transition duration-200 ease-in-out {formDisabled ? 'translate-x-0' : 'translate-x-4'}"
                ></span>
              </button>
            </div>

            <div class="grid grid-cols-2 gap-3">
              <div>
                <label for="mcp-key" class="block text-primary-theme font-medium mb-1">
                  Server ID / Alias <span class="text-rose-400">*</span>
                </label>
                <input
                  id="mcp-key"
                  type="text"
                  bind:value={formServerKey}
                  placeholder="e.g. github, postgres, memory"
                  class="w-full px-3 py-2 bg-app border border-theme-default rounded-lg text-primary-theme font-mono focus:outline-none focus:border-accent-theme"
                />
                <span class="text-[10px] text-muted-theme mt-1 block">Tools will be prefixed as <code>mcp_{formServerKey || 'alias'}_*</code></span>
              </div>

              <div>
                <label for="mcp-cmd" class="block text-primary-theme font-medium mb-1">
                  Command Executable <span class="text-rose-400">*</span>
                </label>
                <input
                  id="mcp-cmd"
                  type="text"
                  bind:value={formCommand}
                  placeholder="docker, npx, uvx, node, python"
                  class="w-full px-3 py-2 bg-app border border-theme-default rounded-lg text-primary-theme font-mono focus:outline-none focus:border-accent-theme"
                />
                <span class="text-[10px] text-muted-theme mt-1 block">Binary executed to spawn the MCP server.</span>
              </div>
            </div>

            <div>
              <label for="mcp-args" class="block text-primary-theme font-medium mb-1">
                Arguments (one per line)
              </label>
              <textarea
                id="mcp-args"
                bind:value={formArgsText}
                rows="4"
                placeholder="run&#10;-i&#10;--rm&#10;ghcr.io/github/github-mcp-server:latest"
                class="w-full px-3 py-2 bg-app border border-theme-default rounded-lg text-primary-theme font-mono text-xs focus:outline-none focus:border-accent-theme resize-none"
              ></textarea>
              <span class="text-[10px] text-muted-theme mt-1 block">Arguments passed sequentially to the command.</span>
            </div>

            <!-- Environment Variables -->
            <div>
              <div class="flex items-center justify-between mb-1.5">
                <span class="block text-primary-theme font-medium">Environment Variables</span>
                <button
                  type="button"
                  onclick={addEnvRow}
                  class="flex items-center gap-1 text-[11px] text-accent-theme hover:underline cursor-pointer"
                >
                  <Plus size={12} />
                  <span>Add Variable</span>
                </button>
              </div>

              {#if formEnvList.length === 0}
                <div class="p-2.5 bg-app/50 border border-dashed border-subtle rounded-lg text-[11px] text-muted-theme text-center">
                  No custom environment variables defined. (Tokens like <code>GITHUB_PERSONAL_ACCESS_TOKEN</code> can be added here)
                </div>
              {:else}
                <div class="space-y-1.5 max-h-36 overflow-y-auto pr-1">
                  {#each formEnvList as envItem, i}
                    <div class="flex items-center gap-2">
                      <input
                        type="text"
                        placeholder="KEY (e.g. API_KEY)"
                        bind:value={envItem.key}
                        class="w-2/5 px-2.5 py-1.5 bg-app border border-theme-default rounded text-primary-theme font-mono text-xs focus:outline-none focus:border-accent-theme"
                      />
                      <input
                        type="text"
                        placeholder={'VALUE (or ${VAR_NAME})'}
                        bind:value={envItem.value}
                        class="flex-1 px-2.5 py-1.5 bg-app border border-theme-default rounded text-primary-theme font-mono text-xs focus:outline-none focus:border-accent-theme"
                      />
                      <button
                        type="button"
                        onclick={() => removeEnvRow(i)}
                        class="p-1.5 text-secondary-theme hover:text-rose-400 rounded transition-colors cursor-pointer"
                        title="Remove variable"
                      >
                        <Trash2 size={13} />
                      </button>
                    </div>
                  {/each}
                </div>
              {/if}
            </div>

            <div>
              <label for="mcp-cwd" class="block text-primary-theme font-medium mb-1">
                Working Directory (Optional)
              </label>
              <input
                id="mcp-cwd"
                type="text"
                bind:value={formCwd}
                placeholder="Leave blank to run in workspace root"
                class="w-full px-3 py-2 bg-app border border-theme-default rounded-lg text-primary-theme font-mono focus:outline-none focus:border-accent-theme"
              />
            </div>

            <!-- Footer Save / Delete Buttons -->
            <div class="pt-3 flex items-center justify-between border-t border-subtle mt-4">
              {#if selectedServerKey && servers[selectedServerKey]}
                <button
                  type="button"
                  onclick={() => handleDeleteServer(selectedServerKey)}
                  class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-rose-400 hover:bg-rose-500/10 transition-colors cursor-pointer"
                >
                  <Trash2 size={14} />
                  <span>Delete Server</span>
                </button>
              {:else}
                <div></div>
              {/if}

              <button
                type="button"
                onclick={handleSaveForm}
                disabled={isLoading}
                class="flex items-center gap-1.5 px-4 py-1.5 rounded-lg bg-accent-theme hover:bg-accent-hover text-white font-semibold transition-colors shadow-xs disabled:opacity-50 cursor-pointer"
              >
                <Check size={14} />
                <span>Save Server</span>
              </button>
            </div>
          </div>
        </div>
      {:else}
        <!-- Raw JSON Editor View -->
        <div class="p-5 flex-1 flex flex-col min-h-[380px] overflow-hidden text-xs">
          <div class="mb-2 flex items-center justify-between">
            <span class="text-secondary-theme text-[11px]">
              Directly edit the <code>"mcpServers"</code> JSON configuration. Preserves other settings in <code>settings.json</code>.
            </span>
            <span class="text-[10px] text-muted-theme font-mono">Format: Record&lt;string, McpServerConfig&gt;</span>
          </div>

          <textarea
            bind:value={rawJsonText}
            rows="14"
            class="flex-1 w-full p-3 bg-code text-code font-mono text-xs border border-theme-default rounded-lg focus:outline-none focus:border-accent-theme resize-none"
            placeholder={'{\n  "github": {\n    "command": "docker",\n    "args": ["run", "-i"]\n  }\n}'}
          ></textarea>

          <div class="pt-3 flex items-center justify-between border-t border-subtle mt-3">
            <span class="text-[11px] text-muted-theme">
              Changes apply to {activeScope === "workspace" ? "current workspace" : "all sessions"}.
            </span>
            <button
              type="button"
              onclick={handleSaveJson}
              disabled={isLoading}
              class="flex items-center gap-1.5 px-4 py-1.5 rounded-lg bg-accent-theme hover:bg-accent-hover text-white font-semibold transition-colors shadow-xs disabled:opacity-50 cursor-pointer"
            >
              <Check size={14} />
              <span>Save JSON Settings</span>
            </button>
          </div>
        </div>
      {/if}
    </div>
  </div>
{/if}
