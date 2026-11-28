<script lang="ts">
  import type { ToolExecutionStep, TurnMilestoneStep } from "$lib/types";
  import ToolCallNode from "$lib/components/ToolCallNode.svelte";
  import {
    formatDuration,
    buildLiveWorkFocus,
    buildTurnReceiptSummary,
    classifyToolStep,
    deriveProgressPhases,
    extractImpactChips,
    extractTodoMilestonesFromToolStep,
    isTodoMetaStep,
  } from "$lib/utils/toolSteps";
  import {
    ChevronRight,
    CheckCircle2,
    AlertCircle,
    AlertTriangle,
    Loader2,
    Circle,
    Terminal,
    FileEdit,
    FileSearch,
    Globe,
    Sparkles,
    ChevronsUpDown,
    ChevronsDownUp,
  } from "lucide-svelte";

  let {
    steps = [],
    milestones = [],
    isStreaming = false,
    turnDurationMs,
    streamingStartedAt,
    onInspectFile,
  }: {
    steps: ToolExecutionStep[];
    milestones?: TurnMilestoneStep[];
    isStreaming?: boolean;
    turnDurationMs?: number;
    streamingStartedAt?: number;
    onInspectFile?: (fileName: string) => void;
  } = $props();

  type FilterMode = "all" | "files" | "commands" | "errors";

  // Explicit user toggle overrides collapsed default
  let userExpanded = $state<boolean | null>(null);
  let filterMode = $state<FilterMode>("all");
  let batchExpandState = $state<boolean | null>(null);
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

  // Reset inspector expansion when an active stream finishes into its 1-line Work Receipt
  let wasStreaming = $state(false);
  $effect(() => {
    if (isStreaming) {
      wasStreaming = true;
    } else if (wasStreaming) {
      wasStreaming = false;
      userExpanded = null;
    }
  });

  let effectiveMilestones = $derived.by(() => {
    if (milestones && milestones.length > 0) return milestones;
    for (let i = steps.length - 1; i >= 0; i--) {
      const extracted = extractTodoMilestonesFromToolStep(steps[i]);
      if (extracted && extracted.length > 0) return extracted;
    }
    return [];
  });

  let actionableSteps = $derived(steps.filter((s) => !isTodoMetaStep(s)));
  let isExpanded = $derived(userExpanded !== null ? userExpanded : false);

  let phases = $derived(deriveProgressPhases(actionableSteps, effectiveMilestones, isStreaming));
  let liveFocus = $derived(buildLiveWorkFocus(actionableSteps, isStreaming));
  let impact = $derived(extractImpactChips(actionableSteps));
  let receipt = $derived(buildTurnReceiptSummary(actionableSteps, effectiveMilestones));

  // Keep the top live strip compact (up to 3 visible phase pills + overflow counts)
  let visiblePhases = $derived.by(() => {
    if (phases.length <= 3) {
      return { items: phases, hiddenBefore: 0, hiddenAfter: 0 };
    }
    const activeIdx = phases.findIndex((p) => p.status === "in_progress");
    const pivot = activeIdx >= 0 ? activeIdx : Math.max(0, phases.length - 1);
    const start = Math.max(0, Math.min(pivot - 1, phases.length - 3));
    const end = Math.min(phases.length, start + 3);
    return {
      items: phases.slice(start, end),
      hiddenBefore: start,
      hiddenAfter: Math.max(0, phases.length - end),
    };
  });

  let fileStepsCount = $derived(
    actionableSteps.filter((s) => classifyToolStep(s) === "edit").length,
  );
  let commandStepsCount = $derived(
    actionableSteps.filter((s) => classifyToolStep(s) === "command").length,
  );
  let failedCount = $derived(actionableSteps.filter((s) => s.status === "failed").length);

  let filteredSteps = $derived.by(() => {
    if (filterMode === "files") {
      return actionableSteps.filter((s) => classifyToolStep(s) === "edit");
    }
    if (filterMode === "commands") {
      return actionableSteps.filter((s) => classifyToolStep(s) === "command");
    }
    if (filterMode === "errors") {
      return actionableSteps.filter((s) => s.status === "failed" || !!s.error);
    }
    return actionableSteps;
  });

  let totalDurationMs = $derived.by(() => {
    if (isStreaming && streamingStartedAt) {
      return Math.max(0, liveNow - streamingStartedAt);
    }
    if (turnDurationMs && turnDurationMs > 0) {
      return turnDurationMs;
    }
    let minStart = Infinity;
    let maxEnd = -Infinity;
    for (const s of actionableSteps) {
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
    return actionableSteps.reduce((acc, s) => acc + (s.duration_ms || 0), 0);
  });

  function toggleExpand() {
    userExpanded = !isExpanded;
  }

  function toggleBatchExpand() {
    batchExpandState = batchExpandState === true ? false : true;
  }

  function getFocusIcon(category: string) {
    switch (category) {
      case "command":
        return Terminal;
      case "edit":
        return FileEdit;
      case "explore":
        return FileSearch;
      case "web":
        return Globe;
      default:
        return Sparkles;
    }
  }
</script>

{#snippet inspectorDrawer()}
  <div class="p-2.5 border-t border-subtle/70 bg-app/40 space-y-2.5">
    {#if !isStreaming && receipt.editedFiles.length > 0 && onInspectFile}
      <div class="flex items-center gap-1.5 flex-wrap px-1">
        <span class="text-[10px] text-muted-theme mr-1">Files:</span>
        {#each receipt.editedFiles as fileName}
          <button
            type="button"
            onclick={() => onInspectFile(fileName)}
            class="inline-flex items-center gap-1 px-2 py-0.5 rounded-md text-[10px] font-mono bg-surface-elevated border border-subtle text-secondary-theme hover:text-primary-theme hover:border-accent-theme/40 transition-colors cursor-pointer"
          >
            <FileEdit size={10} class="text-sky-400 shrink-0" />
            <span>{fileName}</span>
          </button>
        {/each}
      </div>
    {/if}

    {#if effectiveMilestones.length > 0}
      <div class="rounded-lg border border-subtle bg-surface/60 p-2.5 space-y-1.5">
        <div class="text-[10px] font-semibold uppercase tracking-wider text-muted-theme">
          Tasks ({effectiveMilestones.filter((m) => m.status === "completed").length}/{effectiveMilestones.length})
        </div>
        <div class="space-y-1">
          {#each effectiveMilestones as m (m.id)}
            <div class="flex items-center gap-2 text-[11px]">
              {#if m.status === "completed" || (!isStreaming && m.status !== "failed")}
                <CheckCircle2 size={12} class="text-emerald-400 shrink-0" />
                <span class="text-secondary-theme">{m.title}</span>
              {:else if m.status === "in_progress"}
                <Loader2 size={12} class="animate-spin text-accent-theme shrink-0" />
                <span class="font-medium text-primary-theme">{m.title}</span>
              {:else if m.status === "failed"}
                <AlertCircle size={12} class="text-rose-400 shrink-0" />
                <span class="text-rose-300">{m.title}</span>
              {:else}
                <Circle size={11} class="text-muted-theme/70 shrink-0" />
                <span class="text-muted-theme">{m.title}</span>
              {/if}
            </div>
          {/each}
        </div>
      </div>
    {/if}

    {#if actionableSteps.length > 0}
      <div class="flex items-center justify-between gap-2 px-1 text-[11px]">
        <div class="flex items-center gap-1.5 flex-wrap">
          <button
            type="button"
            onclick={() => (filterMode = "all")}
            class="px-2 py-0.5 rounded-md font-medium transition-colors cursor-pointer border {filterMode === 'all' ? 'bg-surface-elevated text-primary-theme border-subtle font-semibold shadow-2xs' : 'text-muted-theme hover:text-secondary-theme border-transparent'}"
          >
            All ({actionableSteps.length})
          </button>
          {#if fileStepsCount > 0}
            <button
              type="button"
              onclick={() => (filterMode = "files")}
              class="px-2 py-0.5 rounded-md font-medium transition-colors cursor-pointer border {filterMode === 'files' ? 'bg-surface-elevated text-primary-theme border-subtle font-semibold shadow-2xs' : 'text-muted-theme hover:text-secondary-theme border-transparent'}"
            >
              Files ({fileStepsCount})
            </button>
          {/if}
          {#if commandStepsCount > 0}
            <button
              type="button"
              onclick={() => (filterMode = "commands")}
              class="px-2 py-0.5 rounded-md font-medium transition-colors cursor-pointer border {filterMode === 'commands' ? 'bg-surface-elevated text-primary-theme border-subtle font-semibold shadow-2xs' : 'text-muted-theme hover:text-secondary-theme border-transparent'}"
            >
              Commands ({commandStepsCount})
            </button>
          {/if}
          {#if failedCount > 0}
            <button
              type="button"
              onclick={() => (filterMode = "errors")}
              class="px-2 py-0.5 rounded-md font-medium transition-colors cursor-pointer border flex items-center gap-1 {filterMode === 'errors' ? 'bg-rose-950/40 text-rose-300 border-rose-500/30 font-semibold shadow-2xs' : 'text-rose-400/80 hover:text-rose-300 border-transparent'}"
            >
              <AlertCircle size={11} class="text-rose-400" />
              <span>Errors ({failedCount})</span>
            </button>
          {/if}
        </div>

        <button
          type="button"
          onclick={toggleBatchExpand}
          class="flex items-center gap-1 px-2 py-0.5 rounded-md text-secondary-theme hover:text-primary-theme hover:bg-surface-elevated/70 transition-colors cursor-pointer border border-transparent hover:border-subtle shrink-0"
        >
          {#if batchExpandState === true}
            <ChevronsDownUp size={12} />
            <span>Collapse All</span>
          {:else}
            <ChevronsUpDown size={12} />
            <span>Expand All</span>
          {/if}
        </button>
      </div>

      {#if filteredSteps.length > 0}
        <div class="border-l-2 border-subtle/80 ml-2.5 pl-3 space-y-1.5">
          {#each filteredSteps as step, idx (step.id || idx)}
            <ToolCallNode {step} forceExpanded={batchExpandState} />
          {/each}
        </div>
      {:else}
        <div class="py-2 text-center text-muted-theme italic text-[11px]">
          No matching steps in this filter.
        </div>
      {/if}
    {/if}
  </div>
{/snippet}

{#if actionableSteps.length > 0 || effectiveMilestones.length > 0}
  {#if isStreaming}
    <!-- MODE A: LIVING NOW CAPSULE -->
    <div class="my-2 rounded-xl border border-subtle bg-surface/55 overflow-hidden text-xs shadow-2xs">
      {#if phases.length > 0}
        <div class="px-3 py-2 flex items-center justify-between gap-2 border-b border-subtle/60 bg-surface-elevated/40">
          <div class="flex items-center gap-1.5 min-w-0 overflow-hidden flex-wrap">
            {#if visiblePhases.hiddenBefore > 0}
              <span class="text-[10px] text-muted-theme font-mono shrink-0">
                +{visiblePhases.hiddenBefore} done
              </span>
              <span class="text-muted-theme/50 text-[10px]">→</span>
            {/if}

            {#each visiblePhases.items as phase, idx (phase.id)}
              {#if idx > 0}
                <span class="text-muted-theme/40 text-[10px] shrink-0">→</span>
              {/if}
              <span
                class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[11px] max-w-[210px] truncate transition-colors {phase.status === 'in_progress'
                  ? 'bg-accent-theme/15 text-primary-theme border border-accent-theme/30 font-medium'
                  : phase.status === 'failed'
                  ? 'bg-rose-500/10 text-rose-300 border border-rose-500/30'
                  : phase.status === 'completed'
                  ? 'text-muted-theme'
                  : 'text-muted-theme/70'}"
                title={phase.label}
              >
                {#if phase.status === "completed"}
                  <CheckCircle2 size={11} class="text-emerald-400 shrink-0" />
                {:else if phase.status === "in_progress"}
                  <Loader2 size={11} class="animate-spin text-accent-theme shrink-0" />
                {:else if phase.status === "failed"}
                  <AlertCircle size={11} class="text-rose-400 shrink-0" />
                {:else}
                  <Circle size={10} class="text-muted-theme/60 shrink-0" />
                {/if}
                <span class="truncate">{phase.label}</span>
              </span>
            {/each}

            {#if visiblePhases.hiddenAfter > 0}
              <span class="text-[10px] text-muted-theme font-mono shrink-0">
                +{visiblePhases.hiddenAfter} left
              </span>
            {/if}
          </div>

          <div class="flex items-center gap-2 shrink-0 text-[11px] text-muted-theme font-mono">
            {#if totalDurationMs > 0}
              <span class="text-accent-theme font-medium">{formatDuration(totalDurationMs)}</span>
            {/if}
            {#if actionableSteps.length > 0 || effectiveMilestones.length > 0}
              <button
                type="button"
                onclick={toggleExpand}
                class="font-sans text-[10px] text-secondary-theme hover:text-accent-theme transition-colors cursor-pointer flex items-center gap-0.5"
                aria-expanded={isExpanded}
              >
                <span>{isExpanded ? "Hide" : `Details (${actionableSteps.length})`}</span>
                <ChevronRight
                  size={12}
                  class="transition-transform duration-150 {isExpanded ? 'rotate-90' : ''}"
                />
              </button>
            {/if}
          </div>
        </div>
      {/if}

      {#if liveFocus}
        {@const FocusIcon = getFocusIcon(liveFocus.category)}
        <div class="px-3 py-2 flex items-center justify-between gap-2">
          <div class="flex items-center gap-2 min-w-0 flex-1">
            {#if liveFocus.category === "other" && !liveFocus.target}
              <Loader2 size={13} class="animate-spin text-accent-theme shrink-0" />
            {:else}
              <FocusIcon size={13} class="text-accent-theme shrink-0 animate-pulse" />
            {/if}
            <span class="font-medium text-primary-theme shrink-0">{liveFocus.verb}</span>
            {#if liveFocus.target}
              <span class="font-mono text-[11px] text-secondary-theme truncate" title={liveFocus.target}>
                {liveFocus.target}
              </span>
            {/if}
          </div>

          {#if phases.length === 0}
            <div class="flex items-center gap-2 shrink-0 text-[11px] text-muted-theme font-mono">
              {#if totalDurationMs > 0}
                <span class="text-accent-theme font-medium">{formatDuration(totalDurationMs)}</span>
              {/if}
              <button
                type="button"
                onclick={toggleExpand}
                class="font-sans text-[10px] text-secondary-theme hover:text-accent-theme transition-colors cursor-pointer flex items-center gap-0.5"
                aria-expanded={isExpanded}
              >
                <span>{isExpanded ? "Hide" : "Details"}</span>
                <ChevronRight
                  size={12}
                  class="transition-transform duration-150 {isExpanded ? 'rotate-90' : ''}"
                />
              </button>
            </div>
          {/if}
        </div>
      {/if}

      {#if impact.chips.length > 0}
        <div class="px-3 pb-2 pt-0.5 flex items-center gap-1.5 flex-wrap">
          {#each impact.chips as chip (chip.id)}
            {#if chip.kind === "file"}
              <button
                type="button"
                onclick={() => (onInspectFile ? onInspectFile(chip.label) : toggleExpand())}
                class="inline-flex items-center gap-1 px-2 py-0.5 rounded-md text-[10px] font-mono bg-surface-elevated border border-subtle text-secondary-theme hover:text-primary-theme hover:border-accent-theme/40 transition-colors cursor-pointer"
                title={onInspectFile ? `View session changes (${chip.label})` : chip.label}
              >
                <FileEdit size={10} class="text-sky-400 shrink-0" />
                <span>{chip.label}</span>
              </button>
            {:else if chip.kind === "command_err"}
              <button
                type="button"
                onclick={() => {
                  filterMode = "errors";
                  userExpanded = true;
                }}
                class="inline-flex items-center gap-1 px-2 py-0.5 rounded-md text-[10px] bg-rose-500/10 border border-rose-500/30 text-rose-300 hover:bg-rose-500/20 transition-colors cursor-pointer"
              >
                <AlertCircle size={10} class="text-rose-400 shrink-0" />
                <span>{chip.label}</span>
              </button>
            {:else if chip.kind === "command_ok"}
              <span class="inline-flex items-center gap-1 px-2 py-0.5 rounded-md text-[10px] bg-emerald-500/10 border border-emerald-500/25 text-emerald-300">
                <CheckCircle2 size={10} class="text-emerald-400 shrink-0" />
                <span>{chip.label}</span>
              </span>
            {/if}
          {/each}
          {#if impact.extraFileCount > 0}
            <span class="text-[10px] text-muted-theme font-mono">
              +{impact.extraFileCount} more
            </span>
          {/if}
        </div>
      {/if}

      {#if isExpanded}
        {@render inspectorDrawer()}
      {/if}
    </div>
  {:else}
    <!-- MODE B: COMPLETED 1-LINE WORK RECEIPT -->
    <div class="my-2 rounded-xl border {receipt.hasFailure ? 'border-rose-500/30 bg-rose-500/5' : 'border-subtle bg-surface/40'} overflow-hidden text-xs shadow-2xs">
      <button
        type="button"
        onclick={toggleExpand}
        class="w-full px-3 py-2 flex items-center justify-between text-left hover:bg-surface-hover/70 transition-colors cursor-pointer select-none group gap-2"
        aria-expanded={isExpanded}
      >
        <div class="flex items-center gap-2 min-w-0 pr-2">
          <ChevronRight
            size={14}
            class="text-secondary-theme transition-transform duration-200 shrink-0 {isExpanded ? 'rotate-90 text-primary-theme' : ''}"
          />
          {#if receipt.hasFailure}
            <AlertTriangle size={13} class="text-rose-400 shrink-0" />
          {:else}
            <CheckCircle2 size={13} class="text-emerald-400 shrink-0" />
          {/if}
          <span class="font-medium {receipt.hasFailure ? 'text-rose-200' : 'text-primary-theme'} truncate">
            {receipt.headline}
          </span>
        </div>

        <div class="flex items-center gap-2 shrink-0 text-[11px] text-muted-theme font-mono">
          {#if totalDurationMs > 0}
            <span>{formatDuration(totalDurationMs)}</span>
          {/if}
          <span class="text-[10px] text-accent-theme font-sans group-hover:underline">
            {isExpanded ? "Hide" : receipt.hasFailure ? "Show failure" : "Inspect"}
          </span>
        </div>
      </button>

      {#if isExpanded}
        {@render inspectorDrawer()}
      {/if}
    </div>
  {/if}
{/if}
