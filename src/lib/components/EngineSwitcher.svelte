<script lang="ts">
  import { engineStore } from "$lib/stores/engineStore.svelte";
  import type { CliEngine } from "$lib/types";
  import { ChevronDown, Check } from "lucide-svelte";

  let {
    variant = "compact-pill",
    onSwitch,
  }: {
    variant?: "compact-pill" | "toggle-bar" | "cards";
    onSwitch?: (engineId: CliEngine) => void | Promise<void>;
  } = $props();

  let isMenuOpen = $state(false);

  async function selectEngine(engineId: CliEngine) {
    isMenuOpen = false;
    if (engineStore.activeEngine === engineId || engineStore.isSwitching) return;
    await engineStore.switchEngine(engineId);
    if (onSwitch) {
      await onSwitch(engineId);
    }
  }
</script>

{#if variant === "compact-pill"}
  <div class="relative shrink-0">
    <button
      type="button"
      onclick={() => (isMenuOpen = !isMenuOpen)}
      disabled={engineStore.isSwitching}
      class="flex items-center gap-1.5 px-2.5 py-1 text-[11px] font-mono rounded-full bg-surface-elevated hover:bg-surface-hover border border-subtle hover:border-theme-default text-primary-theme transition-colors cursor-pointer disabled:opacity-50"
      title={`Active CLI Engine: ${engineStore.activeDescriptor.displayName} (Click to switch)`}
      aria-label="Switch CLI Engine"
      aria-expanded={isMenuOpen}
    >
      <span
        class="inline-block w-1.5 h-1.5 rounded-full {engineStore.activeDescriptor.installed ? 'bg-emerald-400' : 'bg-amber-400'}"
      ></span>
      <span>{engineStore.activeDescriptor.badgeIcon} {engineStore.activeDescriptor.id}</span>
      <ChevronDown size={11} class="text-muted-theme transition-transform {isMenuOpen ? 'rotate-180' : ''}" />
    </button>

    {#if isMenuOpen}
      <div
        class="fixed inset-0 z-35"
        onclick={() => (isMenuOpen = false)}
        role="presentation"
      ></div>
      <div class="absolute left-0 top-full mt-1.5 w-64 bg-surface-elevated border border-theme-default rounded-xl shadow-2xl py-1.5 z-40 text-xs">
        <div class="px-3 py-1 text-[10px] font-semibold uppercase tracking-wider text-muted-theme border-b border-subtle">
          CLI Agent Engine
        </div>
        {#each engineStore.catalog as eng (eng.id)}
          <button
            type="button"
            onclick={() => selectEngine(eng.id)}
            class="w-full px-3 py-2 text-left flex items-center justify-between gap-2 hover:bg-surface-hover transition-colors cursor-pointer {engineStore.activeEngine === eng.id ? 'bg-accent-theme/10 text-primary-theme' : 'text-secondary-theme'}"
          >
            <div class="min-w-0">
              <div class="font-medium text-primary-theme flex items-center gap-1.5">
                <span>{eng.badgeIcon} {eng.shortName}</span>
                <span class="font-mono text-[10px] text-muted-theme">({eng.id})</span>
              </div>
              <div class="text-[10px] flex items-center gap-1.5 mt-0.5 {eng.installed ? 'text-emerald-400' : 'text-amber-400'}">
                <span class="w-1.5 h-1.5 rounded-full {eng.installed ? 'bg-emerald-400' : 'bg-amber-400'}"></span>
                <span>{eng.installed ? "Detected" : "Not found in PATH"}</span>
              </div>
            </div>
            {#if engineStore.activeEngine === eng.id}
              <Check size={13} class="text-accent-theme shrink-0" />
            {/if}
          </button>
        {/each}
      </div>
    {/if}
  </div>
{:else if variant === "toggle-bar"}
  <div class="p-3.5 rounded-lg border border-subtle bg-surface flex items-center justify-between gap-3 flex-wrap">
    <div>
      <span class="text-xs font-semibold text-primary-theme">Active Agent Engine</span>
      <p class="text-[11px] text-muted-theme mt-0.5">
        Switch between registered CLI engines seamlessly.
      </p>
    </div>
    <div class="flex items-center gap-1.5 p-1 bg-surface-elevated rounded-lg border border-subtle flex-wrap">
      {#each engineStore.catalog as eng (eng.id)}
        <button
          type="button"
          onclick={() => selectEngine(eng.id)}
          disabled={engineStore.isSwitching}
          class="px-2.5 py-1 text-xs rounded-md font-medium transition-all cursor-pointer disabled:opacity-50 {engineStore.activeEngine === eng.id ? 'bg-accent-theme text-white shadow-xs' : 'text-secondary-theme hover:text-primary-theme'}"
        >
          {eng.badgeIcon} {eng.shortName}
        </button>
      {/each}
    </div>
  </div>
{:else}
  <div class="grid grid-cols-1 sm:grid-cols-2 gap-3 pt-1">
    {#each engineStore.catalog as eng (eng.id)}
      <button
        type="button"
        onclick={() => selectEngine(eng.id)}
        disabled={engineStore.isSwitching}
        class="flex flex-col text-left p-3 rounded-lg border transition-all cursor-pointer disabled:opacity-50 {engineStore.activeEngine === eng.id ? 'border-accent-theme bg-accent-theme/10 shadow-xs ring-1 ring-accent-theme/50' : 'border-subtle bg-surface-elevated hover:border-theme-default'}"
      >
        <div class="flex items-center justify-between w-full mb-1">
          <span class="text-xs font-semibold text-primary-theme flex items-center gap-1.5">
            {eng.badgeIcon} {eng.shortName} (<code class="font-mono text-[11px]">{eng.id}</code>)
          </span>
          {#if engineStore.activeEngine === eng.id}
            <span class="w-2 h-2 rounded-full bg-emerald-400"></span>
          {/if}
        </div>
        <p class="text-[10px] text-secondary-theme leading-normal">
          {eng.description}
        </p>
        <div class="mt-2 text-[10px] flex items-center gap-1.5 font-mono {eng.installed ? 'text-emerald-400' : 'text-amber-400'}">
          <span class="w-1.5 h-1.5 rounded-full {eng.installed ? 'bg-emerald-400' : 'bg-amber-400'}"></span>
          <span>{eng.installed ? "Installed & Detected" : "Not found in PATH"}</span>
        </div>
      </button>
    {/each}
  </div>
{/if}
