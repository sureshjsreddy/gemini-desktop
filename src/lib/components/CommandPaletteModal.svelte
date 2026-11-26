<script lang="ts">
  import type { Workspace, Session, CliEngine } from "$lib/types";
  import { themeManager, THEMES, type ThemeId } from "$lib/theme.svelte";
  import { engineStore } from "$lib/stores/engineStore.svelte";
  import {
    Search,
    X,
    Sparkles,
    Terminal,
    FolderKanban,
    MessageSquare,
    Palette,
    Cpu,
    Server,
    Package,
    Sliders,
    Bug,
    FolderGit2,
    ListTodo,
    Minimize2,
    Plus,
    Check,
    ArrowRight,
  } from "lucide-svelte";
  import { tick } from "svelte";

  interface PaletteItem {
    id: string;
    title: string;
    subtitle?: string;
    category: "Commands" | "Workspaces" | "Recent Chats" | "Themes";
    icon: any;
    iconClass?: string;
    badge?: string;
    action: () => void;
  }

  let {
    isOpen = false,
    workspaces = [],
    activeWorkspace = null,
    sessions = [],
    activeSessionId = null,
    activeEngine = "gemini",
    onClose,
    onSelectWorkspace,
    onSelectSession,
    onNewSession,
    onSwitchEngine,
    onOpenTerminal,
    onOpenGitChanges,
    onOpenDebugDoctor,
    onOpenMcp,
    onOpenPlugins,
    onOpenCliSettings,
    onOpenCliSetup,
    onOpenTheme,
    onTriggerCompress,
    onTriggerPlan,
  }: {
    isOpen: boolean;
    workspaces: Workspace[];
    activeWorkspace: Workspace | null;
    sessions: Session[];
    activeSessionId: string | null;
    activeEngine?: CliEngine;
    onClose: () => void;
    onSelectWorkspace: (ws: Workspace) => void;
    onSelectSession: (s: Session) => void;
    onNewSession: () => void;
    onSwitchEngine: (engine: CliEngine) => void;
    onOpenTerminal?: () => void;
    onOpenGitChanges?: () => void;
    onOpenDebugDoctor?: () => void;
    onOpenMcp?: () => void;
    onOpenPlugins?: () => void;
    onOpenCliSettings?: () => void;
    onOpenCliSetup?: () => void;
    onOpenTheme?: () => void;
    onTriggerCompress?: () => void;
    onTriggerPlan?: () => void;
  } = $props();

  let query = $state("");
  let selectedIndex = $state(0);
  let inputElem = $state<HTMLInputElement | null>(null);

  // Focus input automatically when opened
  $effect(() => {
    if (isOpen) {
      query = "";
      selectedIndex = 0;
      setTimeout(() => inputElem?.focus(), 50);
    }
  });

  let allItems = $derived.by<PaletteItem[]>(() => {
    const items: PaletteItem[] = [];

    // 1. Primary Commands
    items.push({
      id: "cmd-new-chat",
      title: "New Conversation",
      subtitle: "Start a fresh chat in the active workspace",
      category: "Commands",
      icon: Plus,
      iconClass: "text-accent-theme",
      action: () => {
        onNewSession();
        onClose();
      },
    });

    for (const eng of engineStore.catalog) {
      if (eng.id === activeEngine) continue;
      items.push({
        id: `cmd-switch-${eng.id}`,
        title: `Switch CLI Engine to ${eng.displayName}`,
        subtitle: eng.description,
        category: "Commands",
        icon: Sparkles,
        iconClass: eng.id === "agy" ? "text-purple-400" : "text-sky-400",
        badge: "Engine",
        action: () => {
          onSwitchEngine(eng.id);
          onClose();
        },
      });
    }

    if (onOpenTerminal) {
      items.push({
        id: "cmd-terminal",
        title: "Toggle Integrated Terminal Drawer",
        subtitle: "Open PowerShell workspace console with .env context",
        category: "Commands",
        icon: Terminal,
        iconClass: "text-emerald-400",
        action: () => {
          onOpenTerminal();
          onClose();
        },
      });
    }

    if (onOpenGitChanges) {
      items.push({
        id: "cmd-git",
        title: "Open Git Changes & Diff Window",
        subtitle: "Review unstaged diffs, stage files, and generate AI commits",
        category: "Commands",
        icon: FolderGit2,
        iconClass: "text-amber-400",
        action: () => {
          onOpenGitChanges();
          onClose();
        },
      });
    }

    if (onOpenDebugDoctor) {
      items.push({
        id: "cmd-doctor",
        title: "Instant Debug Doctor",
        subtitle: "Sniff clipboard for stack traces & exceptions across C#, C++, Rust, Node",
        category: "Commands",
        icon: Bug,
        iconClass: "text-rose-400",
        action: () => {
          onOpenDebugDoctor();
          onClose();
        },
      });
    }

    if (onTriggerPlan) {
      items.push({
        id: "cmd-plan",
        title: "Interactive Plan Mode Workbench",
        subtitle: "Open task checklist and structured planning mode",
        category: "Commands",
        icon: ListTodo,
        iconClass: "text-sky-400",
        action: () => {
          onTriggerPlan();
          onClose();
        },
      });
    }

    if (onTriggerCompress) {
      items.push({
        id: "cmd-compress",
        title: "Compress Session Context (/compact)",
        subtitle: "Distill conversation history into dense milestone summary",
        category: "Commands",
        icon: Minimize2,
        iconClass: "text-accent-theme",
        action: () => {
          onTriggerCompress();
          onClose();
        },
      });
    }

    if (onOpenMcp) {
      items.push({
        id: "cmd-mcp",
        title: "Model Context Protocol (MCP) Servers",
        subtitle: "Configure global and workspace MCP servers",
        category: "Commands",
        icon: Server,
        iconClass: "text-indigo-400",
        action: () => {
          onOpenMcp();
          onClose();
        },
      });
    }

    if (onOpenPlugins) {
      items.push({
        id: "cmd-plugins",
        title: "Antigravity Plugins & Extensions Hub",
        subtitle: "Manage, install, and toggle native CLI agent plugins",
        category: "Commands",
        icon: Package,
        iconClass: "text-purple-400",
        action: () => {
          onOpenPlugins();
          onClose();
        },
      });
    }

    if (onOpenCliSetup) {
      items.push({
        id: "cmd-setup",
        title: "CLI & Authentication Setup",
        subtitle: "Configure API Key, Vertex AI, or Google Cloud ADC",
        category: "Commands",
        icon: Cpu,
        iconClass: "text-accent-theme",
        action: () => {
          onOpenCliSetup();
          onClose();
        },
      });
    }

    if (onOpenCliSettings) {
      items.push({
        id: "cmd-settings",
        title: "Global CLI Settings",
        subtitle: "Context compression threshold, file filters, and session retention",
        category: "Commands",
        icon: Sliders,
        iconClass: "text-secondary-theme",
        action: () => {
          onOpenCliSettings();
          onClose();
        },
      });
    }

    // 2. Workspaces
    for (const ws of workspaces) {
      const isActive = ws.id === activeWorkspace?.id;
      items.push({
        id: `ws-${ws.id}`,
        title: ws.name,
        subtitle: ws.path,
        category: "Workspaces",
        icon: FolderKanban,
        iconClass: isActive ? "text-accent-theme" : "text-secondary-theme",
        badge: isActive ? "Active" : undefined,
        action: () => {
          onSelectWorkspace(ws);
          onClose();
        },
      });
    }

    // 3. Recent Sessions
    for (const sess of sessions.slice(0, 10)) {
      const isActive = sess.id === activeSessionId;
      items.push({
        id: `sess-${sess.id}`,
        title: sess.title || "Untitled Conversation",
        subtitle: new Date(sess.updated_at || sess.created_at).toLocaleString(),
        category: "Recent Chats",
        icon: MessageSquare,
        iconClass: isActive ? "text-accent-theme" : "text-muted-theme",
        badge: isActive ? "Current" : undefined,
        action: () => {
          onSelectSession(sess);
          onClose();
        },
      });
    }

    // 4. Themes
    for (const th of THEMES) {
      const isCurrent = themeManager.current === th.id;
      items.push({
        id: `theme-${th.id}`,
        title: `Theme: ${th.name}`,
        subtitle: th.tagline,
        category: "Themes",
        icon: Palette,
        iconClass: "text-amber-400",
        badge: isCurrent ? "Active" : undefined,
        action: () => {
          themeManager.setTheme(th.id as ThemeId);
          onClose();
        },
      });
    }

    return items;
  });

  let filteredItems = $derived.by(() => {
    const q = query.trim().toLowerCase();
    if (!q) return allItems;

    // Direct command mode: if query starts with ">" or "/"
    const cleanQ = q.startsWith(">") || q.startsWith("/") ? q.slice(1).trim() : q;

    return allItems.filter((item) => {
      const matchTitle = item.title.toLowerCase().includes(cleanQ);
      const matchSub = (item.subtitle || "").toLowerCase().includes(cleanQ);
      const matchCat = item.category.toLowerCase().includes(cleanQ);
      return matchTitle || matchSub || matchCat;
    });
  });

  // Keep selected index within bounds
  $effect(() => {
    if (selectedIndex >= filteredItems.length) {
      selectedIndex = Math.max(0, filteredItems.length - 1);
    }
  });

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      if (filteredItems.length > 0) {
        selectedIndex = (selectedIndex + 1) % filteredItems.length;
      }
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      if (filteredItems.length > 0) {
        selectedIndex = (selectedIndex - 1 + filteredItems.length) % filteredItems.length;
      }
    } else if (e.key === "Enter") {
      e.preventDefault();
      if (filteredItems.length > 0 && filteredItems[selectedIndex]) {
        filteredItems[selectedIndex].action();
      }
    } else if (e.key === "Escape") {
      e.preventDefault();
      onClose();
    }
  }
</script>

{#if isOpen}
  <div
    class="fixed inset-0 bg-black/60 backdrop-blur-xs z-50 flex items-start justify-center pt-20 p-4 select-none"
    onclick={onClose}
    onkeydown={handleKeyDown}
    role="presentation"
  >
    <!-- Modal Card -->
    <div
      class="w-full max-w-2xl bg-surface border border-theme-default rounded-2xl shadow-2xl overflow-hidden flex flex-col max-h-[75vh] animate-in fade-in zoom-in-95 duration-100"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
      role="dialog"
      aria-modal="true"
      tabindex="-1"
    >
      <!-- Search Input Header -->
      <div class="p-3.5 border-b border-subtle flex items-center gap-3 bg-surface-elevated/40">
        <Search size={18} class="text-accent-theme shrink-0" />
        <input
          bind:this={inputElem}
          bind:value={query}
          onkeydown={handleKeyDown}
          type="text"
          placeholder="Type a command, workspace, conversation, or theme... (Esc to close)"
          class="flex-1 bg-transparent text-primary-theme text-xs sm:text-sm focus:outline-none placeholder:text-muted-theme"
        />
        <div class="flex items-center gap-2">
          <span class="text-[10px] font-mono px-1.5 py-0.5 rounded bg-surface border border-subtle text-muted-theme hidden sm:inline">
            Ctrl+P
          </span>
          <button
            type="button"
            onclick={onClose}
            class="p-1 text-secondary-theme hover:text-primary-theme rounded transition-colors cursor-pointer"
            aria-label="Close command palette"
          >
            <X size={16} />
          </button>
        </div>
      </div>

      <!-- Filtered Results List -->
      <div class="flex-1 overflow-y-auto p-2 space-y-1">
        {#if filteredItems.length === 0}
          <div class="py-12 text-center text-xs text-muted-theme">
            No matching commands, workspaces, or conversations found for "<span class="text-primary-theme font-semibold">{query}</span>".
          </div>
        {:else}
          {#each filteredItems as item, idx (item.id)}
            {@const Icon = item.icon}
            {@const isSelected = idx === selectedIndex}
            <button
              type="button"
              onclick={() => item.action()}
              onmouseenter={() => (selectedIndex = idx)}
              class="w-full text-left px-3 py-2.5 rounded-xl transition-all cursor-pointer flex items-center justify-between gap-3 {isSelected ? 'bg-surface-elevated border border-accent-theme/40 shadow-xs' : 'border border-transparent hover:bg-surface-elevated/50'}"
            >
              <div class="flex items-center gap-3 min-w-0 flex-1">
                <div class="w-7 h-7 rounded-lg bg-surface border border-subtle flex items-center justify-center shrink-0 {isSelected ? 'bg-accent-subtle/40 border-accent-subtle' : ''}">
                  <Icon size={14} class={item.iconClass || 'text-secondary-theme'} />
                </div>
                <div class="min-w-0 flex-1">
                  <div class="flex items-center gap-2">
                    <span class="text-xs font-medium text-primary-theme truncate {isSelected ? 'text-accent-theme' : ''}">
                      {item.title}
                    </span>
                    {#if item.badge}
                      <span class="text-[10px] px-1.5 py-0.2 rounded font-mono font-semibold {item.badge === 'Active' || item.badge === 'Current' ? 'bg-accent-subtle text-accent-theme border border-accent-subtle' : 'bg-surface-elevated text-secondary-theme'}">
                        {item.badge}
                      </span>
                    {/if}
                  </div>
                  {#if item.subtitle}
                    <div class="text-[11px] text-muted-theme truncate font-mono mt-0.5">
                      {item.subtitle}
                    </div>
                  {/if}
                </div>
              </div>

              <div class="flex items-center gap-2 shrink-0">
                <span class="text-[10px] text-muted-theme px-1.5 py-0.5 rounded bg-surface/60 font-mono">
                  {item.category}
                </span>
                {#if isSelected}
                  <ArrowRight size={13} class="text-accent-theme animate-in fade-in" />
                {/if}
              </div>
            </button>
          {/each}
        {/if}
      </div>

      <!-- Keyboard Footer Help -->
      <div class="px-4 py-2 border-t border-subtle bg-surface-elevated/30 flex items-center justify-between text-[11px] text-muted-theme select-none">
        <div class="flex items-center gap-3">
          <span><kbd class="font-mono bg-surface px-1.5 py-0.5 rounded border border-subtle">↑</kbd> <kbd class="font-mono bg-surface px-1.5 py-0.5 rounded border border-subtle">↓</kbd> Navigate</span>
          <span><kbd class="font-mono bg-surface px-1.5 py-0.5 rounded border border-subtle">Enter</kbd> Run</span>
          <span><kbd class="font-mono bg-surface px-1.5 py-0.5 rounded border border-subtle">Esc</kbd> Close</span>
        </div>
        <div class="font-mono text-[10px] opacity-75">
          {filteredItems.length} result{filteredItems.length === 1 ? '' : 's'}
        </div>
      </div>
    </div>
  </div>
{/if}
