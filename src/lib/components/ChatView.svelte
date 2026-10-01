<script lang="ts">
  import type {
    Workspace,
    Session,
    Message,
    ToolPermissionPayload,
    WorkspaceFileEntry,
    ApprovalMode,
  } from "$lib/types";
  import ToolPermissionCard from "$lib/components/ToolPermissionCard.svelte";
  import ChatPromptComposer from "$lib/components/ChatPromptComposer.svelte";
  import TerminalDrawer from "$lib/components/TerminalDrawer.svelte";
  import Tooltip from "$lib/components/Tooltip.svelte";
  import { renderMarkdown } from "$lib/markdown";
  import { getModeConfig } from "$lib/utils/approvalModes";
  import {
    Download,
    Sparkles,
    User,
    Copy,
    Check,
    RotateCcw,
    Edit3,
    ArrowDown,
    PanelLeftClose,
    PanelLeftOpen,
    FileText,
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
    approvalMode = "default",
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

  let composerRef: ReturnType<typeof ChatPromptComposer> | null = $state(null);
  let chatViewport: HTMLDivElement | null = $state(null);
  let showExportMenu = $state(false);
  let isUserScrolledUp = $state(false);
  let copiedMessageId = $state<string | null>(null);

  let effectiveApprovalMode = $derived(
    workspace?.approval_mode || approvalMode || "default"
  );
  let currentModeConfig = $derived(getModeConfig(effectiveApprovalMode));
  let ModeIcon = $derived(currentModeConfig.icon);

  export function insertFileMention(relPath: string) {
    composerRef?.insertFileMention(relPath);
  }

  export function attachItem(path: string, isDir: boolean, name?: string) {
    composerRef?.attachItem(path, isDir, name);
  }

  export function attachMultiple(items: { path: string; isDir: boolean; name?: string }[]) {
    composerRef?.attachMultiple(items);
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
    composerRef?.setPrompt(promptText);
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
      <!-- Execution Approval Mode Status Badge (Static indicator, configured in Workspace Settings) -->
      <Tooltip
        text={`Policy Approval Mode: ${currentModeConfig.label}`}
        subtext={`${currentModeConfig.description} (Change in Workspace Settings)`}
        position="bottom"
      >
        <div
          class="flex items-center gap-1.5 px-2.5 py-1.5 text-xs rounded-lg border font-semibold shadow-xs select-none cursor-default {currentModeConfig.badgeClass}"
        >
          <ModeIcon size={13} class={currentModeConfig.iconClass} />
          <span>{currentModeConfig.shortLabel}</span>
        </div>
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
    <ToolPermissionCard {toolPermission} onRespond={onToolResponse} />
  {/if}

  <!-- Prompt Input Bar -->
  <ChatPromptComposer
    bind:this={composerRef}
    {workspace}
    {workspaceFiles}
    {isStreaming}
    {approvalMode}
    onSendPrompt={(prompt) => {
      isUserScrolledUp = false;
      tick().then(() => scrollToBottom(true));
      onSendPrompt(prompt);
    }}
    {onCancelPrompt}
  />

  <!-- Terminal Drawer -->
  <TerminalDrawer bind:isOpen={showTerminalDrawer} {workspace} />
</div>
