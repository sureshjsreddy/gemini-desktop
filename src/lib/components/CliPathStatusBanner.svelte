<script lang="ts">
  import { engineStore } from "$lib/stores/engineStore.svelte";
  import { Globe, Check, CheckCircle2, AlertCircle } from "lucide-svelte";

  let {
    engineId,
  }: {
    engineId?: string;
  } = $props();

  let descriptor = $derived(engineStore.getDescriptor(engineId));
  let isInPath = $derived.by(() => {
    if (descriptor.id === "agy" && engineStore.cliPathStatus) {
      return engineStore.cliPathStatus.agyInUserPath;
    }
    if (descriptor.id === "gemini" && engineStore.cliPathStatus) {
      return engineStore.cliPathStatus.geminiInUserPath;
    }
    return descriptor.inUserPath;
  });

  let binaryDir = $derived.by(() => {
    if (descriptor.id === "agy" && engineStore.cliPathStatus?.agyDir) {
      return engineStore.cliPathStatus.agyDir;
    }
    if (descriptor.id === "gemini" && engineStore.cliPathStatus?.geminiDir) {
      return engineStore.cliPathStatus.geminiDir;
    }
    return descriptor.binaryDir || null;
  });
</script>

<div class="space-y-2">
  <div class="p-2.5 rounded-lg bg-surface-elevated border border-subtle flex items-center justify-between text-xs gap-2">
    <div class="min-w-0 mr-2">
      <div class="flex items-center gap-1.5 flex-wrap">
        <Globe size={13} class="text-accent-theme shrink-0" />
        <span class="font-medium text-primary-theme">{descriptor.displayName}</span>
        {#if isInPath}
          <span class="px-1.5 py-0.5 rounded text-[10px] bg-emerald-500/20 text-emerald-400 font-mono font-medium flex items-center gap-1">
            <Check size={10} /> In User PATH
          </span>
        {:else}
          <span class="px-1.5 py-0.5 rounded text-[10px] bg-amber-500/20 text-amber-400 font-mono font-medium">
            Not in User PATH
          </span>
        {/if}
      </div>
      <p class="text-[11px] font-mono text-muted-theme truncate mt-0.5">
        {binaryDir || "Directory not located"}
      </p>
    </div>

    {#if !isInPath}
      <button
        type="button"
        onclick={() => engineStore.addToPath(descriptor.id)}
        disabled={engineStore.isAddingToPath || !binaryDir}
        class="px-2.5 py-1 text-[11px] font-medium rounded-md bg-accent-theme/15 hover:bg-accent-theme/25 text-accent-theme border border-accent-theme/30 transition-colors cursor-pointer disabled:opacity-50 shrink-0"
      >
        {engineStore.isAddingToPath ? "Adding..." : "Add to PATH"}
      </button>
    {/if}
  </div>

  {#if engineStore.pathAddResult}
    <div class="p-2.5 rounded-lg text-xs flex items-start gap-2 {engineStore.pathAddResult.success ? 'bg-emerald-500/10 border border-emerald-500/30 text-emerald-300' : 'bg-rose-500/10 border border-rose-500/30 text-rose-300'}">
      {#if engineStore.pathAddResult.success}
        <CheckCircle2 size={14} class="text-emerald-400 shrink-0 mt-0.5" />
      {:else}
        <AlertCircle size={14} class="text-rose-400 shrink-0 mt-0.5" />
      {/if}
      <span class="leading-relaxed">{engineStore.pathAddResult.message}</span>
    </div>
  {/if}
</div>
