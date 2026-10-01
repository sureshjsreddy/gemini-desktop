<script lang="ts">
  import type { TreeNode } from "$lib/types";
  import Tooltip from "$lib/components/Tooltip.svelte";
  import { getFileIconMeta } from "$lib/utils/fileIcons";
  import { isNodeVisible } from "$lib/utils/fileFilters";
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
    ExternalLink,
    Eye,
    Copy,
  } from "lucide-svelte";

  let {
    node,
    depth = 0,
    expandedDirs,
    selectedNode = null,
    checkedItems,
    isSelectMode = false,
    attachedPath = null,
    copiedPath = null,
    showHiddenFiles = false,
    onSelectNode,
    onToggleDir,
    onToggleCheckItem,
    onOpenFile,
    onContextMenu,
    onAttachNode,
    onInsertMention,
    onRevealInExplorer,
    onCopyPath,
  }: {
    node: TreeNode;
    depth?: number;
    expandedDirs: Set<string>;
    selectedNode?: TreeNode | null;
    checkedItems: Map<string, { path: string; isDir: boolean; name: string }>;
    isSelectMode?: boolean;
    attachedPath?: string | null;
    copiedPath?: string | null;
    showHiddenFiles?: boolean;
    onSelectNode: (node: TreeNode) => void;
    onToggleDir: (node: TreeNode, e?: MouseEvent) => void;
    onToggleCheckItem: (node: TreeNode, e?: MouseEvent) => void;
    onOpenFile: (path: string, e?: MouseEvent) => void;
    onContextMenu: (node: TreeNode, e: MouseEvent) => void;
    onAttachNode: (node: TreeNode, e?: MouseEvent) => void;
    onInsertMention: (node: TreeNode, e?: MouseEvent) => void;
    onRevealInExplorer: (path: string, e?: MouseEvent) => void;
    onCopyPath: (path: string, e?: MouseEvent) => void;
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
</script>

<!-- Tree Row Item (VS 2022 Height: 24px) -->
<div
  role="treeitem"
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
  onkeydown={(e) => {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      onSelectNode(node);
      if (isSelectMode) {
        onToggleCheckItem(node);
      } else if (node.isDir) {
        onToggleDir(node);
      }
    }
  }}
  class="group relative flex items-center h-6 pr-2 cursor-pointer transition-colors select-none {isSelected ? 'bg-accent-subtle text-primary-theme font-medium border-l-2 border-accent-theme' : 'text-secondary-theme hover:bg-surface-hover/80 hover:text-primary-theme'} {isHiddenItem ? 'opacity-70' : ''}"
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
      class="w-3.5 h-3.5 mr-1 flex items-center justify-center text-muted-theme hover:text-accent-theme cursor-pointer"
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
  <span class="truncate text-xs leading-none mr-2 font-normal {isHiddenItem ? 'italic' : ''}" title={node.path}>
    {node.name}
  </span>

  <!-- Folder Child Count Badge -->
  {#if node.isDir && node.children.length > 0}
    <span class="text-[10px] text-muted-theme/80 font-mono group-hover:opacity-0 transition-opacity ml-auto shrink-0">
      {node.children.length}
    </span>
  {/if}

  <!-- Action Buttons on Row Hover -->
  <div class="hidden group-hover:flex items-center gap-0.5 ml-auto shrink-0 bg-surface/90 rounded px-0.5 border border-subtle/50">
    <!-- Direct "Attach to Chat" Button -->
    <Tooltip text={isAttached ? "Attached to Chat!" : node.isDir ? "Attach Folder to Chat" : "Attach File to Chat"} position="left">
      <button
        type="button"
        onclick={(e) => {
          e.stopPropagation();
          onAttachNode(node, e);
        }}
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

    <!-- Insert @mention into Chat Prompt -->
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

    {#if !node.isDir}
      <!-- Open with Default App -->
      <Tooltip text="Open File" position="left">
        <button
          type="button"
          onclick={(e) => {
            e.stopPropagation();
            onOpenFile(node.path, e);
          }}
          class="p-0.5 text-muted-theme hover:text-primary-theme rounded hover:bg-surface-elevated transition-colors cursor-pointer"
          aria-label="Open file"
        >
          <ExternalLink size={11} />
        </button>
      </Tooltip>
    {/if}

    <!-- Reveal in Explorer -->
    <Tooltip text="Reveal in Windows Explorer" position="left">
      <button
        type="button"
        onclick={(e) => {
          e.stopPropagation();
          onRevealInExplorer(node.path, e);
        }}
        class="p-0.5 text-muted-theme hover:text-primary-theme rounded hover:bg-surface-elevated transition-colors cursor-pointer"
        aria-label="Reveal in File Explorer"
      >
        <Eye size={11} />
      </button>
    </Tooltip>

    <!-- Copy Relative Path -->
    <Tooltip text={copiedPath === node.path ? "Copied!" : "Copy Relative Path"} position="left">
      <button
        type="button"
        onclick={(e) => {
          e.stopPropagation();
          onCopyPath(node.path, e);
        }}
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
            {copiedPath}
            {showHiddenFiles}
            {onSelectNode}
            {onToggleDir}
            {onToggleCheckItem}
            {onOpenFile}
            {onContextMenu}
            {onAttachNode}
            {onInsertMention}
            {onRevealInExplorer}
            {onCopyPath}
          />
        {/each}
      {/if}
    {/if}
  </div>
{/if}
