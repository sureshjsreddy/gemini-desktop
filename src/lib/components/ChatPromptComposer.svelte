<script lang="ts">
  import type {
    Workspace,
    WorkspaceFileEntry,
    AttachmentItem,
    ApprovalMode,
    MentionOption,
  } from "$lib/types";
  import FilePickerModal from "$lib/components/FilePickerModal.svelte";
  import ChatMentionPopover from "$lib/components/ChatMentionPopover.svelte";
  import Tooltip from "$lib/components/Tooltip.svelte";
  import { dialogManager } from "$lib/dialog.svelte";
  import { getModeConfig } from "$lib/utils/approvalModes";
  import {
    Send,
    Square,
    Paperclip,
    ChevronDown,
    File,
    Folder,
    GitBranch,
    Plus,
    X,
    FileCode,
  } from "lucide-svelte";
  import { tick } from "svelte";

  let {
    workspace = null,
    workspaceFiles = [],
    isStreaming = false,
    approvalMode = "default",
    onSendPrompt,
    onCancelPrompt,
  }: {
    workspace: Workspace | null;
    workspaceFiles?: WorkspaceFileEntry[];
    isStreaming: boolean;
    approvalMode?: ApprovalMode;
    onSendPrompt: (prompt: string) => void;
    onCancelPrompt: () => void;
  } = $props();

  let inputPrompt = $state("");
  let textareaElem: HTMLTextAreaElement | null = $state(null);

  // File & Git Attachments State
  let attachments: AttachmentItem[] = $state([]);
  let showAttachMenu = $state(false);
  let showFilePickerModal = $state(false);
  let filePickerMode: "file" | "directory" = $state("file");

  // Inline @ Mention Autocomplete State
  let showMentionPopover = $state(false);
  let mentionQuery = $state("");
  let mentionMatchStart = $state(0);
  let mentionSelectedIndex = $state(0);
  let dismissedMatchStart: number | null = $state(null);

  const gitOptions: MentionOption[] = [
    {
      id: "git-diff",
      label: "@git:diff",
      insertText: "@git:diff ",
      title: "Git Diff (@git:diff)",
      subtitle: "Working directory unstaged changes",
      kind: "git",
    },
    {
      id: "git-staged",
      label: "@git:staged",
      insertText: "@git:staged ",
      title: "Git Staged (@git:staged)",
      subtitle: "Staged index changes",
      kind: "git",
    },
    {
      id: "git-status",
      label: "@git:status",
      insertText: "@git:status ",
      title: "Git Status (@git:status)",
      subtitle: "Repository status overview",
      kind: "git",
    },
  ];

  let filteredMentionOptions = $derived.by(() => {
    const q = mentionQuery.toLowerCase().trim();
    const results: MentionOption[] = [];

    // 1. Git suggestions
    for (const opt of gitOptions) {
      if (!q || opt.label.toLowerCase().includes(q) || opt.subtitle.toLowerCase().includes(q)) {
        results.push(opt);
      }
    }

    // 2. Workspace file/directory suggestions
    if (workspaceFiles && workspaceFiles.length > 0) {
      for (const f of workspaceFiles) {
        const relPath = f.relative_path.replace(/\\/g, "/");
        const label = f.is_dir ? `@${relPath}/` : `@${relPath}`;
        if (!q || label.toLowerCase().includes(q) || f.name.toLowerCase().includes(q)) {
          results.push({
            id: `file-${relPath}`,
            label,
            insertText: label + " ",
            title: f.name + (f.is_dir ? "/" : ""),
            subtitle: relPath + (f.is_dir ? "/" : ""),
            kind: f.is_dir ? "directory" : "file",
          });
          if (results.length >= 25) break;
        }
      }
    }

    return results;
  });

  let effectiveApprovalMode = $derived(
    workspace?.approval_mode || approvalMode || "default"
  );
  let currentModeConfig = $derived(getModeConfig(effectiveApprovalMode));
  let ModeIcon = $derived(currentModeConfig.icon);

  export function insertFileMention(relPath: string) {
    const mention = `@${relPath} `;
    if (inputPrompt && !inputPrompt.endsWith(" ")) {
      inputPrompt += " " + mention;
    } else {
      inputPrompt += mention;
    }
    tick().then(() => {
      if (textareaElem) {
        textareaElem.focus();
        const pos = inputPrompt.length;
        textareaElem.setSelectionRange(pos, pos);
      }
    });
  }

  export function attachItem(path: string, isDir: boolean, name?: string) {
    const itemName = name || path.split("/").pop() || path;
    const cleanPath = isDir && !path.endsWith("/") ? `${path}/` : path;
    addAttachment({
      id: `${isDir ? "dir" : "file"}-${Date.now()}-${Math.random().toString(36).substring(2, 6)}`,
      name: itemName,
      path: cleanPath,
      kind: isDir ? "directory" : "file",
    });
    tick().then(() => {
      if (textareaElem) {
        textareaElem.focus();
      }
    });
  }

  export function attachMultiple(items: { path: string; isDir: boolean; name?: string }[]) {
    for (const item of items) {
      attachItem(item.path, item.isDir, item.name);
    }
  }

  export function setPrompt(text: string) {
    inputPrompt = text;
    tick().then(() => {
      if (textareaElem) {
        textareaElem.focus();
        textareaElem.setSelectionRange(inputPrompt.length, inputPrompt.length);
      }
    });
  }

  export function focus() {
    textareaElem?.focus();
  }

  function checkMentionTrigger() {
    if (!textareaElem) return;
    const cursor = textareaElem.selectionStart;
    const textBeforeCursor = inputPrompt.slice(0, cursor);

    // Matches e.g. "@", "@src", "@git:", "@comp" right before cursor
    const match = textBeforeCursor.match(/(?:^|\s)@([^\s]*)$/);
    if (match) {
      const matchStart = cursor - match[1].length - 1;
      if (dismissedMatchStart === matchStart) {
        showMentionPopover = false;
        return;
      }
      mentionQuery = match[1];
      mentionMatchStart = matchStart;
      showMentionPopover = true;
      mentionSelectedIndex = 0;
    } else {
      showMentionPopover = false;
      dismissedMatchStart = null;
    }
  }

  function dismissMentionPopover() {
    showMentionPopover = false;
    dismissedMatchStart = mentionMatchStart;
  }

  function applyMention(option: MentionOption) {
    if (!textareaElem) return;
    const cursor = textareaElem.selectionStart;
    const textBefore = inputPrompt.slice(0, mentionMatchStart);
    const textAfter = inputPrompt.slice(cursor);

    inputPrompt = textBefore + option.insertText + textAfter;
    showMentionPopover = false;
    dismissedMatchStart = null;

    tick().then(() => {
      if (textareaElem) {
        const newPos = textBefore.length + option.insertText.length;
        textareaElem.focus();
        textareaElem.setSelectionRange(newPos, newPos);
      }
    });
  }

  function addAttachment(item: AttachmentItem) {
    if (!attachments.some((a) => a.path === item.path)) {
      attachments = [...attachments, item];
    }
    showAttachMenu = false;
  }

  function removeAttachment(id: string) {
    attachments = attachments.filter((a) => a.id !== id);
  }

  function handleAddGitPreset(kind: "diff" | "staged" | "status") {
    addAttachment({
      id: "git-" + kind + "-" + Date.now(),
      name: `@git:${kind}`,
      path: `git:${kind}`,
      kind: "git",
    });
  }

  async function handleCustomPathPrompt() {
    showAttachMenu = false;
    const path = await dialogManager.prompt(
      "Enter relative or absolute path to attach (e.g. src/main.rs or docs/):",
      "",
      {
        title: "Attach Custom Path",
        placeholder: "e.g. src/main.rs or docs/",
        confirmText: "Attach",
      }
    );
    if (path && path.trim()) {
      const clean = path.trim().replace(/^@/, "");
      addAttachment({
        id: "custom-" + Date.now(),
        name: clean,
        path: clean,
        kind: clean.endsWith("/") ? "directory" : "file",
      });
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (showMentionPopover && filteredMentionOptions.length > 0) {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        mentionSelectedIndex = (mentionSelectedIndex + 1) % filteredMentionOptions.length;
        return;
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        mentionSelectedIndex =
          (mentionSelectedIndex - 1 + filteredMentionOptions.length) % filteredMentionOptions.length;
        return;
      } else if (e.key === "Enter" || e.key === "Tab") {
        e.preventDefault();
        applyMention(filteredMentionOptions[mentionSelectedIndex]);
        return;
      } else if (e.key === "Escape") {
        e.preventDefault();
        dismissMentionPopover();
        return;
      }
    }

    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSubmit();
    } else if (e.key === "Escape" && isStreaming) {
      e.preventDefault();
      onCancelPrompt();
    }
  }

  function handleSubmit() {
    const trimmed = inputPrompt.trim();
    if ((!trimmed && attachments.length === 0) || isStreaming) return;

    let finalPrompt = trimmed;

    // Append attached context chips if not already present in the prompt
    if (attachments.length > 0) {
      const unmentioned = attachments.filter((att) => {
        const ref = `@${att.path}`;
        return !finalPrompt.includes(ref) && !finalPrompt.includes(att.path);
      });

      if (unmentioned.length > 0) {
        const refsText = unmentioned.map((att) => `@${att.path}`).join(" ");
        finalPrompt = finalPrompt ? `${finalPrompt}\n\n${refsText}` : refsText;
      }
    }

    inputPrompt = "";
    attachments = [];
    showMentionPopover = false;
    onSendPrompt(finalPrompt);
  }
</script>

<!-- Prompt Input Bar -->
<div class="p-4 bg-surface border-t border-subtle relative z-20">
  <div class="max-w-4xl mx-auto relative rounded-xl border border-theme-default bg-app/80 focus-within:border-accent-theme transition-colors shadow-inner">
    
    <!-- Inline @ Mention Autocomplete Floating Popover -->
    <ChatMentionPopover
      show={showMentionPopover}
      options={filteredMentionOptions}
      selectedIndex={mentionSelectedIndex}
      onSelect={applyMention}
      onDismiss={dismissMentionPopover}
      onSelectIndex={(idx) => (mentionSelectedIndex = idx)}
    />

    <!-- Visual Attachment Chips Row -->
    {#if attachments.length > 0}
      <div class="px-3.5 pt-2.5 pb-1.5 flex flex-wrap items-center gap-1.5 border-b border-subtle/60 bg-surface/50 rounded-t-xl">
        <span class="text-[10px] uppercase font-bold tracking-wider text-muted-theme mr-1 flex items-center gap-1">
          <Paperclip size={10} class="text-accent-theme" />
          <span>Attached Context:</span>
        </span>
        {#each attachments as att (att.id)}
          <div class="inline-flex items-center gap-1.5 px-2 py-0.5 rounded-md bg-surface-elevated border border-subtle text-xs text-primary-theme shadow-xs">
            {#if att.kind === "git"}
              <GitBranch size={12} class="text-amber-400" />
            {:else if att.kind === "directory"}
              <Folder size={12} class="text-sky-400" />
            {:else}
              <FileCode size={12} class="text-accent-theme" />
            {/if}
            <span class="font-mono text-[11px] truncate max-w-[220px]" title={att.path}>{att.name}</span>
            <button
              type="button"
              onclick={() => removeAttachment(att.id)}
              class="text-muted-theme hover:text-rose-400 p-0.5 rounded transition-colors cursor-pointer"
              title="Remove attachment"
            >
              <X size={11} />
            </button>
          </div>
        {/each}
        <button
          type="button"
          onclick={() => (attachments = [])}
          class="text-[10px] text-muted-theme hover:text-rose-400 underline ml-1 cursor-pointer transition-colors"
        >
          Clear all
        </button>
      </div>
    {/if}

    <textarea
      bind:this={textareaElem}
      bind:value={inputPrompt}
      onkeydown={handleKeyDown}
      oninput={checkMentionTrigger}
      placeholder="Type prompt or use @file, @dir/, @git:diff... (Shift+Enter for newline, Enter to send)"
      rows="3"
      class="w-full px-3.5 py-2.5 bg-transparent text-primary-theme placeholder:text-muted-theme text-xs focus:outline-none resize-none font-sans select-text"
    ></textarea>

    <div class="flex items-center justify-between px-3 py-2 border-t border-subtle text-[11px] text-muted-theme">
      <div class="flex items-center gap-2.5 relative">
        <!-- Attachment Dropdown Action Button -->
        <div class="relative">
          <Tooltip
            text="Attach Context"
            subtext="Select workspace files, folders, or git diffs (@git:diff, @git:staged) to include in prompt"
            shortcut="@"
            position="top"
          >
            <button
              type="button"
              onclick={() => (showAttachMenu = !showAttachMenu)}
              class="flex items-center gap-1.5 px-2.5 py-1 rounded-lg bg-surface hover:bg-surface-hover text-secondary-theme hover:text-primary-theme transition-colors border border-theme-default cursor-pointer text-xs"
            >
              <Paperclip size={13} class="text-accent-theme" />
              <span class="font-medium">Attach</span>
              <ChevronDown size={11} class="opacity-60" />
            </button>
          </Tooltip>

          <!-- Attachment Quick Menu -->
          {#if showAttachMenu}
            <div
              class="fixed inset-0 z-30"
              onclick={() => (showAttachMenu = false)}
              role="presentation"
            ></div>
            <div class="absolute bottom-full left-0 mb-2 w-56 bg-surface-elevated border border-subtle rounded-xl shadow-2xl py-1.5 z-40 text-xs">
              <button
                type="button"
                onclick={() => {
                  showAttachMenu = false;
                  filePickerMode = "file";
                  showFilePickerModal = true;
                }}
                class="w-full px-3 py-1.5 text-left hover:bg-surface-hover text-secondary-theme hover:text-primary-theme flex items-center gap-2 cursor-pointer"
              >
                <File size={14} class="text-accent-theme" />
                <span>Attach Workspace File...</span>
              </button>

              <button
                type="button"
                onclick={() => {
                  showAttachMenu = false;
                  filePickerMode = "directory";
                  showFilePickerModal = true;
                }}
                class="w-full px-3 py-1.5 text-left hover:bg-surface-hover text-secondary-theme hover:text-primary-theme flex items-center gap-2 cursor-pointer"
              >
                <Folder size={14} class="text-sky-400" />
                <span>Attach Workspace Folder...</span>
              </button>

              <div class="my-1 border-t border-subtle"></div>
              <div class="px-3 py-1 text-[10px] font-semibold uppercase tracking-wider text-muted-theme">Git Context</div>

              <button
                type="button"
                onclick={() => handleAddGitPreset("diff")}
                class="w-full px-3 py-1.5 text-left hover:bg-surface-hover text-secondary-theme hover:text-primary-theme flex items-center gap-2 cursor-pointer"
              >
                <GitBranch size={14} class="text-amber-400" />
                <span class="font-mono text-[11px]">@git:diff</span>
                <span class="text-[10px] text-muted-theme ml-auto">unstaged</span>
              </button>

              <button
                type="button"
                onclick={() => handleAddGitPreset("staged")}
                class="w-full px-3 py-1.5 text-left hover:bg-surface-hover text-secondary-theme hover:text-primary-theme flex items-center gap-2 cursor-pointer"
              >
                <GitBranch size={14} class="text-emerald-400" />
                <span class="font-mono text-[11px]">@git:staged</span>
                <span class="text-[10px] text-muted-theme ml-auto">staged</span>
              </button>

              <button
                type="button"
                onclick={() => handleAddGitPreset("status")}
                class="w-full px-3 py-1.5 text-left hover:bg-surface-hover text-secondary-theme hover:text-primary-theme flex items-center gap-2 cursor-pointer"
              >
                <GitBranch size={14} class="text-sky-400" />
                <span class="font-mono text-[11px]">@git:status</span>
                <span class="text-[10px] text-muted-theme ml-auto">overview</span>
              </button>

              <div class="my-1 border-t border-subtle"></div>

              <button
                type="button"
                onclick={handleCustomPathPrompt}
                class="w-full px-3 py-1.5 text-left hover:bg-surface-hover text-secondary-theme hover:text-primary-theme flex items-center gap-2 cursor-pointer"
              >
                <Plus size={14} class="text-muted-theme" />
                <span>Custom Path / Ref...</span>
              </button>
            </div>
          {/if}
        </div>

        <Tooltip
          text="Workspace Working Directory"
          subtext="Gemini CLI commands, MCP tools, and file paths execute relative to this root folder."
          position="top"
        >
          <span class="cursor-help">Working Dir: <span class="text-secondary-theme font-mono">{workspace?.path || "C:\\"}</span></span>
        </Tooltip>

        <span class="text-muted-theme/40 select-none">&bull;</span>

        <Tooltip
          text={`Policy Approval Mode: ${currentModeConfig.label}`}
          subtext={`${currentModeConfig.description} (Change in Workspace Settings)`}
          position="top"
        >
          <div class="inline-flex items-center gap-1.5 px-2 py-0.5 rounded-md border text-[11px] font-semibold cursor-default select-none {currentModeConfig.badgeClass}">
            <ModeIcon size={11} class={currentModeConfig.iconClass} />
            <span>{currentModeConfig.shortLabel}</span>
          </div>
        </Tooltip>
      </div>

      <div class="flex items-center gap-2">
        {#if isStreaming}
          <Tooltip
            text="Cancel Generation"
            subtext="Interrupt active model output or tool execution"
            shortcut="Esc"
            position="top"
          >
            <button
              onclick={onCancelPrompt}
              class="flex items-center gap-1.5 px-3 py-1 rounded-lg bg-rose-500 hover:bg-rose-400 text-white font-medium text-xs transition-colors cursor-pointer"
            >
              <Square size={12} />
              <span>Stop</span>
              <span class="text-[10px] opacity-75 font-mono">Esc</span>
            </button>
          </Tooltip>
        {:else}
          <Tooltip
            text="Send Message"
            subtext="Submit prompt and attached files to Gemini"
            shortcut="Enter"
            position="top"
          >
            <button
              onclick={handleSubmit}
              disabled={!inputPrompt.trim() && attachments.length === 0}
              class="flex items-center gap-1.5 px-3 py-1 rounded-lg bg-accent-theme hover:bg-accent-hover disabled:opacity-40 disabled:hover:bg-accent-theme text-white font-semibold text-xs transition-colors shadow-xs cursor-pointer"
            >
              <Send size={12} />
              <span>Send</span>
              <span class="text-[10px] opacity-75 font-mono">Enter</span>
            </button>
          </Tooltip>
        {/if}
      </div>
    </div>
  </div>
</div>

<!-- Workspace File / Folder Picker Modal -->
<FilePickerModal
  isOpen={showFilePickerModal}
  mode={filePickerMode}
  workspaceFiles={workspaceFiles || []}
  onClose={() => (showFilePickerModal = false)}
  onSelect={(entry: WorkspaceFileEntry) => {
    addAttachment({
      id: "att-" + Date.now() + "-" + Math.random().toString(36).substring(2, 6),
      name: entry.name + (entry.is_dir ? "/" : ""),
      path: entry.relative_path.replace(/\\/g, "/") + (entry.is_dir ? "/" : ""),
      kind: entry.is_dir ? "directory" : "file",
    });
  }}
  onSelectCustomPath={(customPath: string) => {
    const clean = customPath.replace(/^@/, "").trim();
    addAttachment({
      id: "att-" + Date.now() + "-" + Math.random().toString(36).substring(2, 6),
      name: clean,
      path: clean,
      kind: clean.endsWith("/") ? "directory" : "file",
    });
  }}
/>
