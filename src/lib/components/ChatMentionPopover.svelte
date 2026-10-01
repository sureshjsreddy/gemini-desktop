<script lang="ts">
  import type { MentionOption } from "$lib/types";
  import { X, GitBranch, Folder, FileCode } from "lucide-svelte";

  let {
    show = false,
    options = [],
    selectedIndex = 0,
    onSelect,
    onDismiss,
    onSelectIndex,
  }: {
    show: boolean;
    options: MentionOption[];
    selectedIndex: number;
    onSelect: (option: MentionOption) => void;
    onDismiss: () => void;
    onSelectIndex?: (index: number) => void;
  } = $props();
</script>

{#if show && options.length > 0}
  <div
    class="fixed inset-0 z-35"
    onclick={onDismiss}
    role="presentation"
  ></div>
  <div class="absolute bottom-full left-0 mb-2 w-96 max-h-64 overflow-y-auto bg-surface-elevated border border-subtle rounded-xl shadow-2xl py-1 z-40 text-xs backdrop-blur-md">
    <div class="px-3 py-1.5 text-[10px] font-semibold uppercase tracking-wider text-muted-theme border-b border-subtle flex items-center justify-between">
      <span class="flex items-center gap-1.5">
        <span class="text-accent-theme font-mono font-bold">@</span>
        <span>Insert Reference</span>
      </span>
      <div class="flex items-center gap-2">
        <span class="font-mono text-[9px] opacity-75">↑↓ &bull; Enter/Tab &bull; Esc</span>
        <button
          type="button"
          onclick={onDismiss}
          class="p-0.5 rounded text-muted-theme hover:text-primary-theme hover:bg-surface transition-colors cursor-pointer"
          title="Dismiss (Esc)"
        >
          <X size={12} />
        </button>
      </div>
    </div>
    {#each options as opt, idx (opt.id)}
      <button
        type="button"
        class="w-full px-3 py-2 text-left flex items-center gap-2.5 transition-colors cursor-pointer {idx === selectedIndex ? 'bg-accent-theme/15 text-accent-theme font-medium border-l-2 border-accent-theme' : 'hover:bg-surface-hover text-secondary-theme'}"
        onmousedown={(e) => {
          e.preventDefault();
          onSelect(opt);
        }}
        onmouseenter={() => onSelectIndex?.(idx)}
      >
        {#if opt.kind === "git"}
          <GitBranch size={14} class="text-amber-400 shrink-0" />
        {:else if opt.kind === "directory"}
          <Folder size={14} class="text-sky-400 shrink-0" />
        {:else}
          <FileCode size={14} class="text-accent-theme shrink-0" />
        {/if}
        <div class="truncate flex-1">
          <div class="truncate text-primary-theme text-xs font-mono">{opt.label}</div>
          <div class="truncate text-[10px] text-muted-theme font-sans">{opt.subtitle}</div>
        </div>
      </button>
    {/each}
  </div>
{/if}
