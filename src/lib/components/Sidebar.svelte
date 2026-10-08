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
    Palette,
    Server,
    Terminal,
    FolderTree,
    PanelLeftClose,
    PanelLeftOpen,
    RefreshCw,
    Cpu,
  } from "lucide-svelte";
  import Tooltip from "$lib/components/Tooltip.svelte";
  import { APP_VERSION_FALLBACK } from "$lib/version";

  let {
    isOpen = true,
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
    onCheckUpdate,
    isCheckingUpdate = false,
    onOpenCliSetup,
  }: {
    isOpen?: boolean;
    workspaces: Workspace[];
    activeWorkspace: Workspace | null;
    sessions: Session[];
    activeSession: Session | null;
    generatingSessionIds?: Set<string>;
    appVersion?: string;
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
  } = $props();

  let showWorkspaceMenu = $state(false);
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

  <!-- Quick Action Navigation -->
  <div class="p-3 space-y-1 border-b border-subtle">
    <Tooltip
      class="w-full block"
      text="New Conversation"
      subtext="Start a fresh chat thread with a clean context"
      shortcut="Ctrl+N"
      position="right"
    >
      <button
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
        onclick={onOpenSearch}
        class="w-full flex items-center gap-2.5 px-3 py-2 rounded-lg hover:bg-surface-hover text-secondary-theme hover:text-primary-theme text-xs transition-colors cursor-pointer"
      >
        <Search size={14} class="text-muted-theme" />
        <span>Search History</span>
        <span class="ml-auto text-[10px] text-muted-theme font-mono">Ctrl+K</span>
      </button>
    </Tooltip>

    <Tooltip
      class="w-full block"
      text="Prompt Library"
      subtext="Browse, manage, and quickly insert pre-built prompt templates for coding, refactoring, and review"
      position="right"
    >
      <button
        onclick={onOpenTemplates}
        class="w-full flex items-center gap-2.5 px-3 py-2 rounded-lg hover:bg-surface-hover text-secondary-theme hover:text-primary-theme text-xs transition-colors cursor-pointer"
      >
        <BookOpen size={14} class="text-muted-theme" />
        <span>Prompt Library</span>
      </button>
    </Tooltip>

    <Tooltip
      class="w-full block"
      text="Theme & Colors"
      subtext="Customize UI palette with Midnight Dark, Cyberpunk, Obsidian, or custom colors"
      position="right"
    >
      <button
        onclick={onOpenThemeModal}
        class="w-full flex items-center gap-2.5 px-3 py-2 rounded-lg hover:bg-surface-hover text-secondary-theme hover:text-primary-theme text-xs transition-colors cursor-pointer"
      >
        <Palette size={14} class="text-accent-theme" />
        <span>Theme & Colors</span>
        <span class="ml-auto text-[10px] capitalize text-muted-theme font-medium">{themeManager.current.replace('-', ' ')}</span>
      </button>
    </Tooltip>

    <Tooltip
      class="w-full block"
      text="Model Context Protocol (MCP)"
      subtext="Connect external tools, databases, and services (Azure DevOps, GitHub, SQLite) into Gemini CLI"
      shortcut="Ctrl+M"
      position="right"
    >
      <button
        onclick={onOpenMcpModal}
        class="w-full flex items-center gap-2.5 px-3 py-2 rounded-lg hover:bg-surface-hover text-secondary-theme hover:text-primary-theme text-xs transition-colors cursor-pointer"
      >
        <Server size={14} class="text-accent-theme" />
        <span>MCP Servers</span>
        <span class="ml-auto text-[10px] text-muted-theme font-mono">Ctrl+M</span>
      </button>
    </Tooltip>

    <!-- CLI & Auth Setup -->
    {#if onOpenCliSetup}
      <Tooltip
        class="w-full block"
        text="CLI & Auth Setup"
        subtext="Install Gemini CLI, configure Google AI Studio key or Corporate Vertex AI Project"
        position="right"
      >
        <button
          onclick={onOpenCliSetup}
          class="w-full flex items-center gap-2.5 px-3 py-2 rounded-lg hover:bg-surface-hover text-secondary-theme hover:text-primary-theme text-xs transition-colors cursor-pointer"
        >
          <Cpu size={14} class="text-accent-theme" />
          <span>CLI & Auth Setup</span>
          <span class="ml-auto flex items-center gap-1">
            <span class="inline-block w-1.5 h-1.5 rounded-full {envStatus?.installed ? 'bg-emerald-400' : 'bg-amber-400'}"></span>
          </span>
        </button>
      </Tooltip>
    {/if}

    {#if onToggleTerminal}
      <Tooltip
        class="w-full block"
        text="Terminal Console"
        subtext="Open embedded PowerShell drawer in workspace directory to inspect git, run tests, and reload .env"
        shortcut="Ctrl+`"
        position="right"
      >
        <button
          onclick={onToggleTerminal}
          class="w-full flex items-center gap-2.5 px-3 py-2 rounded-lg hover:bg-surface-hover text-secondary-theme hover:text-primary-theme text-xs transition-colors cursor-pointer"
        >
          <Terminal size={14} class="text-accent-theme" />
          <span>Terminal Console</span>
          <span class="ml-auto text-[10px] text-muted-theme font-mono">Ctrl+`</span>
        </button>
      </Tooltip>
    {/if}

    {#if onToggleExplorer}
      <Tooltip
        class="w-full block"
        text="Workspace Explorer"
        subtext="Toggle workspace file and folder tree"
        shortcut="Ctrl+Alt+L"
        position="right"
      >
        <button
          type="button"
          onclick={onToggleExplorer}
          class="w-full flex items-center gap-2.5 px-3 py-2 rounded-lg hover:bg-surface-hover text-secondary-theme hover:text-primary-theme text-xs transition-colors cursor-pointer"
        >
          <FolderTree size={14} class="text-[#a855f7]" />
          <span>Workspace Explorer</span>
          <span class="ml-auto text-[10px] text-muted-theme font-mono">Ctrl+Alt+L</span>
        </button>
      </Tooltip>
    {/if}
  </div>

  <!-- Chat History Sessions List -->
  <div class="flex-1 overflow-y-auto p-2 space-y-0.5">
    <div class="px-2 py-1.5 text-[11px] font-semibold uppercase text-muted-theme tracking-wider">
      Recent Chats
    </div>

    {#if sessions.length === 0}
      <div class="px-3 py-6 text-center text-xs text-muted-theme">
        No conversations yet in this workspace.
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
          <div class="flex items-center gap-2 truncate flex-1 mr-1">
            {#if generatingSessionIds?.has(session.id)}
              <span class="relative flex h-2 w-2 shrink-0">
                <span class="animate-ping absolute inline-flex h-full w-full rounded-full bg-accent-theme opacity-75"></span>
                <span class="relative inline-flex rounded-full h-2 w-2 bg-accent-theme"></span>
              </span>
            {:else}
              <MessageSquare size={14} class="shrink-0 {activeSession?.id === session.id ? 'text-accent-theme' : 'text-muted-theme'}" />
            {/if}
            <span class="truncate">{session.title}</span>
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

  <!-- Bottom App & Workspace Info -->
  <div class="border-t border-subtle bg-surface/40">
    <div class="px-3 pt-2 pb-1 flex items-center justify-between text-[10px] text-muted-theme">
      <span class="font-mono">v{appVersion}</span>
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
