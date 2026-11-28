<script lang="ts">
  import type { TreeNode, GitFileItem } from "$lib/types";
  import Tooltip from "$lib/components/Tooltip.svelte";
  import { getFileIconMeta } from "$lib/utils/fileIcons";
  import { isNodeVisible } from "$lib/utils/fileFilters";
  import { getGitStatusBadge } from "$lib/utils/gitStatus";
  import ExplorerTreeRow from "$lib/components/ExplorerTreeRow.svelte";
  import {
    Folder,
    FolderOpen,
    ChevronRight,
    RotateCw,
    CheckSquare,
    Square,
    FileCode,
    Database,
    Terminal,
    Image,
    FileText,
    Check,
    Paperclip,
    AtSign,
    MoreHorizontal,
  } from "lucide-svelte";

  let {
    node,
    depth = 0,
    expandedDirs,
    selectedNode = null,
    checkedItems,
    isSelectMode = false,
    attachedPath = null,
    showHiddenFiles = false,
    gitFileStatusMap,
    gitDirtyDirs,
    onSelectNode,
    onToggleDir,
    onToggleCheckItem,
    onOpenFile,
    onContextMenu,
    onAttachNode,
    onInsertMention,
  }: {
    node: TreeNode;
    depth?: number;
    expandedDirs: Set<string>;
    selectedNode?: TreeNode | null;
    checkedItems: Map<string, { path: string; isDir: boolean; name: string }>;
    isSelectMode?: boolean;
    attachedPath?: string | null;
    showHiddenFiles?: boolean;
    gitFileStatusMap?: Map<string, GitFileItem>;
    gitDirtyDirs?: Set<string>;
    onSelectNode: (node: TreeNode) => void;
    onToggleDir: (node: TreeNode, e?: MouseEvent) => void;
    onToggleCheckItem: (node: TreeNode, e?: Event) => void;
    onOpenFile: (path: string, e?: MouseEvent) => void;
    onContextMenu: (node: TreeNode, e: MouseEvent) => void;
    onAttachNode: (node: TreeNode, e?: MouseEvent) => void;
    onInsertMention: (node: TreeNode, e?: MouseEvent) => void;
  } = $props();

  let isExpanded = $derived(node.isDir && expandedDirs.has(node.path));
  let isSelected = $derived(selectedNode?.path === node.path);
  let isChecked = $derived(checkedItems.has(node.path));
  let isAttached = $derived(attachedPath === node.path);
  let meta = $derived(getFileIconMeta(node));
  let isHiddenItem = $derived(node.name.startsWith("."));
  let visibleChildren = $derived(
    node.isDir && node.children ? node.children.filter((c) => isNodeVisible(c, showHiddenFiles)) : []
  );

  let gitItem = $derived(!node.isDir ? gitFileStatusMap?.get(node.path) : undefined);
  let gitBadge = $derived(gitItem ? getGitStatusBadge(gitItem.status) : null);
  let hasDirtyChildren = $derived(node.isDir && Boolean(gitDirtyDirs?.has(node.path)));

  function moveTreeFocus(currentEl: HTMLElement, delta: number) {
    const container = currentEl.closest("[data-explorer-tree]");
    if (!container) return;
    const rows = Array.from(container.querySelectorAll<HTMLElement>("[data-tree-row]"));
    const idx = rows.indexOf(currentEl);
    if (idx === -1) return;
    const next = rows[idx + delta];
    if (next) next.focus();
  }

  function focusParentDir(currentEl: HTMLElement) {
    const slashIdx = node.path.lastIndexOf("/");
    if (slashIdx === -1) return;
    const parentPath = node.path.slice(0, slashIdx);
    const container = currentEl.closest("[data-explorer-tree]");
    if (!container) return;
    const rows = Array.from(container.querySelectorAll<HTMLElement>("[data-tree-row]"));
    const parentEl = rows.find((el) => el.getAttribute("data-tree-row") === parentPath);
    if (parentEl) parentEl.focus();
  }

  function handleKeyDown(e: KeyboardEvent) {
    const el = e.currentTarget as HTMLElement;
    if (e.key === "ArrowDown") {
      e.preventDefault();
      moveTreeFocus(el, 1);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      moveTreeFocus(el, -1);
    } else if (e.key === "ArrowRight") {
      if (node.isDir) {
        e.preventDefault();
        if (!isExpanded) {
          onToggleDir(node);
        } else {
          moveTreeFocus(el, 1);
        }
      }
    } else if (e.key === "ArrowLeft") {
      e.preventDefault();
      if (node.isDir && isExpanded) {
        onToggleDir(node);
      } else {
        focusParentDir(el);
      }
    } else if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      onSelectNode(node);
      if (isSelectMode) {
        onToggleCheckItem(node);
      } else if (node.isDir) {
        onToggleDir(node);
      } else if (e.key === "Enter") {
        onOpenFile(node.path);
      }
    }
  }

  function openAnchoredContextMenu(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const syntheticEvent = new MouseEvent("contextmenu", {
      clientX: rect.left - 140,
      clientY: rect.bottom + 4,
      bubbles: true,
    });
    onContextMenu(node, syntheticEvent);
  }
</script>

<!-- Tree Row Item (VS 2022 Height: 24px) -->
<div
  role="treeitem"
  data-tree-row={node.path}
  aria-selected={isSelected}
  aria-expanded={node.isDir ? isExpanded : undefined}
  tabindex="0"
  onclick={() => {
    onSelectNode(node);
    if (isSelectMode) {
      onToggleCheckItem(node);
    } else if (node.isDir) {
      onToggleDir(node);
    }
  }}
  ondblclick={(e) => {
    if (!node.isDir) {
      onOpenFile(node.path, e);
    }
  }}
  oncontextmenu={(e) => onContextMenu(node, e)}
  onkeydown={handleKeyDown}
  class="group relative flex items-center h-6 pr-2 cursor-pointer transition-colors select-none focus:outline-none focus-visible:ring-1 focus-visible:ring-inset focus-visible:ring-accent-theme {isSelected ? 'bg-accent-subtle text-primary-theme font-medium border-l-2 border-accent-theme' : 'text-secondary-theme hover:bg-surface-hover/80 hover:text-primary-theme'} {isHiddenItem ? 'opacity-70' : ''}"
  style="padding-left: {depth * 14 + 6}px;"
>
  <!-- Visual Studio Tree Indentation Guide Line -->
  {#if depth > 0}
    <div
      class="absolute top-0 bottom-0 border-l border-subtle/40 pointer-events-none"
      style="left: {(depth - 1) * 14 + 11}px;"
    ></div>
  {/if}

  <!-- Optional Selection Checkbox in Select Mode -->
  {#if isSelectMode}
    <button
      type="button"
      onclick={(e) => {
        e.stopPropagation();
        onToggleCheckItem(node, e);
      }}
      class="w-3.5 h-3.5 mr-1 flex items-center justify-center text-muted-theme hover:text-accent-theme cursor-pointer shrink-0"
      aria-label="Check item to attach"
    >
      {#if isChecked}
        <CheckSquare size={13} class="text-accent-theme" />
      {:else}
        <Square size={13} class="opacity-60" />
      {/if}
    </button>
  {/if}

  <!-- Caret Arrow for Folders / Spacer for Files -->
  {#if node.isDir}
    <button
      type="button"
      onclick={(e) => {
        e.stopPropagation();
        onToggleDir(node, e);
      }}
      class="w-3.5 h-3.5 flex items-center justify-center text-muted-theme hover:text-primary-theme shrink-0 cursor-pointer"
      aria-label={isExpanded ? "Collapse folder" : "Expand folder"}
    >
      {#if node.isLoading}
        <RotateCw size={11} class="animate-spin text-accent-theme" />
      {:else}
        <ChevronRight
          size={12}
          class="transition-transform duration-150 {isExpanded ? 'rotate-90 text-primary-theme' : ''}"
        />
      {/if}
    </button>
  {:else}
    <span class="w-3.5 h-3.5 shrink-0"></span>
  {/if}

  <!-- File / Folder Icon -->
  <div class="w-4 h-4 mr-1.5 flex items-center justify-center shrink-0">
    {#if node.isDir}
      {#if isExpanded}
        <FolderOpen size={14} class="text-[#f59e0b]" />
      {:else}
        <Folder size={14} class="text-[#f59e0b]" />
      {/if}
    {:else}
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
    {/if}
  </div>

  <!-- Node Name -->
  <span
    class="truncate text-xs leading-none mr-1.5 font-normal {isHiddenItem ? 'italic' : ''} {gitBadge && gitItem?.status !== 'modified' ? gitBadge.textColor : ''}"
    title={gitBadge ? `${node.path} (${gitBadge.label})` : node.path}
  >
    {node.name}
  </span>

  <!-- Right-side Passive Badges (Git Status / Dirty Folder Dot / Child Count) — hidden cleanly on hover -->
  <div class="ml-auto flex items-center gap-1 shrink-0 group-hover:hidden">
    {#if hasDirtyChildren}
      <span
        class="w-1.5 h-1.5 rounded-full bg-sky-400/85 shrink-0"
        title="Contains modified or untracked Git files"
      ></span>
    {/if}
    {#if gitBadge}
      <span
        class="text-[9px] font-mono font-bold px-1 rounded border leading-tight {gitBadge.badgeClass}"
        title={gitBadge.title}
      >
        {gitBadge.letter}
      </span>
    {:else if node.isDir && node.children.length > 0}
      <span class="text-[10px] text-muted-theme/70 font-mono">
        {node.children.length}
      </span>
    {/if}
  </div>

  <!-- Streamlined 3-Action Hover Pill (keeps filenames readable on narrow panels) -->
  <div class="hidden group-hover:flex items-center gap-0.5 ml-auto shrink-0 bg-surface/95 rounded px-0.5 border border-subtle/60 shadow-xs">
    <Tooltip text={isAttached ? "Attached!" : node.isDir ? "Attach Folder to Chat" : "Attach File to Chat"} position="left">
      <button
        type="button"
        onclick={(e) => {
          e.stopPropagation();
          onAttachNode(node, e);
        }}
        class="p-0.5 rounded hover:bg-surface-elevated transition-colors cursor-pointer {isAttached ? 'text-emerald-500 font-bold' : 'text-accent-theme hover:text-accent-hover'}"
        aria-label="Attach to Chat"
      >
        {#if isAttached}
          <Check size={11} />
        {:else}
          <Paperclip size={11} />
        {/if}
      </button>
    </Tooltip>

    <Tooltip text={node.isDir ? "Mention Folder (@dir/)" : "Mention File (@file)"} position="left">
      <button
        type="button"
        onclick={(e) => {
          e.stopPropagation();
          onInsertMention(node, e);
        }}
        class="p-0.5 text-muted-theme hover:text-accent-theme rounded hover:bg-surface-elevated transition-colors cursor-pointer"
        aria-label="Insert mention"
      >
        <AtSign size={11} />
      </button>
    </Tooltip>

    <Tooltip text="More Actions" position="left">
      <button
        type="button"
        onclick={openAnchoredContextMenu}
        class="p-0.5 text-muted-theme hover:text-primary-theme rounded hover:bg-surface-elevated transition-colors cursor-pointer"
        aria-label="More actions"
      >
        <MoreHorizontal size={11} />
      </button>
    </Tooltip>
  </div>
</div>

<!-- Recursive Render Children when Folder is Expanded -->
{#if node.isDir && isExpanded}
  <div>
    {#if node.isLoading}
      <div
        class="flex items-center gap-1.5 h-6 text-muted-theme text-[11px] select-none italic"
        style="padding-left: {(depth + 1) * 14 + 6}px;"
      >
        <RotateCw size={11} class="animate-spin text-accent-theme" />
        <span>Loading...</span>
      </div>
    {:else}
      {#if node.isLoaded && visibleChildren.length === 0}
        <div
          class="flex items-center h-6 text-muted-theme/60 text-[11px] select-none italic"
          style="padding-left: {(depth + 1) * 14 + 6}px;"
        >
          (empty)
        </div>
      {:else}
        {#each visibleChildren as child (child.path)}
          <ExplorerTreeRow
            node={child}
            depth={depth + 1}
            {expandedDirs}
            {selectedNode}
            {checkedItems}
            {isSelectMode}
            {attachedPath}
            {showHiddenFiles}
            {gitFileStatusMap}
            {gitDirtyDirs}
            {onSelectNode}
            {onToggleDir}
            {onToggleCheckItem}
            {onOpenFile}
            {onContextMenu}
            {onAttachNode}
            {onInsertMention}
          />
        {/each}
      {/if}
    {/if}
  </div>
{/if}
