<script lang="ts">
  import type { ToolPermissionPayload } from "$lib/types";
  import {
    CheckCircle,
    XCircle,
    FileText,
    FileCode,
    Terminal,
  } from "lucide-svelte";

  let {
    toolPermission,
    onRespond,
  }: {
    toolPermission: ToolPermissionPayload;
    onRespond: (requestId: number, optionId?: string, allowed?: boolean) => void;
  } = $props();
</script>

<div class="mx-6 mb-3 p-4 bg-surface-elevated border-2 border-amber-500/50 rounded-xl shadow-xl transition-all duration-200">
  <div class="flex items-start justify-between gap-3">
    <div class="flex items-start gap-3">
      <div class="p-2 rounded-lg bg-amber-500/15 text-amber-500 border border-amber-500/30 mt-0.5 shrink-0">
        {#if toolPermission.kind === "edit"}
          <FileCode size={18} />
        {:else if toolPermission.kind === "read"}
          <FileText size={18} />
        {:else}
          <Terminal size={18} />
        {/if}
      </div>
      <div>
        <div class="flex items-center gap-2 flex-wrap">
          <span class="text-xs font-semibold text-primary-theme">
            Permission Request: <span class="font-mono text-accent-theme font-bold">{toolPermission.title || toolPermission.tool_name}</span>
          </span>
          {#if toolPermission.kind}
            <span class="px-1.5 py-0.5 rounded text-[10px] font-mono uppercase font-semibold bg-amber-500/15 text-amber-600 dark:text-amber-400 border border-amber-500/30">
              {toolPermission.kind}
            </span>
          {/if}
        </div>
        <div class="text-xs text-secondary-theme mt-0.5">
          {toolPermission.reason || "Gemini CLI requests confirmation to execute this action."}
        </div>
      </div>
    </div>
  </div>

  <!-- Locations / Target Paths -->
  {#if toolPermission.locations && (Array.isArray(toolPermission.locations) ? toolPermission.locations.length > 0 : true)}
    <div class="mt-2.5 p-2.5 bg-app rounded-lg border border-subtle text-xs font-mono text-primary-theme">
      <div class="text-[10px] uppercase font-sans font-semibold text-muted-theme mb-1">Target Location:</div>
      {#if Array.isArray(toolPermission.locations)}
        {#each toolPermission.locations as loc}
          <div class="truncate select-all text-accent-theme font-medium">
            {typeof loc === "string" ? loc : loc?.path || JSON.stringify(loc)}
          </div>
        {/each}
      {:else}
        <div class="truncate select-all text-accent-theme font-medium">
          {typeof toolPermission.locations === "string"
            ? toolPermission.locations
            : toolPermission.locations?.path || JSON.stringify(toolPermission.locations)}
        </div>
      {/if}
    </div>
  {/if}

  <!-- Command or Parameters Detail -->
  {#if toolPermission.parameters}
    {#if toolPermission.parameters.command || toolPermission.parameters.cmd}
      <div class="mt-2.5 p-2.5 bg-app rounded-lg border border-subtle text-xs font-mono overflow-x-auto text-primary-theme">
        <div class="text-[10px] uppercase font-sans font-semibold text-muted-theme mb-1">Command:</div>
        <code class="text-emerald-600 dark:text-emerald-400 font-semibold">{toolPermission.parameters.command || toolPermission.parameters.cmd}</code>
      </div>
    {:else if typeof toolPermission.parameters === "object" && Object.keys(toolPermission.parameters).length > 0 && !toolPermission.locations}
      <div class="mt-2.5 p-2.5 bg-app rounded-lg border border-subtle text-xs font-mono text-primary-theme max-h-32 overflow-y-auto">
        <div class="text-[10px] uppercase font-sans font-semibold text-muted-theme mb-1">Parameters:</div>
        <pre class="text-[11px] whitespace-pre-wrap text-secondary-theme">{JSON.stringify(toolPermission.parameters, null, 2)}</pre>
      </div>
    {/if}
  {/if}

  <!-- Dynamic Options Buttons -->
  <div class="flex items-center justify-end flex-wrap gap-2 mt-3.5">
    {#if toolPermission.options && toolPermission.options.length > 0}
      <div class="flex items-center flex-wrap gap-2 justify-end">
        {#each toolPermission.options as opt}
          <button
            onclick={() => onRespond(toolPermission.request_id, opt.option_id, opt.kind?.startsWith("allow") ?? true)}
            class="px-3.5 py-1.5 rounded-lg text-xs font-semibold flex items-center gap-1.5 transition-all shadow-xs cursor-pointer {
              opt.kind?.startsWith('allow') || opt.name.toLowerCase().includes('allow')
                ? 'bg-emerald-600 hover:bg-emerald-500 text-white font-bold'
                : opt.kind?.startsWith('reject') || opt.name.toLowerCase().includes('reject') || opt.name.toLowerCase().includes('deny')
                ? 'bg-rose-500/15 hover:bg-rose-500/25 text-rose-600 dark:text-rose-300 border border-rose-500/30'
                : 'bg-surface hover:bg-surface-hover text-secondary-theme hover:text-primary-theme border border-theme-default'
            }"
          >
            {#if opt.kind?.startsWith("allow") || opt.name.toLowerCase().includes("allow")}
              <CheckCircle size={14} />
            {:else if opt.kind?.startsWith("reject") || opt.name.toLowerCase().includes("reject") || opt.name.toLowerCase().includes("deny")}
              <XCircle size={14} />
            {/if}
            <span>{opt.name}</span>
          </button>
        {/each}
      </div>
    {:else}
      <!-- Fallback standard Allow/Deny -->
      <div class="flex items-center gap-2 justify-end">
        <button
          onclick={() => onRespond(toolPermission.request_id, undefined, true)}
          class="px-3.5 py-1.5 rounded-lg bg-emerald-600 hover:bg-emerald-500 text-white font-bold text-xs flex items-center gap-1.5 transition-colors shadow-xs cursor-pointer"
        >
          <CheckCircle size={14} />
          <span>Allow</span>
        </button>
        <button
          onclick={() => onRespond(toolPermission.request_id, undefined, false)}
          class="px-3.5 py-1.5 rounded-lg bg-surface hover:bg-surface-hover text-secondary-theme hover:text-rose-500 hover:border-rose-500/40 font-semibold text-xs flex items-center gap-1.5 transition-colors border border-theme-default shadow-xs cursor-pointer"
        >
          <XCircle size={14} />
          <span>Deny</span>
        </button>
      </div>
    {/if}
  </div>
</div>
