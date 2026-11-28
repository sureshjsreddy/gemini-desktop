<script lang="ts">
  import type { TreeNode, GitFileItem } from "$lib/types";
  import { getGitStatusBadge } from "$lib/utils/gitStatus";
  import {
    Paperclip,
    AtSign,
    ExternalLink,
    FileText,
    Eye,
    Copy,
    RotateCw,
    FileDiff,
  } from "lucide-svelte";

  let {
    contextMenu,
    gitItem = null,
    onClose,
    onAttach,
    onMention,
    onOpenFile,
    onOpenWithNotepad,
    onReveal,
    onCopyPath,
    onRefreshDir,
    onViewGitDiff,
  }: {
    contextMenu: { x: number; y: number; node: TreeNode } | null;
    gitItem?: GitFileItem | null;
    onClose: () => void;
    onAttach: (node: TreeNode) => void;
    onMention: (node: TreeNode) => void;
    onOpenFile: (path: string) => void;
    onOpenWithNotepad: (path: string) => void;
    onReveal: (path: string) => void;
    onCopyPath: (path: string, full?: boolean) => void;
    onRefreshDir?: (node: TreeNode) => void;
    onViewGitDiff?: (gitItem: GitFileItem) => void;
  } = $props();
</script>

{#if contextMenu}
  {@const gitBadge = gitItem ? getGitStatusBadge(gitItem.status) : null}

  <!-- Backdrop to close context menu -->
  <div
    class="fixed inset-0 z-50 bg-transparent"
    onclick={onClose}
    oncontextmenu={(e) => {
      e.preventDefault();
      onClose();
    }}
    role="presentation"
  ></div>

  <!-- Context Menu Card -->
  <div
    class="fixed z-50 w-52 bg-surface-elevated border border-subtle rounded-lg shadow-2xl py-1 text-xs text-primary-theme"
    style="left: {Math.min(contextMenu.x, window.innerWidth - 220)}px; top: {Math.min(contextMenu.y, window.innerHeight - 310)}px;"
    role="menu"
    tabindex="-1"
    onkeydown={(e) => {
      if (e.key === "Escape") onClose();
    }}
  >
    <div class="px-2.5 py-1.5 text-[10px] text-muted-theme border-b border-subtle/60 font-mono flex items-center justify-between gap-1.5">
      <span class="truncate">{contextMenu.node.name}</span>
      {#if gitBadge}
        <span class="text-[9px] font-bold px-1 rounded border shrink-0 {gitBadge.badgeClass}">
          {gitBadge.letter}
        </span>
      {/if}
    </div>

    <!-- Attach to Chat -->
    <button
      type="button"
      onclick={() => onAttach(contextMenu.node)}
      class="w-full px-2.5 py-1.5 text-left hover:bg-surface-hover flex items-center gap-2 text-accent-theme font-medium cursor-pointer"
    >
      <Paperclip size={13} />
      <span>Attach to Chat</span>
    </button>

    <!-- Mention in Chat -->
    <button
      type="button"
      onclick={() => onMention(contextMenu.node)}
      class="w-full px-2.5 py-1.5 text-left hover:bg-surface-hover flex items-center gap-2 cursor-pointer"
    >
      <AtSign size={13} class="text-secondary-theme" />
      <span>Mention in Prompt (@)</span>
    </button>

    {#if gitItem && onViewGitDiff && !contextMenu.node.isDir}
      <button
        type="button"
        onclick={() => {
          onViewGitDiff(gitItem);
          onClose();
        }}
        class="w-full px-2.5 py-1.5 text-left hover:bg-surface-hover flex items-center gap-2 cursor-pointer text-emerald-500 font-medium"
      >
        <FileDiff size={13} />
        <span>View Git Diff</span>
      </button>
    {/if}

    <div class="my-1 border-t border-subtle/60"></div>

    {#if !contextMenu.node.isDir}
      <!-- Open File with Default App -->
      <button
        type="button"
        onclick={() => onOpenFile(contextMenu.node.path)}
        class="w-full px-2.5 py-1.5 text-left hover:bg-surface-hover flex items-center gap-2 cursor-pointer"
      >
        <ExternalLink size={13} class="text-secondary-theme" />
        <span>Open with Default App</span>
      </button>

      <!-- Open File with Notepad -->
      <button
        type="button"
        onclick={() => onOpenWithNotepad(contextMenu.node.path)}
        class="w-full px-2.5 py-1.5 text-left hover:bg-surface-hover flex items-center gap-2 cursor-pointer text-secondary-theme hover:text-primary-theme"
      >
        <FileText size={13} class="text-muted-theme" />
        <span>Open with Notepad</span>
      </button>
    {:else if onRefreshDir}
      <button
        type="button"
        onclick={() => {
          onRefreshDir(contextMenu.node);
          onClose();
        }}
        class="w-full px-2.5 py-1.5 text-left hover:bg-surface-hover flex items-center gap-2 cursor-pointer"
      >
        <RotateCw size={13} class="text-secondary-theme" />
        <span>Refresh Folder</span>
      </button>
    {/if}

    <!-- Reveal in Explorer -->
    <button
      type="button"
      onclick={() => onReveal(contextMenu.node.path)}
      class="w-full px-2.5 py-1.5 text-left hover:bg-surface-hover flex items-center gap-2 cursor-pointer"
    >
      <Eye size={13} class="text-secondary-theme" />
      <span>Reveal in Explorer</span>
    </button>

    <div class="my-1 border-t border-subtle/60"></div>

    <!-- Copy Relative Path -->
    <button
      type="button"
      onclick={() => onCopyPath(contextMenu.node.path, false)}
      class="w-full px-2.5 py-1.5 text-left hover:bg-surface-hover flex items-center gap-2 cursor-pointer"
    >
      <Copy size={13} class="text-secondary-theme" />
      <span>Copy Relative Path</span>
    </button>

    <!-- Copy Full Path -->
    <button
      type="button"
      onclick={() => onCopyPath(contextMenu.node.path, true)}
      class="w-full px-2.5 py-1.5 text-left hover:bg-surface-hover flex items-center gap-2 cursor-pointer"
    >
      <Copy size={13} class="text-muted-theme" />
      <span>Copy Full Path</span>
    </button>
  </div>
{/if}
