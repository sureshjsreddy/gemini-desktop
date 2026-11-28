<script lang="ts">
  import type { Workspace, GitStatusResult, GitFileItem } from "$lib/types";
  import { invoke } from "@tauri-apps/api/core";
  import { openPath, revealItemInDir } from "@tauri-apps/plugin-opener";
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
    ShieldCheck,
    Search,
    Paperclip,
    AtSign,
    CheckCircle2,
  } from "lucide-svelte";
  import Tooltip from "$lib/components/Tooltip.svelte";
  import { dialogManager } from "$lib/dialog.svelte";
  import { getFileIconMeta } from "$lib/utils/fileIcons";
  import {
    getGitStatusBadge,
    splitGitFilePath,
    toWorkspaceRelativeGitPath,
  } from "$lib/utils/gitStatus";

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
    onAttachFile,
    onInsertMention,
    getContextHint,
  }: {
    workspace: Workspace | null;
    isOpen?: boolean;
    gitStatus?: GitStatusResult;
    isLoading?: boolean;
    onRefresh?: () => Promise<void> | void;
    onCompareFile?: (relPath: string, isStaged: boolean) => void;
    onAttachFile?: (relPath: string, name?: string) => void;
    onInsertMention?: (relPath: string) => void;
    getContextHint?: () => string;
  } = $props();

  let isGeneratingCommit = $state(false);
  let isCommitting = $state(false);
  let isPulling = $state(false);
  let isPushing = $state(false);
  let isRepoActionBusy = $state(false);
  let commitMessage = $state("");
  let filterText = $state("");
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

  let stagedList = $derived(gitStatus?.staged || []);
  let unstagedList = $derived(gitStatus?.unstaged || []);
  let totalChanges = $derived(stagedList.length + unstagedList.length);

  let normalizedFilter = $derived(filterText.trim().toLowerCase());
  let filteredStaged = $derived(
    normalizedFilter
      ? stagedList.filter((i) => i.path.toLowerCase().includes(normalizedFilter))
      : stagedList
  );
  let filteredUnstaged = $derived(
    normalizedFilter
      ? unstagedList.filter((i) => i.path.toLowerCase().includes(normalizedFilter))
      : unstagedList
  );

  let canCommit = $derived(
    totalChanges > 0 && commitMessage.trim().length > 0 && !isCommitting
  );

  // Reset filter when panel closes or workspace changes
  $effect(() => {
    const _ws = workspace?.path;
    filterText = "";
    commitFeedback = null;
    syncFeedback = null;
  });

  export async function refreshStatus() {
    if (onRefresh) {
      await onRefresh();
    }
  }

  async function handleTrustRepository() {
    if (!workspace?.path || isRepoActionBusy) return;
    isRepoActionBusy = true;
    try {
      await invoke("git_trust_repository", { workspaceDir: workspace.path });
      await refreshStatus();
    } catch (err: any) {
      await dialogManager.alert(
        typeof err === "string" ? err : err?.message || "Failed to update Git safe.directory",
        "Git Safe Directory"
      );
    } finally {
      isRepoActionBusy = false;
    }
  }

  async function handleInitRepo() {
    if (!workspace?.path || isRepoActionBusy) return;
    isRepoActionBusy = true;
    try {
      await invoke("git_init_repo", { workspaceDir: workspace.path });
      await refreshStatus();
    } catch (err: any) {
      await dialogManager.alert(
        typeof err === "string" ? err : err?.message || "Failed to initialize Git repository",
        "Initialize Git"
      );
    } finally {
      isRepoActionBusy = false;
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

  async function handleCommit(): Promise<boolean> {
    if (!workspace?.path || !canCommit) return false;
    isCommitting = true;
    commitFeedback = null;

    // Visual Studio priority rule:
    // If staged changes exist, commit ONLY staged changes.
    // If NO staged changes exist, commit all modified/untracked files.
    const commitAll = stagedList.length === 0;

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
      return true;
    } catch (err: any) {
      commitFeedback = {
        type: "error",
        text: typeof err === "string" ? err : "Git commit failed",
      };
      return false;
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
    const ok = await handleCommit();
    if (ok) {
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
    const baseDir = gitStatus?.repo_root || workspace?.path || "";
    if (!baseDir) return relPath;
    const base = baseDir.replace(/[/\\]+$/, "");
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
        workspacePath: gitStatus?.repo_root || workspace?.path,
      });
    } catch (err) {
      console.error("Failed to open file:", err);
      await dialogManager.alert(
        `Unable to open "${relPath}".\n\n${err instanceof Error ? err.message : String(err)}`,
        "Cannot Open File"
      );
    }
    contextMenu = null;
  }

  async function handleRevealInExplorer(relPath: string) {
    const full = resolveFullPath(relPath);
    try {
      await revealItemInDir(full);
    } catch {
      try {
        await openPath(gitStatus?.repo_root || workspace?.path || full);
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

  function handleAttachGitItem(item: GitFileItem) {
    const wsRel = toWorkspaceRelativeGitPath(item.path, workspace?.path, gitStatus?.repo_root);
    const target = wsRel || item.path;
    const fileMeta = splitGitFilePath(target);
    onAttachFile?.(target, fileMeta.name);
    contextMenu = null;
  }

  function handleMentionGitItem(item: GitFileItem) {
    const wsRel = toWorkspaceRelativeGitPath(item.path, workspace?.path, gitStatus?.repo_root);
    onInsertMention?.(wsRel || item.path);
    contextMenu = null;
  }
</script>

<svelte:window
  onclick={() => {
    if (contextMenu) contextMenu = null;
  }}
/>

<div
  class="h-full flex flex-col bg-sidebar text-primary-theme select-none overflow-hidden"
  data-git-panel-open={isOpen}
>
  <!-- Top Bar: Repository Name, Active Branch & Sync Controls -->
  <div class="px-2.5 py-2 bg-surface/60 border-b border-subtle flex items-center justify-between gap-2">
    <div class="flex items-center gap-1.5 min-w-0 flex-1">
      <FolderGit2 class="w-4 h-4 text-emerald-500 shrink-0" />
      <span
        class="text-xs font-semibold text-primary-theme truncate max-w-[90px]"
        title={gitStatus.repo_name || "Repository"}
      >
        {gitStatus.repo_name || "Repository"}
      </span>
      {#if gitStatus.is_git_repo && gitStatus.active_branch}
        <span
          class="text-[11px] font-mono px-1.5 py-0.5 rounded bg-emerald-500/12 text-emerald-500 border border-emerald-500/25 flex items-center gap-1 min-w-0 max-w-[125px] sm:max-w-[150px]"
          title="{gitStatus.active_branch}{gitStatus.ahead ? ` (${gitStatus.ahead} unpushed)` : ''}{gitStatus.behind ? ` (${gitStatus.behind} unpulled)` : ''}"
        >
          <GitBranch class="w-3 h-3 shrink-0" />
          <span class="truncate">{gitStatus.active_branch}</span>
          {#if (gitStatus.ahead || 0) > 0}
            <span class="text-sky-500 font-semibold shrink-0" title="{gitStatus.ahead} commits ahead">
              ↑{gitStatus.ahead}
            </span>
          {/if}
          {#if (gitStatus.behind || 0) > 0}
            <span class="text-amber-500 font-semibold shrink-0" title="{gitStatus.behind} commits behind">
              ↓{gitStatus.behind}
            </span>
          {/if}
        </span>
      {/if}
    </div>

    <div class="flex items-center gap-1 shrink-0">
      {#if gitStatus.is_git_repo}
        <Tooltip
          text="Pull from remote{gitStatus.behind ? ` (${gitStatus.behind} incoming)` : ''}"
          position="bottom"
        >
          <button
            type="button"
            class="flex items-center gap-1 px-1.5 py-0.5 text-[11px] font-medium rounded text-secondary-theme hover:text-primary-theme hover:bg-surface-hover border border-theme-default transition-colors disabled:opacity-40 cursor-pointer"
            onclick={handlePull}
            disabled={isLoading || isPulling || isPushing}
          >
            <ArrowDown class="w-3 h-3 text-amber-500 {isPulling ? 'animate-bounce' : ''}" />
            <span>Pull</span>
            {#if (gitStatus.behind || 0) > 0}
              <span class="text-[10px] text-amber-500 font-bold font-mono">({gitStatus.behind})</span>
            {/if}
          </button>
        </Tooltip>

        <Tooltip
          text="Push to remote{gitStatus.ahead ? ` (${gitStatus.ahead} outgoing)` : ''}"
          position="bottom"
        >
          <button
            type="button"
            class="flex items-center gap-1 px-1.5 py-0.5 text-[11px] font-medium rounded text-secondary-theme hover:text-primary-theme hover:bg-surface-hover border border-theme-default transition-colors disabled:opacity-40 cursor-pointer"
            onclick={handlePush}
            disabled={isLoading || isPulling || isPushing}
          >
            <ArrowUp class="w-3 h-3 text-sky-500 {isPushing ? 'animate-bounce' : ''}" />
            <span>Push</span>
            {#if (gitStatus.ahead || 0) > 0}
              <span class="text-[10px] text-sky-500 font-bold font-mono">({gitStatus.ahead})</span>
            {/if}
          </button>
        </Tooltip>
      {/if}

      <Tooltip text="Refresh Git Status" position="bottom">
        <button
          type="button"
          aria-label="Refresh Git Status"
          class="p-1 rounded text-muted-theme hover:text-primary-theme hover:bg-surface-hover transition-colors cursor-pointer"
          onclick={refreshStatus}
          disabled={isLoading || isPulling || isPushing}
        >
          <RotateCw class="w-3.5 h-3.5 {isLoading ? 'animate-spin text-emerald-500' : ''}" />
        </button>
      </Tooltip>
    </div>
  </div>

  {#if !gitStatus.is_git_repo}
    <!-- Diagnostic or Non-Git Workspace States -->
    <div class="p-6 text-center text-secondary-theme flex flex-col items-center justify-center flex-1 gap-3">
      {#if gitStatus.error_kind === "dubious_ownership"}
        <div class="p-2.5 rounded-full bg-amber-500/15 border border-amber-500/30 text-amber-500">
          <ShieldCheck class="w-6 h-6" />
        </div>
        <div class="text-xs font-semibold text-primary-theme">Repository Ownership Check (safe.directory)</div>
        <div class="text-[11px] text-muted-theme max-w-xs leading-relaxed">
          Git blocked access because this directory has a different system owner. Add it to your Git <code class="font-mono">safe.directory</code> list to inspect changes.
        </div>
        <button
          type="button"
          onclick={handleTrustRepository}
          disabled={isRepoActionBusy}
          class="flex items-center gap-1.5 px-3 py-1.5 rounded-md bg-emerald-600 hover:bg-emerald-500 text-white text-xs font-medium transition-colors cursor-pointer disabled:opacity-50"
        >
          <ShieldCheck class="w-3.5 h-3.5" />
          <span>{isRepoActionBusy ? "Trusting..." : "Trust Repository"}</span>
        </button>
      {:else if gitStatus.error_kind === "git_not_found"}
        <AlertCircle class="w-8 h-8 text-rose-500/80" />
        <div class="text-xs font-semibold text-primary-theme">Git CLI not found</div>
        <div class="text-[11px] text-muted-theme max-w-xs leading-relaxed">
          Install Git for Windows or ensure <code class="font-mono">git.exe</code> is available on your PATH.
        </div>
        <button
          type="button"
          onclick={refreshStatus}
          class="px-3 py-1.5 rounded-md bg-surface-elevated hover:bg-surface-hover border border-theme-default text-xs font-medium cursor-pointer"
        >
          Retry
        </button>
      {:else if gitStatus.error_kind === "git_error"}
        <AlertCircle class="w-8 h-8 text-amber-500/80" />
        <div class="text-xs font-semibold text-primary-theme">Unable to read Git status</div>
        <div class="text-[11px] text-muted-theme max-w-xs break-words">
          {gitStatus.error_message || "Unknown Git error."}
        </div>
        <button
          type="button"
          onclick={refreshStatus}
          class="px-3 py-1.5 rounded-md bg-surface-elevated hover:bg-surface-hover border border-theme-default text-xs font-medium cursor-pointer"
        >
          Retry
        </button>
      {:else}
        <FolderGit2 class="w-8 h-8 text-muted-theme/70" />
        <div class="text-xs font-semibold text-primary-theme">Not a Git repository</div>
        <div class="text-[11px] text-muted-theme max-w-xs leading-relaxed">
          The current workspace folder is not initialized with Git version control.
        </div>
        {#if workspace?.path}
          <button
            type="button"
            onclick={handleInitRepo}
            disabled={isRepoActionBusy}
            class="flex items-center gap-1.5 px-3 py-1.5 rounded-md bg-surface-elevated hover:bg-surface-hover border border-theme-default text-primary-theme text-xs font-medium transition-colors cursor-pointer disabled:opacity-50"
          >
            <FolderGit2 class="w-3.5 h-3.5 text-emerald-500" />
            <span>{isRepoActionBusy ? "Initializing..." : "Initialize Git Repository"}</span>
          </button>
        {/if}
      {/if}
    </div>
  {:else}
    <!-- Commit Message Box (shown always so feedback persists after commit, compact & clean) -->
    <div class="p-2.5 border-b border-subtle bg-surface/35 flex flex-col gap-2">
      <div class="relative">
        <textarea
          bind:value={commitMessage}
          onkeydown={handleKeyDown}
          placeholder="Commit message (Ctrl+Enter to commit)..."
          rows="3"
          class="w-full text-xs font-sans bg-app border border-theme-default rounded-md px-2.5 py-2 text-primary-theme placeholder:text-muted-theme focus:outline-none focus:border-emerald-500/70 focus:ring-1 focus:ring-emerald-500/30 transition-all resize-none"
        ></textarea>
      </div>

      <!-- Action Row: AI Generate + Commit Buttons -->
      <div class="flex items-center justify-between gap-2 flex-wrap">
        <button
          type="button"
          class="flex items-center gap-1.5 px-2 py-1 text-[11px] font-medium text-secondary-theme bg-surface hover:bg-surface-hover hover:text-emerald-500 border border-theme-default rounded transition-all disabled:opacity-45 cursor-pointer"
          onclick={handleGenerateCommitMessage}
          disabled={isGeneratingCommit || totalChanges === 0}
          title="Analyze uncommitted diff and generate a Conventional Commit message"
        >
          <Sparkles class="w-3.5 h-3.5 {isGeneratingCommit ? 'animate-spin text-emerald-500' : 'text-emerald-500'}" />
          <span>{isGeneratingCommit ? "Generating..." : "AI Generate"}</span>
        </button>

        <div class="flex items-center gap-1.5">
          <button
            type="button"
            class="flex items-center gap-1 px-2.5 py-1 text-xs font-semibold rounded transition-all shadow-xs {canCommit ? 'bg-emerald-600 hover:bg-emerald-500 text-white cursor-pointer' : 'bg-surface text-muted-theme cursor-not-allowed border border-subtle'}"
            onclick={handleCommit}
            disabled={!canCommit || isCommitting || isPushing || isPulling}
          >
            {#if isCommitting && !isPushing}
              <RotateCw class="w-3.5 h-3.5 animate-spin mr-0.5" />
              <span>Committing...</span>
            {:else if stagedList.length > 0}
              <Check class="w-3.5 h-3.5 mr-0.5" />
              <span>Commit Staged ({stagedList.length})</span>
            {:else}
              <Check class="w-3.5 h-3.5 mr-0.5" />
              <span>Commit All ({unstagedList.length})</span>
            {/if}
          </button>

          <Tooltip text="Commit and immediately push to remote" position="left">
            <button
              type="button"
              class="flex items-center gap-1 px-2 py-1 text-xs font-semibold rounded transition-all shadow-xs {canCommit ? 'bg-sky-600 hover:bg-sky-500 text-white cursor-pointer' : 'bg-surface text-muted-theme cursor-not-allowed border border-subtle'}"
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
        <div class="text-[11px] px-2 py-1 rounded flex items-center justify-between gap-1.5 {commitFeedback.type === 'success' ? 'bg-emerald-500/12 text-emerald-500 border border-emerald-500/25' : 'bg-rose-500/12 text-rose-500 border border-rose-500/25'}">
          <div class="flex items-center gap-1.5 min-w-0">
            {#if commitFeedback.type === 'success'}
              <Check class="w-3 h-3 shrink-0" />
            {:else}
              <AlertCircle class="w-3 h-3 shrink-0" />
            {/if}
            <span class="truncate" title={commitFeedback.text}>{commitFeedback.text}</span>
          </div>
          <button
            type="button"
            class="text-muted-theme hover:text-primary-theme p-0.5 cursor-pointer"
            onclick={() => (commitFeedback = null)}
            aria-label="Dismiss commit notification"
          >
            <X class="w-3 h-3" />
          </button>
        </div>
      {/if}

      {#if syncFeedback}
        <div class="text-[11px] px-2 py-1 rounded flex items-center justify-between gap-1.5 {syncFeedback.type === 'success' ? 'bg-sky-500/12 text-sky-500 border border-sky-500/25' : 'bg-rose-500/12 text-rose-500 border border-rose-500/25'}">
          <div class="flex items-center gap-1.5 min-w-0">
            {#if syncFeedback.type === 'success'}
              <Check class="w-3 h-3 shrink-0" />
            {:else}
              <AlertCircle class="w-3 h-3 shrink-0" />
            {/if}
            <span class="truncate" title={syncFeedback.text}>{syncFeedback.text}</span>
          </div>
          <button
            type="button"
            class="text-muted-theme hover:text-primary-theme p-0.5 cursor-pointer"
            onclick={() => (syncFeedback = null)}
            aria-label="Dismiss sync notification"
          >
            <X class="w-3 h-3" />
          </button>
        </div>
      {/if}
    </div>

    {#if totalChanges === 0}
      <!-- Clean Working Tree Empty State -->
      <div class="flex-1 flex flex-col items-center justify-center p-6 text-center gap-2.5 text-muted-theme">
        <CheckCircle2 class="w-7 h-7 text-emerald-500/75" />
        <div class="text-xs font-semibold text-primary-theme">Working tree clean</div>
        <p class="text-[11px] text-muted-theme max-w-[210px] leading-relaxed">
          {#if (gitStatus.ahead || 0) > 0}
            {gitStatus.ahead} local commit{gitStatus.ahead === 1 ? "" : "s"} ready to push.
          {:else if (gitStatus.behind || 0) > 0}
            {gitStatus.behind} incoming commit{gitStatus.behind === 1 ? "" : "s"} available to pull.
          {:else}
            No uncommitted changes on <span class="font-mono text-secondary-theme">{gitStatus.active_branch || "HEAD"}</span>.
          {/if}
        </p>
        <div class="flex items-center gap-2 mt-1">
          {#if (gitStatus.ahead || 0) > 0}
            <button
              type="button"
              onclick={handlePush}
              disabled={isPushing || isPulling}
              class="flex items-center gap-1 px-2.5 py-1 rounded-md bg-sky-600 hover:bg-sky-500 text-white text-xs font-medium cursor-pointer disabled:opacity-50"
            >
              <ArrowUp class="w-3.5 h-3.5" />
              <span>Push ({gitStatus.ahead})</span>
            </button>
          {/if}
          {#if (gitStatus.behind || 0) > 0}
            <button
              type="button"
              onclick={handlePull}
              disabled={isPushing || isPulling}
              class="flex items-center gap-1 px-2.5 py-1 rounded-md bg-amber-600 hover:bg-amber-500 text-white text-xs font-medium cursor-pointer disabled:opacity-50"
            >
              <ArrowDown class="w-3.5 h-3.5" />
              <span>Pull ({gitStatus.behind})</span>
            </button>
          {/if}
        </div>
      </div>
    {:else}
      <!-- Filter Bar (shown only when there are 6+ changed files to keep small lists zero-clutter) -->
      {#if totalChanges >= 6}
        <div class="px-2.5 py-1.5 border-b border-subtle/70 bg-sidebar">
          <div class="relative flex items-center">
            <Search size={12} class="absolute left-2 text-muted-theme pointer-events-none" />
            <input
              type="text"
              bind:value={filterText}
              placeholder="Filter {totalChanges} changed files..."
              class="w-full pl-7 pr-6 py-1 text-xs bg-app text-primary-theme placeholder:text-muted-theme rounded border border-theme-default focus:border-emerald-500/70 focus:outline-none"
            />
            {#if filterText}
              <button
                type="button"
                onclick={() => (filterText = "")}
                class="absolute right-1.5 text-muted-theme hover:text-primary-theme p-0.5 cursor-pointer"
                aria-label="Clear filter"
              >
                <X size={12} />
              </button>
            {/if}
          </div>
        </div>
      {/if}

      <!-- Scrollable Lists Area -->
      <div class="flex-1 overflow-y-auto divide-y divide-subtle/70 text-xs">
        <!-- Section 1: Staged Changes -->
        <div class="py-1">
          <div class="px-2.5 py-1.5 flex items-center justify-between text-secondary-theme hover:bg-surface-hover/50 rounded transition-colors group">
            <button
              type="button"
              class="flex items-center gap-1 text-[11px] font-semibold text-primary-theme uppercase tracking-wider flex-1 text-left cursor-pointer"
              onclick={() => (isStagedOpen = !isStagedOpen)}
            >
              {#if isStagedOpen}
                <ChevronDown class="w-3.5 h-3.5 text-muted-theme" />
              {:else}
                <ChevronRight class="w-3.5 h-3.5 text-muted-theme" />
              {/if}
              <span>Staged Changes</span>
              <span class="ml-1 text-[10px] font-mono px-1.5 py-0.2 rounded-full {stagedList.length > 0 ? 'bg-emerald-500/20 text-emerald-500 font-bold' : 'bg-surface text-muted-theme'}">
                {normalizedFilter ? `${filteredStaged.length}/${stagedList.length}` : stagedList.length}
              </span>
            </button>

            {#if stagedList.length > 0}
              <div class="flex items-center gap-1 opacity-75 group-hover:opacity-100 transition-opacity">
                <Tooltip text="Unstage All Changes" position="left">
                  <button
                    type="button"
                    aria-label="Unstage All Changes"
                    class="p-1 rounded text-secondary-theme hover:text-primary-theme hover:bg-surface-hover cursor-pointer"
                    onclick={handleUnstageAll}
                  >
                    <Minus class="w-3 h-3" />
                  </button>
                </Tooltip>
              </div>
            {/if}
          </div>

          {#if isStagedOpen}
            {#if filteredStaged.length === 0}
              <div class="px-7 py-2 text-[11px] text-muted-theme italic">
                {normalizedFilter ? "No staged files match filter." : "No staged changes."}
              </div>
            {:else}
              <div class="flex flex-col">
                {#each filteredStaged as item (item.path)}
                  {@render gitFileRow(item, true)}
                {/each}
              </div>
            {/if}
          {/if}
        </div>

        <!-- Section 2: Changes (Unstaged) -->
        <div class="py-1">
          <div class="px-2.5 py-1.5 flex items-center justify-between text-secondary-theme hover:bg-surface-hover/50 rounded transition-colors group">
            <button
              type="button"
              class="flex items-center gap-1 text-[11px] font-semibold text-primary-theme uppercase tracking-wider flex-1 text-left cursor-pointer"
              onclick={() => (isChangesOpen = !isChangesOpen)}
            >
              {#if isChangesOpen}
                <ChevronDown class="w-3.5 h-3.5 text-muted-theme" />
              {:else}
                <ChevronRight class="w-3.5 h-3.5 text-muted-theme" />
              {/if}
              <span>Changes</span>
              <span class="ml-1 text-[10px] font-mono px-1.5 py-0.2 rounded-full {unstagedList.length > 0 ? 'bg-sky-500/20 text-sky-500 font-bold' : 'bg-surface text-muted-theme'}">
                {normalizedFilter ? `${filteredUnstaged.length}/${unstagedList.length}` : unstagedList.length}
              </span>
            </button>

            {#if unstagedList.length > 0}
              <div class="flex items-center gap-1 opacity-75 group-hover:opacity-100 transition-opacity">
                <Tooltip text="Stage All Changes" position="left">
                  <button
                    type="button"
                    aria-label="Stage All Changes"
                    class="p-1 rounded text-secondary-theme hover:text-emerald-500 hover:bg-surface-hover cursor-pointer"
                    onclick={handleStageAll}
                  >
                    <Plus class="w-3 h-3" />
                  </button>
                </Tooltip>
                <Tooltip text="Undo All Changes" position="left">
                  <button
                    type="button"
                    aria-label="Undo All Changes"
                    class="p-1 rounded text-secondary-theme hover:text-rose-500 hover:bg-surface-hover cursor-pointer"
                    onclick={handleUndoAll}
                  >
                    <Undo2 class="w-3 h-3" />
                  </button>
                </Tooltip>
              </div>
            {/if}
          </div>

          {#if isChangesOpen}
            {#if filteredUnstaged.length === 0}
              <div class="px-7 py-2 text-[11px] text-muted-theme italic">
                {normalizedFilter ? "No unstaged files match filter." : "No unstaged changes."}
              </div>
            {:else}
              <div class="flex flex-col">
                {#each filteredUnstaged as item (item.path)}
                  {@render gitFileRow(item, false)}
                {/each}
              </div>
            {/if}
          {/if}
        </div>
      </div>
    {/if}
  {/if}

  <!-- Right-Click Context Menu -->
  {#if contextMenu}
    {@const menuBadge = getGitStatusBadge(contextMenu.item.status)}
    {@const menuFileMeta = splitGitFilePath(contextMenu.item.path)}
    <div
      class="fixed z-50 bg-surface-elevated border border-theme-default rounded-lg shadow-2xl py-1 w-56 text-xs text-primary-theme backdrop-blur-md"
      style="left: {Math.min(contextMenu.x, window.innerWidth - 235)}px; top: {Math.min(contextMenu.y, window.innerHeight - 330)}px;"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => {
        if (e.key === "Escape") contextMenu = null;
      }}
      role="menu"
      tabindex="-1"
    >
      <!-- Visual Studio Context Menu Header -->
      <div class="px-2.5 py-1.5 border-b border-subtle bg-surface/70 flex items-center justify-between gap-1.5 mb-1">
        <div class="flex items-center gap-1.5 min-w-0">
          <span class="text-[9px] font-mono font-bold px-1 py-0.2 rounded border {menuBadge.badgeClass} shrink-0">
            {menuBadge.letter}
          </span>
          <span class="truncate font-semibold text-primary-theme text-[11px]" title={contextMenu.item.path}>
            {menuFileMeta.name}
          </span>
        </div>
        <span class="text-[9px] font-mono uppercase tracking-wider px-1 py-0.5 rounded bg-surface-hover text-muted-theme shrink-0">
          {contextMenu.item.is_staged ? "Staged" : "Unstaged"}
        </span>
      </div>

      {#if contextMenu.item.is_staged}
        <button
          type="button"
          class="w-full text-left px-3 py-1.5 hover:bg-surface-hover flex items-center gap-2 text-amber-500 cursor-pointer"
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
          class="w-full text-left px-3 py-1.5 hover:bg-surface-hover flex items-center gap-2 text-primary-theme cursor-pointer"
          onclick={() => {
            onCompareFile?.(contextMenu!.item.path, true);
            contextMenu = null;
          }}
        >
          <FileDiff class="w-3.5 h-3.5 text-emerald-500" />
          <span>Compare with HEAD</span>
        </button>
      {:else}
        <button
          type="button"
          class="w-full text-left px-3 py-1.5 hover:bg-surface-hover flex items-center gap-2 text-emerald-500 cursor-pointer"
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
          class="w-full text-left px-3 py-1.5 hover:bg-surface-hover flex items-center gap-2 text-rose-500 cursor-pointer"
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
          class="w-full text-left px-3 py-1.5 hover:bg-surface-hover flex items-center gap-2 text-primary-theme cursor-pointer"
          onclick={() => {
            onCompareFile?.(contextMenu!.item.path, false);
            contextMenu = null;
          }}
        >
          <FileDiff class="w-3.5 h-3.5 text-sky-500" />
          <span>Compare with Unmodified</span>
        </button>
      {/if}

      {#if (onAttachFile || onInsertMention) && contextMenu.item.status !== "deleted"}
        <div class="h-px bg-subtle my-1"></div>

        {#if onAttachFile}
          <button
            type="button"
            class="w-full text-left px-3 py-1.5 hover:bg-surface-hover flex items-center gap-2 text-accent-theme font-medium cursor-pointer"
            onclick={() => handleAttachGitItem(contextMenu!.item)}
          >
            <Paperclip class="w-3.5 h-3.5" />
            <span>Attach to Chat</span>
          </button>
        {/if}

        {#if onInsertMention}
          <button
            type="button"
            class="w-full text-left px-3 py-1.5 hover:bg-surface-hover flex items-center gap-2 cursor-pointer"
            onclick={() => handleMentionGitItem(contextMenu!.item)}
          >
            <AtSign class="w-3.5 h-3.5 text-secondary-theme" />
            <span>Mention in Prompt (@)</span>
          </button>
        {/if}
      {/if}

      {#if contextMenu.item.status !== "deleted"}
        <div class="h-px bg-subtle my-1"></div>

        <button
          type="button"
          class="w-full text-left px-3 py-1.5 hover:bg-surface-hover flex items-center gap-2 cursor-pointer"
          onclick={() => handleOpenFile(contextMenu!.item.path)}
        >
          <ExternalLink class="w-3.5 h-3.5 text-secondary-theme" />
          <span>Open with Default App</span>
        </button>

        <button
          type="button"
          class="w-full text-left px-3 py-1.5 hover:bg-surface-hover flex items-center gap-2 cursor-pointer"
          onclick={() => handleOpenFile(contextMenu!.item.path, { forceNotepad: true })}
        >
          <FileText class="w-3.5 h-3.5 text-secondary-theme" />
          <span>Open with Notepad</span>
        </button>

        <button
          type="button"
          class="w-full text-left px-3 py-1.5 hover:bg-surface-hover flex items-center gap-2 cursor-pointer"
          onclick={() => handleRevealInExplorer(contextMenu!.item.path)}
        >
          <CornerDownLeft class="w-3.5 h-3.5 text-secondary-theme" />
          <span>Reveal in File Explorer</span>
        </button>
      {/if}

      <div class="h-px bg-subtle my-1"></div>

      <button
        type="button"
        class="w-full text-left px-3 py-1.5 hover:bg-surface-hover flex items-center gap-2 cursor-pointer"
        onclick={() => handleCopyPath(contextMenu!.item.path, false)}
      >
        <Copy class="w-3.5 h-3.5 text-secondary-theme" />
        <span>Copy Relative Path</span>
      </button>

      <button
        type="button"
        class="w-full text-left px-3 py-1.5 hover:bg-surface-hover flex items-center gap-2 cursor-pointer"
        onclick={() => handleCopyPath(contextMenu!.item.path, true)}
      >
        <Copy class="w-3.5 h-3.5 text-muted-theme" />
        <span>Copy Full Path</span>
      </button>
    </div>
  {/if}
</div>

{#snippet gitFileRow(item: GitFileItem, isStaged: boolean)}
  {@const badge = getGitStatusBadge(item.status)}
  {@const fileMeta = splitGitFilePath(item.path)}
  {@const iconMeta = getFileIconMeta({ name: fileMeta.name })}

  <div
    role="button"
    tabindex="0"
    class="group flex items-center justify-between px-3 py-1 text-secondary-theme hover:bg-surface-hover/80 hover:text-primary-theme cursor-pointer transition-colors"
    ondblclick={() => onCompareFile?.(item.path, isStaged)}
    onkeydown={(e) => {
      if (e.key === "Enter") onCompareFile?.(item.path, isStaged);
    }}
    oncontextmenu={(e) => {
      e.preventDefault();
      contextMenu = { x: e.clientX, y: e.clientY, item };
    }}
  >
    <div class="flex items-center gap-2 min-w-0 flex-1 pr-2">
      <span
        class="text-[10px] font-mono font-bold px-1 rounded border {badge.badgeClass} shrink-0"
        title={badge.title}
        aria-label="{badge.label} file"
      >
        {badge.letter}
      </span>

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

      <span class="truncate font-medium {badge.textColor}" title="{item.path} ({badge.label})">
        {fileMeta.name}
      </span>
      {#if fileMeta.dir}
        <span class="truncate text-[10px] text-muted-theme font-mono">
          {fileMeta.dir}
        </span>
      {/if}
    </div>

    <!-- Quick Action Buttons -->
    <div class="flex items-center gap-0.5 opacity-0 group-hover:opacity-100 group-focus-within:opacity-100 transition-opacity shrink-0">
      <Tooltip text={isStaged ? "Compare with HEAD" : "Compare with Unmodified"} position="left">
        <button
          type="button"
          aria-label={isStaged ? "Compare with HEAD" : "Compare with Unmodified"}
          class="p-1 rounded text-muted-theme hover:text-sky-500 hover:bg-surface-elevated cursor-pointer"
          onclick={(e) => {
            e.stopPropagation();
            onCompareFile?.(item.path, isStaged);
          }}
        >
          <FileDiff class="w-3.5 h-3.5" />
        </button>
      </Tooltip>

      {#if isStaged}
        <Tooltip text="Unstage this file" position="left">
          <button
            type="button"
            aria-label="Unstage this file"
            class="p-1 rounded text-muted-theme hover:text-amber-500 hover:bg-surface-elevated cursor-pointer"
            onclick={(e) => {
              e.stopPropagation();
              handleUnstageFile(item.path);
            }}
          >
            <Minus class="w-3.5 h-3.5" />
          </button>
        </Tooltip>
      {:else}
        <Tooltip text="Stage this file" position="left">
          <button
            type="button"
            aria-label="Stage this file"
            class="p-1 rounded text-muted-theme hover:text-emerald-500 hover:bg-surface-elevated cursor-pointer"
            onclick={(e) => {
              e.stopPropagation();
              handleStageFile(item.path);
            }}
          >
            <Plus class="w-3.5 h-3.5" />
          </button>
        </Tooltip>
        <Tooltip text="Undo changes" position="left">
          <button
            type="button"
            aria-label="Undo changes"
            class="p-1 rounded text-muted-theme hover:text-rose-500 hover:bg-surface-elevated cursor-pointer"
            onclick={(e) => {
              e.stopPropagation();
              handleUndoFile(item);
            }}
          >
            <Undo2 class="w-3.5 h-3.5" />
          </button>
        </Tooltip>
      {/if}
    </div>
  </div>
{/snippet}
