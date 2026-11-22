<script lang="ts">
  import type { ToolExecutionStep } from "$lib/types";
  import { resolveHumanFriendlyTitle, formatDuration } from "$lib/utils/toolSteps";
  import {
    ChevronRight,
    CheckCircle2,
    AlertCircle,
    Loader2,
    Terminal,
    FileCode,
    Search,
    Wrench,
    Copy,
    Check,
  } from "lucide-svelte";

  let {
    step,
    forceExpanded = null,
  }: {
    step: ToolExecutionStep;
    forceExpanded?: boolean | null;
  } = $props();

  // User manual override: null means "use default based on step.status", true/false means user explicitly toggled it
  let userExpanded = $state<boolean | null>(null);
  let copiedSection = $state<string | null>(null);

  $effect(() => {
    if (forceExpanded !== null && forceExpanded !== undefined) {
      userExpanded = forceExpanded;
    }
  });

  // Derived: if user hasn't explicitly toggled, open while in_progress/pending; collapsed once completed
  let isOpen = $derived(
    userExpanded !== null ? userExpanded : (step.status === "in_progress" || step.status === "pending")
  );

  function toggleOpen() {
    userExpanded = !isOpen;
  }

  function getToolIcon(name: string, title?: string) {
    const combined = `${name || ""} ${title || ""}`.toLowerCase();
    if (combined.includes("terminal") || combined.includes("bash") || combined.includes("command") || combined.includes("shell") || combined.includes("exec")) {
      return Terminal;
    }
    if (combined.includes("read") || combined.includes("write") || combined.includes("edit") || combined.includes("patch") || combined.includes("file")) {
      return FileCode;
    }
    if (combined.includes("search") || combined.includes("grep") || combined.includes("find") || combined.includes("list")) {
      return Search;
    }
    return Wrench;
  }

  let ToolIcon = $derived(getToolIcon(step.tool_name, step.title));

  let displayTitle = $derived.by(() => {
    const summary = (step.summary || "").trim();
    if (summary && !summary.includes("call_") && !summary.includes("tool-") && !summary.endsWith(": tool")) {
      return summary;
    }
    const cleanTitle = resolveHumanFriendlyTitle(step);
    if (step.status === "completed") {
      return `✅ ${cleanTitle}`;
    }
    if (step.status === "in_progress" || step.status === "pending") {
      return `⚙️ ${cleanTitle}`;
    }
    if (step.status === "failed") {
      return `❌ ${cleanTitle}`;
    }
    return cleanTitle;
  });

  async function copyText(text: string, sectionId: string) {
    try {
      await navigator.clipboard.writeText(text);
      copiedSection = sectionId;
      setTimeout(() => {
        if (copiedSection === sectionId) copiedSection = null;
      }, 1500);
    } catch (e) {
      console.warn("Failed to copy tool details:", e);
    }
  }

  function formatParams(params: any): string {
    if (!params) return "";
    if (typeof params === "string") return params;
    try {
      return JSON.stringify(params, null, 2);
    } catch {
      return String(params);
    }
  }

  let formattedParams = $derived(formatParams(step.parameters));
</script>

<div class="my-1.5 rounded-xl border border-subtle bg-surface/60 overflow-hidden text-xs transition-all shadow-xs">
  <!-- Single-line Summary Header (Always Visible) -->
  <button
    type="button"
    onclick={toggleOpen}
    class="w-full px-3 py-2 flex items-center justify-between text-left hover:bg-surface-hover/80 transition-colors cursor-pointer select-none group"
    aria-expanded={isOpen}
  >
    <div class="flex items-center gap-2 truncate min-w-0 pr-2">
      <!-- Chevron Toggle Indicator -->
      <ChevronRight
        size={14}
        class="text-secondary-theme transition-transform duration-200 shrink-0 {isOpen ? 'rotate-90 text-primary-theme' : ''}"
      />

      <!-- Status Icon -->
      {#if step.status === "in_progress" || step.status === "pending"}
        <Loader2 size={13} class="animate-spin text-accent-theme shrink-0" />
      {:else if step.status === "completed"}
        <CheckCircle2 size={13} class="text-emerald-400 shrink-0" />
      {:else}
        <AlertCircle size={13} class="text-rose-400 shrink-0" />
      {/if}

      <!-- Tool Category Icon & Title -->
      <div class="flex items-center gap-1.5 truncate min-w-0">
        <ToolIcon size={13} class="text-secondary-theme shrink-0" />
        <span class="font-mono font-medium text-primary-theme truncate">
          {displayTitle}
        </span>
      </div>
    </div>

    <!-- Right Metadata Badge: Duration & Status -->
    <div class="flex items-center gap-2 shrink-0 text-[11px] text-muted-theme font-mono">
      {#if step.status === "in_progress" || step.status === "pending"}
        <span class="text-accent-theme animate-pulse font-sans font-medium text-[11px]">Running...</span>
      {:else if step.duration_ms && step.duration_ms > 0}
        <span>{formatDuration(step.duration_ms)}</span>
      {/if}
      <span
        class="text-[10px] uppercase font-semibold tracking-wider px-1.5 py-0.5 rounded border {step.status === 'completed'
          ? 'bg-emerald-950/30 text-emerald-400 border-emerald-500/20'
          : step.status === 'failed'
          ? 'bg-rose-950/30 text-rose-400 border-rose-500/20'
          : 'bg-accent-subtle text-accent-theme border-accent-subtle'}"
      >
        {step.status === "in_progress" ? "RUNNING" : step.status}
      </span>
    </div>
  </button>

  <!-- Expanded Details Section (Hidden by default once completed) -->
  {#if isOpen}
    <div class="px-3.5 py-2.5 border-t border-subtle bg-code/40 text-[11px] font-mono space-y-2.5">
      {#if formattedParams}
        <div>
          <div class="flex items-center justify-between text-muted-theme mb-1">
            <span class="font-semibold uppercase text-[10px] tracking-wider">Input Parameters</span>
            <button
              type="button"
              onclick={() => copyText(formattedParams, 'params')}
              class="flex items-center gap-1 text-[10px] hover:text-primary-theme text-secondary-theme transition-colors cursor-pointer"
            >
              {#if copiedSection === 'params'}
                <Check size={11} class="text-emerald-400" />
                <span class="text-emerald-400">Copied</span>
              {:else}
                <Copy size={11} />
                <span>Copy</span>
              {/if}
            </button>
          </div>
          <pre class="p-2 rounded-lg bg-surface-elevated/70 text-secondary-theme overflow-x-auto max-h-40 overflow-y-auto whitespace-pre-wrap leading-relaxed border border-subtle">{formattedParams}</pre>
        </div>
      {/if}

      {#if step.raw_output}
        <div>
          <div class="flex items-center justify-between text-muted-theme mb-1">
            <span class="font-semibold uppercase text-[10px] tracking-wider">Output</span>
            <button
              type="button"
              onclick={() => copyText(step.raw_output || '', 'output')}
              class="flex items-center gap-1 text-[10px] hover:text-primary-theme text-secondary-theme transition-colors cursor-pointer"
            >
              {#if copiedSection === 'output'}
                <Check size={11} class="text-emerald-400" />
                <span class="text-emerald-400">Copied</span>
              {:else}
                <Copy size={11} />
                <span>Copy</span>
              {/if}
            </button>
          </div>
          <pre class="p-2 rounded-lg bg-surface-elevated/70 text-primary-theme overflow-x-auto max-h-48 overflow-y-auto whitespace-pre-wrap leading-relaxed border border-subtle">{step.raw_output}</pre>
        </div>
      {/if}

      {#if step.error}
        <div>
          <div class="text-rose-400 mb-1 font-semibold uppercase text-[10px] tracking-wider">Error Details</div>
          <pre class="p-2 rounded-lg bg-rose-950/20 text-rose-300 overflow-x-auto whitespace-pre-wrap leading-relaxed border border-rose-500/20">{step.error}</pre>
        </div>
      {/if}

      {#if !formattedParams && !step.raw_output && !step.error}
        <div class="text-muted-theme italic py-1">
          {step.status === "in_progress" ? "Executing task in background..." : "Task completed with no additional output."}
        </div>
      {/if}
    </div>
  {/if}
</div>
