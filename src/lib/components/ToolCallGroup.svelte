<script lang="ts">
  import type { ToolExecutionStep } from "$lib/types";
  import ToolCallNode from "$lib/components/ToolCallNode.svelte";
  import { formatDuration } from "$lib/utils/toolSteps";
  import {
    ChevronRight,
    CheckCircle2,
    AlertCircle,
    Loader2,
  } from "lucide-svelte";

  let {
    steps = [],
    isStreaming = false,
    turnDurationMs,
    streamingStartedAt,
  }: {
    steps: ToolExecutionStep[];
    isStreaming?: boolean;
    turnDurationMs?: number;
    streamingStartedAt?: number;
  } = $props();

  // User manual override: null = auto (collapsed by default), true/false = explicit toggle
  let userExpanded = $state<boolean | null>(null);

  let liveNow = $state(Date.now());

  $effect(() => {
    if (isStreaming && streamingStartedAt) {
      liveNow = Date.now();
      const interval = setInterval(() => {
        liveNow = Date.now();
      }, 1000);
      return () => clearInterval(interval);
    }
  });

  let completedSteps = $derived(
    steps.filter((s) => s.status === "completed" || s.status === "failed")
  );
  let activeStep = $derived(
    steps.find((s) => s.status === "in_progress" || s.status === "pending")
  );
  let failedCount = $derived(steps.filter((s) => s.status === "failed").length);

  let totalDurationMs = $derived.by(() => {
    if (isStreaming && streamingStartedAt) {
      return Math.max(0, liveNow - streamingStartedAt);
    }
    if (turnDurationMs && turnDurationMs > 0) {
      return turnDurationMs;
    }
    // Try wall-clock from steps timestamps if available
    let minStart = Infinity;
    let maxEnd = -Infinity;
    for (const s of steps) {
      if (s.created_at) {
        const t = new Date(s.created_at).getTime();
        if (!isNaN(t)) {
          if (t < minStart) minStart = t;
          const dur = s.duration_ms || 0;
          if (t + dur > maxEnd) maxEnd = t + dur;
        }
      }
    }
    if (minStart !== Infinity && maxEnd !== -Infinity && maxEnd > minStart) {
      return maxEnd - minStart;
    }
    return steps.reduce((acc, s) => acc + (s.duration_ms || 0), 0);
  });

  let isExpanded = $derived(
    userExpanded !== null ? userExpanded : false
  );

  function toggleExpand() {
    userExpanded = !isExpanded;
  }
</script>

{#if steps.length === 1}
  <div class="my-1.5">
    <ToolCallNode step={steps[0]} />
  </div>
{:else if steps.length > 1}
  <div class="my-2 rounded-xl border border-subtle bg-surface/40 overflow-hidden text-xs shadow-2xs">
    <!-- Collapsible Tree / Progress Stack Header -->
    <button
      type="button"
      onclick={toggleExpand}
      class="w-full px-3 py-2 flex items-center justify-between text-left hover:bg-surface-hover/80 transition-colors cursor-pointer select-none group"
      aria-expanded={isExpanded}
    >
      <div class="flex items-center gap-2 truncate min-w-0 pr-2">
        <ChevronRight
          size={14}
          class="text-secondary-theme transition-transform duration-200 shrink-0 {isExpanded ? 'rotate-90 text-primary-theme' : ''}"
        />

        {#if activeStep}
          <Loader2 size={13} class="animate-spin text-accent-theme shrink-0" />
          <span class="font-medium text-primary-theme truncate">
            Executing tasks ({completedSteps.length}/{steps.length} completed)...
          </span>
        {:else if failedCount > 0}
          <AlertCircle size={13} class="text-rose-400 shrink-0" />
          <span class="font-medium text-primary-theme truncate">
            {completedSteps.length - failedCount} completed, {failedCount} failed
          </span>
        {:else}
          <CheckCircle2 size={13} class="text-emerald-400 shrink-0" />
          <span class="font-medium text-primary-theme truncate">
            {steps.length} steps completed
          </span>
        {/if}
      </div>

      <div class="flex items-center gap-2 shrink-0 text-[11px] text-muted-theme font-mono">
        {#if totalDurationMs > 0}
          <span class={isStreaming ? "text-accent-theme font-medium animate-pulse" : ""}>
            {formatDuration(totalDurationMs)}
          </span>
        {/if}
        <span class="text-[10px] text-accent-theme font-sans group-hover:underline">
          {isExpanded ? "Hide tree" : "View tree"}
        </span>
      </div>
    </button>

    <!-- Expanded Hierarchy View: Full Tree of Completed & Active Steps -->
    {#if isExpanded}
      <div class="p-2.5 border-t border-subtle/70 bg-app/40 space-y-1.5">
        <div class="border-l-2 border-subtle/80 ml-2.5 pl-3 space-y-1.5">
          {#each steps as step, idx (step.id || idx)}
            <ToolCallNode {step} />
          {/each}
        </div>
      </div>
    {:else if activeStep}
      <!-- Collapsed View while actively running: Show ONLY the latest working step! -->
      <div class="px-2.5 pb-2.5 pt-0">
        <ToolCallNode step={activeStep} />
      </div>
    {/if}
  </div>
{/if}
