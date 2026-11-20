<script lang="ts">
  import type { ParsedDebugError, FileSnippetResult } from "$lib/types";
  import { formatDebugPrompt } from "$lib/utils/debugParser";
  import {
    Bug,
    Sparkles,
    FileCode,
    X,
    ChevronDown,
    ChevronUp,
    AlertCircle,
    Copy,
    Check,
    Edit3
  } from "lucide-svelte";

  let {
    debugError,
    fileSnippet = null,
    onDiagnose,
    onPrefill,
    onDismiss,
  }: {
    debugError: ParsedDebugError;
    fileSnippet?: FileSnippetResult | null;
    onDiagnose: (prompt: string) => void;
    onPrefill: (prompt: string) => void;
    onDismiss: () => void;
  } = $props();

  let isSnippetExpanded = $state(true);
  let isTraceExpanded = $state(false);
  let hasCopied = $state(false);

  let formattedPrompt = $derived(formatDebugPrompt(debugError, fileSnippet));

  async function handleCopyPrompt() {
    try {
      await navigator.clipboard.writeText(formattedPrompt);
      hasCopied = true;
      setTimeout(() => {
        hasCopied = false;
      }, 2000);
    } catch {}
  }
</script>

<div class="mx-auto w-full max-w-4xl lg:max-w-5xl xl:max-w-6xl 2xl:max-w-7xl mb-3 rounded-xl border border-rose-500/30 bg-rose-950/20 backdrop-blur-md shadow-lg overflow-hidden transition-all animate-in fade-in slide-in-from-top-2 duration-200">
  <!-- Header Bar -->
  <div class="px-3.5 py-2.5 bg-rose-950/40 border-b border-rose-500/20 flex items-center justify-between gap-3">
    <div class="flex items-center gap-2 min-w-0">
      <div class="p-1 rounded-lg bg-rose-500/20 text-rose-400 border border-rose-500/30 shrink-0">
        <Bug size={15} />
      </div>
      <div class="min-w-0 flex items-center gap-2 flex-wrap">
        <span class="text-xs font-semibold text-rose-200 tracking-wide flex items-center gap-1.5">
          <span>Instant Debug Doctor</span>
          <span class="text-[10px] px-1.5 py-0.5 rounded-full bg-rose-500/20 text-rose-300 font-mono font-medium border border-rose-500/30">
            {debugError.sourceFramework?.toUpperCase() || "DEBUG"}
          </span>
        </span>
        <span class="text-xs text-rose-300/80 font-mono font-medium truncate max-w-[280px]" title={debugError.exceptionType}>
          {debugError.exceptionType}
        </span>
      </div>
    </div>

    <!-- Right Controls -->
    <div class="flex items-center gap-1.5 shrink-0">
      <button
        type="button"
        onclick={handleCopyPrompt}
        class="text-xs text-rose-300/80 hover:text-rose-100 p-1 rounded hover:bg-rose-500/10 transition-colors flex items-center gap-1 cursor-pointer"
        title="Copy diagnostic prompt to clipboard"
      >
        {#if hasCopied}
          <Check size={12} class="text-emerald-400" />
          <span class="text-[10px] text-emerald-400">Copied</span>
        {:else}
          <Copy size={12} />
        {/if}
      </button>

      <button
        type="button"
        onclick={onDismiss}
        class="text-rose-400/80 hover:text-rose-200 p-1 rounded-md hover:bg-rose-500/20 transition-colors cursor-pointer"
        title="Dismiss debug exception banner"
        aria-label="Dismiss banner"
      >
        <X size={14} />
      </button>
    </div>
  </div>

  <!-- Body Content -->
  <div class="p-3 text-xs space-y-2.5">
    <!-- Message & Location Details -->
    <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-2 text-rose-100">
      <div class="min-w-0 flex-1">
        {#if debugError.message}
          <p class="font-medium text-rose-200 leading-relaxed text-[12px]">
            {debugError.message}
          </p>
        {/if}
      </div>

      {#if debugError.fileName || debugError.filePath}
        <div class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-md bg-surface-elevated/70 border border-rose-500/30 text-rose-200 text-[11px] font-mono shrink-0 shadow-xs">
          <FileCode size={12} class="text-rose-400" />
          <span class="truncate max-w-[200px]" title={debugError.filePath || debugError.fileName}>
            {debugError.fileName || "File"}
          </span>
          {#if debugError.lineNumber}
            <span class="text-rose-400 font-semibold">:line {debugError.lineNumber}</span>
          {/if}
        </div>
      {/if}
    </div>

    <!-- Source Snippet Preview -->
    {#if fileSnippet && fileSnippet.exists && fileSnippet.snippet}
      <div class="rounded-lg border border-subtle/80 bg-surface/80 overflow-hidden">
        <button
          type="button"
          onclick={() => (isSnippetExpanded = !isSnippetExpanded)}
          class="w-full px-2.5 py-1 bg-surface-elevated/50 flex items-center justify-between text-[11px] text-secondary-theme hover:text-primary-theme transition-colors cursor-pointer"
        >
          <span class="flex items-center gap-1.5 font-medium">
            <span class="text-accent-theme">📄 Local Source Context:</span>
            <span class="font-mono text-[10px] text-muted-theme">{fileSnippet.file_name} (lines {fileSnippet.start_line}–{fileSnippet.end_line})</span>
          </span>
          <span class="text-muted-theme">
            {#if isSnippetExpanded}
              <ChevronUp size={12} />
            {:else}
              <ChevronDown size={12} />
            {/if}
          </span>
        </button>

        {#if isSnippetExpanded}
          <pre class="p-2.5 text-[11px] font-mono leading-relaxed overflow-x-auto text-primary-theme bg-surface max-h-48 selection:bg-rose-500/30"><code class="language-{fileSnippet.language}">{fileSnippet.snippet}</code></pre>
        {/if}
      </div>
    {:else if debugError.filePath}
      <div class="flex items-center gap-1.5 text-[11px] text-amber-300/80 bg-amber-950/20 px-2.5 py-1.5 rounded-lg border border-amber-500/20">
        <AlertCircle size={13} class="shrink-0 text-amber-400" />
        <span>Target file not located in current workspace. Full trace attached for diagnosis.</span>
      </div>
    {/if}

    <!-- Call Stack Drawer Toggle -->
    <div>
      <button
        type="button"
        onclick={() => (isTraceExpanded = !isTraceExpanded)}
        class="text-[10px] text-muted-theme hover:text-rose-300 flex items-center gap-1 cursor-pointer transition-colors"
      >
        <span>{isTraceExpanded ? "Hide" : "Show"} raw stack trace ({debugError.rawTrace.split('\n').length} lines)</span>
        {#if isTraceExpanded}
          <ChevronUp size={10} />
        {:else}
          <ChevronDown size={10} />
        {/if}
      </button>

      {#if isTraceExpanded}
        <pre class="mt-1 p-2 rounded-md bg-surface text-[10px] font-mono leading-normal text-muted-theme overflow-x-auto max-h-36 border border-subtle">{debugError.rawTrace}</pre>
      {/if}
    </div>

    <!-- Action Buttons -->
    <div class="pt-1 flex items-center justify-end gap-2 border-t border-rose-500/20">
      <button
        type="button"
        onclick={() => onPrefill(formattedPrompt)}
        class="px-2.5 py-1.5 rounded-lg bg-surface hover:bg-surface-hover text-secondary-theme hover:text-primary-theme border border-theme-default transition-colors text-xs flex items-center gap-1.5 cursor-pointer"
        title="Open and edit diagnostic prompt in composer"
      >
        <Edit3 size={12} />
        <span>Edit Prompt</span>
      </button>

      <button
        type="button"
        onclick={() => onDiagnose(formattedPrompt)}
        class="px-3 py-1.5 rounded-lg bg-rose-600 hover:bg-rose-500 text-white font-medium shadow-md shadow-rose-950/30 transition-all text-xs flex items-center gap-1.5 cursor-pointer active:scale-98"
        title="Send structured diagnosis prompt to Gemini"
      >
        <Sparkles size={13} />
        <span>⚡ Analyze & Fix with Gemini</span>
      </button>
    </div>
  </div>
</div>
