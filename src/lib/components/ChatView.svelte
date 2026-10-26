<script lang="ts">
  import type {
    Workspace,
    Session,
    Message,
    ToolPermissionPayload,
    WorkspaceFileEntry,
    ApprovalMode,
    ParsedDebugError,
    FileSnippetResult,
    ToolExecutionStep,
    PlanTaskItem,
    SessionFileChange,
  } from "$lib/types";
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import SessionChangesModal from "$lib/components/SessionChangesModal.svelte";
  import ToolCallNode from "$lib/components/ToolCallNode.svelte";
  import { parseToolSteps, cleanMessageContent } from "$lib/utils/toolSteps";
  import ToolPermissionCard from "$lib/components/ToolPermissionCard.svelte";
  import DebugDoctorBanner from "$lib/components/DebugDoctorBanner.svelte";
  import ChatPromptComposer from "$lib/components/ChatPromptComposer.svelte";
  import PlanWorkbench from "$lib/components/PlanWorkbench.svelte";
  import { extractTasksFromMarkdown, syncTasksCompletionFromMarkdown } from "$lib/utils/planParser";
  import { modalManager } from "$lib/modal.svelte";
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
    History,
    Compass,
    Edit3,
    ArrowDown,
    PanelLeftClose,
    PanelLeftOpen,
    FileText,
    ListTodo,
    FileDiff,
  } from "lucide-svelte";
  import { tick, onMount, onDestroy } from "svelte";

  let {
    workspace = null,
    session = null,
    workspaceFiles = [],
    messages = [],
    isStreaming = false,
    streamingText = "",
    streamingToolCalls = [],
    streamingSteeringHints = [],
    toolPermission = null,
    showSidebar = true,
    onToggleSidebar,
    showTerminalDrawer = $bindable(false),
    showSolutionExplorer = $bindable(true),
    approvalMode = "default",
    onSendPrompt,
    onCancelPrompt,
    onSteerPrompt,
    onExecutePlan,
    onToolResponse,
    onExport,
    onOpenMcpModal,
    onOpenRewindModal,
    activeDebugError = null,
    activeFileSnippet = null,
    onDiagnoseDebugError,
    onPrefillDebugError,
    onDismissDebugError,
    onTriggerDebugDoctor,
    onDebugErrorDetected,
  }: {
    workspace: Workspace | null;
    session: Session | null;
    workspaceFiles?: WorkspaceFileEntry[];
    messages: Message[];
    isStreaming: boolean;
    streamingText: string;
    streamingToolCalls?: ToolExecutionStep[];
    streamingSteeringHints?: string[];
    toolPermission: ToolPermissionPayload | null;
    showSidebar?: boolean;
    onToggleSidebar?: () => void;
    showTerminalDrawer?: boolean;
    showSolutionExplorer?: boolean;
    approvalMode?: ApprovalMode;
    onSendPrompt: (prompt: string) => void;
    onCancelPrompt: () => void;
    onSteerPrompt?: (hint: string) => void;
    onExecutePlan?: (mode: "auto_edit" | "default", tasks: PlanTaskItem[]) => void;
    onToolResponse: (requestId: number, optionId?: string, allowed?: boolean) => void;
    onExport: (format: string) => void;
    onOpenMcpModal?: () => void;
    onOpenRewindModal?: (targetId?: string) => void;
    activeDebugError?: ParsedDebugError | null;
    activeFileSnippet?: FileSnippetResult | null;
    onDiagnoseDebugError?: (prompt: string) => void;
    onPrefillDebugError?: (prompt: string) => void;
    onDismissDebugError?: () => void;
    onTriggerDebugDoctor?: () => void;
    onDebugErrorDetected?: (error: ParsedDebugError) => void;
  } = $props();

  let composerRef: ReturnType<typeof ChatPromptComposer> | null = $state(null);
  let chatViewport: HTMLDivElement | null = $state(null);
  let showExportMenu = $state(false);
  let isUserScrolledUp = $state(false);
  let copiedMessageId = $state<string | null>(null);

  let isPlanWorkbenchOpen = $state(false);
  let wasExecutingPlan = $state(false);
  let sessionPlanTasks = $state<Record<string, PlanTaskItem[]>>({});
  let lastParsedPlanContent = $state<Record<string, string>>({});
  let hasAutoOpenedForStreamingPlan = $state(false);

  let currentSessionId = $derived(session?.id || "");
  let currentTasks = $derived(
    currentSessionId && sessionPlanTasks[currentSessionId]
      ? sessionPlanTasks[currentSessionId]
      : []
  );

  let sessionChanges = $state<SessionFileChange[]>([]);
  let isChangesModalOpen = $state(false);

  let totalAdditions = $derived(sessionChanges.reduce((acc, c) => acc + c.additions, 0));
  let totalDeletions = $derived(sessionChanges.reduce((acc, c) => acc + c.deletions, 0));

  async function loadSessionChanges() {
    if (!workspace?.track_file_changes || !currentSessionId || !workspace?.path) {
      sessionChanges = [];
      return;
    }
    try {
      const res = await invoke<SessionFileChange[]>("get_session_file_changes", {
        sessionId: currentSessionId,
        workspacePath: workspace.path,
      });
      sessionChanges = res || [];
    } catch (e) {
      console.warn("Failed to load session file changes", e);
    }
  }

  $effect(() => {
    const _len = messages.length;
    const _streaming = isStreaming;
    if (workspace?.track_file_changes && currentSessionId && workspace?.path) {
      loadSessionChanges();
    } else {
      sessionChanges = [];
    }
  });

  let unlistenFileUpdates: UnlistenFn | null = null;
  onMount(async () => {
    try {
      unlistenFileUpdates = await listen<{ session_id: string }>("session-files-updated", (event: { payload: { session_id: string } }) => {
        if (workspace?.track_file_changes && (!event.payload?.session_id || event.payload.session_id === currentSessionId)) {
          loadSessionChanges();
        }
      });
    } catch (e) {
      console.warn("Failed to setup session-files-updated listener", e);
    }
  });

  onDestroy(() => {
    if (unlistenFileUpdates) {
      unlistenFileUpdates();
    }
  });

  async function handleRevertFile(filePath: string) {
    if (!currentSessionId || !workspace?.path) return;
    try {
      await invoke("revert_session_file", {
        sessionId: currentSessionId,
        workspacePath: workspace.path,
        filePath,
      });
      await loadSessionChanges();
    } catch (e) {
      console.error("Failed to revert file", e);
    }
  }

  async function handleRevertAll() {
    if (!currentSessionId || !workspace?.path) return;
    try {
      await invoke("revert_all_session_files", {
        sessionId: currentSessionId,
        workspacePath: workspace.path,
      });
      await loadSessionChanges();
    } catch (e) {
      console.error("Failed to revert all files", e);
    }
  }

  let lastPlanContent = $derived.by(() => {
    if (isStreaming && streamingText && (streamingText.includes("- [ ]") || streamingText.includes("- [x]") || streamingText.includes("Step 1:"))) {
      return streamingText;
    }
    // Only inspect the most recent assistant message in this conversation
    for (let i = messages.length - 1; i >= Math.max(0, messages.length - 2); i--) {
      const m = messages[i];
      if (m.role === "assistant" && (m.content.includes("- [ ]") || m.content.includes("- [x]") || m.content.includes("Step 1:"))) {
        return m.content;
      }
    }
    return "";
  });

  // When session changes: keep workbench closed by default (never auto-pop on load/switch)
  let prevSessionId = $state("");
  $effect(() => {
    const sid = currentSessionId;
    if (sid !== prevSessionId) {
      prevSessionId = sid;
      isPlanWorkbenchOpen = false;
      hasAutoOpenedForStreamingPlan = false;

      // Load tasks for this specific session from localStorage if not already in memory
      if (sid && !sessionPlanTasks[sid] && typeof window !== "undefined") {
        try {
          const saved = localStorage.getItem(`gemini_plan_tasks_${sid}`);
          if (saved) {
            const parsed = JSON.parse(saved);
            if (Array.isArray(parsed) && parsed.length > 0) {
              sessionPlanTasks[sid] = parsed;
            }
          }
        } catch (e) {
          console.warn("Failed to load plan tasks for session", sid, e);
        }
      }
    }
  });

  // If this session has no tasks yet, but messages contain a plan, parse it for this session
  $effect(() => {
    const sid = currentSessionId;
    if (sid && lastPlanContent && lastPlanContent !== lastParsedPlanContent[sid]) {
      lastParsedPlanContent[sid] = lastPlanContent;
      if (!sessionPlanTasks[sid] || sessionPlanTasks[sid].length === 0) {
        const extracted = extractTasksFromMarkdown(lastPlanContent);
        if (extracted.length > 0) {
          sessionPlanTasks[sid] = extracted;
          try {
            localStorage.setItem(`gemini_plan_tasks_${sid}`, JSON.stringify(extracted));
          } catch {}
        }
      }
    }
  });

  // Auto-open workbench ONLY when real-time streaming actively generates a new plan
  $effect(() => {
    if (!isStreaming) {
      hasAutoOpenedForStreamingPlan = false;
      return;
    }
    const sid = currentSessionId;
    if (!sid) return;

    if (streamingText && (streamingText.includes("- [ ]") || streamingText.includes("Step 1:"))) {
      if (!hasAutoOpenedForStreamingPlan) {
        hasAutoOpenedForStreamingPlan = true;
        isPlanWorkbenchOpen = true;
      }
      const liveExtracted = extractTasksFromMarkdown(streamingText);
      if (liveExtracted.length > 0) {
        sessionPlanTasks[sid] = liveExtracted;
        try {
          localStorage.setItem(`gemini_plan_tasks_${sid}`, JSON.stringify(liveExtracted));
        } catch {}
      }
    }
  });

  function handleUpdateTasks(updated: PlanTaskItem[]) {
    const sid = currentSessionId;
    if (!sid) return;
    sessionPlanTasks[sid] = updated;
    try {
      localStorage.setItem(`gemini_plan_tasks_${sid}`, JSON.stringify(updated));
    } catch {}
  }

  // Live sync of completed checkboxes from streaming text or assistant messages
  $effect(() => {
    const sid = currentSessionId;
    if (!sid || !sessionPlanTasks[sid] || sessionPlanTasks[sid].length === 0) return;

    const sourceText = isStreaming ? streamingText : lastPlanContent;
    if (sourceText && (sourceText.includes("- [x]") || sourceText.includes("- [X]") || sourceText.includes("* [x]"))) {
      const synced = syncTasksCompletionFromMarkdown(sessionPlanTasks[sid], sourceText);
      if (synced !== sessionPlanTasks[sid]) {
        sessionPlanTasks[sid] = synced;
        try {
          localStorage.setItem(`gemini_plan_tasks_${sid}`, JSON.stringify(synced));
        } catch {}
      }
    }
  });

  // Turn finish: auto-mark remaining tasks as completed when plan execution concludes
  $effect(() => {
    if (!isStreaming && wasExecutingPlan) {
      wasExecutingPlan = false;
      const sid = currentSessionId;
      if (sid && sessionPlanTasks[sid] && sessionPlanTasks[sid].length > 0) {
        const completedTasks = sessionPlanTasks[sid].map((t) => ({ ...t, completed: true }));
        sessionPlanTasks[sid] = completedTasks;
        try {
          localStorage.setItem(`gemini_plan_tasks_${sid}`, JSON.stringify(completedTasks));
        } catch {}
      }
    }
  });

  let lastEscTime = 0;
  function handleEscEsc(e: KeyboardEvent) {
    if (e.key === "Escape" && !isStreaming) {
      const now = Date.now();
      if (now - lastEscTime < 450) {
        modalManager.open("rewind");
        lastEscTime = 0;
      } else {
        lastEscTime = now;
      }
    }
  }

  $effect(() => {
    window.addEventListener("keydown", handleEscEsc);
    return () => {
      window.removeEventListener("keydown", handleEscEsc);
    };
  });

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
  <header class="h-14 px-6 border-b border-subtle flex items-center justify-between gap-3 bg-surface/70 backdrop-blur-xs relative z-30">
    <div class="flex items-center gap-3 min-w-0">
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
        <span class="text-[11px] px-2 py-0.5 rounded-full bg-surface-elevated text-accent-theme font-mono border border-subtle shrink-0 hidden sm:inline-block">
          {workspace.name} &bull; {workspace.model}
        </span>
      {/if}
    </div>

    <div class="flex items-center gap-2 shrink-0">
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

      {#if effectiveApprovalMode === "plan" || currentTasks.length > 0 || lastPlanContent}
        <Tooltip text={isPlanWorkbenchOpen ? "Hide Plan Workbench" : "Open Plan Workbench"} position="bottom">
          <button
            type="button"
            onclick={() => (isPlanWorkbenchOpen = !isPlanWorkbenchOpen)}
            class="flex items-center gap-1.5 px-2.5 py-1.5 text-xs rounded-lg border font-medium shadow-xs transition-colors cursor-pointer {isPlanWorkbenchOpen ? 'bg-sky-500/15 border-sky-500/40 text-sky-300' : 'bg-surface hover:bg-surface-hover text-secondary-theme border-subtle'}"
          >
            <ListTodo size={13} class={currentTasks.length > 0 && currentTasks.every(t => t.completed) ? 'text-emerald-400' : 'text-sky-400'} />
            <span>Plan</span>
            {#if currentTasks.length > 0}
              <span class="ml-0.5 px-1.5 py-0.2 rounded-full text-[10px] {currentTasks.every(t => t.completed) ? 'bg-emerald-500/20 text-emerald-300' : 'bg-sky-500/20 text-sky-300'} font-semibold">
                {currentTasks.filter(t => t.completed).length}/{currentTasks.length}
              </span>
            {/if}
          </button>
        </Tooltip>
      {/if}

      <!-- Session File Changes Badge (Works in non-git Windows folders when enabled in Workspace Settings) -->
      {#if workspace?.track_file_changes && sessionChanges.length > 0}
        <Tooltip
          text="Inspect Session File Changes"
          subtext={`${sessionChanges.length} file(s) modified in this session (+${totalAdditions} -${totalDeletions})`}
          position="bottom"
        >
          <button
            type="button"
            onclick={() => (isChangesModalOpen = true)}
            class="flex items-center gap-1.5 px-2.5 py-1.5 text-xs rounded-lg border font-medium shadow-xs transition-colors cursor-pointer bg-amber-500/10 border-amber-500/30 text-amber-300 hover:bg-amber-500/20"
          >
            <FileDiff size={13} class="text-amber-400" />
            <span>Changes</span>
            <span class="px-1.5 py-0.2 rounded-full text-[10px] bg-amber-500/20 text-amber-200 font-semibold font-mono">
              {sessionChanges.length}
            </span>
            {#if totalAdditions > 0 || totalDeletions > 0}
              <span class="text-[10px] font-mono opacity-80 hidden md:inline">
                <span class="text-emerald-400">+{totalAdditions}</span>
                <span class="text-rose-400">-{totalDeletions}</span>
              </span>
            {/if}
          </button>
        </Tooltip>
      {/if}

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
              <Tooltip text="Rewind conversation to this turn (Esc Esc)" position="top">
                <button
                  type="button"
                  onclick={() => onOpenRewindModal ? onOpenRewindModal(msg.id) : modalManager.open("rewind")}
                  class="p-1 rounded hover:bg-surface-elevated text-secondary-theme hover:text-amber-400 transition-colors cursor-pointer flex items-center gap-1"
                  aria-label="Rewind to prompt"
                >
                  <History size={11} />
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
            {@const toolSteps = parseToolSteps(msg)}
            {@const cleanContent = cleanMessageContent(msg.content, toolSteps.length > 0)}

            {#if toolSteps.length > 0}
              <div class="mb-3 space-y-1">
                {#each toolSteps as step (step.id)}
                  <ToolCallNode {step} />
                {/each}
              </div>
            {/if}

            {#if cleanContent}
              <div class="markdown-body">
                {@html renderMarkdown(cleanContent)}
              </div>
            {/if}

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
                <Tooltip text="Rewind to this turn (Esc Esc)" position="top">
                  <button
                    type="button"
                    onclick={() => onOpenRewindModal ? onOpenRewindModal(msg.id) : modalManager.open("rewind")}
                    class="p-1 rounded hover:bg-surface-hover text-secondary-theme hover:text-amber-400 transition-colors cursor-pointer flex items-center gap-1 text-[11px]"
                    aria-label="Rewind to this turn"
                  >
                    <History size={13} />
                    <span>Rewind</span>
                  </button>
                </Tooltip>
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
          {#if streamingSteeringHints && streamingSteeringHints.length > 0}
            <div class="mb-3 space-y-1.5">
              {#each streamingSteeringHints as hint}
                <div class="flex items-center gap-2 px-3 py-1.5 rounded-lg bg-amber-500/10 border border-amber-500/30 text-amber-300 text-xs">
                  <Compass size={13} class="text-amber-400 shrink-0" />
                  <span class="font-semibold text-amber-400">Steering injected:</span>
                  <span class="truncate">{hint}</span>
                </div>
              {/each}
            </div>
          {/if}

          {#if streamingToolCalls && streamingToolCalls.length > 0}
            <div class="mb-3 space-y-1">
              {#each streamingToolCalls as step (step.id)}
                <ToolCallNode {step} />
              {/each}
            </div>
          {/if}

          {#if streamingText}
            <div class="markdown-body">
              {@html renderMarkdown(streamingText)}
            </div>
          {:else if !streamingToolCalls || streamingToolCalls.length === 0}
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

  <!-- Instant Debug Doctor Diagnostic Banner -->
  {#if activeDebugError}
    <div class="px-4 pt-2">
      <DebugDoctorBanner
        debugError={activeDebugError}
        fileSnippet={activeFileSnippet}
        onDiagnose={(prompt) => onDiagnoseDebugError?.(prompt)}
        onPrefill={(prompt) => onPrefillDebugError?.(prompt)}
        onDismiss={() => onDismissDebugError?.()}
      />
    </div>
  {/if}

  <!-- Plan Mode Interactive Workbench -->
  <PlanWorkbench
    {workspace}
    sessionId={session?.id}
    approvalMode={effectiveApprovalMode}
    tasks={currentTasks}
    isOpen={isPlanWorkbenchOpen}
    onTasksChange={handleUpdateTasks}
    onExecutePlan={(targetMode, tasks) => {
      wasExecutingPlan = true;
      isPlanWorkbenchOpen = false;
      onExecutePlan?.(targetMode, tasks);
    }}
    onRefinePlan={() => composerRef?.focus()}
    onClose={() => (isPlanWorkbenchOpen = false)}
  />

  <!-- Prompt Input Bar -->
  <ChatPromptComposer
    bind:this={composerRef}
    {workspace}
    {workspaceFiles}
    {isStreaming}
    {approvalMode}
    {onTriggerDebugDoctor}
    {onDebugErrorDetected}
    {onSteerPrompt}
    onSendPrompt={(prompt) => {
      isUserScrolledUp = false;
      tick().then(() => scrollToBottom(true));
      onSendPrompt(prompt);
    }}
    onCancelPrompt={() => {
      wasExecutingPlan = false;
      onCancelPrompt();
    }}
  />

  <!-- Terminal Drawer -->
  <TerminalDrawer bind:isOpen={showTerminalDrawer} {workspace} />

  <!-- Session Changes & Visual Diff Modal -->
  <SessionChangesModal
    isOpen={isChangesModalOpen}
    changes={sessionChanges}
    workspacePath={workspace?.path}
    onClose={() => (isChangesModalOpen = false)}
    onRevertFile={handleRevertFile}
    onRevertAll={handleRevertAll}
  />
</div>
