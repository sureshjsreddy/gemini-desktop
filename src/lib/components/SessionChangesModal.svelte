<script lang="ts">
  import type { SessionFileChange } from "$lib/types";
  import {
    X,
    FileDiff,
    RotateCcw,
    FileCode,
    FilePlus,
    FileMinus,
    FileEdit,
    Check,
    AlertTriangle,
    ExternalLink,
    FileText,
    Eye,
    Copy,
  } from "lucide-svelte";
  import Tooltip from "$lib/components/Tooltip.svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { revealItemInDir } from "@tauri-apps/plugin-opener";

  let {
    isOpen = false,
    changes = [],
    workspacePath = "",
    onClose,
    onRevertFile,
    onRevertAll,
  }: {
    isOpen: boolean;
    changes: SessionFileChange[];
    workspacePath?: string;
    onClose: () => void;
    onRevertFile: (filePath: string) => Promise<void> | void;
    onRevertAll: () => Promise<void> | void;
  } = $props();

  let selectedFilePath = $state<string>("");
  let isReverting = $state<string | null>(null);
  let isRevertingAll = $state(false);
  let confirmRevertAll = $state(false);
  let contextMenu = $state<{ x: number; y: number; filePath: string } | null>(null);

  // Automatically select the first file when modal opens or selection is empty
  $effect(() => {
    if (changes.length > 0) {
      if (!selectedFilePath || !changes.some((c) => c.file_path === selectedFilePath)) {
        selectedFilePath = changes[0].file_path;
      }
    } else {
      selectedFilePath = "";
    }
  });

  let selectedChange = $derived(
    changes.find((c) => c.file_path === selectedFilePath) || changes[0] || null
  );

  let totalAdditions = $derived(changes.reduce((acc, c) => acc + c.additions, 0));
  let totalDeletions = $derived(changes.reduce((acc, c) => acc + c.deletions, 0));

  function resolveFullPath(relPath: string): string {
    if (!workspacePath) return relPath;
    const base = workspacePath.replace(/[/\\]+$/, "");
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
    } catch (err) {
      console.warn("Could not reveal file in Windows explorer:", err);
    }
    contextMenu = null;
  }

  async function handleCopyPath(relPath: string, full = false) {
    const text = full ? resolveFullPath(relPath) : relPath;
    try {
      await navigator.clipboard.writeText(text);
    } catch (err) {
      console.error("Failed to copy path:", err);
    }
    contextMenu = null;
  }

  function handleContextMenu(relPath: string, e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    selectedFilePath = relPath;
    const clickX = e.clientX;
    const clickY = e.clientY;
    const x = Math.min(clickX, window.innerWidth - 220);
    const y = Math.min(clickY, window.innerHeight - 260);
    contextMenu = { x, y, filePath: relPath };
  }

  // Parse unified diff into structured lines with line numbers
  interface DiffLine {
    type: "add" | "delete" | "context" | "header";
    content: string;
    oldLine?: number;
    newLine?: number;
  }

  let parsedLines = $derived.by<DiffLine[]>(() => {
    if (!selectedChange || !selectedChange.diff_text) return [];

    const rawLines = selectedChange.diff_text.split("\n");
    const result: DiffLine[] = [];

    let oldLineNum = 0;
    let newLineNum = 0;

    for (const line of rawLines) {
      if (line.startsWith("---") || line.startsWith("+++")) {
        continue;
      }

      if (line.startsWith("@@")) {
        const match = line.match(/@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@/);
        if (match) {
          oldLineNum = parseInt(match[1], 10);
          newLineNum = parseInt(match[2], 10);
        }
        result.push({
          type: "header",
          content: line,
        });
        continue;
      }

      if (line.startsWith("+")) {
        result.push({
          type: "add",
          content: line.slice(1),
          newLine: newLineNum++,
        });
      } else if (line.startsWith("-")) {
        result.push({
          type: "delete",
          content: line.slice(1),
          oldLine: oldLineNum++,
        });
      } else if (line.startsWith(" ")) {
        result.push({
          type: "context",
          content: line.slice(1),
          oldLine: oldLineNum++,
          newLine: newLineNum++,
        });
      } else if (line.length > 0) {
        result.push({
          type: "context",
          content: line,
        });
      }
    }

    return result;
  });

  async function handleRevertSingle(filePath: string) {
    if (isReverting) return;
    isReverting = filePath;
    contextMenu = null;
    try {
      await onRevertFile(filePath);
    } finally {
      isReverting = null;
    }
  }

  async function handleRevertEverything() {
    if (isRevertingAll) return;
    isRevertingAll = true;
    confirmRevertAll = false;
    contextMenu = null;
    try {
      await onRevertAll();
      onClose();
    } finally {
      isRevertingAll = false;
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      if (contextMenu) {
        contextMenu = null;
      } else if (confirmRevertAll) {
        confirmRevertAll = false;
      } else {
        onClose();
      }
    }
  }
</script>

<svelte:window onkeydown={handleKeyDown} onclick={() => (contextMenu = null)} />

{#if isOpen}
  <!-- Backdrop -->
  <div
    class="fixed inset-0 z-50 bg-black/60 backdrop-blur-xs flex items-center justify-center p-4 sm:p-6"
    role="presentation"
  >
    <!-- Modal Container -->
    <div
      class="w-full max-w-5xl h-[85vh] bg-surface-elevated border border-theme-default rounded-2xl shadow-2xl flex flex-col overflow-hidden relative"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
      role="dialog"
      tabindex="-1"
      aria-modal="true"
      aria-labelledby="session-diff-title"
    >
      <!-- Modal Header -->
      <div class="h-14 px-6 border-b border-subtle flex items-center justify-between gap-4 bg-surface/50 shrink-0">
        <div class="flex items-center gap-3 min-w-0">
          <div class="p-2 rounded-xl bg-amber-500/15 border border-amber-500/30 text-amber-400 shrink-0">
            <FileDiff size={18} />
          </div>
          <div class="min-w-0">
            <h2 id="session-diff-title" class="font-semibold text-sm text-primary-theme flex items-center gap-2">
              <span>Session File Changes & Visual Diff</span>
              <span class="text-xs px-2 py-0.5 rounded-full bg-surface-hover text-secondary-theme font-mono font-normal">
                {changes.length} {changes.length === 1 ? "file" : "files"}
              </span>
            </h2>
            <p class="text-[11px] text-muted-theme truncate">
              Review and inspect file modifications made during this conversation &bull; Right-click any file for native Windows open options
            </p>
          </div>
        </div>

        <div class="flex items-center gap-2 shrink-0">
          {#if changes.length > 0}
            <div class="hidden sm:flex items-center gap-2 font-mono text-xs px-2.5 py-1 rounded-lg bg-surface border border-subtle">
              <span class="text-emerald-400">+{totalAdditions}</span>
              <span class="text-rose-400">-{totalDeletions}</span>
            </div>

            <!-- Revert All Button -->
            {#if !confirmRevertAll}
              <button
                type="button"
                onclick={() => (confirmRevertAll = true)}
                class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg border border-rose-500/30 bg-rose-500/10 hover:bg-rose-500/20 text-rose-300 text-xs font-medium transition-colors cursor-pointer"
              >
                <RotateCcw size={13} />
                <span>Revert All</span>
              </button>
            {:else}
              <div class="flex items-center gap-1.5 bg-rose-500/20 border border-rose-500/50 p-1 rounded-lg">
                <span class="text-[11px] text-rose-200 px-1 font-medium">Revert all {changes.length} files?</span>
                <button
                  type="button"
                  onclick={handleRevertEverything}
                  disabled={isRevertingAll}
                  class="px-2 py-1 rounded bg-rose-600 hover:bg-rose-700 text-white text-xs font-semibold cursor-pointer"
                >
                  {isRevertingAll ? "Reverting..." : "Confirm"}
                </button>
                <button
                  type="button"
                  onclick={() => (confirmRevertAll = false)}
                  class="px-2 py-1 rounded bg-surface hover:bg-surface-hover text-secondary-theme text-xs cursor-pointer"
                >
                  Cancel
                </button>
              </div>
            {/if}
          {/if}

          <!-- Close Modal -->
          <button
            type="button"
            onclick={onClose}
            class="p-1.5 rounded-lg text-secondary-theme hover:text-primary-theme hover:bg-surface-hover transition-colors cursor-pointer"
            aria-label="Close modal"
          >
            <X size={17} />
          </button>
        </div>
      </div>

      <!-- Modal Body (Split: Files list on left, Diff viewer on right) -->
      <div class="flex-1 flex min-h-0 overflow-hidden">
        {#if changes.length === 0}
          <div class="flex-1 flex flex-col items-center justify-center text-center p-8 text-muted-theme">
            <Check size={32} class="text-emerald-400 mb-2 opacity-80" />
            <h3 class="text-sm font-semibold text-primary-theme">No File Changes in this Session</h3>
            <p class="text-xs text-secondary-theme mt-1 max-w-sm">
              Gemini has not modified or created any files in this workspace during the active conversation.
            </p>
          </div>
        {:else}
          <!-- Left Column: Changed Files List -->
          <div class="w-72 sm:w-80 border-r border-subtle flex flex-col bg-surface/30 shrink-0">
            <div class="p-3 border-b border-subtle/60 text-[11px] font-semibold uppercase tracking-wider text-muted-theme flex items-center justify-between">
              <span>Changed Files ({changes.length})</span>
              <span class="text-[10px] text-muted-theme/80 font-normal lowercase">right-click for menu</span>
            </div>
            <div class="flex-1 overflow-y-auto p-2 space-y-1">
              {#each changes as item (item.file_path)}
                {@const isSelected = selectedFilePath === item.file_path}
                <div
                  role="button"
                  tabindex="0"
                  onclick={() => (selectedFilePath = item.file_path)}
                  onkeydown={(e) => e.key === "Enter" && (selectedFilePath = item.file_path)}
                  oncontextmenu={(e) => handleContextMenu(item.file_path, e)}
                  class="w-full text-left p-2.5 rounded-xl border transition-all cursor-pointer flex items-center justify-between gap-2 group {isSelected
                    ? 'bg-accent-subtle/60 border-accent-theme/40 text-primary-theme shadow-xs'
                    : 'bg-surface/40 hover:bg-surface-hover/60 border-transparent text-secondary-theme'}"
                >
                  <div class="flex items-center gap-2 min-w-0 flex-1">
                    {#if item.status === "created"}
                      <FilePlus size={15} class="text-emerald-400 shrink-0" />
                    {:else if item.status === "deleted"}
                      <FileMinus size={15} class="text-rose-400 shrink-0" />
                    {:else}
                      <FileEdit size={15} class="text-sky-400 shrink-0" />
                    {/if}
                    <div class="min-w-0 flex-1">
                      <div class="font-mono text-xs truncate" title={item.file_path}>
                        {item.file_path.split("/").pop()}
                      </div>
                      <div class="text-[10px] text-muted-theme truncate font-mono" title={item.file_path}>
                        {item.file_path}
                      </div>
                    </div>
                  </div>

                  <div class="flex items-center gap-1.5 shrink-0">
                    <span class="text-[10px] font-mono">
                      {#if item.additions > 0}
                        <span class="text-emerald-400">+{item.additions}</span>
                      {/if}
                      {#if item.deletions > 0}
                        <span class="text-rose-400">-{item.deletions}</span>
                      {/if}
                    </span>

                    <Tooltip text="Revert this file" position="left">
                      <button
                        type="button"
                        onclick={(e) => {
                          e.stopPropagation();
                          handleRevertSingle(item.file_path);
                        }}
                        disabled={isReverting === item.file_path}
                        class="p-1 rounded-md hover:bg-surface-elevated text-muted-theme hover:text-rose-400 transition-colors opacity-0 group-hover:opacity-100 cursor-pointer"
                        aria-label="Revert file"
                      >
                        <RotateCcw size={12} class={isReverting === item.file_path ? "animate-spin" : ""} />
                      </button>
                    </Tooltip>
                  </div>
                </div>
              {/each}
            </div>
          </div>

          <!-- Right Column: Visual Diff Viewer -->
          <div class="flex-1 flex flex-col bg-app min-w-0 overflow-hidden">
            {#if selectedChange}
              <!-- Diff Header Bar with Quick Action Buttons -->
              <div class="px-4 py-2.5 border-b border-subtle bg-surface/50 flex items-center justify-between gap-3">
                <div class="flex items-center gap-2 min-w-0">
                  <FileCode size={15} class="text-accent-theme shrink-0" />
                  <span class="font-mono text-xs font-semibold text-primary-theme truncate" title={selectedChange.file_path}>
                    {selectedChange.file_path}
                  </span>
                  {#if selectedChange.status === "created"}
                    <span class="px-2 py-0.5 rounded text-[10px] font-semibold uppercase tracking-wider bg-emerald-500/15 text-emerald-300 border border-emerald-500/30">
                      Created
                    </span>
                  {:else if selectedChange.status === "deleted"}
                    <span class="px-2 py-0.5 rounded text-[10px] font-semibold uppercase tracking-wider bg-rose-500/15 text-rose-300 border border-rose-500/30">
                      Deleted
                    </span>
                  {:else}
                    <span class="px-2 py-0.5 rounded text-[10px] font-semibold uppercase tracking-wider bg-sky-500/15 text-sky-300 border border-sky-500/30">
                      Modified
                    </span>
                  {/if}
                </div>

                <div class="flex items-center gap-1.5 shrink-0">
                  <!-- Quick File Actions (Workspace Explorer style) -->
                  {#if selectedChange.status !== "deleted"}
                    <Tooltip text="Open with Default App" position="bottom">
                      <button
                        type="button"
                        onclick={() => handleOpenFile(selectedChange.file_path)}
                        class="p-1.5 rounded-lg border border-subtle bg-surface hover:bg-surface-hover text-secondary-theme hover:text-primary-theme transition-colors cursor-pointer"
                        aria-label="Open with Default App"
                      >
                        <ExternalLink size={13} />
                      </button>
                    </Tooltip>

                    <Tooltip text="Open with Notepad" position="bottom">
                      <button
                        type="button"
                        onclick={() => handleOpenFile(selectedChange.file_path, { forceNotepad: true })}
                        class="p-1.5 rounded-lg border border-subtle bg-surface hover:bg-surface-hover text-secondary-theme hover:text-primary-theme transition-colors cursor-pointer"
                        aria-label="Open with Notepad"
                      >
                        <FileText size={13} />
                      </button>
                    </Tooltip>

                    <Tooltip text="Reveal in Windows Explorer" position="bottom">
                      <button
                        type="button"
                        onclick={() => handleRevealInExplorer(selectedChange.file_path)}
                        class="p-1.5 rounded-lg border border-subtle bg-surface hover:bg-surface-hover text-secondary-theme hover:text-primary-theme transition-colors cursor-pointer"
                        aria-label="Reveal in Explorer"
                      >
                        <Eye size={13} />
                      </button>
                    </Tooltip>
                  {/if}

                  <Tooltip text="Copy Relative Path" position="bottom">
                    <button
                      type="button"
                      onclick={() => handleCopyPath(selectedChange.file_path)}
                      class="p-1.5 rounded-lg border border-subtle bg-surface hover:bg-surface-hover text-secondary-theme hover:text-primary-theme transition-colors cursor-pointer"
                      aria-label="Copy path"
                    >
                      <Copy size={13} />
                    </button>
                  </Tooltip>

                  <div class="h-4 w-px bg-subtle mx-0.5"></div>

                  <span class="text-xs font-mono font-medium hidden md:inline">
                    {#if selectedChange.additions > 0}
                      <span class="text-emerald-400 mr-1.5">+{selectedChange.additions} lines</span>
                    {/if}
                    {#if selectedChange.deletions > 0}
                      <span class="text-rose-400">-{selectedChange.deletions} lines</span>
                    {/if}
                  </span>

                  <!-- Revert File -->
                  <button
                    type="button"
                    onclick={() => handleRevertSingle(selectedChange.file_path)}
                    disabled={isReverting === selectedChange.file_path}
                    class="flex items-center gap-1.5 px-2.5 py-1 rounded-lg border border-rose-500/30 bg-surface hover:bg-rose-500/10 text-rose-400 text-xs font-medium transition-colors cursor-pointer"
                  >
                    <RotateCcw size={12} class={isReverting === selectedChange.file_path ? "animate-spin" : ""} />
                    <span>Revert File</span>
                  </button>
                </div>
              </div>

              <!-- Diff Content Scroll Area -->
              <div class="flex-1 overflow-auto font-mono text-xs p-2 select-text">
                {#if parsedLines.length === 0}
                  <div class="p-6 text-center text-muted-theme">
                    {selectedChange.diff_text || "No line changes detected."}
                  </div>
                {:else}
                  <div class="min-w-full inline-block">
                    {#each parsedLines as line}
                      {#if line.type === "header"}
                        <div class="px-3 py-1 bg-sky-950/30 text-sky-400 border-y border-sky-900/30 font-semibold my-1 select-none">
                          {line.content}
                        </div>
                      {:else}
                        <div
                          class="flex items-start px-2 py-0.5 hover:bg-white/5 transition-colors {line.type === 'add'
                            ? 'bg-emerald-950/20 text-emerald-200'
                            : line.type === 'delete'
                            ? 'bg-rose-950/20 text-rose-200'
                            : 'text-secondary-theme'}"
                        >
                          <!-- Old Line Number -->
                          <span class="w-10 text-right pr-2 select-none text-[11px] text-muted-theme shrink-0 opacity-60">
                            {line.oldLine ?? ""}
                          </span>
                          <!-- New Line Number -->
                          <span class="w-10 text-right pr-3 select-none text-[11px] text-muted-theme shrink-0 opacity-60">
                            {line.newLine ?? ""}
                          </span>
                          <!-- Change Indicator (+ / -) -->
                          <span class="w-4 select-none shrink-0 font-bold {line.type === 'add' ? 'text-emerald-400' : line.type === 'delete' ? 'text-rose-400' : 'text-transparent'}">
                            {line.type === "add" ? "+" : line.type === "delete" ? "-" : " "}
                          </span>
                          <!-- Code Content -->
                          <span class="flex-1 whitespace-pre-wrap break-all pl-1">{line.content}</span>
                        </div>
                      {/if}
                    {/each}
                  </div>
                {/if}
              </div>
            {/if}
          </div>
        {/if}
      </div>
    </div>
  </div>
{/if}

<!-- Right-Click Context Menu (Matching Workspace Explorer) -->
{#if contextMenu}
  <!-- Backdrop -->
  <div
    class="fixed inset-0 z-50 bg-transparent"
    onclick={() => (contextMenu = null)}
    oncontextmenu={(e) => {
      e.preventDefault();
      contextMenu = null;
    }}
    role="presentation"
  ></div>

  <!-- Context Menu Card -->
  <div
    class="fixed z-50 w-52 bg-surface-elevated border border-subtle rounded-xl shadow-2xl py-1 text-xs text-primary-theme"
    style="left: {contextMenu.x}px; top: {contextMenu.y}px;"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
    role="menu"
    tabindex="-1"
  >
    <div class="px-3 py-1.5 text-[10px] text-muted-theme border-b border-subtle/60 font-mono truncate">
      {contextMenu.filePath.split("/").pop()}
    </div>

    <!-- Open with Default App -->
    <button
      type="button"
      onclick={() => contextMenu && handleOpenFile(contextMenu.filePath)}
      class="w-full px-3 py-1.5 text-left hover:bg-surface-hover flex items-center gap-2 cursor-pointer text-primary-theme"
    >
      <ExternalLink size={13} class="text-accent-theme" />
      <span>Open with Default App</span>
    </button>

    <!-- Open with Notepad -->
    <button
      type="button"
      onclick={() => contextMenu && handleOpenFile(contextMenu.filePath, { forceNotepad: true })}
      class="w-full px-3 py-1.5 text-left hover:bg-surface-hover flex items-center gap-2 cursor-pointer text-secondary-theme hover:text-primary-theme"
    >
      <FileText size={13} class="text-muted-theme" />
      <span>Open with Notepad</span>
    </button>

    <!-- Reveal in Explorer -->
    <button
      type="button"
      onclick={() => contextMenu && handleRevealInExplorer(contextMenu.filePath)}
      class="w-full px-3 py-1.5 text-left hover:bg-surface-hover flex items-center gap-2 cursor-pointer text-secondary-theme hover:text-primary-theme"
    >
      <Eye size={13} class="text-secondary-theme" />
      <span>Reveal in Explorer</span>
    </button>

    <div class="my-1 border-t border-subtle/60"></div>

    <!-- Copy Relative Path -->
    <button
      type="button"
      onclick={() => contextMenu && handleCopyPath(contextMenu.filePath, false)}
      class="w-full px-3 py-1.5 text-left hover:bg-surface-hover flex items-center gap-2 cursor-pointer text-secondary-theme hover:text-primary-theme"
    >
      <Copy size={13} class="text-muted-theme" />
      <span>Copy Relative Path</span>
    </button>

    <!-- Copy Full Path -->
    <button
      type="button"
      onclick={() => contextMenu && handleCopyPath(contextMenu.filePath, true)}
      class="w-full px-3 py-1.5 text-left hover:bg-surface-hover flex items-center gap-2 cursor-pointer text-secondary-theme hover:text-primary-theme"
    >
      <Copy size={13} class="text-muted-theme" />
      <span>Copy Full Path</span>
    </button>

    <div class="my-1 border-t border-subtle/60"></div>

    <!-- Revert File -->
    <button
      type="button"
      onclick={() => contextMenu && handleRevertSingle(contextMenu.filePath)}
      class="w-full px-3 py-1.5 text-left hover:bg-rose-500/10 flex items-center gap-2 cursor-pointer text-rose-400"
    >
      <RotateCcw size={13} />
      <span>Revert this File</span>
    </button>
  </div>
{/if}
