<script lang="ts">
  import type { Workspace, Session } from "$lib/types";
  import { themeManager } from "$lib/theme.svelte";
  import {
    Plus,
    Search,
    BookOpen,
    FolderKanban,
    MessageSquare,
    Trash2,
    Edit2,
    Settings,
    Sparkles,
    ChevronDown,
    ChevronUp,
    Palette,
    Server,
    Terminal,
    FolderTree,
    PanelLeftClose,
    PanelLeftOpen,
    RefreshCw,
    Cpu,
    Sliders,
    Package,
  } from "lucide-svelte";
  import Tooltip from "$lib/components/Tooltip.svelte";
  import { APP_VERSION_FALLBACK } from "$lib/version";

  let {
    isOpen = false,
    workspaces = [],
    activeWorkspace = null,
    sessions = [],
    activeSession = null,
    generatingSessionIds = new Set<string>(),
    appVersion = APP_VERSION_FALLBACK,
    onSelectWorkspace,
    onSelectSession,
    onNewSession,
    onRenameSession,
    onDeleteSession,
    onOpenSearch,
    onOpenTemplates,
    onOpenWorkspaceModal,
    onOpenThemeModal,
    onOpenMcpModal,
    onToggleTerminal,
    onToggleExplorer,
    onToggle,
    envStatus = null,
    activeEngine = "gemini",
    onCheckUpdate,
    isCheckingUpdate = false,
    onOpenCliSetup,
    onOpenCliSettings,
    onOpenPluginsModal,
  }: {
    isOpen?: boolean;
    workspaces: Workspace[];
    activeWorkspace: Workspace | null;
    sessions: Session[];
    activeSession: Session | null;
    generatingSessionIds?: Set<string>;
    appVersion?: string;
    activeEngine?: string;
    onSelectWorkspace: (ws: Workspace) => void;
    onSelectSession: (s: Session) => void;
    onNewSession: () => void;
    onRenameSession: (s: Session) => void;
    onDeleteSession: (s: Session) => void;
    onOpenSearch: () => void;
    onOpenTemplates: () => void;
    onOpenWorkspaceModal: () => void;
    onOpenThemeModal: () => void;
    onOpenMcpModal: () => void;
    onToggleTerminal?: () => void;
    onToggleExplorer?: () => void;
    onToggle?: () => void;
    envStatus: any;
    onCheckUpdate?: () => void;
    isCheckingUpdate?: boolean;
    onOpenCliSetup?: () => void;
    onOpenCliSettings?: () => void;
    onOpenPluginsModal?: () => void;
  } = $props();

  let showWorkspaceMenu = $state(false);
  let showToolsMenu = $state(false);
</script>

{#if !isOpen}
  <!-- Collapsed Left Navigation Strip -->
  <aside
    class="w-12 h-screen flex flex-col items-center py-2.5 bg-sidebar border-r border-subtle select-none shrink-0 z-20 justify-between"
    aria-label="Navigation collapsed strip"
  >
    <!-- Top Action Icons Group -->
    <div class="flex flex-col items-center gap-1.5 w-full">
      <!-- App Brand Logo / Expand Button -->
      {#if onToggle}
        <Tooltip text="Expand Sidebar" shortcut="Ctrl+B" position="right">
          <button
            type="button"
            onclick={onToggle}
            class="w-8 h-8 rounded-lg flex items-center justify-center text-white shadow-sm shadow-black/20 hover:scale-105 active:scale-95 transition-all cursor-pointer mb-0.5"
            style="background: var(--accent-gradient);"
            aria-label="Expand sidebar"
          >
            <Sparkles size={16} />
          </button>
        </Tooltip>
      {:else}
        <div
          class="w-8 h-8 rounded-lg flex items-center justify-center text-white shadow-sm shadow-black/20 mb-0.5"
          style="background: var(--accent-gradient);"
        >
          <Sparkles size={16} />
        </div>
      {/if}

      <div class="w-6 h-px bg-border/40 my-0.5"></div>

      <!-- New Chat Button -->
      <Tooltip text="New Conversation" shortcut="Ctrl+N" position="right">
        <button
          type="button"
          onclick={onNewSession}
          class="w-8 h-8 rounded-lg flex items-center justify-center text-secondary-theme hover:text-accent-theme hover:bg-surface transition-colors cursor-pointer"
          aria-label="New conversation"
        >
          <Plus size={16} />
        </button>
      </Tooltip>

      <!-- Search History Button -->
      <Tooltip text="Search History" shortcut="Ctrl+K" position="right">
        <button
          type="button"
          onclick={onOpenSearch}
          class="w-8 h-8 rounded-lg flex items-center justify-center text-secondary-theme hover:text-primary-theme hover:bg-surface transition-colors cursor-pointer"
          aria-label="Search past conversations"
        >
          <Search size={15} />
        </button>
      </Tooltip>

      <!-- Prompt Templates Button -->
      <Tooltip text="Prompt Library" position="right">
        <button
          type="button"
          onclick={onOpenTemplates}
          class="w-8 h-8 rounded-lg flex items-center justify-center text-secondary-theme hover:text-primary-theme hover:bg-surface transition-colors cursor-pointer"
          aria-label="Prompt templates"
        >
          <BookOpen size={15} />
        </button>
      </Tooltip>

      <!-- Workspace Profile Selector / Modal -->
      <Tooltip text={`Workspace: ${activeWorkspace?.name || 'Default'}`} position="right">
        <button
          type="button"
          onclick={onOpenWorkspaceModal}
          class="w-8 h-8 rounded-lg flex items-center justify-center text-secondary-theme hover:text-accent-theme hover:bg-surface transition-colors cursor-pointer"
          aria-label="Workspace profile"
        >
          <FolderKanban size={15} />
        </button>
      </Tooltip>

      <div class="w-6 h-px bg-border/40 my-0.5"></div>

      <!-- MCP Servers Modal -->
      {#if onOpenMcpModal}
        <Tooltip text="MCP Servers" shortcut="Ctrl+M" position="right">
          <button
            type="button"
            onclick={onOpenMcpModal}
            class="w-8 h-8 rounded-lg flex items-center justify-center text-secondary-theme hover:text-accent-theme hover:bg-surface transition-colors cursor-pointer"
            aria-label="MCP servers"
          >
            <Server size={15} />
          </button>
        </Tooltip>
      {/if}

      <!-- Plugins & Extensions Hub -->
      {#if onOpenPluginsModal}
        <Tooltip text="Plugins & Extensions" position="right">
          <button
            type="button"
            onclick={onOpenPluginsModal}
            class="w-8 h-8 rounded-lg flex items-center justify-center text-secondary-theme hover:text-purple-400 hover:bg-surface transition-colors cursor-pointer"
            aria-label="Plugins and extensions"
          >
            <Package size={15} />
          </button>
        </Tooltip>
      {/if}

      <!-- CLI & Auth Setup -->
      {#if onOpenCliSetup}
        <Tooltip text="CLI & Auth Setup" position="right">
          <button
            type="button"
            onclick={onOpenCliSetup}
            class="w-8 h-8 rounded-lg flex items-center justify-center text-secondary-theme hover:text-accent-theme hover:bg-surface transition-colors cursor-pointer"
            aria-label="CLI and Auth setup"
          >
            <Cpu size={15} />
          </button>
        </Tooltip>
      {/if}

      <!-- CLI Global Settings -->
      {#if onOpenCliSettings}
        <Tooltip text="CLI Settings" position="right">
          <button
            type="button"
            onclick={onOpenCliSettings}
            class="w-8 h-8 rounded-lg flex items-center justify-center text-secondary-theme hover:text-accent-theme hover:bg-surface transition-colors cursor-pointer"
            aria-label="CLI global settings"
          >
            <Sliders size={15} />
          </button>
        </Tooltip>
      {/if}

      <!-- Terminal Console Drawer Toggle -->
      {#if onToggleTerminal}
        <Tooltip text="Terminal Console" shortcut="Ctrl+`" position="right">
          <button
            type="button"
            onclick={onToggleTerminal}
            class="w-8 h-8 rounded-lg flex items-center justify-center text-secondary-theme hover:text-accent-theme hover:bg-surface transition-colors cursor-pointer"
            aria-label="Terminal console"
          >
            <Terminal size={15} />
          </button>
        </Tooltip>
      {/if}

      <!-- Workspace Explorer Panel Toggle -->
      {#if onToggleExplorer}
        <Tooltip text="Workspace Explorer" shortcut="Ctrl+Alt+L" position="right">
          <button
            type="button"
            onclick={onToggleExplorer}
            class="w-8 h-8 rounded-lg flex items-center justify-center text-secondary-theme hover:text-[#a855f7] hover:bg-surface transition-colors cursor-pointer"
            aria-label="Workspace explorer"
          >
            <FolderTree size={15} />
          </button>
        </Tooltip>
      {/if}
    </div>

    <!-- Bottom Action Icons Group -->
    <div class="flex flex-col items-center gap-1.5 w-full">
      <!-- Theme & Colors -->
      <Tooltip text="Theme & Colors" position="right">
        <button
          type="button"
          onclick={onOpenThemeModal}
          class="w-8 h-8 rounded-lg flex items-center justify-center text-secondary-theme hover:text-accent-theme hover:bg-surface transition-colors cursor-pointer"
          aria-label="Theme settings"
        >
          <Palette size={15} />
        </button>
      </Tooltip>

      <!-- WinGet Update Check / Version Indicator -->
      {#if onCheckUpdate}
        <Tooltip text={isCheckingUpdate ? "Checking updates..." : `v${appVersion} • Check for Updates`} position="right">
          <button
            type="button"
            onclick={onCheckUpdate}
            disabled={isCheckingUpdate}
            class="w-8 h-8 rounded-lg flex items-center justify-center text-secondary-theme hover:text-accent-theme hover:bg-surface transition-colors cursor-pointer disabled:opacity-50"
            aria-label="Check updates"
          >
            <RefreshCw size={14} class={isCheckingUpdate ? "animate-spin text-accent-theme" : ""} />
          </button>
        </Tooltip>
      {/if}

      <!-- Expand Sidebar Button at the very bottom -->
      {#if onToggle}
        <Tooltip text="Expand Sidebar" shortcut="Ctrl+B" position="right">
          <button
            type="button"
            onclick={onToggle}
            class="w-8 h-8 rounded-lg flex items-center justify-center text-muted-theme hover:text-primary-theme hover:bg-surface transition-colors cursor-pointer mt-0.5"
            aria-label="Expand sidebar"
          >
            <PanelLeftOpen size={16} />
          </button>
        </Tooltip>
      {/if}
    </div>
  </aside>
{:else}
  <!-- Full Left Navigation Sidebar -->
  <aside class="w-72 h-screen flex flex-col bg-sidebar border-r border-subtle select-none shrink-0">
  <!-- Top App Brand -->
  <div class="p-3.5 border-b border-subtle flex items-center justify-between">
    <div class="flex items-center gap-2.5 truncate">
      <div
        class="w-8 h-8 rounded-lg flex items-center justify-center text-white shadow-md shadow-black/20 shrink-0"
        style="background: var(--accent-gradient);"
      >
        <Sparkles size={18} />
      </div>
      <div class="truncate">
        <h1 class="text-sm font-semibold tracking-tight text-primary-theme flex items-center gap-1.5 truncate">
          Gemini Desktop
        </h1>
        <Tooltip
          text={envStatus?.installed ? "Gemini CLI Connected" : "Mock / Standby Mode"}
          subtext={envStatus?.installed
            ? "Gemini CLI is installed and communicating via ACP JSON-RPC. Click to manage CLI & Auth."
            : "Gemini CLI not detected in system PATH. Click to setup CLI & credentials."}
          position="right"
        >
          <button
            type="button"
            onclick={onOpenCliSetup}
            class="flex items-center gap-1.5 text-[11px] text-muted-theme hover:text-primary-theme transition-colors cursor-pointer text-left"
          >
            <span class="inline-block w-1.5 h-1.5 rounded-full {envStatus?.installed ? 'bg-emerald-400' : 'bg-amber-400'}"></span>
            <span>{envStatus?.installed ? 'CLI Connected' : 'Setup CLI'}</span>
          </button>
        </Tooltip>
      </div>
    </div>

    <!-- Collapse Sidebar Button -->
    {#if onToggle}
      <Tooltip text="Collapse Sidebar" shortcut="Ctrl+B" position="bottom">
        <button
          type="button"
          onclick={onToggle}
          class="p-1 rounded-lg text-muted-theme hover:text-primary-theme hover:bg-surface transition-colors cursor-pointer shrink-0"
          aria-label="Collapse sidebar"
        >
          <PanelLeftClose size={16} />
        </button>
      </Tooltip>
    {/if}
  </div>

  <!-- Workspace Selector -->
  <div class="p-3 border-b border-subtle">
    <div class="text-[11px] font-semibold uppercase text-muted-theme tracking-wider mb-1.5 px-1">
      Workspace Profile
    </div>
    <div class="relative">
      <button
        onclick={() => (showWorkspaceMenu = !showWorkspaceMenu)}
        class="w-full flex items-center justify-between px-3 py-2 rounded-lg bg-surface hover:bg-surface-hover border border-theme-default text-xs text-primary-theme transition-all text-left"
      >
        <div class="flex items-center gap-2 truncate">
          <FolderKanban size={14} class="text-accent-theme shrink-0" />
          <span class="truncate font-medium">{activeWorkspace?.name || "Select Workspace"}</span>
        </div>
        <ChevronDown size={14} class="text-secondary-theme shrink-0" />
      </button>

      {#if showWorkspaceMenu}
        <div
          class="fixed inset-0 z-30"
          onclick={() => (showWorkspaceMenu = false)}
          role="presentation"
        ></div>
        <div class="absolute left-0 right-0 mt-1 bg-surface-elevated border border-subtle rounded-lg shadow-xl py-1 z-40">
          {#each workspaces as ws}
            <button
              onclick={() => {
                onSelectWorkspace(ws);
                showWorkspaceMenu = false;
              }}
              class="w-full px-3 py-1.5 text-xs text-left flex items-center justify-between hover:bg-surface-hover transition-colors {activeWorkspace?.id === ws.id ? 'text-accent-theme font-medium' : 'text-secondary-theme'}"
            >
              <span class="truncate">{ws.name}</span>
              <span class="text-[10px] text-muted-theme font-mono">{ws.model}</span>
            </button>
          {/each}
          <div class="border-t border-subtle my-1"></div>
          <button
            onclick={() => {
              showWorkspaceMenu = false;
              onOpenWorkspaceModal();
            }}
            class="w-full px-3 py-1.5 text-xs text-left text-accent-theme hover:bg-surface-hover flex items-center gap-1.5"
          >
            <Settings size={12} />
            <span>Manage Workspaces...</span>
          </button>
        </div>
      {/if}
    </div>
  </div>

  <!-- Primary Quick Action Navigation (New Chat & Search) -->
  <div class="p-2.5 space-y-1.5 border-b border-subtle">
    <Tooltip
      class="w-full block"
      text="New Conversation"
      subtext="Start a fresh chat thread with a clean context"
      shortcut="Ctrl+N"
      position="right"
    >
      <button
        type="button"
        onclick={onNewSession}
        class="w-full flex items-center gap-2.5 px-3 py-2 rounded-lg bg-surface hover:bg-surface-hover text-primary-theme hover:border-accent-theme/50 border border-theme-default text-xs font-medium transition-all shadow-2xs group cursor-pointer"
      >
        <Plus size={15} class="text-accent-theme group-hover:scale-110 transition-transform shrink-0" />
        <span class="truncate font-semibold">New Chat</span>
        <span class="ml-auto text-[10px] text-muted-theme font-mono">Ctrl+N</span>
      </button>
    </Tooltip>

    <Tooltip
      class="w-full block"
      text="Search History"
      subtext="Search across past messages, code blocks, and sessions using local SQLite FTS5"
      shortcut="Ctrl+K"
      position="right"
    >
      <button
        type="button"
        onclick={onOpenSearch}
        class="w-full flex items-center gap-2.5 px-3 py-1.5 rounded-lg hover:bg-surface-hover text-secondary-theme hover:text-primary-theme text-xs transition-colors cursor-pointer"
      >
        <Search size={14} class="text-muted-theme shrink-0" />
        <span class="truncate">Search History</span>
        <span class="ml-auto text-[10px] text-muted-theme font-mono">Ctrl+K</span>
      </button>
    </Tooltip>
  </div>

  <!-- Chat History Sessions List (Flexible Primary Real Estate) -->
  <div class="flex-1 min-h-0 overflow-y-auto p-2 space-y-0.5">
    <div class="px-2 py-1.5 text-[11px] font-semibold uppercase text-muted-theme tracking-wider flex items-center justify-between">
      <span>Recent Chats</span>
      {#if sessions.length > 0}
        <span class="text-[10px] text-muted-theme font-mono font-normal">{sessions.length}</span>
      {/if}
    </div>

    {#if sessions.length === 0}
      <div class="px-3 py-8 text-center text-xs text-muted-theme flex flex-col items-center justify-center gap-1.5">
        <MessageSquare size={18} class="text-muted-theme/50 mb-1" />
        <span>No conversations yet</span>
        <span class="text-[11px] text-muted-theme/70">Click New Chat to begin</span>
      </div>
    {:else}
      {#each sessions as session (session.id)}
        <div
          role="button"
          tabindex="0"
          class="group flex items-center justify-between px-2.5 py-2 rounded-lg text-xs cursor-pointer transition-colors {activeSession?.id === session.id ? 'bg-surface-elevated text-primary-theme font-medium border border-subtle shadow-xs' : 'text-secondary-theme hover:bg-surface-hover hover:text-primary-theme'}"
          onclick={() => onSelectSession(session)}
          onkeydown={(e) => {
            if (e.key === "Enter" || e.key === " ") {
              e.preventDefault();
              onSelectSession(session);
            }
          }}
        >
          <div class="flex items-center gap-2 truncate flex-1 min-w-0 mr-1">
            {#if generatingSessionIds?.has(session.id)}
              <span class="relative flex h-2 w-2 shrink-0">
                <span class="animate-ping absolute inline-flex h-full w-full rounded-full bg-accent-theme opacity-75"></span>
                <span class="relative inline-flex rounded-full h-2 w-2 bg-accent-theme"></span>
              </span>
            {:else}
              <MessageSquare size={14} class="shrink-0 {activeSession?.id === session.id ? 'text-accent-theme' : 'text-muted-theme'}" />
            {/if}
            <span class="truncate min-w-0">{session.title}</span>
            {#if generatingSessionIds?.has(session.id)}
              <span class="ml-auto text-[10px] text-accent-theme font-medium animate-pulse shrink-0">
                generating...
              </span>
            {/if}
          </div>

          <!-- Hover Action buttons -->
          <div class="opacity-0 group-hover:opacity-100 flex items-center gap-1 transition-opacity">
            <Tooltip text="Rename Chat" position="top">
              <button
                class="p-1 hover:text-accent-theme rounded text-secondary-theme cursor-pointer"
                onclick={(e) => {
                  e.stopPropagation();
                  onRenameSession(session);
                }}
              >
                <Edit2 size={12} />
              </button>
            </Tooltip>
            <Tooltip text="Delete Chat" position="top">
              <button
                class="p-1 hover:text-rose-400 rounded text-secondary-theme cursor-pointer"
                onclick={(e) => {
                  e.stopPropagation();
                  onDeleteSession(session);
                }}
              >
                <Trash2 size={12} />
              </button>
            </Tooltip>
          </div>
        </div>
      {/each}
    {/if}
  </div>

  <!-- Bottom Tools & Panes Strip -->
  <div class="p-2 border-t border-subtle bg-surface/30 space-y-1.5 relative">
    <!-- Developer Panes: Terminal & Workspace Explorer -->
    <div class="grid grid-cols-2 gap-1.5">
      {#if onToggleTerminal}
        <Tooltip text="Terminal Console" subtext="Open embedded PowerShell drawer" shortcut="Ctrl+`" position="top">
          <button
            type="button"
            onclick={onToggleTerminal}
            class="w-full flex items-center justify-center gap-1.5 px-2 py-1.5 rounded-lg bg-surface hover:bg-surface-hover border border-subtle hover:border-accent-theme/40 text-xs text-secondary-theme hover:text-primary-theme transition-colors cursor-pointer"
            aria-label="Toggle terminal console"
          >
            <Terminal size={13} class="text-accent-theme shrink-0" />
            <span class="truncate font-medium text-[11px]">Terminal</span>
          </button>
        </Tooltip>
      {/if}

      {#if onToggleExplorer}
        <Tooltip text="Workspace Explorer" subtext="Toggle workspace file tree" shortcut="Ctrl+Alt+L" position="top">
          <button
            type="button"
            onclick={onToggleExplorer}
            class="w-full flex items-center justify-center gap-1.5 px-2 py-1.5 rounded-lg bg-surface hover:bg-surface-hover border border-subtle hover:border-[#a855f7]/40 text-xs text-secondary-theme hover:text-primary-theme transition-colors cursor-pointer"
            aria-label="Toggle workspace explorer"
          >
            <FolderTree size={13} class="text-[#a855f7] shrink-0" />
            <span class="truncate font-medium text-[11px]">Explorer</span>
          </button>
        </Tooltip>
      {/if}
    </div>

    <!-- Tools & Settings Popover Launcher -->
    <div class="relative">
      <button
        type="button"
        onclick={() => (showToolsMenu = !showToolsMenu)}
        class="w-full flex items-center justify-between px-2.5 py-1.5 rounded-lg bg-surface hover:bg-surface-hover border border-subtle hover:border-theme-default text-xs text-secondary-theme hover:text-primary-theme transition-colors cursor-pointer"
        aria-label="Open tools and settings menu"
      >
        <div class="flex items-center gap-2 truncate">
          <Sliders size={13} class="text-accent-theme shrink-0" />
          <span class="font-medium truncate text-[11px]">Tools & Settings</span>
        </div>
        <div class="flex items-center gap-1.5 shrink-0 text-muted-theme">
          <span class="inline-block w-1.5 h-1.5 rounded-full {envStatus?.installed ? 'bg-emerald-400' : 'bg-amber-400'}"></span>
          <ChevronUp size={13} class="transition-transform duration-200 {showToolsMenu ? 'rotate-180' : ''}" />
        </div>
      </button>

      {#if showToolsMenu}
        <div
          class="fixed inset-0 z-40"
          onclick={() => (showToolsMenu = false)}
          role="presentation"
        ></div>
        <div
          class="absolute bottom-full left-0 right-0 mb-1.5 bg-surface-elevated border border-subtle rounded-xl shadow-2xl py-1 z-50 text-xs"
        >
          <div class="px-3 py-1.5 text-[10px] font-semibold uppercase text-muted-theme tracking-wider border-b border-subtle flex items-center justify-between">
            <span>Tools & Settings</span>
            <span class="font-mono text-[9px] lowercase text-accent-theme">{activeEngine}</span>
          </div>

          <!-- Prompt Library -->
          <button
            type="button"
            onclick={() => {
              showToolsMenu = false;
              onOpenTemplates();
            }}
            class="w-full px-3 py-2 text-left flex items-center gap-2.5 hover:bg-surface-hover text-secondary-theme hover:text-primary-theme transition-colors cursor-pointer"
          >
            <BookOpen size={14} class="text-muted-theme shrink-0" />
            <div class="truncate flex-1">
              <div class="font-medium text-primary-theme">Prompt Library</div>
              <div class="text-[10px] text-muted-theme">Pre-built prompt templates</div>
            </div>
          </button>

          <!-- MCP Servers -->
          {#if onOpenMcpModal}
            <button
              type="button"
              onclick={() => {
                showToolsMenu = false;
                onOpenMcpModal();
              }}
              class="w-full px-3 py-2 text-left flex items-center gap-2.5 hover:bg-surface-hover text-secondary-theme hover:text-primary-theme transition-colors cursor-pointer"
            >
              <Server size={14} class="text-accent-theme shrink-0" />
              <div class="truncate flex-1">
                <div class="font-medium text-primary-theme flex items-center justify-between">
                  <span>MCP Servers</span>
                  <span class="text-[10px] font-mono text-muted-theme">Ctrl+M</span>
                </div>
                <div class="text-[10px] text-muted-theme">External tools & context</div>
              </div>
            </button>
          {/if}

          <!-- Plugins & Extensions -->
          {#if onOpenPluginsModal}
            <button
              type="button"
              onclick={() => {
                showToolsMenu = false;
                onOpenPluginsModal();
              }}
              class="w-full px-3 py-2 text-left flex items-center gap-2.5 hover:bg-surface-hover text-secondary-theme hover:text-primary-theme transition-colors cursor-pointer"
            >
              <Package size={14} class="text-purple-400 shrink-0" />
              <div class="truncate flex-1">
                <div class="font-medium text-primary-theme flex items-center justify-between">
                  <span>Plugins & Extensions</span>
                  <span class="text-[9px] font-mono px-1.5 py-0.2 rounded bg-purple-500/15 text-purple-400 border border-purple-500/30">agy</span>
                </div>
                <div class="text-[10px] text-muted-theme">Bundles, skills & custom tools</div>
              </div>
            </button>
          {/if}

          <!-- CLI & Auth Setup -->
          {#if onOpenCliSetup}
            <button
              type="button"
              onclick={() => {
                showToolsMenu = false;
                onOpenCliSetup();
              }}
              class="w-full px-3 py-2 text-left flex items-center gap-2.5 hover:bg-surface-hover text-secondary-theme hover:text-primary-theme transition-colors cursor-pointer"
            >
              <Cpu size={14} class="text-accent-theme shrink-0" />
              <div class="truncate flex-1">
                <div class="font-medium text-primary-theme flex items-center justify-between">
                  <span>CLI & Auth Setup</span>
                  <span class="flex items-center gap-1 text-[10px] font-mono {envStatus?.installed ? 'text-emerald-400' : 'text-amber-400'}">
                    <span class="inline-block w-1.5 h-1.5 rounded-full {envStatus?.installed ? 'bg-emerald-400' : 'bg-amber-400'}"></span>
                    {envStatus?.installed ? 'Ready' : 'Setup'}
                  </span>
                </div>
                <div class="text-[10px] text-muted-theme">API keys & Vertex project</div>
              </div>
            </button>
          {/if}

          <!-- CLI Global Settings -->
          {#if onOpenCliSettings}
            <button
              type="button"
              onclick={() => {
                showToolsMenu = false;
                onOpenCliSettings();
              }}
              class="w-full px-3 py-2 text-left flex items-center gap-2.5 hover:bg-surface-hover text-secondary-theme hover:text-primary-theme transition-colors cursor-pointer"
            >
              <Sliders size={14} class="text-accent-theme shrink-0" />
              <div class="truncate flex-1">
                <div class="font-medium text-primary-theme">CLI Settings</div>
                <div class="text-[10px] text-muted-theme">Tokens, turns & compression</div>
              </div>
            </button>
          {/if}

          <div class="border-t border-subtle my-1"></div>

          <!-- Theme & Colors -->
          <button
            type="button"
            onclick={() => {
              showToolsMenu = false;
              onOpenThemeModal();
            }}
            class="w-full px-3 py-2 text-left flex items-center gap-2.5 hover:bg-surface-hover text-secondary-theme hover:text-primary-theme transition-colors cursor-pointer"
          >
            <Palette size={14} class="text-accent-theme shrink-0" />
            <div class="truncate flex-1">
              <div class="font-medium text-primary-theme flex items-center justify-between">
                <span>Theme & Colors</span>
                <span class="text-[10px] font-mono text-muted-theme capitalize">{themeManager.current.replace('-', ' ')}</span>
              </div>
              <div class="text-[10px] text-muted-theme">Palette, eye comfort & fonts</div>
            </div>
          </button>
        </div>
      {/if}
    </div>
  </div>

  <!-- Bottom App & Workspace Info -->
  <div class="border-t border-subtle bg-surface/40">
    <div class="px-3 pt-2 pb-1 flex items-center justify-between text-[10px] text-muted-theme">
      <div class="flex items-center gap-1.5">
        <span class="font-mono">v{appVersion}</span>
        <button
          type="button"
          onclick={onOpenCliSettings}
          class="px-1.5 py-0.5 rounded text-[10px] font-mono flex items-center gap-1 cursor-pointer bg-surface-elevated border border-subtle hover:border-theme-default text-primary-theme transition-colors"
          title="Active CLI Agent Engine (Click to switch)"
        >
          <span>{activeEngine === 'agy' ? '🚀 agy' : '♊ gemini'}</span>
        </button>
      </div>
      {#if onCheckUpdate}
        <button
          type="button"
          onclick={onCheckUpdate}
          disabled={isCheckingUpdate}
          class="hover:text-accent-theme flex items-center gap-1 transition-colors cursor-pointer disabled:opacity-50"
          title="Check for updates on WinGet"
        >
          <RefreshCw class="w-2.5 h-2.5 {isCheckingUpdate ? 'animate-spin text-accent-theme' : ''}" />
          <span>{isCheckingUpdate ? "Checking..." : "Check for Updates"}</span>
        </button>
      {/if}
    </div>
    <Tooltip
      class="w-full block"
      text="Current Workspace Root"
      subtext="All file attachments, git operations, and terminal executions run inside this directory."
      position="top"
    >
      <div class="px-3 pb-2 pt-0.5 text-[11px] text-muted-theme flex items-center justify-between cursor-help">
        <span class="truncate">{activeWorkspace?.path || "C:\\"}</span>
        <span class="text-accent-theme font-mono shrink-0 ml-2">{activeWorkspace?.model}</span>
      </div>
    </Tooltip>
  </div>
</aside>
{/if}
