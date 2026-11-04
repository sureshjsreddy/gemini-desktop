<script lang="ts">
  import { onMount } from "svelte";
  import type { Workspace, GitStatusResult, GitFileItem } from "$lib/types";
  import { invoke } from "@tauri-apps/api/core";
  import {
    GitBranch,
    FolderGit2,
    RotateCw,
    Plus,
    Minus,
    Undo2,
    Sparkles,
    Check,
    AlertCircle,
    ChevronRight,
    ChevronDown,
    ExternalLink,
    FileText,
    Copy,
    CornerDownLeft,
    X,
    FileDiff,
    FileCode,
    Database,
    Terminal,
    Image,
    ArrowDown,
    ArrowUp,
    UploadCloud,
  } from "lucide-svelte";
  import Tooltip from "$lib/components/Tooltip.svelte";
  import { dialogManager } from "$lib/dialog.svelte";
  import { openPath, revealItemInDir } from "@tauri-apps/plugin-opener";
  import { getFileIconMeta } from "$lib/utils/fileIcons";

  let {
    workspace = null,
    isOpen = false,
    gitStatus = {
      is_git_repo: false,
      repo_name: "",
      active_branch: "",
      staged: [],
      unstaged: [],
    },
    isLoading = false,
    onRefresh,
    onCompareFile,
    getContextHint,
  }: {
    workspace: Workspace | null;
    isOpen: boolean;
    gitStatus?: GitStatusResult;
    isLoading?: boolean;
    onRefresh?: () => Promise<void> | void;
    onCompareFile?: (relPath: string, isStaged: boolean) => void;
    getContextHint?: () => string;
  } = $props();

  let isGeneratingCommit = $state(false);
  let isCommitting = $state(false);
  let isPulling = $state(false);
  let isPushing = $state(false);
  let commitMessage = $state("");
  let commitFeedback = $state<{ type: "success" | "error"; text: string } | null>(null);
  let syncFeedback = $state<{ type: "success" | "error"; text: string } | null>(null);

  // Accordion expansion states
  let isStagedOpen = $state(true);
  let isChangesOpen = $state(true);

  // Context menu state
  let contextMenu = $state<{
    x: number;
    y: number;
    item: GitFileItem;
  } | null>(null);

  let totalChanges = $derived(
    (gitStatus?.staged?.length || 0) + (gitStatus?.unstaged?.length || 0)
  );
  let canCommit = $derived(
    totalChanges > 0 && commitMessage.trim().length > 0 && !isCommitting
  );

  export async function refreshStatus() {
    if (onRefresh) {
      await onRefresh();
    }
  }

  async function handleStageFile(path: string) {
    if (!workspace?.path) return;
    commitFeedback = null;
    try {
      await invoke("git_stage_file", { workspaceDir: workspace.path, path });
      await onRefresh?.();
    } catch (err: any) {
      console.error("Failed to stage file:", err);
      commitFeedback = {
        type: "error",
        text: typeof err === "string" ? err : "Failed to stage file",
      };
    }
  }

  async function handleUnstageFile(path: string) {
    if (!workspace?.path) return;
    commitFeedback = null;
    try {
      await invoke("git_unstage_file", { workspaceDir: workspace.path, path });
      await onRefresh?.();
    } catch (err: any) {
      console.error("Failed to unstage file:", err);
      commitFeedback = {
        type: "error",
        text: typeof err === "string" ? err : "Failed to unstage file",
      };
    }
  }

  async function handleStageAll() {
    if (!workspace?.path) return;
    commitFeedback = null;
    try {
      await invoke("git_stage_all", { workspaceDir: workspace.path });
      await onRefresh?.();
    } catch (err: any) {
      console.error("Failed to stage all files:", err);
      commitFeedback = {
        type: "error",
        text: typeof err === "string" ? err : "Failed to stage all files",
      };
    }
  }

  async function handleUnstageAll() {
    if (!workspace?.path) return;
    commitFeedback = null;
    try {
      await invoke("git_unstage_all", { workspaceDir: workspace.path });
      await onRefresh?.();
    } catch (err: any) {
      console.error("Failed to unstage all files:", err);
      commitFeedback = {
        type: "error",
        text: typeof err === "string" ? err : "Failed to unstage all files",
      };
    }
  }

  async function handleUndoFile(item: GitFileItem) {
    if (!workspace?.path) return;
    const confirmed = await dialogManager.confirm(
      `Are you sure you want to discard changes in '${item.path}'?\nThis operation cannot be undone.`,
      {
        title: "Undo Changes",
        confirmText: "Discard Changes",
        isDestructive: true,
      }
    );
    if (!confirmed) return;

    commitFeedback = null;
    try {
      await invoke("git_undo_change", {
        workspaceDir: workspace.path,
        path: item.path,
        isUntracked: item.status === "untracked",
      });
      await onRefresh?.();
    } catch (err: any) {
      console.error("Failed to undo file changes:", err);
      commitFeedback = {
        type: "error",
        text: typeof err === "string" ? err : "Failed to undo file changes",
      };
    }
  }

  async function handleUndoAll() {
    if (!workspace?.path) return;
    const confirmed = await dialogManager.confirm(
      "Are you sure you want to discard ALL uncommitted changes in this repository?\nAll modified and untracked files will be permanently reverted.",
      {
        title: "Undo All Changes",
        confirmText: "Discard All Changes",
        isDestructive: true,
      }
    );
    if (!confirmed) return;

    commitFeedback = null;
    try {
      await invoke("git_undo_all", { workspaceDir: workspace.path });
      await onRefresh?.();
    } catch (err: any) {
      console.error("Failed to undo all changes:", err);
      commitFeedback = {
        type: "error",
        text: typeof err === "string" ? err : "Failed to undo all changes",
      };
    }
  }

  async function handleGenerateCommitMessage() {
    if (!workspace?.path || isGeneratingCommit) return;
    isGeneratingCommit = true;
    commitFeedback = null;
    try {
      const hint = getContextHint?.() || "";
      const msg = await invoke<string>("git_generate_commit_message", {
        workspaceDir: workspace.path,
        contextHint: hint || null,
      });
      if (msg && msg.trim()) {
        const lower = msg.toLowerCase();
        if (
          lower.includes("no git diff was included") ||
          lower.includes("direct shell execution") ||
          lower.includes("shell execution is not enabled") ||
          lower.includes("cannot retrieve git diff")
        ) {
          commitFeedback = {
            type: "error",
            text: "No git diff available to generate commit message. Please stage or save file modifications first.",
          };
        } else {
          commitMessage = msg;
        }
      }
    } catch (err: any) {
      commitFeedback = {
        type: "error",
        text: typeof err === "string" ? err : "Failed to generate commit message",
      };
    } finally {
      isGeneratingCommit = false;
    }
  }

  async function handleCommit() {
    if (!workspace?.path || !canCommit) return;
    isCommitting = true;
    commitFeedback = null;

    // Visual Studio priority rule:
    // If staged changes exist, commit ONLY staged changes.
    // If NO staged changes exist, commit all modified/untracked files.
    const commitAll = gitStatus.staged.length === 0;

    try {
      const result = await invoke<string>("git_commit", {
        workspaceDir: workspace.path,
        message: commitMessage,
        commitAll,
      });
      commitMessage = "";
      commitFeedback = {
        type: "success",
        text: result.split("\n")[0] || "Changes committed successfully.",
      };
      await refreshStatus();
      setTimeout(() => {
        if (commitFeedback?.type === "success") {
          commitFeedback = null;
        }
      }, 4000);
    } catch (err: any) {
      commitFeedback = {
        type: "error",
        text: typeof err === "string" ? err : "Git commit failed",
      };
    } finally {
      isCommitting = false;
    }
  }

  async function handlePull() {
    if (!workspace?.path || isPulling || isPushing) return;
    isPulling = true;
    syncFeedback = null;
    commitFeedback = null;
    try {
      const summary = await invoke<string>("git_pull", { workspaceDir: workspace.path });
      syncFeedback = {
        type: "success",
        text: summary.split("\n")[0] || "Pull completed successfully.",
      };
      await refreshStatus();
      setTimeout(() => {
        if (syncFeedback?.type === "success") {
          syncFeedback = null;
        }
      }, 5000);
    } catch (err: any) {
      console.error("Failed to pull from remote:", err);
      syncFeedback = {
        type: "error",
        text: typeof err === "string" ? err : "Failed to pull from remote",
      };
    } finally {
      isPulling = false;
    }
  }

  async function handlePush() {
    if (!workspace?.path || isPulling || isPushing) return;
    isPushing = true;
    syncFeedback = null;
    commitFeedback = null;
    try {
      const summary = await invoke<string>("git_push", {
        workspaceDir: workspace.path,
        setUpstream: false,
      });
      syncFeedback = {
        type: "success",
        text: summary.split("\n")[0] || "Pushed successfully.",
      };
      await refreshStatus();
      setTimeout(() => {
        if (syncFeedback?.type === "success") {
          syncFeedback = null;
        }
      }, 5000);
    } catch (err: any) {
      console.error("Failed to push to remote:", err);
      syncFeedback = {
        type: "error",
        text: typeof err === "string" ? err : "Failed to push to remote",
      };
    } finally {
      isPushing = false;
    }
  }

  async function handleCommitAndPush() {
    if (!workspace?.path || !canCommit || isCommitting || isPushing) return;
    await handleCommit();
    if (!commitFeedback || commitFeedback.type === "success") {
      await handlePush();
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
      if (canCommit) {
        e.preventDefault();
        handleCommit();
      }
    }
  }

  function resolveFullPath(relPath: string): string {
    if (!workspace?.path) return relPath;
    const base = workspace.path.replace(/[/\\]+$/, "");
    const cleanRel = relPath.replace(/^[/\\]+/, "");
    return `${base}\\${cleanRel}`.replace(/\//g, "\\");
  }

  async function handleOpenFile(relPath: string, options?: { forceNotepad?: boolean }) {
    const full = resolveFullPath(relPath);
    const forceNotepad = options?.forceNotepad ?? false;
    try {
      await invoke("open_workspace_file", {
        path: full,
        withApp: forceNotepad ? "notepad" : null,
      });
    } catch (err) {
      console.error("Failed to open file:", err);
    }
    contextMenu = null;
  }

  async function handleRevealInExplorer(relPath: string) {
    const full = resolveFullPath(relPath);
    try {
      await revealItemInDir(full);
    } catch {
      try {
        await openPath(workspace?.path || full);
      } catch (err) {
        console.error("Failed to reveal file in explorer:", err);
      }
    }
    contextMenu = null;
  }

  async function handleCopyPath(path: string, full = false) {
    const textToCopy = full ? resolveFullPath(path) : path.replace(/\\/g, "/");
    try {
      await navigator.clipboard.writeText(textToCopy);
    } catch (err) {
      console.error("Failed to copy path:", err);
    }
    contextMenu = null;
  }

  function getStatusBadge(status: string) {
    switch (status) {
      case "added":
        return {
          letter: "A",
          label: "Added",
          title: "Added — New file in Git index",
          class: "bg-emerald-500/25 text-emerald-300 border-emerald-500/40",
          textColor: "text-emerald-300",
        };
      case "modified":
        return {
          letter: "M",
          label: "Modified",
          title: "Modified — File has changes",
          class: "bg-sky-500/25 text-sky-300 border-sky-500/40",
          textColor: "text-slate-100",
        };
      case "deleted":
        return {
          letter: "D",
          label: "Deleted",
          title: "Deleted — File was removed",
          class: "bg-rose-500/25 text-rose-300 border-rose-500/40",
          textColor: "text-rose-400 line-through opacity-75",
        };
      case "renamed":
        return {
          letter: "R",
          label: "Renamed",
          title: "Renamed — File was renamed or moved",
          class: "bg-amber-500/25 text-amber-300 border-amber-500/40",
          textColor: "text-amber-200",
        };
      case "conflict":
        return {
          letter: "!",
          label: "Conflict",
          title: "Conflict — Merge conflict",
          class: "bg-red-500/30 text-red-300 border-red-500/50",
          textColor: "text-red-300 font-semibold",
        };
      case "untracked":
      default:
        return {
          letter: "U",
          label: "Untracked",
          title: "Untracked — New file on disk, not yet in Git",
          class: "bg-emerald-500/15 text-emerald-300 border-emerald-500/30",
          textColor: "text-emerald-300/90",
        };
    }
  }

  function splitFilePath(fullPath: string) {
    const normalized = fullPath.replace(/\\/g, "/");
    const lastSlash = normalized.lastIndexOf("/");
    if (lastSlash === -1) {
      return { name: normalized, dir: "" };
    }
    return {
      name: normalized.substring(lastSlash + 1),
      dir: normalized.substring(0, lastSlash),
    };
  }
</script>

<svelte:window
  onclick={() => {
    if (contextMenu) contextMenu = null;
  }}
/>

<div class="h-full flex flex-col bg-slate-900 border-l border-slate-800 text-slate-200 select-none overflow-hidden">
  <!-- Top Bar: Repository Name & Active Branch -->
  <div class="px-3 py-2.5 bg-slate-900/90 border-b border-slate-800 flex items-center justify-between gap-2">
    <div class="flex items-center gap-1.5 min-w-0 flex-1">
      <FolderGit2 class="w-4 h-4 text-emerald-400 shrink-0" />
      <span class="text-xs font-semibold text-slate-100 truncate max-w-[90px]" title={gitStatus.repo_name || "Repository"}>
        {gitStatus.repo_name || "Repository"}
      </span>
      {#if gitStatus.active_branch}
        <span
          class="text-[11px] font-mono px-1.5 py-0.5 rounded bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 flex items-center gap-1 min-w-0 max-w-[125px] sm:max-w-[150px]"
          title="{gitStatus.active_branch}{gitStatus.ahead ? ` (${gitStatus.ahead} unpushed)` : ''}{gitStatus.behind ? ` (${gitStatus.behind} unpulled)` : ''}"
        >
          <GitBranch class="w-3 h-3 shrink-0" />
          <span class="truncate">{gitStatus.active_branch}</span>
          {#if (gitStatus.ahead || 0) > 0}
            <span class="text-sky-300 font-semibold shrink-0" title="{gitStatus.ahead} commits ahead of remote">↑{gitStatus.ahead}</span>
          {/if}
          {#if (gitStatus.behind || 0) > 0}
            <span class="text-amber-300 font-semibold shrink-0" title="{gitStatus.behind} commits behind remote">↓{gitStatus.behind}</span>
          {/if}
        </span>
      {/if}
    </div>

    <div class="flex items-center gap-1 shrink-0">
      <Tooltip text="Pull from remote {gitStatus.behind ? `(${gitStatus.behind} unpulled commits)` : ''}">
        <button
          type="button"
          class="flex items-center gap-1 px-1.5 py-0.5 text-[11px] font-medium rounded text-slate-300 hover:text-white hover:bg-slate-800 border border-slate-700/60 transition-colors disabled:opacity-40"
          onclick={handlePull}
          disabled={isLoading || isPulling || isPushing}
        >
          <ArrowDown class="w-3 h-3 text-amber-400 {isPulling ? 'animate-bounce' : ''}" />
          <span>Pull</span>
          {#if (gitStatus.behind || 0) > 0}
            <span class="text-[10px] text-amber-300 font-bold font-mono">({gitStatus.behind})</span>
          {/if}
        </button>
      </Tooltip>

      <Tooltip text="Push to remote {gitStatus.ahead ? `(${gitStatus.ahead} unpushed commits)` : ''}">
        <button
          type="button"
          class="flex items-center gap-1 px-1.5 py-0.5 text-[11px] font-medium rounded text-slate-300 hover:text-white hover:bg-slate-800 border border-slate-700/60 transition-colors disabled:opacity-40"
          onclick={handlePush}
          disabled={isLoading || isPulling || isPushing}
        >
          <ArrowUp class="w-3 h-3 text-sky-400 {isPushing ? 'animate-bounce' : ''}" />
          <span>Push</span>
          {#if (gitStatus.ahead || 0) > 0}
            <span class="text-[10px] text-sky-300 font-bold font-mono">({gitStatus.ahead})</span>
          {/if}
        </button>
      </Tooltip>

      <Tooltip text="Refresh Git Status">
        <button
          type="button"
          aria-label="Refresh Git Status"
          class="p-1 rounded text-slate-400 hover:text-slate-200 hover:bg-slate-800 transition-colors"
          onclick={refreshStatus}
          disabled={isLoading || isPulling || isPushing}
        >
          <RotateCw class="w-3.5 h-3.5 {isLoading ? 'animate-spin text-emerald-400' : ''}" />
        </button>
      </Tooltip>
    </div>
  </div>

  {#if !gitStatus.is_git_repo}
    <!-- Non-Git Workspace Notice -->
    <div class="p-6 text-center text-slate-400 flex flex-col items-center justify-center h-64 gap-3">
      <AlertCircle class="w-8 h-8 text-amber-400/80" />
      <div class="text-xs font-medium text-slate-300">Not a Git repository</div>
      <div class="text-[11px] text-slate-500 max-w-xs">
        The current workspace directory is not managed under Git version control.
      </div>
    </div>
  {:else}
    <!-- Commit Message Input Box -->
    <div class="p-3 border-b border-slate-800/80 bg-slate-900/50 flex flex-col gap-2">
      <div class="relative">
        <textarea
          bind:value={commitMessage}
          onkeydown={handleKeyDown}
          placeholder="Enter a commit message (Ctrl+Enter to commit)..."
          rows="3"
          class="w-full text-xs font-sans bg-slate-950/80 border border-slate-800 rounded-md px-2.5 py-2 text-slate-100 placeholder:text-slate-500 focus:outline-none focus:border-emerald-500/60 focus:ring-1 focus:ring-emerald-500/30 transition-all resize-none"
        ></textarea>
      </div>

      <!-- Action Row: AI Generate + Commit Buttons -->
      <div class="flex items-center justify-between gap-2 flex-wrap">
        <button
          type="button"
          class="flex items-center gap-1.5 px-2 py-1 text-[11px] font-medium text-slate-300 bg-slate-800/70 hover:bg-slate-800 hover:text-emerald-400 border border-slate-700/60 rounded transition-all disabled:opacity-50"
          onclick={handleGenerateCommitMessage}
          disabled={isGeneratingCommit || totalChanges === 0}
          title="Analyze uncommitted diff and generate a Conventional Commit message"
        >
          <Sparkles class="w-3.5 h-3.5 {isGeneratingCommit ? 'animate-spin text-emerald-400' : 'text-emerald-400'}" />
          <span>{isGeneratingCommit ? "Generating..." : "AI Generate"}</span>
        </button>

        <div class="flex items-center gap-1.5">
          <button
            type="button"
            class="flex items-center gap-1 px-2.5 py-1 text-xs font-semibold rounded transition-all shadow-sm {canCommit ? 'bg-emerald-600 hover:bg-emerald-500 text-white' : 'bg-slate-800 text-slate-500 cursor-not-allowed border border-slate-700/40'}"
            onclick={handleCommit}
            disabled={!canCommit || isCommitting || isPushing || isPulling}
          >
            {#if isCommitting && !isPushing}
              <RotateCw class="w-3.5 h-3.5 animate-spin mr-1" />
              <span>Committing...</span>
            {:else if gitStatus.staged.length > 0}
              <Check class="w-3.5 h-3.5 mr-0.5" />
              <span>Commit Staged ({gitStatus.staged.length})</span>
            {:else}
              <Check class="w-3.5 h-3.5 mr-0.5" />
              <span>Commit All ({gitStatus.unstaged.length})</span>
            {/if}
          </button>

          <Tooltip text="Commit and immediately push to remote">
            <button
              type="button"
              class="flex items-center gap-1 px-2 py-1 text-xs font-semibold rounded transition-all shadow-sm {canCommit ? 'bg-sky-600 hover:bg-sky-500 text-white' : 'bg-slate-800 text-slate-500 cursor-not-allowed border border-slate-700/40'}"
              onclick={handleCommitAndPush}
              disabled={!canCommit || isCommitting || isPushing || isPulling}
            >
              {#if isCommitting || isPushing}
                <RotateCw class="w-3.5 h-3.5 animate-spin mr-0.5" />
              {:else}
                <UploadCloud class="w-3.5 h-3.5 mr-0.5" />
              {/if}
              <span>& Push</span>
            </button>
          </Tooltip>
        </div>
      </div>

      {#if commitFeedback}
        <div class="text-[11px] px-2 py-1 rounded flex items-center justify-between gap-1.5 {commitFeedback.type === 'success' ? 'bg-emerald-500/10 text-emerald-300 border border-emerald-500/20' : 'bg-rose-500/10 text-rose-300 border border-rose-500/20'}">
          <div class="flex items-center gap-1.5 min-w-0">
            {#if commitFeedback.type === 'success'}
              <Check class="w-3 h-3 shrink-0" />
            {:else}
              <AlertCircle class="w-3 h-3 shrink-0" />
            {/if}
            <span class="truncate">{commitFeedback.text}</span>
          </div>
          <button
            type="button"
            class="text-slate-400 hover:text-slate-200 p-0.5"
            onclick={() => (commitFeedback = null)}
          >
            <X class="w-3 h-3" />
          </button>
        </div>
      {/if}

      {#if syncFeedback}
        <div class="text-[11px] px-2 py-1 rounded flex items-center justify-between gap-1.5 {syncFeedback.type === 'success' ? 'bg-sky-500/10 text-sky-300 border border-sky-500/20' : 'bg-rose-500/10 text-rose-300 border border-rose-500/20'}">
          <div class="flex items-center gap-1.5 min-w-0">
            {#if syncFeedback.type === 'success'}
              <Check class="w-3 h-3 shrink-0 text-sky-400" />
            {:else}
              <AlertCircle class="w-3 h-3 shrink-0 text-rose-400" />
            {/if}
            <span class="truncate">{syncFeedback.text}</span>
          </div>
          <button
            type="button"
            class="text-slate-400 hover:text-slate-200 p-0.5"
            onclick={() => (syncFeedback = null)}
          >
            <X class="w-3 h-3" />
          </button>
        </div>
      {/if}
    </div>

    <!-- Scrollable Lists Area -->
    <div class="flex-1 overflow-y-auto divide-y divide-slate-800/60 text-xs">
      <!-- Section 1: Staged Changes -->
      <div class="py-1">
        <div class="px-2.5 py-1.5 flex items-center justify-between text-slate-400 hover:bg-slate-800/40 rounded transition-colors group">
          <button
            type="button"
            class="flex items-center gap-1 text-[11px] font-semibold text-slate-300 uppercase tracking-wider flex-1 text-left"
            onclick={() => (isStagedOpen = !isStagedOpen)}
          >
            {#if isStagedOpen}
              <ChevronDown class="w-3.5 h-3.5 text-slate-400" />
            {:else}
              <ChevronRight class="w-3.5 h-3.5 text-slate-400" />
            {/if}
            <span>Staged Changes</span>
            <span class="ml-1 text-[10px] font-mono px-1.5 py-0.2 rounded-full {gitStatus.staged.length > 0 ? 'bg-emerald-500/20 text-emerald-400 font-bold' : 'bg-slate-800 text-slate-500'}">
              {gitStatus.staged.length}
            </span>
          </button>

          {#if gitStatus.staged.length > 0}
            <div class="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
              <Tooltip text="Unstage All Changes">
                <button
                  type="button"
                  aria-label="Unstage All Changes"
                  class="p-1 rounded text-slate-400 hover:text-slate-100 hover:bg-slate-700/50"
                  onclick={handleUnstageAll}
                >
                  <Minus class="w-3 h-3" />
                </button>
              </Tooltip>
            </div>
          {/if}
        </div>

        {#if isStagedOpen}
          {#if gitStatus.staged.length === 0}
            <div class="px-7 py-2 text-[11px] text-slate-500 italic">
              No staged changes.
            </div>
          {:else}
            <div class="flex flex-col">
              {#each gitStatus.staged as item (item.path)}
                {@const badge = getStatusBadge(item.status)}
                {@const fileMeta = splitFilePath(item.path)}
                {@const iconMeta = getFileIconMeta({ name: fileMeta.name })}
                <div
                  role="button"
                  tabindex="0"
                  class="group flex items-center justify-between px-3 py-1 text-slate-300 hover:bg-slate-800/70 hover:text-slate-100 cursor-pointer transition-colors"
                  ondblclick={() => onCompareFile?.(item.path, true)}
                  onkeydown={(e) => {
                    if (e.key === "Enter") onCompareFile?.(item.path, true);
                  }}
                  oncontextmenu={(e) => {
                    e.preventDefault();
                    contextMenu = { x: e.clientX, y: e.clientY, item };
                  }}
                >
                  <div class="flex items-center gap-2 min-w-0 flex-1 pr-2">
                    <Tooltip text="{badge.label} — {badge.title}" position="right">
                      <span class="text-[10px] font-mono font-bold px-1 rounded border {badge.class} shrink-0 cursor-help" aria-label="{badge.label} file">
                        {badge.letter}
                      </span>
                    </Tooltip>

                    <div class="w-3.5 h-3.5 flex items-center justify-center shrink-0">
                      {#if iconMeta.badge}
                        <span
                          class="font-mono text-[8px] font-bold px-0.5 rounded leading-none flex items-center justify-center shrink-0 border"
                          style="color: {iconMeta.color}; border-color: {iconMeta.color}40; background-color: {iconMeta.color}15;"
                        >
                          {iconMeta.badge}
                        </span>
                      {:else if iconMeta.type === "code"}
                        <FileCode size={13} style="color: {iconMeta.color};" />
                      {:else if iconMeta.type === "database"}
                        <Database size={13} style="color: {iconMeta.color};" />
                      {:else if iconMeta.type === "terminal"}
                        <Terminal size={13} style="color: {iconMeta.color};" />
                      {:else if iconMeta.type === "image"}
                        <Image size={13} style="color: {iconMeta.color};" />
                      {:else}
                        <FileText size={13} style="color: {iconMeta.color};" />
                      {/if}
                    </div>

                    <span class="truncate font-medium {badge.textColor}" title="{fileMeta.name} ({badge.label})">
                      {fileMeta.name}
                    </span>
                    {#if fileMeta.dir}
                      <span class="truncate text-[10px] text-slate-500 font-mono">
                        {fileMeta.dir}
                      </span>
                    {/if}
                  </div>

                  <!-- Quick Action Buttons -->
                  <div class="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity shrink-0">
                    <Tooltip text="Compare with HEAD">
                      <button
                        type="button"
                        aria-label="Compare with HEAD"
                        class="p-1 rounded text-slate-400 hover:text-emerald-400 hover:bg-slate-700"
                        onclick={(e) => {
                          e.stopPropagation();
                          onCompareFile?.(item.path, true);
                        }}
                      >
                        <FileDiff class="w-3.5 h-3.5" />
                      </button>
                    </Tooltip>
                    <Tooltip text="Unstage this file">
                      <button
                        type="button"
                        aria-label="Unstage this file"
                        class="p-1 rounded text-slate-400 hover:text-amber-400 hover:bg-slate-700"
                        onclick={(e) => {
                          e.stopPropagation();
                          handleUnstageFile(item.path);
                        }}
                      >
                        <Minus class="w-3.5 h-3.5" />
                      </button>
                    </Tooltip>
                  </div>
                </div>
              {/each}
            </div>
          {/if}
        {/if}
      </div>

      <!-- Section 2: Changes (Unstaged) -->
      <div class="py-1">
        <div class="px-2.5 py-1.5 flex items-center justify-between text-slate-400 hover:bg-slate-800/40 rounded transition-colors group">
          <button
            type="button"
            class="flex items-center gap-1 text-[11px] font-semibold text-slate-300 uppercase tracking-wider flex-1 text-left"
            onclick={() => (isChangesOpen = !isChangesOpen)}
          >
            {#if isChangesOpen}
              <ChevronDown class="w-3.5 h-3.5 text-slate-400" />
            {:else}
              <ChevronRight class="w-3.5 h-3.5 text-slate-400" />
            {/if}
            <span>Changes</span>
            <span class="ml-1 text-[10px] font-mono px-1.5 py-0.2 rounded-full {gitStatus.unstaged.length > 0 ? 'bg-sky-500/20 text-sky-400 font-bold' : 'bg-slate-800 text-slate-500'}">
              {gitStatus.unstaged.length}
            </span>
          </button>

          {#if gitStatus.unstaged.length > 0}
            <div class="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
              <Tooltip text="Stage All Changes">
                <button
                  type="button"
                  aria-label="Stage All Changes"
                  class="p-1 rounded text-slate-400 hover:text-slate-100 hover:bg-slate-700/50"
                  onclick={handleStageAll}
                >
                  <Plus class="w-3 h-3" />
                </button>
              </Tooltip>
              <Tooltip text="Undo All Changes">
                <button
                  type="button"
                  aria-label="Undo All Changes"
                  class="p-1 rounded text-slate-400 hover:text-rose-400 hover:bg-slate-700/50"
                  onclick={handleUndoAll}
                >
                  <Undo2 class="w-3 h-3" />
                </button>
              </Tooltip>
            </div>
          {/if}
        </div>

        {#if isChangesOpen}
          {#if gitStatus.unstaged.length === 0}
            <div class="px-7 py-2 text-[11px] text-slate-500 italic">
              No unstaged changes.
            </div>
          {:else}
            <div class="flex flex-col">
              {#each gitStatus.unstaged as item (item.path)}
                {@const badge = getStatusBadge(item.status)}
                {@const fileMeta = splitFilePath(item.path)}
                {@const iconMeta = getFileIconMeta({ name: fileMeta.name })}
                <div
                  role="button"
                  tabindex="0"
                  class="group flex items-center justify-between px-3 py-1 text-slate-300 hover:bg-slate-800/70 hover:text-slate-100 cursor-pointer transition-colors"
                  ondblclick={() => onCompareFile?.(item.path, false)}
                  onkeydown={(e) => {
                    if (e.key === "Enter") onCompareFile?.(item.path, false);
                  }}
                  oncontextmenu={(e) => {
                    e.preventDefault();
                    contextMenu = { x: e.clientX, y: e.clientY, item };
                  }}
                >
                  <div class="flex items-center gap-2 min-w-0 flex-1 pr-2">
                    <Tooltip text="{badge.label} — {badge.title}" position="right">
                      <span class="text-[10px] font-mono font-bold px-1 rounded border {badge.class} shrink-0 cursor-help" aria-label="{badge.label} file">
                        {badge.letter}
                      </span>
                    </Tooltip>

                    <div class="w-3.5 h-3.5 flex items-center justify-center shrink-0">
                      {#if iconMeta.badge}
                        <span
                          class="font-mono text-[8px] font-bold px-0.5 rounded leading-none flex items-center justify-center shrink-0 border"
                          style="color: {iconMeta.color}; border-color: {iconMeta.color}40; background-color: {iconMeta.color}15;"
                        >
                          {iconMeta.badge}
                        </span>
                      {:else if iconMeta.type === "code"}
                        <FileCode size={13} style="color: {iconMeta.color};" />
                      {:else if iconMeta.type === "database"}
                        <Database size={13} style="color: {iconMeta.color};" />
                      {:else if iconMeta.type === "terminal"}
                        <Terminal size={13} style="color: {iconMeta.color};" />
                      {:else if iconMeta.type === "image"}
                        <Image size={13} style="color: {iconMeta.color};" />
                      {:else}
                        <FileText size={13} style="color: {iconMeta.color};" />
                      {/if}
                    </div>

                    <span class="truncate font-medium {badge.textColor}" title="{fileMeta.name} ({badge.label})">
                      {fileMeta.name}
                    </span>
                    {#if fileMeta.dir}
                      <span class="truncate text-[10px] text-slate-500 font-mono">
                        {fileMeta.dir}
                      </span>
                    {/if}
                  </div>

                  <!-- Quick Action Buttons -->
                  <div class="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity shrink-0">
                    <Tooltip text="Compare with Unmodified">
                      <button
                        type="button"
                        aria-label="Compare with Unmodified"
                        class="p-1 rounded text-slate-400 hover:text-sky-400 hover:bg-slate-700"
                        onclick={(e) => {
                          e.stopPropagation();
                          onCompareFile?.(item.path, false);
                        }}
                      >
                        <FileDiff class="w-3.5 h-3.5" />
                      </button>
                    </Tooltip>
                    <Tooltip text="Stage this file">
                      <button
                        type="button"
                        aria-label="Stage this file"
                        class="p-1 rounded text-slate-400 hover:text-emerald-400 hover:bg-slate-700"
                        onclick={(e) => {
                          e.stopPropagation();
                          handleStageFile(item.path);
                        }}
                      >
                        <Plus class="w-3.5 h-3.5" />
                      </button>
                    </Tooltip>
                    <Tooltip text="Undo changes">
                      <button
                        type="button"
                        aria-label="Undo changes"
                        class="p-1 rounded text-slate-400 hover:text-rose-400 hover:bg-slate-700"
                        onclick={(e) => {
                          e.stopPropagation();
                          handleUndoFile(item);
                        }}
                      >
                        <Undo2 class="w-3.5 h-3.5" />
                      </button>
                    </Tooltip>
                  </div>
                </div>
              {/each}
            </div>
          {/if}
        {/if}
      </div>
    </div>
  {/if}

  <!-- Right-Click Context Menu -->
  {#if contextMenu}
    {@const menuBadge = getStatusBadge(contextMenu.item.status)}
    {@const menuFileMeta = splitFilePath(contextMenu.item.path)}
    <div
      class="fixed z-50 bg-slate-900 border border-slate-700 rounded-md shadow-2xl py-1 w-56 text-xs text-slate-200 backdrop-blur-md"
      style="left: {Math.min(contextMenu.x, window.innerWidth - 235)}px; top: {Math.min(contextMenu.y, window.innerHeight - 300)}px;"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => {
        if (e.key === "Escape") contextMenu = null;
      }}
      role="menu"
      tabindex="-1"
    >
      <!-- Visual Studio Context Menu Header -->
      <div class="px-2.5 py-1.5 border-b border-slate-800 bg-slate-950/70 flex items-center justify-between gap-1.5 mb-1">
        <div class="flex items-center gap-1.5 min-w-0">
          <span class="text-[9px] font-mono font-bold px-1 py-0.2 rounded border {menuBadge.class} shrink-0">
            {menuBadge.letter}
          </span>
          <span class="truncate font-semibold text-slate-200 text-[11px]" title={contextMenu.item.path}>
            {menuFileMeta.name}
          </span>
        </div>
        <span class="text-[9px] font-mono uppercase tracking-wider px-1 py-0.5 rounded bg-slate-800/80 text-slate-400 shrink-0">
          {contextMenu.item.is_staged ? "Staged" : "Unstaged"}
        </span>
      </div>

      {#if contextMenu.item.is_staged}
        <button
          type="button"
          class="w-full text-left px-3 py-1.5 hover:bg-slate-800 flex items-center gap-2 text-amber-300"
          onclick={() => {
            handleUnstageFile(contextMenu!.item.path);
            contextMenu = null;
          }}
        >
          <Minus class="w-3.5 h-3.5" />
          <span>Unstage</span>
        </button>
        <button
          type="button"
          class="w-full text-left px-3 py-1.5 hover:bg-slate-800 flex items-center gap-2 text-slate-200"
          onclick={() => {
            onCompareFile?.(contextMenu!.item.path, true);
            contextMenu = null;
          }}
        >
          <FileDiff class="w-3.5 h-3.5 text-emerald-400" />
          <span>Compare with HEAD</span>
        </button>
      {:else}
        <button
          type="button"
          class="w-full text-left px-3 py-1.5 hover:bg-slate-800 flex items-center gap-2 text-emerald-300"
          onclick={() => {
            handleStageFile(contextMenu!.item.path);
            contextMenu = null;
          }}
        >
          <Plus class="w-3.5 h-3.5" />
          <span>Stage</span>
        </button>
        <button
          type="button"
          class="w-full text-left px-3 py-1.5 hover:bg-slate-800 flex items-center gap-2 text-rose-300"
          onclick={() => {
            const item = contextMenu!.item;
            contextMenu = null;
            handleUndoFile(item);
          }}
        >
          <Undo2 class="w-3.5 h-3.5" />
          <span>Undo Changes...</span>
        </button>
        <button
          type="button"
          class="w-full text-left px-3 py-1.5 hover:bg-slate-800 flex items-center gap-2 text-slate-200"
          onclick={() => {
            onCompareFile?.(contextMenu!.item.path, false);
            contextMenu = null;
          }}
        >
          <FileDiff class="w-3.5 h-3.5 text-sky-400" />
          <span>Compare with Unmodified</span>
        </button>
      {/if}

      <div class="h-px bg-slate-800 my-1"></div>

      <button
        type="button"
        class="w-full text-left px-3 py-1.5 hover:bg-slate-800 flex items-center gap-2"
        onclick={() => handleOpenFile(contextMenu!.item.path)}
      >
        <ExternalLink class="w-3.5 h-3.5 text-slate-400" />
        <span>Open with Default App</span>
      </button>

      <button
        type="button"
        class="w-full text-left px-3 py-1.5 hover:bg-slate-800 flex items-center gap-2"
        onclick={() => handleOpenFile(contextMenu!.item.path, { forceNotepad: true })}
      >
        <FileText class="w-3.5 h-3.5 text-slate-400" />
        <span>Open with Notepad</span>
      </button>

      <button
        type="button"
        class="w-full text-left px-3 py-1.5 hover:bg-slate-800 flex items-center gap-2"
        onclick={() => handleRevealInExplorer(contextMenu!.item.path)}
      >
        <CornerDownLeft class="w-3.5 h-3.5 text-slate-400" />
        <span>Reveal in File Explorer</span>
      </button>

      <div class="h-px bg-slate-800 my-1"></div>

      <button
        type="button"
        class="w-full text-left px-3 py-1.5 hover:bg-slate-800 flex items-center gap-2"
        onclick={() => handleCopyPath(contextMenu!.item.path, false)}
      >
        <Copy class="w-3.5 h-3.5 text-slate-400" />
        <span>Copy Relative Path</span>
      </button>

      <button
        type="button"
        class="w-full text-left px-3 py-1.5 hover:bg-slate-800 flex items-center gap-2"
        onclick={() => handleCopyPath(contextMenu!.item.path, true)}
      >
        <Copy class="w-3.5 h-3.5 text-slate-400" />
        <span>Copy Full Path</span>
      </button>
    </div>
  {/if}
</div>
