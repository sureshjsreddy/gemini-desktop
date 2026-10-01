<script lang="ts">
  import type { TreeNode } from "$lib/types";
  import {
    Paperclip,
    AtSign,
    ExternalLink,
    FileText,
    Eye,
    Copy,
  } from "lucide-svelte";

  let {
    contextMenu,
    onClose,
    onAttach,
    onMention,
    onOpenFile,
    onOpenWithNotepad,
    onReveal,
    onCopyPath,
  }: {
    contextMenu: { x: number; y: number; node: TreeNode } | null;
    onClose: () => void;
    onAttach: (node: TreeNode) => void;
    onMention: (node: TreeNode) => void;
    onOpenFile: (path: string) => void;
    onOpenWithNotepad: (path: string) => void;
    onReveal: (path: string) => void;
    onCopyPath: (path: string) => void;
  } = $props();
</script>

{#if contextMenu}
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
    class="fixed z-50 w-48 bg-surface-elevated border border-subtle rounded-lg shadow-2xl py-1 text-xs text-primary-theme"
    style="left: {contextMenu.x}px; top: {contextMenu.y}px;"
  >
    <div class="px-2.5 py-1 text-[10px] text-muted-theme border-b border-subtle/60 font-mono truncate">
      {contextMenu.node.name}
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
      onclick={() => onCopyPath(contextMenu.node.path)}
      class="w-full px-2.5 py-1.5 text-left hover:bg-surface-hover flex items-center gap-2 cursor-pointer"
    >
      <Copy size={13} class="text-secondary-theme" />
      <span>Copy Relative Path</span>
    </button>
  </div>
{/if}
