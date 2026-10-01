<script lang="ts">
  import type { Workspace, WorkspaceFileEntry, TreeNode } from "$lib/types";
  import ExplorerContextMenu from "$lib/components/ExplorerContextMenu.svelte";
  import ExplorerTreeRow from "$lib/components/ExplorerTreeRow.svelte";
  import { invoke } from "@tauri-apps/api/core";
  import {
    Folder,
    FolderOpen,
    ChevronRight,
    RotateCw,
    ChevronsDownUp,
    PanelRightClose,
    PanelRight,
    Copy,
    ExternalLink,
    FileCode,
    FileText,
    Check,
    Layers,
    AtSign,
    Database,
    Terminal,
    Image,
    Eye,
    EyeOff,
    FolderTree,
    Paperclip,
    CheckSquare,
    Square,
    Search,
    X,
    CornerDownLeft,
  } from "lucide-svelte";
  import Tooltip from "$lib/components/Tooltip.svelte";
  import { openPath, revealItemInDir } from "@tauri-apps/plugin-opener";
  import { dialogManager } from "$lib/dialog.svelte";
  import { getFileIconMeta } from "$lib/utils/fileIcons";
  import { isNodeVisible } from "$lib/utils/fileFilters";

  export type { TreeNode };

  let {
    workspace = null,
    workspaceFiles = [],
    isOpen = true,
    onToggle,
    onRefresh,
    onInsertMention,
    onAttachItem,
    onAttachMultiple,
  }: {
    workspace: Workspace | null;
    workspaceFiles?: WorkspaceFileEntry[];
    isOpen: boolean;
    onToggle: () => void;
    onRefresh?: () => void;
    onInsertMention?: (relativePath: string) => void;
    onAttachItem?: (relativePath: string, isDir: boolean, name?: string) => void;
    onAttachMultiple?: (items: { path: string; isDir: boolean; name?: string }[]) => void;
  } = $props();

  // Dynamic Root State
  let rootNodes = $state<TreeNode[]>([]);
  let isLoadingRoot = $state(false);
  let expandedDirs = $state<Set<string>>(new Set());
  let selectedNode = $state<TreeNode | null>(null);
  let copiedPath = $state<string | null>(null);
  let attachedPath = $state<string | null>(null);
  let isRefreshing = $state(false);

  // Hidden files toggle state (hidden by default)
  let showHiddenFiles = $state(false);

  // Multi-select / Choose mode
  let isSelectMode = $state(false);
  let checkedItems = $state<Map<string, { path: string; isDir: boolean; name: string }>>(new Map());

  // Context Menu State
  let contextMenu = $state<{ x: number; y: number; node: TreeNode } | null>(null);

  // Resizable Panel Width
  let explorerWidth = $state(310);
  let isDragging = $state(false);

  let visibleRootItems = $derived.by(() => {
    return rootNodes.filter((n) => isNodeVisible(n, showHiddenFiles));
  });

  // Count files and directories across loaded nodes
  let visibleFilesCount = $derived.by(() => {
    let count = 0;
    function countFiles(nodes: TreeNode[]) {
      for (const n of nodes) {
        if (!isNodeVisible(n, showHiddenFiles)) continue;
        if (!n.isDir) count++;
        else if (n.isLoaded && expandedDirs.has(n.path)) countFiles(n.children);
      }
    }
    countFiles(rootNodes);
    return count;
  });

  let visibleDirsCount = $derived.by(() => {
    let count = 0;
    function countDirs(nodes: TreeNode[]) {
      for (const n of nodes) {
        if (!isNodeVisible(n, showHiddenFiles)) continue;
        if (n.isDir) {
          count++;
          if (n.isLoaded && expandedDirs.has(n.path)) countDirs(n.children);
        }
      }
    }
    countDirs(rootNodes);
    return count;
  });

  let hiddenItemsCount = $derived.by(() => {
    let count = 0;
    function countHidden(nodes: TreeNode[]) {
      for (const n of nodes) {
        if (n.name.startsWith(".")) count++;
        if (n.isDir && n.isLoaded && expandedDirs.has(n.path)) countHidden(n.children);
      }
    }
    countHidden(rootNodes);
    return count;
  });

  // Search State (Off-thread Native Filesystem Search, Explicit Enter Submission, Files Only)
  let searchQuery = $state("");
  let activeSearch = $state("");
  let isSearching = $state(false);
  let searchResults = $state<TreeNode[]>([]);
  let searchInputEl: HTMLInputElement | null = $state(null);
  let searchReqSeq = 0;

  let visibleSearchResults = $derived.by(() => {
    return searchResults.filter((n) => isNodeVisible(n, showHiddenFiles));
  });

  async function handleSearchSubmit() {
    const q = searchQuery.trim();
    if (!q) {
      clearSearch();
      return;
    }
    if (!workspace) return;
    const thisReq = ++searchReqSeq;
    isSearching = true;
    activeSearch = q;
    try {
      const entries = await invoke<WorkspaceFileEntry[]>("search_workspace_files", {
        workspaceId: workspace.id,
        query: q,
        maxResults: 100,
      });
      // Ignore if another search was triggered or cleared
      if (thisReq !== searchReqSeq || !activeSearch) return;

      searchResults = entries.map((entry) => ({
        name: entry.name,
        path: entry.relative_path,
        isDir: false,
        extension: entry.extension,
        children: [],
        isLoaded: true,
        isLoading: false,
      }));
    } catch (err) {
      if (thisReq === searchReqSeq) {
        console.error("Search failed:", err);
        searchResults = [];
      }
    } finally {
      if (thisReq === searchReqSeq) {
        isSearching = false;
      }
    }
  }

  function clearSearch() {
    searchReqSeq++;
    searchQuery = "";
    activeSearch = "";
    searchResults = [];
    isSearching = false;
    invoke("cancel_workspace_search").catch(() => {});
    searchInputEl?.focus();
  }

  let currentWorkspaceId = $state<string | null>(null);

  // Reset expansion and reload root nodes on workspace change or initial load
  $effect(() => {
    const wsId = workspace?.id || null;
    if (wsId !== currentWorkspaceId) {
      currentWorkspaceId = wsId;
      expandedDirs = new Set();
      checkedItems = new Map();
      selectedNode = null;
      rootNodes = [];
      searchQuery = "";
      activeSearch = "";
      searchResults = [];
      if (wsId) {
        loadRootNodes(wsId);
      }
    }
  });

  async function loadRootNodes(wsId: string) {
    isLoadingRoot = true;
    try {
      const entries = await invoke<WorkspaceFileEntry[]>("read_workspace_dir", {
        workspaceId: wsId,
        relativePath: "",
      });
      rootNodes = entries.map((entry) => ({
        name: entry.name,
        path: entry.relative_path,
        isDir: entry.is_dir,
        extension: entry.extension,
        children: [],
        isLoaded: false,
        isLoading: false,
      }));
    } catch (err) {
      console.error("Failed to load root workspace directory:", err);
      rootNodes = [];
    } finally {
      isLoadingRoot = false;
    }
  }

  // Dynamic lazy loading: fetch subfolder contents only when folder is opened
  async function toggleDir(node: TreeNode, e?: MouseEvent) {
    if (e) e.stopPropagation();
    if (!node.isDir) return;

    const next = new Set(expandedDirs);
    if (next.has(node.path)) {
      next.delete(node.path);
      expandedDirs = next;
      return;
    }

    if (!node.isLoaded && workspace) {
      node.isLoading = true;
      try {
        const entries = await invoke<WorkspaceFileEntry[]>("read_workspace_dir", {
          workspaceId: workspace.id,
          relativePath: node.path,
        });
        node.children = entries.map((entry) => ({
          name: entry.name,
          path: entry.relative_path,
          isDir: entry.is_dir,
          extension: entry.extension,
          children: [],
          isLoaded: false,
          isLoading: false,
        }));
        node.isLoaded = true;
      } catch (err) {
        console.error(`Failed to load directory ${node.path}:`, err);
      } finally {
        node.isLoading = false;
      }
    }

    next.add(node.path);
    expandedDirs = next;
  }

  function collapseAll() {
    expandedDirs = new Set();
  }

  async function handleRefreshClick() {
    if (isRefreshing || !workspace) return;
    isRefreshing = true;
    try {
      await loadRootNodes(workspace.id);
      expandedDirs = new Set();
      if (onRefresh) await onRefresh();
    } finally {
      setTimeout(() => {
        isRefreshing = false;
      }, 500);
    }
  }

  function handleAttachNode(node: TreeNode, e?: MouseEvent) {
    if (e) e.stopPropagation();
    if (onAttachItem) {
      onAttachItem(node.path, node.isDir, node.name);
      attachedPath = node.path;
      setTimeout(() => {
        if (attachedPath === node.path) attachedPath = null;
      }, 1500);
    }
    contextMenu = null;
  }

  function handleAttachChecked() {
    if (checkedItems.size === 0) return;
    const items = Array.from(checkedItems.values());
    if (onAttachMultiple) {
      onAttachMultiple(items);
    } else if (onAttachItem) {
      for (const it of items) {
        onAttachItem(it.path, it.isDir, it.name);
      }
    }
    checkedItems = new Map();
    isSelectMode = false;
  }

  function toggleCheckItem(node: TreeNode, e?: Event) {
    if (e) e.stopPropagation();
    const next = new Map(checkedItems);
    if (next.has(node.path)) {
      next.delete(node.path);
    } else {
      next.set(node.path, { path: node.path, isDir: node.isDir, name: node.name });
    }
    checkedItems = next;
  }

  function clearChecked() {
    checkedItems = new Map();
  }

  function handleInsertMention(node: TreeNode, e?: MouseEvent) {
    if (e) e.stopPropagation();
    if (onInsertMention) {
      const mentionPath = node.isDir
        ? node.path.endsWith("/") ? node.path : `${node.path}/`
        : node.path;
      onInsertMention(mentionPath);
    }
    contextMenu = null;
  }

  async function handleCopyPath(nodePath: string, e?: MouseEvent) {
    if (e) e.stopPropagation();
    try {
      await navigator.clipboard.writeText(nodePath);
      copiedPath = nodePath;
      setTimeout(() => {
        if (copiedPath === nodePath) copiedPath = null;
      }, 1500);
    } catch (err) {
      console.error("Failed to copy path:", err);
    }
    contextMenu = null;
  }

  function resolveFullPath(relPath: string): string {
    if (!relPath) return "";
    if (/^[a-zA-Z]:[/\\]/.test(relPath) || relPath.startsWith("\\\\")) {
      return relPath.replace(/\//g, "\\");
    }
    if (!workspace?.path) return relPath;
    const base = workspace.path.replace(/[/\\]+$/, "");
    const cleanRel = relPath.replace(/^[/\\]+/, "");
    return `${base}\\${cleanRel}`.replace(/\//g, "\\");
  }

  async function handleOpenFile(nodePath: string, e?: MouseEvent, options?: { forceNotepad?: boolean }) {
    if (e) e.stopPropagation();
    const full = resolveFullPath(nodePath);
    const forceNotepad = options?.forceNotepad ?? false;
    try {
      await invoke("open_workspace_file", {
        path: full,
        withApp: forceNotepad ? "notepad" : null,
      });
    } catch (err) {
      console.error("Failed to open file:", err);
      await dialogManager.alert(
        `Unable to open "${nodePath}".\n\n${err instanceof Error ? err.message : String(err)}`,
        "Cannot Open File"
      );
    }
    contextMenu = null;
  }

  async function handleRevealInExplorer(nodePath: string, e?: MouseEvent) {
    if (e) e.stopPropagation();
    const full = resolveFullPath(nodePath);
    try {
      await revealItemInDir(full);
    } catch (err) {
      console.warn("Could not reveal file in Windows explorer:", err);
    }
    contextMenu = null;
  }

  function handleContextMenu(node: TreeNode, e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    selectedNode = node;
    const clickX = e.clientX;
    const clickY = e.clientY;
    // Keep context menu within viewport bounds
    const x = Math.min(clickX, window.innerWidth - 200);
    const y = Math.min(clickY, window.innerHeight - 240);
    contextMenu = { x, y, node };
  }

  // Handle panel resizing via dragging left border
  function handleMouseDown(e: MouseEvent) {
    e.preventDefault();
    isDragging = true;
    const startX = e.clientX;
    const startWidth = explorerWidth;

    function handleMouseMove(moveEvent: MouseEvent) {
      const deltaX = startX - moveEvent.clientX;
      explorerWidth = Math.max(240, Math.min(600, startWidth + deltaX));
    }

    function handleMouseUp() {
      isDragging = false;
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
    }

    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);
  }

  export function focusSearch() {
    if (searchInputEl) {
      searchInputEl.focus();
      searchInputEl.select();
    }
  }
</script>

{#if !isOpen}
  <!-- Workspace Explorer Auto-Hide Collapsed Dock Tab on the Right Margin -->
  <aside
    class="w-7 h-full bg-sidebar border-l border-subtle flex flex-col items-center py-3 select-none z-10 transition-colors hover:bg-surface"
    aria-label="Workspace Explorer collapsed dock"
  >
    <Tooltip text="Expand Workspace Explorer" shortcut="Ctrl+Alt+L" position="left">
      <button
        type="button"
        onclick={onToggle}
        class="w-full flex flex-col items-center gap-3 text-secondary-theme hover:text-accent-theme cursor-pointer group focus:outline-none"
        aria-label="Expand Workspace Explorer panel"
      >
        <div class="p-1 rounded group-hover:bg-surface-elevated text-accent-theme">
          <Layers size={14} class="text-[#a855f7]" />
        </div>
        <span
          class="text-[11px] font-medium tracking-wide uppercase text-muted-theme group-hover:text-primary-theme"
          style="writing-mode: vertical-rl; transform: rotate(180deg);"
        >
          Workspace Explorer
        </span>
      </button>
    </Tooltip>
  </aside>
{:else}
  <!-- Workspace Explorer Full Right Panel -->
  <aside
    class="relative h-full bg-sidebar border-l border-theme-default flex flex-col shrink-0 select-none z-10 overflow-hidden shadow-lg transition-all"
    style="width: {explorerWidth}px;"
    aria-label="Workspace Explorer"
  >
    <!-- Left Resizer Drag Handle -->
    <button
      type="button"
      onmousedown={handleMouseDown}
      aria-label="Resize Workspace Explorer"
      class="absolute left-0 top-0 bottom-0 w-1 cursor-col-resize hover:bg-accent-theme transition-colors z-20 {isDragging ? 'bg-accent-theme' : 'bg-transparent'}"
    ></button>

    <!-- Top Header: Workspace Explorer Title & Toolbar -->
    <header class="h-[34px] min-h-[34px] px-2.5 bg-surface border-b border-subtle flex items-center justify-between gap-1 relative z-20">
      <div class="flex items-center gap-1.5 truncate">
        <!-- Explorer Cube Icon -->
        <div class="w-4 h-4 flex items-center justify-center text-[#a855f7] shrink-0">
          <Layers size={14} />
        </div>
        <span class="text-xs font-semibold text-primary-theme tracking-tight truncate">
          Workspace Explorer
        </span>
      </div>

      <!-- Action Icons Toolbar -->
      <div class="flex items-center gap-0.5 shrink-0 text-muted-theme">
        <!-- Choose / Multi-Select Mode -->
        <Tooltip text={isSelectMode ? "Exit Selection Mode" : "Choose / Multi-Select Items to Attach"} position="bottom">
          <button
            type="button"
            onclick={() => {
              isSelectMode = !isSelectMode;
              if (!isSelectMode) checkedItems = new Map();
            }}
            class="p-1 rounded hover:bg-surface-hover transition-colors cursor-pointer {isSelectMode ? 'bg-accent-subtle text-accent-theme' : 'hover:text-primary-theme'}"
            aria-label="Toggle multi-select mode"
          >
            <CheckSquare size={13} />
          </button>
        </Tooltip>

        <!-- Toggle Hidden Files & Folders -->
        <Tooltip
          text={showHiddenFiles
            ? "Hide Hidden Files & Folders (.*)"
            : `Show Hidden Files & Folders${hiddenItemsCount > 0 ? ` (${hiddenItemsCount} hidden)` : ""}`}
          position="bottom"
        >
          <button
            type="button"
            onclick={() => (showHiddenFiles = !showHiddenFiles)}
            class="p-1 rounded hover:bg-surface-hover transition-colors cursor-pointer {showHiddenFiles ? 'bg-accent-subtle text-accent-theme' : 'hover:text-primary-theme'}"
            aria-label={showHiddenFiles ? "Hide hidden files" : "Show hidden files"}
          >
            {#if showHiddenFiles}
              <Eye size={13} />
            {:else}
              <EyeOff size={13} />
            {/if}
          </button>
        </Tooltip>

        <!-- Refresh -->
        <Tooltip text="Refresh Files" shortcut="F5" position="bottom">
          <button
            type="button"
            onclick={handleRefreshClick}
            class="p-1 rounded hover:bg-surface-hover hover:text-primary-theme transition-colors cursor-pointer"
            aria-label="Refresh workspace files"
          >
            <RotateCw size={13} class={isRefreshing ? "animate-spin text-accent-theme" : ""} />
          </button>
        </Tooltip>

        <!-- Collapse All -->
        <Tooltip text="Collapse All Folders" position="bottom">
          <button
            type="button"
            onclick={collapseAll}
            class="p-1 rounded hover:bg-surface-hover hover:text-primary-theme transition-colors cursor-pointer"
            aria-label="Collapse all folders"
          >
            <ChevronsDownUp size={13} />
          </button>
        </Tooltip>

        <!-- Collapse to Right (Close/Dock) -->
        <Tooltip text="Collapse to Right" shortcut="Ctrl+Alt+L" position="bottom">
          <button
            type="button"
            onclick={onToggle}
            class="p-1 rounded hover:bg-surface-hover hover:text-primary-theme transition-colors cursor-pointer"
            aria-label="Collapse Workspace Explorer to right"
          >
            <PanelRightClose size={13} />
          </button>
        </Tooltip>
      </div>
    </header>

    <!-- Search / Filter Bar (Explicit Enter Submission, Files Only) -->
    <div class="px-2 py-1.5 bg-sidebar border-b border-subtle">
      <div class="relative flex items-center">
        <Search size={12} class="absolute left-2 text-muted-theme pointer-events-none" />
        <input
          bind:this={searchInputEl}
          type="text"
          bind:value={searchQuery}
          onkeydown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault();
              handleSearchSubmit();
            } else if (e.key === "Escape") {
              e.preventDefault();
              clearSearch();
            }
          }}
          placeholder="Filter files (Press Enter)..."
          class="w-full pl-7 pr-12 py-1 text-xs bg-surface text-primary-theme placeholder:text-muted-theme rounded border border-theme-default focus:border-accent-theme focus:outline-none transition-colors"
        />
        <div class="absolute right-1.5 flex items-center gap-0.5">
          {#if searchQuery}
            <button
              type="button"
              onclick={clearSearch}
              class="p-0.5 text-muted-theme hover:text-primary-theme cursor-pointer"
              aria-label="Clear search"
            >
              <X size={12} />
            </button>
          {/if}
          <button
            type="button"
            onclick={handleSearchSubmit}
            disabled={!searchQuery.trim() || isSearching}
            class="p-0.5 text-muted-theme hover:text-accent-theme disabled:opacity-40 cursor-pointer"
            title="Search files (Enter)"
            aria-label="Submit search"
          >
            {#if isSearching}
              <RotateCw size={12} class="animate-spin text-accent-theme" />
            {:else}
              <CornerDownLeft size={12} />
            {/if}
          </button>
        </div>
      </div>
    </div>

    <!-- Multi-Select Action Banner (When Items are Checked) -->
    {#if isSelectMode || checkedItems.size > 0}
      <div class="px-2.5 py-1.5 bg-surface-elevated border-b border-subtle flex items-center justify-between gap-2 text-xs">
        <span class="text-accent-theme font-medium flex items-center gap-1.5 truncate">
          <CheckSquare size={13} />
          <span>{checkedItems.size} chosen</span>
        </span>
        <div class="flex items-center gap-1.5 shrink-0">
          {#if checkedItems.size > 0}
            <button
              type="button"
              onclick={clearChecked}
              class="text-[11px] text-muted-theme hover:text-primary-theme px-1.5 py-0.5 rounded cursor-pointer"
            >
              Clear
            </button>
            <button
              type="button"
              onclick={handleAttachChecked}
              class="flex items-center gap-1 px-2 py-0.5 rounded bg-accent-theme hover:bg-accent-hover text-white font-medium text-[11px] transition-colors cursor-pointer shadow-xs"
            >
              <Paperclip size={11} />
              <span>Attach to Chat</span>
            </button>
          {:else}
            <span class="text-[11px] text-muted-theme">Check files or folders below</span>
          {/if}
        </div>
      </div>
    {/if}

    <!-- Tree Structure Container -->
    <div class="flex-1 overflow-y-auto overflow-x-hidden py-1 text-xs font-sans">
      {#if activeSearch}
        <!-- Filtered Search Results Header -->
        <div
          class="flex items-center justify-between px-2 py-1 text-xs font-medium text-secondary-theme bg-surface-elevated/40 border-b border-subtle/50 select-none group"
        >
          <div class="flex items-center gap-1.5 truncate">
            <Search size={12} class="text-accent-theme shrink-0" />
            <span class="truncate">
              Files matching "{activeSearch}"
            </span>
          </div>
          <div class="flex items-center gap-1.5 shrink-0">
            <span class="text-[10px] text-muted-theme font-mono">
              {visibleSearchResults.length} match{visibleSearchResults.length === 1 ? "" : "es"}
            </span>
            <button
              type="button"
              onclick={clearSearch}
              class="text-[10px] text-accent-theme hover:underline cursor-pointer"
            >
              Clear
            </button>
          </div>
        </div>

        {#if isSearching}
          <div class="p-8 text-center text-muted-theme text-xs flex flex-col items-center justify-center gap-2">
            <RotateCw size={18} class="animate-spin text-accent-theme" />
            <span>Searching files...</span>
          </div>
        {:else if visibleSearchResults.length === 0}
          <div class="p-6 text-center text-muted-theme text-xs flex flex-col items-center gap-2">
            <Search size={22} class="opacity-40 text-muted-theme" />
            <span>No files match "{activeSearch}"</span>
            <button
              type="button"
              onclick={clearSearch}
              class="text-accent-theme hover:underline cursor-pointer text-xs"
            >
              Clear search filter
            </button>
          </div>
        {:else}
          <div class="py-0.5">
            {#each visibleSearchResults as node (node.path)}
              {@render searchResultRow(node)}
            {/each}
          </div>
        {/if}
      {:else}
        <!-- Workspace Root Node -->
        <div
          class="flex items-center justify-between px-2 py-1 text-xs font-medium text-secondary-theme bg-surface-elevated/40 border-b border-subtle/50 select-none group"
        >
          <div class="flex items-center gap-1.5 truncate">
            <div class="w-3.5 h-3.5 flex items-center justify-center text-[#a855f7] shrink-0">
              <Layers size={13} />
            </div>
            <span class="truncate" title={workspace?.path || "No workspace root"}>
              Workspace '{workspace?.name || "Workspace"}'
            </span>
          </div>
          <div class="flex items-center gap-1 shrink-0">
            <span class="text-[10px] text-muted-theme font-mono">
              {visibleFilesCount} files, {visibleDirsCount} folders{#if !showHiddenFiles && hiddenItemsCount > 0}
                <span class="opacity-75"> ({hiddenItemsCount} hidden)</span>
              {/if}
            </span>
          </div>
        </div>

        {#if isLoadingRoot}
          <div class="p-8 text-center text-muted-theme text-xs flex flex-col items-center justify-center gap-2">
            <RotateCw size={18} class="animate-spin text-accent-theme" />
            <span>Loading workspace files...</span>
          </div>
        {:else if visibleRootItems.length === 0}
          <div class="p-4 text-center text-muted-theme text-xs flex flex-col items-center gap-2">
            <FolderTree size={24} class="opacity-40 text-muted-theme" />
            {#if hiddenItemsCount > 0 && !showHiddenFiles}
              <span>Only hidden files exist ({hiddenItemsCount} hidden)</span>
              <button
                type="button"
                onclick={() => (showHiddenFiles = true)}
                class="text-accent-theme hover:underline cursor-pointer"
              >
                Show hidden files
              </button>
            {:else}
              <span>No files found in this workspace</span>
              {#if onRefresh}
                <button
                  type="button"
                  onclick={handleRefreshClick}
                  class="text-accent-theme hover:underline cursor-pointer"
                >
                  Scan workspace files
                </button>
              {/if}
            {/if}
          </div>
        {:else}
          <!-- Tree Nodes with Recursive ExplorerTreeRow -->
          <div class="py-0.5">
            {#each visibleRootItems as node (node.path)}
              <ExplorerTreeRow
                {node}
                depth={0}
                {expandedDirs}
                {selectedNode}
                {checkedItems}
                {isSelectMode}
                {attachedPath}
                {copiedPath}
                {showHiddenFiles}
                onSelectNode={(n) => (selectedNode = n)}
                onToggleDir={toggleDir}
                onToggleCheckItem={toggleCheckItem}
                onOpenFile={handleOpenFile}
                onContextMenu={handleContextMenu}
                onAttachNode={handleAttachNode}
                onInsertMention={handleInsertMention}
                onRevealInExplorer={handleRevealInExplorer}
                onCopyPath={handleCopyPath}
              />
            {/each}
          </div>
        {/if}
      {/if}
    </div>

    <!-- Bottom Status Strip -->
    <footer class="h-7 min-h-7 px-2.5 bg-surface border-t border-subtle flex items-center justify-between text-[11px] text-muted-theme">
      <div class="truncate flex items-center gap-1.5 max-w-[170px]">
        {#if selectedNode}
          <span class="text-primary-theme font-mono truncate" title={selectedNode.path}>
            {selectedNode.path}
          </span>
        {:else}
          <span>Ready</span>
        {/if}
      </div>
      <div class="flex items-center gap-1 shrink-0">
        {#if selectedNode}
          <Tooltip text={selectedNode.isDir ? "Attach selected folder to chat" : "Attach selected file to chat"} position="top">
            <button
              type="button"
              onclick={() => {
                if (selectedNode) handleAttachNode(selectedNode);
              }}
              class="flex items-center gap-1 px-1.5 py-0.5 rounded bg-surface-elevated hover:bg-surface-hover text-accent-theme border border-subtle font-medium text-[10px] cursor-pointer transition-colors"
            >
              <Paperclip size={10} />
              <span>Attach</span>
            </button>
          </Tooltip>
        {/if}
        <span class="font-mono text-[10px] text-muted-theme">
          {#if activeSearch}
            {visibleSearchResults.length} match{visibleSearchResults.length === 1 ? "" : "es"}
          {:else}
            {visibleFilesCount} files
          {/if}
        </span>
      </div>
    </footer>
  </aside>
{/if}

<!-- Context Menu Popup -->
<ExplorerContextMenu
  {contextMenu}
  onClose={() => (contextMenu = null)}
  onAttach={handleAttachNode}
  onMention={handleInsertMention}
  onOpenFile={handleOpenFile}
  onOpenWithNotepad={(path) => handleOpenFile(path, undefined, { forceNotepad: true })}
  onReveal={handleRevealInExplorer}
  onCopyPath={handleCopyPath}
/>

<!-- Search Result Row Snippet (Files Only with Path Context) -->
{#snippet searchResultRow(node: TreeNode)}
  {@const isSelected = selectedNode?.path === node.path}
  {@const isChecked = checkedItems.has(node.path)}
  {@const isAttached = attachedPath === node.path}
  {@const meta = getFileIconMeta(node)}
  {@const isHiddenItem = node.name.startsWith(".")}
  {@const dirPath = node.path.includes("/") ? node.path.slice(0, node.path.lastIndexOf("/")) : ""}

  <div
    role="treeitem"
    aria-selected={isSelected}
    tabindex="0"
    onclick={() => {
      selectedNode = node;
      if (isSelectMode) {
        toggleCheckItem(node);
      }
    }}
    ondblclick={(e) => {
      handleOpenFile(node.path, e);
    }}
    oncontextmenu={(e) => handleContextMenu(node, e)}
    onkeydown={(e) => {
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        selectedNode = node;
        if (isSelectMode) {
          toggleCheckItem(node);
        }
      }
    }}
    class="group relative flex items-center h-6 px-2 cursor-pointer transition-colors select-none {isSelected ? 'bg-accent-subtle text-primary-theme font-medium border-l-2 border-accent-theme' : 'text-secondary-theme hover:bg-surface-hover/80 hover:text-primary-theme'} {isHiddenItem ? 'opacity-70' : ''}"
  >
    <!-- Optional Selection Checkbox in Select Mode -->
    {#if isSelectMode}
      <button
        type="button"
        onclick={(e) => toggleCheckItem(node, e)}
        class="w-3.5 h-3.5 mr-1.5 flex items-center justify-center text-muted-theme hover:text-accent-theme cursor-pointer"
        aria-label="Check item to attach"
      >
        {#if isChecked}
          <CheckSquare size={13} class="text-accent-theme" />
        {:else}
          <Square size={13} class="opacity-60" />
        {/if}
      </button>
    {/if}

    <!-- File Icon -->
    <div class="w-4 h-4 mr-1.5 flex items-center justify-center shrink-0">
      {#if meta.type === "code"}
        <FileCode size={14} style="color: {meta.color};" />
      {:else if meta.type === "database"}
        <Database size={14} style="color: {meta.color};" />
      {:else if meta.type === "terminal"}
        <Terminal size={14} style="color: {meta.color};" />
      {:else if meta.type === "image"}
        <Image size={14} style="color: {meta.color};" />
      {:else}
        <FileText size={14} style="color: {meta.color};" />
      {/if}
    </div>

    <!-- File Name -->
    <span class="truncate text-xs leading-none mr-1 font-normal {isHiddenItem ? 'italic' : ''}" title={node.path}>
      {node.name}
    </span>

    <!-- Relative Directory Breadcrumb / Path Context -->
    {#if dirPath}
      <span class="text-[10px] text-muted-theme/60 font-mono truncate mr-2" title={dirPath}>
        in {dirPath}
      </span>
    {/if}

    <!-- Action Buttons on Row Hover -->
    <div class="hidden group-hover:flex items-center gap-0.5 ml-auto shrink-0 bg-surface/90 rounded px-0.5 border border-subtle/50">
      <Tooltip text={isAttached ? "Attached to Chat!" : "Attach File to Chat"} position="left">
        <button
          type="button"
          onclick={(e) => handleAttachNode(node, e)}
          class="p-0.5 rounded hover:bg-surface-elevated transition-colors cursor-pointer {isAttached ? 'text-emerald-400 font-bold' : 'text-accent-theme hover:text-accent-hover'}"
          aria-label="Attach to Chat"
        >
          {#if isAttached}
            <Check size={11} />
          {:else}
            <Paperclip size={11} />
          {/if}
        </button>
      </Tooltip>

      <Tooltip text="Mention File (@file)" position="left">
        <button
          type="button"
          onclick={(e) => handleInsertMention(node, e)}
          class="p-0.5 text-muted-theme hover:text-accent-theme rounded hover:bg-surface-elevated transition-colors cursor-pointer"
          aria-label="Insert mention"
        >
          <AtSign size={11} />
        </button>
      </Tooltip>

      <Tooltip text="Open File" position="left">
        <button
          type="button"
          onclick={(e) => handleOpenFile(node.path, e)}
          class="p-0.5 text-muted-theme hover:text-primary-theme rounded hover:bg-surface-elevated transition-colors cursor-pointer"
          aria-label="Open file"
        >
          <ExternalLink size={11} />
        </button>
      </Tooltip>

      <Tooltip text="Reveal in Windows Explorer" position="left">
        <button
          type="button"
          onclick={(e) => handleRevealInExplorer(node.path, e)}
          class="p-0.5 text-muted-theme hover:text-primary-theme rounded hover:bg-surface-elevated transition-colors cursor-pointer"
          aria-label="Reveal in File Explorer"
        >
          <Eye size={11} />
        </button>
      </Tooltip>

      <Tooltip text={copiedPath === node.path ? "Copied!" : "Copy Relative Path"} position="left">
        <button
          type="button"
          onclick={(e) => handleCopyPath(node.path, e)}
          class="p-0.5 text-muted-theme hover:text-primary-theme rounded hover:bg-surface-elevated transition-colors cursor-pointer"
          aria-label="Copy path"
        >
          {#if copiedPath === node.path}
            <Check size={11} class="text-emerald-400" />
          {:else}
            <Copy size={11} />
          {/if}
        </button>
      </Tooltip>
    </div>
  </div>
{/snippet}
