<script lang="ts">
  import type { UpdateInfo } from "$lib/types";
  import { invoke } from "@tauri-apps/api/core";
  import { openPath } from "@tauri-apps/plugin-opener";
  import { Sparkles, Terminal, Copy, Check, ExternalLink, X } from "lucide-svelte";

  let {
    updateInfo,
    onDismiss,
  }: {
    updateInfo: UpdateInfo;
    onDismiss: () => void;
  } = $props();

  let copied = $state(false);
  let isUpdating = $state(false);

  const upgradeCommand = $derived(
    `winget upgrade --id ${updateInfo.package_id || "SureshJanakiReddy.GeminiDesktop"}`
  );

  async function handleUpdateNow() {
    isUpdating = true;
    try {
      await invoke("launch_winget_upgrade", { mode: "external" });
    } catch (e) {
      console.error("Failed to launch WinGet upgrade:", e);
    } finally {
      setTimeout(() => {
        isUpdating = false;
      }, 2000);
    }
  }

  async function handleCopyCommand() {
    try {
      await navigator.clipboard.writeText(upgradeCommand);
      copied = true;
      setTimeout(() => {
        copied = false;
      }, 2500);
    } catch (e) {
      console.warn("Failed to copy upgrade command:", e);
    }
  }

  async function handleViewReleaseNotes() {
    if (updateInfo.release_url) {
      try {
        await openPath(updateInfo.release_url);
      } catch {
        window.open(updateInfo.release_url, "_blank");
      }
    }
  }

  function handleDismiss() {
    try {
      localStorage.setItem("gemini_dismissed_update_version", updateInfo.latest_version);
    } catch (e) {
      console.warn("Failed to save dismissed version:", e);
    }
    onDismiss();
  }
</script>

<div
  class="relative z-40 mx-4 mt-2 mb-1 p-3 rounded-xl border border-accent-theme/40 bg-surface/95 backdrop-blur-md shadow-lg shadow-black/20 flex flex-wrap items-center justify-between gap-3 text-xs animate-in fade-in slide-in-from-top-2 duration-300"
>
  <!-- Left info -->
  <div class="flex items-center gap-2.5 min-w-0">
    <div class="w-7 h-7 rounded-lg bg-accent-theme/15 text-accent-theme flex items-center justify-center shrink-0">
      <Sparkles class="w-4 h-4 animate-pulse" />
    </div>
    <div class="flex flex-col">
      <div class="flex items-center gap-2">
        <span class="font-semibold text-primary-theme">New Update Available on WinGet!</span>
        <span class="px-1.5 py-0.5 rounded text-[10px] font-mono font-bold bg-accent-theme/20 text-accent-theme">
          v{updateInfo.latest_version}
        </span>
      </div>
      <p class="text-[11px] text-muted-theme truncate">
        Current version is <span class="font-mono text-secondary-theme">v{updateInfo.current_version}</span>. Update with Windows Package Manager for the latest features.
      </p>
    </div>
  </div>

  <!-- Right actions -->
  <div class="flex items-center gap-2 shrink-0 ml-auto">
    <!-- 1-Click WinGet Upgrade -->
    <button
      type="button"
      onclick={handleUpdateNow}
      disabled={isUpdating}
      class="px-2.5 py-1.5 rounded-lg bg-accent-theme text-white dark:text-slate-950 font-medium hover:opacity-90 active:scale-95 transition-all flex items-center gap-1.5 shadow-sm shadow-accent-theme/20 cursor-pointer disabled:opacity-50"
      title="Launch WinGet upgrade in an interactive command prompt"
    >
      <Terminal class="w-3.5 h-3.5" />
      <span>{isUpdating ? "Launching..." : "Update via WinGet"}</span>
    </button>

    <!-- Copy command button -->
    <button
      type="button"
      onclick={handleCopyCommand}
      class="px-2 py-1.5 rounded-lg border border-subtle bg-surface-hover hover:bg-border-theme/40 text-secondary-theme hover:text-primary-theme transition-colors flex items-center gap-1.5 cursor-pointer"
      title="Copy winget upgrade command to clipboard"
    >
      {#if copied}
        <Check class="w-3.5 h-3.5 text-emerald-400" />
        <span class="text-emerald-400 font-medium">Copied!</span>
      {:else}
        <Copy class="w-3.5 h-3.5" />
        <span class="hidden sm:inline">Copy Command</span>
      {/if}
    </button>

    <!-- Release notes link -->
    {#if updateInfo.release_url}
      <button
        type="button"
        onclick={handleViewReleaseNotes}
        class="p-1.5 rounded-lg border border-subtle bg-surface-hover hover:bg-border-theme/40 text-muted-theme hover:text-primary-theme transition-colors cursor-pointer"
        title="View GitHub Release Notes"
      >
        <ExternalLink class="w-3.5 h-3.5" />
      </button>
    {/if}

    <!-- Dismiss button -->
    <button
      type="button"
      onclick={handleDismiss}
      class="p-1.5 rounded-lg text-muted-theme hover:text-primary-theme hover:bg-surface-hover transition-colors ml-1 cursor-pointer"
      title="Dismiss for this version"
      aria-label="Dismiss update notification"
    >
      <X class="w-4 h-4" />
    </button>
  </div>
</div>
