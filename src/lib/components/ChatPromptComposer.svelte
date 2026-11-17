<script lang="ts">
  import type {
    Workspace,
    WorkspaceFileEntry,
    AttachmentItem,
    ApprovalMode,
    MentionOption,
    SlashCommandOption,
    CliEngine,
  } from "$lib/types";
  import FilePickerModal from "$lib/components/FilePickerModal.svelte";
  import ChatMentionPopover from "$lib/components/ChatMentionPopover.svelte";
  import ChatSlashPopover from "$lib/components/ChatSlashPopover.svelte";
  import { modalManager } from "$lib/modal.svelte";
  import Tooltip from "$lib/components/Tooltip.svelte";
  import { dialogManager } from "$lib/dialog.svelte";
  import { getModeConfig } from "$lib/utils/approvalModes";
  import {
    Compass,
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
    Bug,
  } from "lucide-svelte";
  import { parseDebugError } from "$lib/utils/debugParser";
  import type { ParsedDebugError } from "$lib/types";
  import { tick, onMount, onDestroy } from "svelte";

  let {
    workspace = null,
    workspaceFiles = [],
    isStreaming = false,
    approvalMode = "default",
    activeEngine = "gemini",
    onSendPrompt,
    onCancelPrompt,
    onSteerPrompt,
    onTriggerDebugDoctor,
    onDebugErrorDetected,
    onOpenDiffModal,
    onToggleTerminal,
    onTogglePlanWorkbench,
    onOpenMcpModal,
    onOpenRewindModal,
  }: {
    workspace: Workspace | null;
    workspaceFiles?: WorkspaceFileEntry[];
    isStreaming: boolean;
    approvalMode?: ApprovalMode;
    activeEngine?: CliEngine;
    onSendPrompt: (prompt: string) => void;
    onCancelPrompt: () => void;
    onSteerPrompt?: (hint: string) => void;
    onTriggerDebugDoctor?: () => void;
    onDebugErrorDetected?: (error: ParsedDebugError) => void;
    onOpenDiffModal?: () => void;
    onToggleTerminal?: () => void;
    onTogglePlanWorkbench?: () => void;
    onOpenMcpModal?: () => void;
    onOpenRewindModal?: () => void;
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

  // Inline / Slash Command Autocomplete State
  let showSlashPopover = $state(false);
  let slashQuery = $state("");
  let slashMatchStart = $state(0);
  let slashSelectedIndex = $state(0);
  let dismissedSlashMatchStart: number | null = $state(null);

  let availableSlashCommands = $derived.by(() => {
    const isAgy = activeEngine === "agy";
    const commands: SlashCommandOption[] = [
      {
        id: "plan",
        label: "/plan",
        insertText: "/plan ",
        title: "Plan Mode Workbench",
        description: "Enter structured planning mode and task checklist extraction",
        category: "mode",
        engine: "all",
      },
      {
        id: "compact",
        label: "/compact",
        insertText: "/compact",
        title: "Context Compaction",
        description: "Distill conversation history into a dense summary to save context",
        category: "action",
        engine: "all",
      },
    ];

    if (!isAgy) {
      commands.push({
        id: "compress",
        label: "/compress",
        insertText: "/compress",
        title: "Compress Context",
        description: "Gemini CLI native conversation context compression",
        category: "action",
        engine: "gemini",
      });
    }

    commands.push({
      id: "diff",
      label: "/diff",
      insertText: "/diff",
      title: "File Changes & Git Diff",
      description: "Inspect session modified files, unstaged diffs, and reverts",
      category: "tool",
      engine: "all",
    });

    commands.push({
      id: "rewind",
      label: "/rewind",
      insertText: "/rewind",
      title: "Rewind Turn",
      description: "Rollback to previous conversation turn or checkpoint",
      category: "action",
      engine: "all",
    });

    commands.push({
      id: "terminal",
      label: "/terminal",
      insertText: "/terminal",
      title: "Workspace Terminal",
      description: isAgy
        ? "Toggle integrated terminal with Antigravity agy environment"
        : "Toggle integrated PowerShell workspace terminal drawer",
      category: "tool",
      engine: "all",
    });

    commands.push({
      id: "mcp",
      label: "/mcp",
      insertText: "/mcp",
      title: isAgy ? "Antigravity MCP Config" : "MCP Server Config",
      description: isAgy
        ? "Configure Antigravity MCP servers (.agents/mcp_config.json)"
        : "Configure Gemini CLI MCP servers (~/.gemini/settings.json)",
      category: "config",
      engine: "all",
    });

    if (isAgy) {
      commands.push({
        id: "plugins",
        label: "/plugins",
        insertText: "/plugins",
        title: "Plugins & Skills",
        description: "Inspect active Antigravity plugins, extensions, and custom skills",
        category: "tool",
        engine: "agy",
      });
    }

    commands.push({
      id: "tools",
      label: "/tools",
      insertText: "/tools",
      title: "List Active Tools",
      description: isAgy
        ? "View Antigravity agent tools, plugins, and custom skills"
        : "View official Gemini CLI file, terminal, and MCP tool capabilities",
      category: "tool",
      engine: "all",
    });

    commands.push({
      id: "model",
      label: "/model",
      insertText: "/model ",
      title: "Switch Model",
      description: isAgy
        ? "Switch Antigravity models (gemini-4-argon, claude-sonnet-4-6, 3.8-flash)"
        : "Switch Gemini CLI models (gemini-3.8-flash, 3.5-flash, auto)",
      category: "config",
      engine: "all",
    });

    commands.push({
      id: "mode",
      label: "/mode",
      insertText: "/mode ",
      title: "Approval Mode",
      description: isAgy
        ? "Switch approval mode (yolo, accept-edits, plan, ask)"
        : "Switch approval mode (yolo, auto_edit, plan, ask)",
      category: "mode",
      engine: "all",
    });

    commands.push({
      id: "doctor",
      label: "/doctor",
      insertText: "/doctor",
      title: "Debug Doctor",
      description: "Analyze clipboard errors or run instant diagnostics",
      category: "tool",
      engine: "all",
    });

    commands.push({
      id: "settings",
      label: "/settings",
      insertText: "/settings",
      title: isAgy ? "Antigravity Settings" : "Global CLI Settings",
      description: isAgy
        ? "Configure settings in ~/.gemini/antigravity-cli/settings.json"
        : "Configure settings in ~/.gemini/settings.json",
      category: "config",
      engine: "all",
    });

    commands.push({
      id: "setup",
      label: "/setup",
      insertText: "/setup",
      title: "Engine & Auth Setup",
      description: isAgy
        ? "Configure Antigravity CLI binary, updates, and environment"
        : "Configure Gemini CLI, Google Cloud Vertex AI, and API key",
      category: "config",
      engine: "all",
    });

    commands.push({
      id: "clear",
      label: "/clear",
      insertText: "/clear",
      title: "Clear Composer",
      description: "Clear input prompt and reset attached files",
      category: "action",
      engine: "all",
    });

    return commands;
  });

  let filteredSlashOptions = $derived.by(() => {
    const q = slashQuery.toLowerCase().trim();
    if (!q) return availableSlashCommands;
    return availableSlashCommands.filter((cmd) => {
      return (
        cmd.label.toLowerCase().includes(q) ||
        cmd.title.toLowerCase().includes(q) ||
        cmd.description.toLowerCase().includes(q)
      );
    });
  });

  onMount(() => {
    function handlePrefill(e: Event) {
      const customEvent = e as CustomEvent<string>;
      if (customEvent.detail) {
        inputPrompt = (inputPrompt ? inputPrompt + "\n\n" : "") + customEvent.detail;
        tick().then(() => {
          if (textareaElem) {
            textareaElem.focus();
            textareaElem.setSelectionRange(inputPrompt.length, inputPrompt.length);
          }
        });
      }
    }
    window.addEventListener("gemini-prefill-prompt", handlePrefill);
    return () => {
      window.removeEventListener("gemini-prefill-prompt", handlePrefill);
    };
  });

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

  let folderBasename = $derived(
    workspace?.path
      ? workspace.path.split(/[\/\\]/).filter(Boolean).pop() || workspace.path
      : "C:\\"
  );

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

  export function getPrompt(): string {
    return inputPrompt;
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
      showSlashPopover = false;
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

  function checkSlashTrigger() {
    if (!textareaElem) return;
    const cursor = textareaElem.selectionStart;
    const textBeforeCursor = inputPrompt.slice(0, cursor);

    // Matches "/" or "/xyz" right before cursor at start of line or after whitespace
    const match = textBeforeCursor.match(/(?:^|\s)\/([^\s]*)$/);
    if (match) {
      const matchStart = cursor - match[1].length - 1;
      if (dismissedSlashMatchStart === matchStart) {
        showSlashPopover = false;
        return;
      }
      slashQuery = match[1];
      slashMatchStart = matchStart;
      showSlashPopover = true;
      showMentionPopover = false;
      slashSelectedIndex = 0;
    } else {
      showSlashPopover = false;
      dismissedSlashMatchStart = null;
    }
  }

  function dismissSlashPopover() {
    showSlashPopover = false;
    dismissedSlashMatchStart = slashMatchStart;
  }

  function executeSlashAction(commandId: string, fullPrompt?: string): boolean {
    switch (commandId) {
      case "plan":
        if (fullPrompt && fullPrompt.trim().length > "/plan".length) {
          const taskDescription = fullPrompt.slice(fullPrompt.indexOf(" ") + 1).trim();
          if (onTogglePlanWorkbench) onTogglePlanWorkbench();
          onSendPrompt(`Plan Mode: Please analyze and create a comprehensive step-by-step plan for the following task:\n\n${taskDescription}`);
          return true;
        }
        if (onTogglePlanWorkbench) {
          onTogglePlanWorkbench();
          return true;
        }
        return false;
      case "compact":
      case "compress":
        onSendPrompt("/compact");
        return true;
      case "diff":
        if (onOpenDiffModal) {
          onOpenDiffModal();
          return true;
        }
        return false;
      case "rewind":
        if (onOpenRewindModal) {
          onOpenRewindModal();
        } else {
          modalManager.open("rewind");
        }
        return true;
      case "terminal":
        if (onToggleTerminal) {
          onToggleTerminal();
          return true;
        }
        return false;
      case "mcp":
        if (onOpenMcpModal) {
          onOpenMcpModal();
        } else {
          modalManager.open("mcp");
        }
        return true;
      case "settings":
        modalManager.open("cli-settings");
        return true;
      case "setup":
        modalManager.open("cli-setup");
        return true;
      case "doctor":
        if (onTriggerDebugDoctor) {
          onTriggerDebugDoctor();
          return true;
        }
        return false;
      case "clear":
        inputPrompt = "";
        attachments = [];
        return true;
      default:
        return false;
    }
  }

  function applySlashCommand(option: SlashCommandOption, isEnter: boolean) {
    if (!textareaElem) return;
    const cursor = textareaElem.selectionStart;
    const textBefore = inputPrompt.slice(0, slashMatchStart);
    const textAfter = inputPrompt.slice(cursor);

    showSlashPopover = false;
    dismissedSlashMatchStart = null;

    const directActionIds = ["settings", "setup", "mcp", "rewind", "diff", "terminal", "doctor", "clear", "compact", "compress"];
    if (isEnter && directActionIds.includes(option.id)) {
      if (executeSlashAction(option.id)) {
        inputPrompt = (textBefore + textAfter).trim();
        return;
      }
    }

    const insert = option.insertText.endsWith(" ") ? option.insertText : option.insertText + " ";
    inputPrompt = textBefore + insert + textAfter;
    tick().then(() => {
      if (textareaElem) {
        const newPos = textBefore.length + insert.length;
        textareaElem.focus();
        textareaElem.setSelectionRange(newPos, newPos);
      }
    });
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

  function handleSteer() {
    const trimmed = inputPrompt.trim();
    if (!trimmed) return;
    if (onSteerPrompt) {
      onSteerPrompt(trimmed);
      inputPrompt = "";
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (showSlashPopover && filteredSlashOptions.length > 0) {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        slashSelectedIndex = (slashSelectedIndex + 1) % filteredSlashOptions.length;
        return;
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        slashSelectedIndex =
          (slashSelectedIndex - 1 + filteredSlashOptions.length) % filteredSlashOptions.length;
        return;
      } else if (e.key === "Enter" || e.key === "Tab") {
        e.preventDefault();
        applySlashCommand(filteredSlashOptions[slashSelectedIndex], e.key === "Enter");
        return;
      } else if (e.key === "Escape") {
        e.preventDefault();
        dismissSlashPopover();
        return;
      }
    }

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
      if (isStreaming) {
        handleSteer();
      } else {
        handleSubmit();
      }
    } else if (e.key === "Escape" && isStreaming) {
      e.preventDefault();
      onCancelPrompt();
    }
  }

  function handleSubmit() {
    const trimmed = inputPrompt.trim();
    if ((!trimmed && attachments.length === 0) || isStreaming) return;

    if (trimmed.startsWith("/")) {
      const parts = trimmed.split(/\s+/);
      const commandId = parts[0].replace(/^\//, "").toLowerCase();
      const directActions = ["settings", "setup", "mcp", "rewind", "diff", "terminal", "doctor", "clear"];
      if (directActions.includes(commandId) && trimmed === parts[0]) {
        inputPrompt = "";
        executeSlashAction(commandId);
        return;
      }
      if (commandId === "plan") {
        inputPrompt = "";
        executeSlashAction("plan", trimmed);
        return;
      }
    }

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
  function handlePaste(e: ClipboardEvent) {
    if (!e.clipboardData || !onDebugErrorDetected) return;
    const pasted = e.clipboardData.getData("text");
    if (pasted) {
      const err = parseDebugError(pasted);
      if (err) {
        onDebugErrorDetected(err);
      }
    }
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

    <!-- Inline / Slash Command Autocomplete Floating Popover -->
    <ChatSlashPopover
      show={showSlashPopover}
      options={filteredSlashOptions}
      selectedIndex={slashSelectedIndex}
      {activeEngine}
      onSelect={(opt) => applySlashCommand(opt, true)}
      onDismiss={dismissSlashPopover}
      onSelectIndex={(idx) => (slashSelectedIndex = idx)}
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
      oninput={() => { checkMentionTrigger(); checkSlashTrigger(); }}
      onpaste={handlePaste}
      placeholder={isStreaming
        ? "🎯 Steer active agent in real time (e.g. skip tests, focus on src/api)... Press Enter or click Steer"
        : "Type prompt, '/' for tools & commands, or @file, @git:diff... (Shift+Enter for newline, Enter to send)"}
      rows="3"
      class="w-full px-3.5 py-2.5 bg-transparent text-primary-theme placeholder:text-muted-theme text-xs focus:outline-none resize-none font-sans select-text"
    ></textarea>

    <div class="flex items-center justify-between gap-2 px-3 py-2 border-t border-subtle text-[11px] text-muted-theme min-w-0">
      <div class="flex items-center gap-2 min-w-0 flex-1">
        <!-- Attachment Dropdown Action Button -->
        <div class="relative shrink-0">
          <Tooltip
            text="Attach Context"
            subtext="Select workspace files, folders, or git diffs (@git:diff, @git:staged) to include in prompt"
            shortcut="@"
            position="top"
          >
            <button
              type="button"
              onclick={() => (showAttachMenu = !showAttachMenu)}
              class="flex items-center gap-1.5 px-2.5 py-1 rounded-lg bg-surface hover:bg-surface-hover text-secondary-theme hover:text-primary-theme transition-colors border border-theme-default cursor-pointer text-xs shrink-0"
            >
              <Paperclip size={13} class="text-accent-theme shrink-0" />
              <span class="font-medium hidden sm:inline">Attach</span>
              <ChevronDown size={11} class="opacity-60 shrink-0" />
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

        {#if onTriggerDebugDoctor}
          <Tooltip
            text="Instant Debug Doctor"
            subtext="Scan clipboard for Visual Studio / runtime exceptions & load local source"
            shortcut="Ctrl+Alt+D"
            position="top"
          >
            <button
              type="button"
              onclick={onTriggerDebugDoctor}
              class="flex items-center gap-1.5 px-2.5 py-1 rounded-lg bg-surface hover:bg-rose-500/10 text-rose-400 hover:text-rose-300 transition-colors border border-rose-500/30 cursor-pointer text-xs shrink-0"
            >
              <Bug size={13} class="text-rose-400 shrink-0" />
              <span class="font-medium hidden md:inline">Debug Doctor</span>
            </button>
          </Tooltip>
        {/if}

        <Tooltip
          text={`Working Dir: ${workspace?.path || "C:\\"}`}
          subtext="Gemini CLI commands, MCP tools, and file paths execute relative to this root folder."
          position="top"
          class="min-w-0 inline-flex items-center"
        >
          <div
            title={`Working Dir: ${workspace?.path || "C:\\"}`}
            class="inline-flex items-center gap-1.5 px-2 py-0.5 rounded-md bg-surface/80 border border-subtle/80 text-secondary-theme cursor-help min-w-0 max-w-[120px] sm:max-w-[200px]"
          >
            <Folder size={12} class="text-sky-400 shrink-0" />
            <span class="font-mono text-[11px] truncate">{folderBasename}</span>
          </div>
        </Tooltip>

        <span class="text-muted-theme/40 select-none shrink-0 hidden xs:inline">&bull;</span>

        <Tooltip
          text={`Policy Approval Mode: ${currentModeConfig.label}`}
          subtext={`${currentModeConfig.description} (Change in Workspace Settings)`}
          position="top"
        >
          <div class="inline-flex items-center gap-1.5 px-2 py-0.5 rounded-md border text-[11px] font-semibold cursor-default select-none shrink-0 {currentModeConfig.badgeClass}">
            <ModeIcon size={11} class={currentModeConfig.iconClass} />
            <span class="hidden sm:inline">{currentModeConfig.shortLabel}</span>
          </div>
        </Tooltip>
      </div>

      <div class="flex items-center gap-2 shrink-0 ml-auto">
        {#if isStreaming}
          {#if inputPrompt.trim()}
            <Tooltip
              text="Steer Active Agent"
              subtext="Inject guidance or instruction into the active turn"
              shortcut="Enter"
              position="top"
            >
              <button
                onclick={handleSteer}
                class="flex items-center gap-1.5 px-3 py-1 rounded-lg bg-amber-500 hover:bg-amber-400 text-white font-semibold text-xs transition-colors shadow-xs cursor-pointer"
              >
                <Compass size={12} />
                <span>Steer</span>
                <span class="text-[10px] opacity-75 font-mono">Enter</span>
              </button>
            </Tooltip>
          {/if}
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
