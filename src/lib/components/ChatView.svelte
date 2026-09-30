<script lang="ts">
  import type {
    Workspace,
    Session,
    Message,
    ToolPermissionPayload,
    WorkspaceFileEntry,
    AttachmentItem,
    ApprovalMode,
  } from "$lib/types";
  import FilePickerModal from "$lib/components/FilePickerModal.svelte";
  import TerminalDrawer from "$lib/components/TerminalDrawer.svelte";
  import Tooltip from "$lib/components/Tooltip.svelte";
  import { renderMarkdown } from "$lib/markdown";
  import { dialogManager } from "$lib/dialog.svelte";
  import {
    Send,
    Square,
    Download,
    Terminal,
    Sparkles,
    User,
    CheckCircle,
    XCircle,
    FileText,
    FileCode,
    Paperclip,
    Folder,
    File,
    GitBranch,
    X,
    ChevronDown,
    Plus,
    Server,
    Copy,
    Check,
    RotateCcw,
    Edit3,
    ArrowDown,
    FolderTree,
    PanelLeftClose,
    PanelLeftOpen,
    Shield,
    Zap,
    Rocket,
    BookOpen,
  } from "lucide-svelte";
  import { tick } from "svelte";

  let {
    workspace = null,
    session = null,
    workspaceFiles = [],
    messages = [],
    isStreaming = false,
    streamingText = "",
    toolPermission = null,
    showSidebar = true,
    onToggleSidebar,
    showTerminalDrawer = $bindable(false),
    showSolutionExplorer = $bindable(true),
    approvalMode = "auto_edit",
    onSendPrompt,
    onCancelPrompt,
    onToolResponse,
    onExport,
    onOpenMcpModal,
  }: {
    workspace: Workspace | null;
    session: Session | null;
    workspaceFiles?: WorkspaceFileEntry[];
    messages: Message[];
    isStreaming: boolean;
    streamingText: string;
    toolPermission: ToolPermissionPayload | null;
    showSidebar?: boolean;
    onToggleSidebar?: () => void;
    showTerminalDrawer?: boolean;
    showSolutionExplorer?: boolean;
    approvalMode?: ApprovalMode;
    onSendPrompt: (prompt: string) => void;
    onCancelPrompt: () => void;
    onToolResponse: (requestId: number, optionId?: string, allowed?: boolean) => void;
    onExport: (format: string) => void;
    onOpenMcpModal?: () => void;
  } = $props();

  let inputPrompt = $state("");
  let chatViewport: HTMLDivElement | null = $state(null);
  let textareaElem: HTMLTextAreaElement | null = $state(null);
  let showExportMenu = $state(false);

  const MODES: {
    id: ApprovalMode;
    label: string;
    shortLabel: string;
    description: string;
    icon: any;
    badgeClass: string;
    iconClass: string;
    tagClass: string;
  }[] = [
    {
      id: "auto_edit",
      label: "Auto-Edit",
      shortLabel: "Auto-Edit",
      description: "Auto-approves file changes; asks before running terminal commands",
      icon: Zap,
      badgeClass: "bg-amber-500/15 hover:bg-amber-500/25 text-amber-600 dark:text-amber-400 border-amber-500/30",
      iconClass: "text-amber-500",
      tagClass: "bg-amber-500/15 text-amber-600 dark:text-amber-400 border border-amber-500/30",
    },
    {
      id: "yolo",
      label: "YOLO (Autonomous)",
      shortLabel: "YOLO",
      description: "Auto-approves all file changes and terminal commands without prompting",
      icon: Rocket,
      badgeClass: "bg-emerald-500/15 hover:bg-emerald-500/25 text-emerald-600 dark:text-emerald-400 border-emerald-500/30",
      iconClass: "text-emerald-500",
      tagClass: "bg-emerald-500/15 text-emerald-600 dark:text-emerald-400 border border-emerald-500/30",
    },
    {
      id: "default",
      label: "Ask Permission",
      shortLabel: "Ask",
      description: "Prompts for confirmation before modifying files or executing commands",
      icon: Shield,
      badgeClass: "bg-sky-500/15 hover:bg-sky-500/25 text-sky-600 dark:text-sky-400 border-sky-500/30",
      iconClass: "text-sky-500",
      tagClass: "bg-sky-500/15 text-sky-600 dark:text-sky-400 border border-sky-500/30",
    },
    {
      id: "plan",
      label: "Plan Mode",
      shortLabel: "Plan",
      description: "Read-only research and design phase; file and terminal writes are disabled",
      icon: BookOpen,
      badgeClass: "bg-purple-500/15 hover:bg-purple-500/25 text-purple-600 dark:text-purple-400 border-purple-500/30",
      iconClass: "text-purple-500",
      tagClass: "bg-purple-500/15 text-purple-600 dark:text-purple-400 border border-purple-500/30",
    },
  ];

  let effectiveApprovalMode = $derived(
    workspace?.approval_mode || approvalMode || "auto_edit"
  );
  let currentModeConfig = $derived(
    MODES.find((m) => m.id === effectiveApprovalMode) || MODES[0]
  );
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
  let isUserScrolledUp = $state(false);
  let copiedMessageId = $state<string | null>(null);

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

  interface MentionOption {
    id: string;
    label: string;
    insertText: string;
    title: string;
    subtitle: string;
    kind: "file" | "directory" | "git";
  }

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


  function checkMentionTrigger() {
    if (!textareaElem) return;
    const cursor = textareaElem.selectionStart;
    const textBeforeCursor = inputPrompt.slice(0, cursor);

    // Matches e.g. "@", "@src", "@git:", "@comp" right before cursor
    const match = textBeforeCursor.match(/(?:^|\s)@([^\s]*)$/);
    if (match) {
      const matchStart = cursor - match[1].length - 1;
      // If user previously dismissed the popover for this exact @ mention position, do not reopen
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

  function handleScroll() {
    if (!chatViewport) return;
    const { scrollTop, scrollHeight, clientHeight } = chatViewport;
    const distanceFromBottom = scrollHeight - scrollTop - clientHeight;
    isUserScrolledUp = distanceFromBottom > 80;
  }

  function scrollToBottom(smooth: boolean = false) {
    if (!chatViewport) return;
    isUserScrolledUp = false;
    if (smooth) {
      chatViewport.scrollTo({ top: chatViewport.scrollHeight, behavior: "smooth" });
    } else {
      chatViewport.scrollTop = chatViewport.scrollHeight;
    }
  }

  $effect(() => {
    // React to new tokens, messages, or streaming state changes
    const _txt = streamingText;
    const _count = messages.length;
    const _stream = isStreaming;

    if (!isUserScrolledUp && chatViewport) {
      tick().then(() => {
        if (!isUserScrolledUp && chatViewport) {
          chatViewport.scrollTop = chatViewport.scrollHeight;
        }
      });
    }
  });

  function formatTime(isoStr?: string): string {
    if (!isoStr) return "";
    try {
      const d = new Date(isoStr);
      return d.toLocaleTimeString([], { hour: "numeric", minute: "2-digit" });
    } catch {
      return "";
    }
  }

  async function handleCopyMessage(id: string, content: string) {
    try {
      await navigator.clipboard.writeText(content);
      copiedMessageId = id;
      setTimeout(() => {
        if (copiedMessageId === id) {
          copiedMessageId = null;
        }
      }, 2000);
    } catch (e) {
      console.error("Failed to copy message:", e);
    }
  }

  function handleEditPrompt(promptText: string) {
    inputPrompt = promptText;
    tick().then(() => {
      if (textareaElem) {
        textareaElem.focus();
        textareaElem.setSelectionRange(inputPrompt.length, inputPrompt.length);
      }
    });
  }

  function handleRegenerate() {
    if (isStreaming) return;
    const lastUserMsg = [...messages].reverse().find((m) => m.role === "user");
    if (lastUserMsg) {
      isUserScrolledUp = false;
      tick().then(() => scrollToBottom(true));
      onSendPrompt(lastUserMsg.content);
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
    isUserScrolledUp = false;
    tick().then(() => scrollToBottom(true));
    onSendPrompt(finalPrompt);
  }
</script>

<div class="flex-1 h-screen flex flex-col bg-app text-primary-theme overflow-hidden">
  <!-- Top Bar -->
  <header class="h-14 px-6 border-b border-subtle flex items-center justify-between bg-surface/70 backdrop-blur-xs relative z-30">
    <div class="flex items-center gap-3 truncate">
      {#if onToggleSidebar}
        <Tooltip
          text={showSidebar ? "Collapse Sidebar" : "Expand Sidebar"}
          shortcut="Ctrl+B"
          position="bottom"
        >
          <button
            type="button"
            onclick={onToggleSidebar}
            class="p-1.5 rounded-lg text-secondary-theme hover:text-primary-theme hover:bg-surface transition-colors cursor-pointer border border-theme-default flex items-center justify-center shrink-0 {showSidebar ? '' : 'bg-surface-elevated text-accent-theme'}"
            aria-label="Toggle navigation sidebar"
          >
            {#if showSidebar}
              <PanelLeftClose size={15} />
            {:else}
              <PanelLeftOpen size={15} class="text-accent-theme" />
            {/if}
          </button>
        </Tooltip>
      {/if}
      <h2 class="font-medium text-sm text-primary-theme truncate">
        {session?.title || "Select or start a new session"}
      </h2>
      {#if workspace}
        <span class="text-[11px] px-2 py-0.5 rounded-full bg-surface-elevated text-accent-theme font-mono border border-subtle">
          {workspace.name} &bull; {workspace.model}
        </span>
      {/if}
    </div>

    <div class="flex items-center gap-2">
      <!-- Execution Approval Mode Status Badge -->
      <Tooltip
        text="Policy Mode: {currentModeConfig.label}"
        subtext="{currentModeConfig.description} (Workspace Setting)"
        position="bottom"
      >
        <div
          class="flex items-center gap-1.5 px-2.5 py-1.5 text-xs rounded-lg border font-semibold shadow-xs select-none cursor-help {currentModeConfig.badgeClass}"
        >
          <ModeIcon size={13} class={currentModeConfig.iconClass} />
          <span>{currentModeConfig.shortLabel}</span>
        </div>
      </Tooltip>

      {#if onOpenMcpModal}
        <Tooltip
          text="Model Context Protocol (MCP)"
          subtext="Connect external tools, databases, and APIs like Azure DevOps, GitHub, and SQLite"
          shortcut="Ctrl+M"
          position="bottom"
        >
          <button
            onclick={onOpenMcpModal}
            class="flex items-center gap-1.5 px-2.5 py-1.5 text-xs text-secondary-theme hover:text-primary-theme rounded-lg bg-surface hover:bg-surface-hover transition-colors border border-theme-default cursor-pointer"
          >
            <Server size={14} class="text-accent-theme" />
            <span>MCP Servers</span>
          </button>
        </Tooltip>
      {/if}

      <!-- Terminal Drawer Toggle Button -->
      <Tooltip
        text="Integrated Terminal Console"
        subtext="Open embedded PowerShell drawer to run builds, inspect git, and sync .env"
        shortcut="Ctrl+`"
        position="bottom"
      >
        <button
          type="button"
          onclick={() => (showTerminalDrawer = !showTerminalDrawer)}
          class="flex items-center gap-1.5 px-2.5 py-1.5 text-xs rounded-lg transition-colors border border-theme-default cursor-pointer {showTerminalDrawer ? 'bg-accent-subtle text-accent-theme border-accent-subtle' : 'text-secondary-theme hover:text-primary-theme bg-surface hover:bg-surface-hover'}"
        >
          <Terminal size={14} class={showTerminalDrawer ? 'text-accent-theme' : ''} />
          <span>Terminal</span>
        </button>
      </Tooltip>

      <!-- Workspace Explorer Toggle Button -->
      <Tooltip
        text="Workspace Explorer"
        subtext="Toggle workspace file and folder tree"
        shortcut="Ctrl+Alt+L"
        position="bottom"
      >
        <button
          type="button"
          onclick={() => (showSolutionExplorer = !showSolutionExplorer)}
          class="flex items-center gap-1.5 px-2.5 py-1.5 text-xs rounded-lg transition-colors border border-theme-default cursor-pointer {showSolutionExplorer ? 'bg-accent-subtle text-accent-theme border-accent-subtle' : 'text-secondary-theme hover:text-primary-theme bg-surface hover:bg-surface-hover'}"
          aria-label="Toggle Workspace Explorer"
        >
          <FolderTree size={14} class={showSolutionExplorer ? 'text-accent-theme' : ''} />
          <span>Explorer</span>
        </button>
      </Tooltip>

      <!-- Export Menu -->
      <div class="relative">
        <Tooltip
          text="Export Conversation"
          subtext="Save this conversation thread as Markdown (.md), Plaintext (.txt), or JSON"
          position="bottom"
        >
          <button
            onclick={() => (showExportMenu = !showExportMenu)}
            class="flex items-center gap-1.5 px-2.5 py-1.5 text-xs text-secondary-theme hover:text-primary-theme rounded-lg bg-surface hover:bg-surface-hover transition-colors border border-theme-default cursor-pointer"
          >
            <Download size={14} />
            <span>Export</span>
          </button>
        </Tooltip>

      {#if showExportMenu}
        <!-- Backdrop to close export dropdown on click outside -->
        <div
          class="fixed inset-0 z-30"
          onclick={() => (showExportMenu = false)}
          role="presentation"
        ></div>
        <div class="absolute right-0 mt-1.5 w-40 bg-surface-elevated border border-subtle rounded-lg shadow-xl py-1 z-40 text-xs">
          <button
            onclick={() => {
              showExportMenu = false;
              onExport("md");
            }}
            class="w-full px-3 py-1.5 text-left hover:bg-surface-hover text-secondary-theme hover:text-primary-theme flex items-center gap-2 cursor-pointer"
          >
            <FileText size={14} class="text-accent-theme" />
            <span>Markdown (.md)</span>
          </button>
          <button
            onclick={() => {
              showExportMenu = false;
              onExport("txt");
            }}
            class="w-full px-3 py-1.5 text-left hover:bg-surface-hover text-secondary-theme hover:text-primary-theme flex items-center gap-2 cursor-pointer"
          >
            <FileText size={14} class="text-muted-theme" />
            <span>Plaintext (.txt)</span>
          </button>
          <button
            onclick={() => {
              showExportMenu = false;
              onExport("json");
            }}
            class="w-full px-3 py-1.5 text-left hover:bg-surface-hover text-secondary-theme hover:text-primary-theme flex items-center gap-2 cursor-pointer"
          >
            <FileText size={14} class="text-emerald-400" />
            <span>Raw JSON</span>
          </button>
        </div>
      {/if}
      </div>
    </div>
  </header>

  <!-- Messages Viewport -->
  <div bind:this={chatViewport} onscroll={handleScroll} class="flex-1 overflow-y-auto p-6 space-y-6 relative z-0">
    {#if messages.length === 0 && !isStreaming}
      <div class="h-full flex flex-col items-center justify-center text-center max-w-md mx-auto py-20 text-muted-theme">
        <div class="w-12 h-12 rounded-2xl bg-accent-subtle border border-accent-subtle flex items-center justify-center text-accent-theme mb-4 shadow-inner">
          <Sparkles size={24} />
        </div>
        <h3 class="text-base font-semibold text-primary-theme mb-1">What would you like to build?</h3>
        <p class="text-xs text-secondary-theme leading-relaxed mb-6">
          Start a conversation in the <span class="text-accent-theme font-medium">{workspace?.name}</span> workspace. Gemini CLI will automatically use this directory's files and context.
        </p>
      </div>
    {/if}

    {#each messages as msg, i (msg.id)}
      <div class="flex gap-3.5 {msg.role === 'user' ? 'justify-end' : 'justify-start'}">
        {#if msg.role !== 'user'}
          <div class="w-7 h-7 rounded-lg bg-accent-subtle text-accent-theme border border-accent-subtle flex items-center justify-center shrink-0 mt-1">
            <Sparkles size={14} />
          </div>
        {/if}

        <div class="max-w-3xl group {msg.role === 'user' ? 'bg-bubble-user border border-bubble-user text-primary-theme rounded-2xl rounded-tr-sm px-4 py-2.5 text-xs shadow-xs' : 'bg-bubble-assistant border border-bubble-assistant rounded-2xl rounded-tl-sm px-5 py-4 text-xs text-primary-theme shadow-xs w-full'}">
          {#if msg.role === 'user'}
            <div class="whitespace-pre-wrap leading-relaxed select-text">{msg.content}</div>
            <div class="mt-1.5 flex items-center justify-end gap-1.5 text-[10px] text-muted-theme select-none opacity-0 group-hover:opacity-100 transition-opacity">
              {#if msg.created_at}
                <span class="mr-1">{formatTime(msg.created_at)}</span>
              {/if}
              <Tooltip text={copiedMessageId === msg.id ? "Copied!" : "Copy prompt"} position="top">
                <button
                  type="button"
                  onclick={() => handleCopyMessage(msg.id, msg.content)}
                  class="p-1 rounded hover:bg-surface-elevated text-secondary-theme hover:text-primary-theme transition-colors cursor-pointer flex items-center gap-1"
                  aria-label="Copy prompt"
                >
                  {#if copiedMessageId === msg.id}
                    <Check size={11} class="text-emerald-400" />
                    <span class="text-emerald-400 font-medium">Copied</span>
                  {:else}
                    <Copy size={11} />
                  {/if}
                </button>
              </Tooltip>
              <Tooltip text="Edit prompt" position="top">
                <button
                  type="button"
                  onclick={() => handleEditPrompt(msg.content)}
                  class="p-1 rounded hover:bg-surface-elevated text-secondary-theme hover:text-primary-theme transition-colors cursor-pointer flex items-center gap-1"
                  aria-label="Edit prompt"
                >
                  <Edit3 size={11} />
                </button>
              </Tooltip>
            </div>
          {:else}
            <div class="markdown-body">
              {@html renderMarkdown(msg.content)}
            </div>
            <div class="mt-2.5 pt-2 border-t border-subtle/50 flex items-center justify-between text-[11px] text-muted-theme select-none">
              <div class="flex items-center gap-2">
                {#if msg.created_at}
                  <span>{formatTime(msg.created_at)}</span>
                {/if}
                {#if msg.token_count > 0}
                  <span>&bull; ~{msg.token_count} tokens</span>
                {/if}
              </div>
              <div class="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                <Tooltip text={copiedMessageId === msg.id ? "Copied!" : "Copy response"} position="top">
                  <button
                    type="button"
                    onclick={() => handleCopyMessage(msg.id, msg.content)}
                    class="p-1 rounded hover:bg-surface-hover text-secondary-theme hover:text-primary-theme transition-colors cursor-pointer flex items-center gap-1 text-[11px]"
                    aria-label="Copy response"
                  >
                    {#if copiedMessageId === msg.id}
                      <Check size={13} class="text-emerald-400" />
                      <span class="text-emerald-400 font-medium">Copied</span>
                    {:else}
                      <Copy size={13} />
                      <span>Copy</span>
                    {/if}
                  </button>
                </Tooltip>

                {#if !messages.slice(i + 1).some(m => m.role === 'assistant') && !isStreaming}
                  <Tooltip text="Regenerate this response" position="top">
                    <button
                      type="button"
                      onclick={handleRegenerate}
                      class="p-1 rounded hover:bg-surface-hover text-secondary-theme hover:text-primary-theme transition-colors cursor-pointer flex items-center gap-1 text-[11px]"
                      aria-label="Regenerate response"
                    >
                      <RotateCcw size={13} />
                      <span>Retry</span>
                    </button>
                  </Tooltip>
                {/if}
              </div>
            </div>
          {/if}
        </div>

        {#if msg.role === 'user'}
          <div class="w-7 h-7 rounded-lg bg-surface-elevated text-secondary-theme border border-subtle flex items-center justify-center shrink-0 mt-1">
            <User size={14} />
          </div>
        {/if}
      </div>
    {/each}

    <!-- Live Streaming Response -->
    {#if isStreaming}
      <div class="flex gap-3.5 justify-start">
        <div class="w-7 h-7 rounded-lg bg-accent-subtle text-accent-theme border border-accent-subtle flex items-center justify-center shrink-0 mt-1 animate-pulse">
          <Sparkles size={14} />
        </div>
        <div class="max-w-3xl bg-bubble-assistant border border-bubble-assistant rounded-2xl rounded-tl-sm px-5 py-4 text-xs text-primary-theme shadow-xs w-full">
          {#if streamingText}
            <div class="markdown-body">
              {@html renderMarkdown(streamingText)}
            </div>
          {:else}
            <div class="flex items-center gap-2 text-muted-theme italic">
              <span class="inline-block w-2 h-2 rounded-full bg-accent-theme animate-ping"></span>
              Thinking...
            </div>
          {/if}
        </div>
      </div>
    {/if}

    <!-- Floating Scroll to Bottom Button -->
    {#if isUserScrolledUp}
      <div class="sticky bottom-2 flex justify-center pointer-events-none z-20">
        <Tooltip text="Scroll to bottom" position="top">
          <button
            type="button"
            onclick={() => scrollToBottom(true)}
            class="pointer-events-auto flex items-center gap-1.5 px-3 py-1.5 rounded-full bg-surface-elevated hover:bg-surface-hover border border-theme-default shadow-lg text-xs text-primary-theme font-medium transition-all hover:scale-105 cursor-pointer animate-fade-in"
            aria-label="Scroll to bottom"
          >
            <ArrowDown size={13} class="text-accent-theme animate-bounce" />
            <span>Latest messages</span>
          </button>
        </Tooltip>
      </div>
    {/if}
  </div>

  <!-- Tool Permission Confirmation Banner (ACP) -->
  {#if toolPermission}
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
                {typeof loc === 'string' ? loc : loc?.path || JSON.stringify(loc)}
              </div>
            {/each}
          {:else}
            <div class="truncate select-all text-accent-theme font-medium">
              {typeof toolPermission.locations === 'string' ? toolPermission.locations : toolPermission.locations?.path || JSON.stringify(toolPermission.locations)}
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
        {:else if typeof toolPermission.parameters === 'object' && Object.keys(toolPermission.parameters).length > 0 && !toolPermission.locations}
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
                onclick={() => onToolResponse(toolPermission.request_id, opt.option_id, opt.kind?.startsWith("allow") ?? true)}
                class="px-3.5 py-1.5 rounded-lg text-xs font-semibold flex items-center gap-1.5 transition-all shadow-xs cursor-pointer {
                  opt.kind?.startsWith('allow') || opt.name.toLowerCase().includes('allow')
                    ? 'bg-emerald-600 hover:bg-emerald-500 text-white font-bold'
                    : opt.kind?.startsWith('reject') || opt.name.toLowerCase().includes('reject') || opt.name.toLowerCase().includes('deny')
                    ? 'bg-rose-500/15 hover:bg-rose-500/25 text-rose-600 dark:text-rose-300 border border-rose-500/30'
                    : 'bg-surface hover:bg-surface-hover text-secondary-theme hover:text-primary-theme border border-theme-default'
                }"
              >
                {#if opt.kind?.startsWith('allow') || opt.name.toLowerCase().includes('allow')}
                  <CheckCircle size={14} />
                {:else if opt.kind?.startsWith('reject') || opt.name.toLowerCase().includes('reject') || opt.name.toLowerCase().includes('deny')}
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
              onclick={() => onToolResponse(toolPermission.request_id, undefined, true)}
              class="px-3.5 py-1.5 rounded-lg bg-emerald-600 hover:bg-emerald-500 text-white font-bold text-xs flex items-center gap-1.5 transition-colors shadow-xs cursor-pointer"
            >
              <CheckCircle size={14} />
              <span>Allow</span>
            </button>
            <button
              onclick={() => onToolResponse(toolPermission.request_id, undefined, false)}
              class="px-3.5 py-1.5 rounded-lg bg-surface hover:bg-surface-hover text-secondary-theme hover:text-rose-500 hover:border-rose-500/40 font-semibold text-xs flex items-center gap-1.5 transition-colors border border-theme-default shadow-xs cursor-pointer"
            >
              <XCircle size={14} />
              <span>Deny</span>
            </button>
          </div>
        {/if}
      </div>
    </div>
  {/if}

  <!-- Prompt Input Bar -->
  <div class="p-4 bg-surface border-t border-subtle relative z-20">
    <div class="max-w-4xl mx-auto relative rounded-xl border border-theme-default bg-app/80 focus-within:border-accent-theme transition-colors shadow-inner">
      
      <!-- Inline @ Mention Autocomplete Floating Popover -->
      {#if showMentionPopover && filteredMentionOptions.length > 0}
        <div
          class="fixed inset-0 z-35"
          onclick={dismissMentionPopover}
          role="presentation"
        ></div>
        <div class="absolute bottom-full left-0 mb-2 w-96 max-h-64 overflow-y-auto bg-surface-elevated border border-subtle rounded-xl shadow-2xl py-1 z-40 text-xs backdrop-blur-md">
          <div class="px-3 py-1.5 text-[10px] font-semibold uppercase tracking-wider text-muted-theme border-b border-subtle flex items-center justify-between">
            <span class="flex items-center gap-1.5">
              <span class="text-accent-theme font-mono font-bold">@</span>
              <span>Insert Reference</span>
            </span>
            <div class="flex items-center gap-2">
              <span class="font-mono text-[9px] opacity-75">↑↓ &bull; Enter/Tab &bull; Esc</span>
              <button
                type="button"
                onclick={dismissMentionPopover}
                class="p-0.5 rounded text-muted-theme hover:text-primary-theme hover:bg-surface transition-colors cursor-pointer"
                title="Dismiss (Esc)"
              >
                <X size={12} />
              </button>
            </div>
          </div>
          {#each filteredMentionOptions as opt, idx (opt.id)}
            <button
              type="button"
              class="w-full px-3 py-2 text-left flex items-center gap-2.5 transition-colors cursor-pointer {idx === mentionSelectedIndex ? 'bg-accent-theme/15 text-accent-theme font-medium border-l-2 border-accent-theme' : 'hover:bg-surface-hover text-secondary-theme'}"
              onmousedown={(e) => {
                e.preventDefault();
                applyMention(opt);
              }}
              onmouseenter={() => (mentionSelectedIndex = idx)}
            >
              {#if opt.kind === "git"}
                <GitBranch size={14} class="text-amber-400 shrink-0" />
              {:else if opt.kind === "directory"}
                <Folder size={14} class="text-sky-400 shrink-0" />
              {:else}
                <FileCode size={14} class="text-accent-theme shrink-0" />
              {/if}
              <div class="truncate flex-1">
                <div class="truncate text-primary-theme text-xs font-mono">{opt.label}</div>
                <div class="truncate text-[10px] text-muted-theme font-sans">{opt.subtitle}</div>
              </div>
            </button>
          {/each}
        </div>
      {/if}

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
            text="Policy Approval Mode: {currentModeConfig.label}"
            subtext="{currentModeConfig.description} (Configured in Workspace Settings)"
            position="top"
          >
            <div class="inline-flex items-center gap-1.5 px-2 py-0.5 rounded-md border text-[11px] font-semibold cursor-help select-none {currentModeConfig.badgeClass}">
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

  <!-- Terminal Drawer -->
  <TerminalDrawer bind:isOpen={showTerminalDrawer} {workspace} />

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
</div>
