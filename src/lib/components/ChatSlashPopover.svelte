<script lang="ts">
  import type { SlashCommandOption, CliEngine } from "$lib/types";
  import { engineStore } from "$lib/stores/engineStore.svelte";
  import {
    X,
    ListTodo,
    Minimize2,
    FileDiff,
    RotateCcw,
    Terminal,
    Server,
    Blocks,
    Wrench,
    Cpu,
    Compass,
    Sparkles,
    Sliders,
    Key,
    Trash2,
    Command as CommandIcon,
  } from "lucide-svelte";

  let {
    show = false,
    options = [],
    selectedIndex = 0,
    activeEngine = "gemini",
    onSelect,
    onDismiss,
    onSelectIndex,
  }: {
    show: boolean;
    options: SlashCommandOption[];
    selectedIndex: number;
    activeEngine?: CliEngine;
    onSelect: (option: SlashCommandOption) => void;
    onDismiss: () => void;
    onSelectIndex?: (index: number) => void;
  } = $props();

  function getCategoryBadgeColor(cat: string) {
    switch (cat) {
      case "action":
        return "bg-emerald-500/10 text-emerald-400 border-emerald-500/20";
      case "tool":
        return "bg-cyan-500/10 text-cyan-400 border-cyan-500/20";
      case "mode":
        return "bg-amber-500/10 text-amber-400 border-amber-500/20";
      case "config":
        return "bg-purple-500/10 text-purple-400 border-purple-500/20";
      default:
        return "bg-secondary-theme/10 text-secondary-theme border-subtle";
    }
  }
</script>

{#if show && options.length > 0}
  <div
    class="fixed inset-0 z-35"
    onclick={onDismiss}
    role="presentation"
  ></div>
  <div class="absolute bottom-full left-0 mb-2 w-[460px] max-w-[calc(100vw-2rem)] max-h-80 overflow-y-auto bg-surface-elevated border border-subtle rounded-xl shadow-2xl py-1 z-40 text-xs backdrop-blur-md">
    <!-- Header -->
    <div class="px-3 py-2 text-[10px] font-semibold uppercase tracking-wider text-muted-theme border-b border-subtle flex items-center justify-between">
      <div class="flex items-center gap-2">
        <span class="inline-flex items-center justify-center w-4 h-4 rounded bg-accent-theme/15 text-accent-theme font-mono font-bold text-xs">/</span>
        <span class="text-primary-theme font-medium tracking-normal text-xs">Slash Commands & Tools</span>
        <span class="px-1.5 py-0.5 rounded text-[9px] font-mono uppercase tracking-wider font-medium border {activeEngine === 'agy' ? 'bg-purple-500/10 text-purple-400 border-purple-500/30' : 'bg-blue-500/10 text-blue-400 border-blue-500/30'}">
          {engineStore.getDescriptor(activeEngine).shortName}
        </span>
      </div>
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

    <!-- Commands List -->
    <div class="divide-y divide-subtle/40">
      {#each options as opt, idx (opt.id)}
        <button
          type="button"
          class="w-full px-3 py-2.5 text-left flex items-start gap-2.5 transition-colors cursor-pointer {idx === selectedIndex ? 'bg-accent-theme/15 text-accent-theme font-medium border-l-2 border-accent-theme' : 'hover:bg-surface-hover text-secondary-theme'}"
          onmousedown={(e) => {
            e.preventDefault();
            onSelect(opt);
          }}
          onmouseenter={() => onSelectIndex?.(idx)}
        >
          <!-- Icon -->
          <div class="p-1.5 rounded-lg bg-surface/80 border border-subtle shrink-0 mt-0.5">
            {#if opt.id === "plan"}
              <ListTodo size={14} class="text-emerald-400" />
            {:else if opt.id === "compact" || opt.id === "compress"}
              <Minimize2 size={14} class="text-indigo-400" />
            {:else if opt.id === "diff"}
              <FileDiff size={14} class="text-amber-400" />
            {:else if opt.id === "rewind"}
              <RotateCcw size={14} class="text-orange-400" />
            {:else if opt.id === "terminal"}
              <Terminal size={14} class="text-cyan-400" />
            {:else if opt.id === "mcp"}
              <Server size={14} class="text-blue-400" />
            {:else if opt.id === "plugins"}
              <Blocks size={14} class="text-purple-400" />
            {:else if opt.id === "tools"}
              <Wrench size={14} class="text-rose-400" />
            {:else if opt.id === "model"}
              <Cpu size={14} class="text-sky-400" />
            {:else if opt.id === "mode"}
              <Compass size={14} class="text-teal-400" />
            {:else if opt.id === "doctor"}
              <Sparkles size={14} class="text-fuchsia-400" />
            {:else if opt.id === "settings"}
              <Sliders size={14} class="text-slate-400" />
            {:else if opt.id === "setup"}
              <Key size={14} class="text-yellow-400" />
            {:else if opt.id === "clear"}
              <Trash2 size={14} class="text-red-400" />
            {:else}
              <CommandIcon size={14} class="text-accent-theme" />
            {/if}
          </div>

          <!-- Command Details -->
          <div class="flex-1 min-w-0">
            <div class="flex items-center gap-2">
              <span class="font-mono text-primary-theme font-semibold text-xs tracking-wide">{opt.label}</span>
              <span class="text-xs text-primary-theme/90 font-medium">{opt.title}</span>
              <span class="px-1.5 py-0.2 rounded text-[9px] uppercase tracking-wider font-medium border {getCategoryBadgeColor(opt.category)}">
                {opt.category}
              </span>
              {#if opt.engine === "agy"}
                <span class="px-1 py-0.2 rounded text-[8px] font-mono uppercase bg-purple-500/10 text-purple-400 border border-purple-500/30">
                  agy
                </span>
              {:else if opt.engine === "gemini"}
                <span class="px-1 py-0.2 rounded text-[8px] font-mono uppercase bg-blue-500/10 text-blue-400 border border-blue-500/30">
                  gemini
                </span>
              {/if}
            </div>
            <div class="text-[11px] text-muted-theme mt-0.5 line-clamp-1">{opt.description}</div>
          </div>

          <!-- Action Hint -->
          <div class="text-[9px] font-mono text-muted-theme shrink-0 self-center opacity-70">
            ↵ Run
          </div>
        </button>
      {/each}
    </div>
  </div>
{/if}
