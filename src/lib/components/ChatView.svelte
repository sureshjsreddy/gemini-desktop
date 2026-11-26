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
    CliEngine,
    AttachmentItem,
    DynamicAcpCommand,
    ContextUsageStats,
    QueuedPromptItem,
  } from "$lib/types";
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import SessionChangesModal from "$lib/components/SessionChangesModal.svelte";
  import ToolCallGroup from "$lib/components/ToolCallGroup.svelte";
  import { parseToolSteps, cleanMessageContent, formatDuration } from "$lib/utils/toolSteps";
  import ToolPermissionCard from "$lib/components/ToolPermissionCard.svelte";
  import DebugDoctorBanner from "$lib/components/DebugDoctorBanner.svelte";
  import ChatPromptComposer from "$lib/components/ChatPromptComposer.svelte";
  import PlanWorkbench from "$lib/components/PlanWorkbench.svelte";
  import { extractTasksFromMarkdown, syncTasksCompletionFromMarkdown } from "$lib/utils/planParser";
  import { modalManager } from "$lib/modal.svelte";
  import EngineSwitcher from "$lib/components/EngineSwitcher.svelte";
  import { engineStore } from "$lib/stores/engineStore.svelte";
  import { getEffectiveModel } from "$lib/utils/engineModels";
  import TerminalDrawer from "$lib/components/TerminalDrawer.svelte";
  import Tooltip from "$lib/components/Tooltip.svelte";
  import { renderMarkdown } from "$lib/markdown";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { dialogManager } from "$lib/dialog.svelte";
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
    ArrowUp,
    PanelLeftClose,
    PanelLeftOpen,
    FileText,
    ListTodo,
    FileDiff,
    Minimize2,
    GitFork,
    Upload,
    Brain,
    ChevronRight,
    Quote,
  } from "lucide-svelte";
  import { tick, onMount, onDestroy } from "svelte";

  let {
    workspace = null,
    session = null,
    workspaceFiles = [],
    messages = [],
    isStreaming = false,
    streamingText = "",
    streamingThoughtText = "",
    streamingThinkingStartedAt = undefined,
    messageThoughts = {},
    streamingToolCalls = [],
    streamingSteeringHints = [],
    streamingStartedAt = undefined,
    toolPermission = null,
    pendingPermissionCount = 0,
    contextUsage = null,
    dynamicCommands = [],
    queuedPrompts = [],
    showSidebar = false,
    onToggleSidebar,
    showTerminalDrawer = $bindable(false),
    showSolutionExplorer = $bindable(true),
    approvalMode = "default",
    activeEngine = "gemini",
    onSendPrompt,
    onCancelPrompt,
    onSteerPrompt,
    onQueuePrompt,
    onRemoveQueuedPrompt,
    onExecutePlan,
    onToolResponse,
    onExport,
    onOpenMcpModal,
    onSwitchEngine,
    onOpenRewindModal,
    onForkSession,
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
    streamingThoughtText?: string;
    streamingThinkingStartedAt?: number;
    messageThoughts?: Record<string, { text: string; durationMs?: number }>;
    streamingToolCalls?: ToolExecutionStep[];
    streamingSteeringHints?: string[];
    streamingStartedAt?: number;
    toolPermission: ToolPermissionPayload | null;
    pendingPermissionCount?: number;
    contextUsage?: ContextUsageStats | null;
    dynamicCommands?: DynamicAcpCommand[];
    queuedPrompts?: QueuedPromptItem[];
    showSidebar?: boolean;
    onToggleSidebar?: () => void;
    showTerminalDrawer?: boolean;
    showSolutionExplorer?: boolean;
    approvalMode?: ApprovalMode;
    activeEngine?: CliEngine;
    onSendPrompt: (prompt: string, attachments?: AttachmentItem[]) => void;
    onCancelPrompt: () => void;
    onSteerPrompt?: (hint: string) => void;
    onQueuePrompt?: (prompt: string, attachments?: AttachmentItem[]) => void;
    onRemoveQueuedPrompt?: (id: string) => void;
    onExecutePlan?: (mode: "auto_edit" | "default", tasks: PlanTaskItem[]) => void;
    onToolResponse: (requestId: number | string, optionId?: string, allowed?: boolean, answer?: string) => void;
    onExport: (format: string) => void;
    onOpenMcpModal?: () => void;
    onSwitchEngine?: (engine: CliEngine) => void | Promise<void>;
    onOpenRewindModal?: (targetId?: string) => void;
    onForkSession?: (messageId: string) => void;
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
  let expandedThinkingMap = $state<Record<string, boolean>>({});
  let currentPromptJumpIdx = $state(-1);

  const INITIAL_VISIBLE_MESSAGES = 60;
  let visibleMessageCount = $state(INITIAL_VISIBLE_MESSAGES);
  let msgWindowOffset = $derived(Math.max(0, messages.length - visibleMessageCount));
  let hiddenMessageCount = $derived(msgWindowOffset);
  let visibleMessages = $derived(
    msgWindowOffset > 0 ? messages.slice(msgWindowOffset) : messages
  );

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

  let totalSessionTokens = $derived(
    messages.reduce((acc, m) => acc + (m.token_count || 0), 0)
  );

  let displayedUsedTokens = $derived(
    contextUsage && contextUsage.usedTokens > 0 ? contextUsage.usedTokens : totalSessionTokens
  );
  let displayedContextWindow = $derived(
    contextUsage && contextUsage.contextWindow > 0 ? contextUsage.contextWindow : 1_000_000
  );

  function formatTokenCount(num: number): string {
    if (num >= 1_000_000) {
      return (num / 1_000_000).toFixed(1) + "M";
    }
    if (num >= 1_000) {
      return (num / 1_000).toFixed(1) + "k";
    }
    return num.toString();
  }

  let contextUsageRatio = $derived(
    Math.min(1, displayedContextWindow > 0 ? displayedUsedTokens / displayedContextWindow : 0)
  );
  let isNearContextCeiling = $derived(contextUsageRatio >= 0.75);

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
  function handleDocClick(e: MouseEvent) {
    handleChatClick(e);
  }

  function handleAcpPlanEntries(e: Event) {
    const detail = (e as CustomEvent<{ sessionId: string; entries: any[] }>).detail;
    if (!detail || !detail.sessionId || detail.sessionId !== currentSessionId) return;
    if (!Array.isArray(detail.entries) || detail.entries.length === 0) return;

    const mapped: PlanTaskItem[] = detail.entries.map((entry, idx) => ({
      id: entry.id ? String(entry.id) : `acp-plan-${idx}`,
      text: String(entry.content || entry.title || entry.text || "").trim(),
      completed: entry.status === "completed" || entry.completed === true,
      category: entry.priority || undefined,
    })).filter((t) => t.text.length > 0);

    if (mapped.length > 0) {
      sessionPlanTasks[detail.sessionId] = mapped;
      try {
        localStorage.setItem(`gemini_plan_tasks_${detail.sessionId}`, JSON.stringify(mapped));
      } catch {}
    }
  }

  function jumpUserPrompt(direction: -1 | 1) {
    if (!chatViewport) return;
    const nodes = Array.from(chatViewport.querySelectorAll<HTMLElement>("[data-user-turn-idx]"));
    if (nodes.length === 0) return;
    if (currentPromptJumpIdx < 0 || currentPromptJumpIdx >= nodes.length) {
      currentPromptJumpIdx = direction === -1 ? nodes.length - 1 : 0;
    } else {
      currentPromptJumpIdx = Math.max(0, Math.min(nodes.length - 1, currentPromptJumpIdx + direction));
    }
    nodes[currentPromptJumpIdx]?.scrollIntoView({ behavior: "smooth", block: "center" });
  }

  function handleQuoteSelectionOrText(rawText: string) {
    const selected = typeof window !== "undefined" ? window.getSelection()?.toString().trim() : "";
    const snippet = selected || rawText.trim().slice(0, 600);
    if (!snippet) return;
    const quoted = snippet
      .split("\n")
      .map((l) => `> ${l}`)
      .join("\n") + "\n\n";
    window.dispatchEvent(new CustomEvent("gemini-prefill-prompt", { detail: quoted }));
  }

  onMount(async () => {
    document.addEventListener("click", handleDocClick);
    window.addEventListener("acp-plan-entries", handleAcpPlanEntries);
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
    document.removeEventListener("click", handleDocClick);
    window.removeEventListener("acp-plan-entries", handleAcpPlanEntries);
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
      visibleMessageCount = INITIAL_VISIBLE_MESSAGES;
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
        const extracted = extractTasksFromMarkdown(lastPlanContent, sessionPlanTasks[sid] || []);
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
      const liveExtracted = extractTasksFromMarkdown(streamingText, sessionPlanTasks[sid] || []);
      if (liveExtracted.length > 0) {
        if (!hasAutoOpenedForStreamingPlan) {
          hasAutoOpenedForStreamingPlan = true;
          isPlanWorkbenchOpen = true;
        }
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

  export function getPromptText(): string {
    return composerRef?.getPrompt?.() || "";
  }

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

  function getTurnDurationMs(msgIndex: number, currentMsg: Message): number | undefined {
    if (currentMsg.role !== "assistant" || !currentMsg.created_at) return undefined;
    for (let j = msgIndex - 1; j >= 0; j--) {
      const prev = messages[j];
      if (prev.role === "user" && prev.created_at) {
        const start = new Date(prev.created_at).getTime();
        const end = new Date(currentMsg.created_at).getTime();
        if (!isNaN(start) && !isNaN(end) && end >= start) {
          return end - start;
        }
        break;
      }
    }
    return undefined;
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

  async function handleOpenResource(rawTarget: string) {
    if (!rawTarget) return;
    let target = rawTarget.trim();
    // Strip leading 📄 or trailing ↗ in case textContent was used as fallback
    target = target.replace(/^📄\s*/, "").replace(/\s*↗$/, "").trim();
    if (!target) return;

    // 1. Web links: http://, https://, mailto:
    if (target.startsWith("http://") || target.startsWith("https://") || target.startsWith("mailto:")) {
      try {
        await openUrl(target);
      } catch {
        // Fallback to backend open_workspace_file
        try {
          await invoke("open_workspace_file", { path: target, workspacePath: workspace?.path });
        } catch (err) {
          console.error("Failed to open URL:", err);
        }
      }
      return;
    }

    // 2. Local file / file:/// URI / relative path / Antigravity brain artifact
    try {
      await invoke("open_workspace_file", {
        path: target,
        workspacePath: workspace?.path,
      });
    } catch (err: any) {
      console.warn("Cannot open file directly:", err);
      const errMsg = err instanceof Error ? err.message : String(err);

      // If the file does not exist on disk, offer to create and open it
      if (errMsg.toLowerCase().includes("does not exist") || errMsg.toLowerCase().includes("not found")) {
        const shouldCreate = await dialogManager.confirm(
          `The referenced file was not found in your workspace or on disk:\n\n${target}\n\nWould you like to create this file and open it in your editor?`,
          {
            title: "File Not Found",
            confirmText: "Create & Open",
          }
        );
        if (shouldCreate) {
          try {
            await invoke("open_workspace_file", {
              path: target,
              workspacePath: workspace?.path,
              createIfMissing: true,
            });
          } catch (createErr: any) {
            const cMsg = createErr instanceof Error ? createErr.message : String(createErr);
            await dialogManager.alert(
              `Failed to create or open file:\n\n${cMsg}`,
              "Error"
            );
          }
        }
        return;
      }

      await dialogManager.alert(
        `Unable to open file:\n\n${target}\n\n${errMsg}`,
        "Cannot Open File"
      );
    }
  }

  function handleChatClick(e: MouseEvent) {
    const copyTarget = (e.target as HTMLElement)?.closest(".copy-code-btn");
    if (copyTarget) {
      e.preventDefault();
      const rawCode = copyTarget.getAttribute("data-code");
      if (rawCode) {
        try {
          const code = decodeURIComponent(rawCode);
          navigator.clipboard.writeText(code).then(() => {
            const orig = copyTarget.textContent;
            copyTarget.textContent = "Copied!";
            setTimeout(() => {
              copyTarget.textContent = orig || "Copy";
            }, 1500);
          }).catch((err) => {
            console.warn("Failed to copy code block:", err);
          });
        } catch (err) {
          console.warn("Failed to decode code block:", err);
        }
      }
      return;
    }

    // Intercept markdown link clicks (web URLs, file://, relative files)
    const linkTarget = (e.target as HTMLElement)?.closest("a");
    if (linkTarget) {
      const rawTarget = linkTarget.getAttribute("data-path")
        || linkTarget.getAttribute("href")
        || linkTarget.textContent?.trim();
      if (rawTarget) {
        e.preventDefault();
        e.stopPropagation();
        handleOpenResource(rawTarget);
      }
    }
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
  let isDraggingFiles = $state(false);
  let dragCounter = 0;

  function handleDragEnter(e: DragEvent) {
    e.preventDefault();
    dragCounter++;
    if (e.dataTransfer?.types?.includes("Files")) {
      isDraggingFiles = true;
    }
  }

  function handleDragOver(e: DragEvent) {
    e.preventDefault();
    if (e.dataTransfer) {
      e.dataTransfer.dropEffect = "copy";
    }
  }

  function handleDragLeave(e: DragEvent) {
    e.preventDefault();
    dragCounter--;
    if (dragCounter <= 0) {
      dragCounter = 0;
      isDraggingFiles = false;
    }
  }

  async function handleDrop(e: DragEvent) {
    e.preventDefault();
    dragCounter = 0;
    isDraggingFiles = false;
    if (e.dataTransfer?.files && e.dataTransfer.files.length > 0) {
      await composerRef?.handleDroppedFiles(e.dataTransfer.files);
    }
  }
</script>

<div
  class="flex-1 h-screen flex flex-col bg-app text-primary-theme overflow-hidden relative"
  ondragenter={handleDragEnter}
  ondragover={handleDragOver}
  ondragleave={handleDragLeave}
  ondrop={handleDrop}
  role="region"
  aria-label="Chat and context drop zone"
>
  <!-- Drag-and-drop file attachment overlay -->
  {#if isDraggingFiles}
    <div class="absolute inset-0 z-50 bg-app/80 backdrop-blur-sm flex flex-col items-center justify-center p-8 pointer-events-none transition-all duration-200">
      <div class="w-full max-w-lg p-10 rounded-2xl border-2 border-dashed border-accent-theme/80 bg-surface/95 shadow-2xl flex flex-col items-center justify-center text-center gap-3">
        <div class="w-14 h-14 rounded-2xl bg-accent-subtle flex items-center justify-center text-accent-theme animate-bounce">
          <Upload size={28} />
        </div>
        <h3 class="text-base font-semibold text-primary-theme">Drop files to attach to conversation</h3>
        <p class="text-xs text-secondary-theme max-w-sm">
          Drag and drop code files, documents, or screenshots to attach them as context.
        </p>
      </div>
    </div>
  {/if}
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
      <EngineSwitcher variant="compact-pill" onSwitch={onSwitchEngine} />
      {#if workspace}
        <span class="text-[11px] px-2 py-0.5 rounded-full bg-surface-elevated text-accent-theme font-mono border border-subtle shrink-0 hidden sm:inline-block">
          {workspace.name} &bull; {getEffectiveModel(workspace.model, activeEngine)}
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

      <!-- Prompt Turn Jump Controls -->
      {#if messages.some((m) => m.role === "user")}
        <div class="hidden md:flex items-center rounded-lg border border-subtle bg-surface overflow-hidden text-xs">
          <Tooltip text="Jump to Previous User Prompt" position="bottom">
            <button
              type="button"
              onclick={() => jumpUserPrompt(-1)}
              class="p-1.5 text-secondary-theme hover:text-primary-theme hover:bg-surface-hover transition-colors cursor-pointer"
              aria-label="Previous user prompt"
            >
              <ArrowUp size={13} />
            </button>
          </Tooltip>
          <div class="w-px h-4 bg-subtle"></div>
          <Tooltip text="Jump to Next User Prompt" position="bottom">
            <button
              type="button"
              onclick={() => jumpUserPrompt(1)}
              class="p-1.5 text-secondary-theme hover:text-primary-theme hover:bg-surface-hover transition-colors cursor-pointer"
              aria-label="Next user prompt"
            >
              <ArrowDown size={13} />
            </button>
          </Tooltip>
        </div>
      {/if}

      <!-- Context Window / Token Meter Gauge (Live ACP Telemetry + Fallback Estimate) -->
      {#if displayedUsedTokens > 0}
        <Tooltip
          text={`Context Usage: ${displayedUsedTokens.toLocaleString()} / ${displayedContextWindow.toLocaleString()} tokens (${(contextUsageRatio * 100).toFixed(1)}%)`}
          subtext={isNearContextCeiling
            ? "⚠️ Approaching context window ceiling. Consider running /compact or Export > Compress Context."
            : contextUsage && contextUsage.usedTokens > 0
              ? "Live token telemetry reported by active ACP runtime."
              : "Estimated cumulative tokens across active conversation turns."}
          position="bottom"
        >
          <div
            class="flex items-center gap-1.5 px-2.5 py-1.5 text-xs rounded-lg border font-mono font-medium shadow-xs select-none transition-colors {isNearContextCeiling ? 'bg-amber-500/15 border-amber-500/40 text-amber-300' : 'bg-surface hover:bg-surface-hover border-subtle text-secondary-theme'}"
          >
            <span class="inline-block w-2 h-2 rounded-full {isNearContextCeiling ? 'bg-amber-400 animate-pulse' : 'bg-accent-theme/70'}"></span>
            <span class="text-[11px] {isNearContextCeiling ? 'text-amber-300 font-bold' : 'text-primary-theme'}">
              {formatTokenCount(displayedUsedTokens)}
            </span>
            <span class="text-[10px] text-muted-theme">/ {formatTokenCount(displayedContextWindow)}</span>
          </div>
        </Tooltip>
      {/if}

      <!-- Export Menu -->
      <div class="relative">
        <Tooltip
          text="Export Conversation & Actions"
          subtext="Save this conversation thread or compress context"
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
        <div class="absolute right-0 mt-1.5 w-48 bg-surface-elevated border border-subtle rounded-lg shadow-xl py-1 z-40 text-xs">
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

          {#if messages.length >= 2}
            <div class="border-t border-subtle my-1"></div>
            <button
              onclick={() => {
                showExportMenu = false;
                onSendPrompt("/compress");
              }}
              disabled={isStreaming}
              class="w-full px-3 py-1.5 text-left hover:bg-surface-hover text-secondary-theme hover:text-accent-theme flex items-center justify-between cursor-pointer disabled:opacity-50"
            >
              <div class="flex items-center gap-2">
                <Minimize2 size={14} class="text-accent-theme" />
                <span>Compress Context</span>
              </div>
              <span class="text-[10px] text-muted-theme font-mono">/compress</span>
            </button>
          {/if}
        </div>
      {/if}
      </div>
    </div>
  </header>

  <!-- Messages Viewport -->
  <div bind:this={chatViewport} onscroll={handleScroll} class="flex-1 overflow-y-auto p-4 sm:p-6 relative z-0">
    {#if messages.length === 0 && !isStreaming}
      <div class="h-full flex flex-col items-center justify-center text-center max-w-xl lg:max-w-2xl xl:max-w-3xl mx-auto py-16 text-muted-theme">
        <div class="w-12 h-12 rounded-2xl bg-accent-subtle border border-accent-subtle flex items-center justify-center text-accent-theme mb-4 shadow-inner">
          <Sparkles size={24} />
        </div>
        <h3 class="text-base font-semibold text-primary-theme mb-1">What would you like to build?</h3>
        <p class="text-xs text-secondary-theme leading-relaxed mb-6">
          Start a conversation in the <span class="text-accent-theme font-medium">{workspace?.name || "active"}</span> workspace. {engineStore.getDescriptor(activeEngine).displayName} will automatically use this directory's files and context.
        </p>

        <!-- Quick Starter Action Cards -->
        <div class="grid grid-cols-1 sm:grid-cols-3 gap-2.5 w-full text-left">
          <button
            type="button"
            onclick={() => onSendPrompt("Explain the architecture, folder structure, and tech stack of this workspace")}
            class="p-3 rounded-xl bg-surface hover:bg-surface-hover border border-subtle hover:border-accent-theme/40 transition-all text-xs text-primary-theme group cursor-pointer shadow-xs"
          >
            <div class="font-semibold text-accent-theme mb-1 flex items-center gap-1.5">
              <span>🔍 Architecture</span>
            </div>
            <div class="text-[11px] text-muted-theme group-hover:text-secondary-theme leading-snug">
              Explain workspace structure & stack
            </div>
          </button>

          <button
            type="button"
            onclick={() => onSendPrompt("Inspect git status, summarize uncommitted changes, and identify what needs attention")}
            class="p-3 rounded-xl bg-surface hover:bg-surface-hover border border-subtle hover:border-amber-400/40 transition-all text-xs text-primary-theme group cursor-pointer shadow-xs"
          >
            <div class="font-semibold text-amber-400 mb-1 flex items-center gap-1.5">
              <span>⚡ Git Changes</span>
            </div>
            <div class="text-[11px] text-muted-theme group-hover:text-secondary-theme leading-snug">
              Inspect git status & uncommitted diffs
            </div>
          </button>

          <button
            type="button"
            onclick={() => onSendPrompt("/plan Create an implementation plan for my next feature or refactoring task")}
            class="p-3 rounded-xl bg-surface hover:bg-surface-hover border border-subtle hover:border-sky-400/40 transition-all text-xs text-primary-theme group cursor-pointer shadow-xs"
          >
            <div class="font-semibold text-sky-400 mb-1 flex items-center gap-1.5">
              <span>📝 Feature Plan</span>
            </div>
            <div class="text-[11px] text-muted-theme group-hover:text-secondary-theme leading-snug">
              Generate an interactive step-by-step plan
            </div>
          </button>
        </div>
      </div>
    {:else}
      <div class="w-full max-w-4xl lg:max-w-5xl xl:max-w-6xl 2xl:max-w-7xl mx-auto space-y-6">
        {#if hiddenMessageCount > 0}
          <div class="flex items-center justify-center gap-2 py-1">
            <button
              type="button"
              onclick={() => (visibleMessageCount += 60)}
              class="px-3 py-1 rounded-full bg-surface-elevated hover:bg-surface-hover border border-subtle text-[11px] text-secondary-theme hover:text-primary-theme transition-colors cursor-pointer"
            >
              Show earlier messages ({hiddenMessageCount} hidden)
            </button>
            <button
              type="button"
              onclick={() => (visibleMessageCount = messages.length)}
              class="px-2.5 py-1 rounded-full bg-surface hover:bg-surface-hover border border-subtle text-[11px] text-accent-theme transition-colors cursor-pointer"
            >
              Load all ({messages.length})
            </button>
          </div>
        {/if}

        {#each visibleMessages as msg, i (msg.id)}
          {@const absIdx = msgWindowOffset + i}
          <div
            data-user-turn-idx={msg.role === "user" ? absIdx : undefined}
            class="flex gap-3.5 {msg.role === 'user' ? 'justify-end' : 'justify-start'}"
          >
            {#if msg.role !== 'user'}
              <div class="w-7 h-7 rounded-lg bg-accent-subtle text-accent-theme border border-accent-subtle flex items-center justify-center shrink-0 mt-1">
                <Sparkles size={14} />
              </div>
            {/if}

            <div class="group {msg.role === 'user' ? 'max-w-xl sm:max-w-2xl lg:max-w-3xl bg-bubble-user border border-bubble-user text-primary-theme rounded-2xl rounded-tr-sm px-4 py-2.5 text-xs shadow-xs' : 'bg-bubble-assistant border border-bubble-assistant rounded-2xl rounded-tl-sm px-5 py-4 text-xs text-primary-theme shadow-xs w-full min-w-0'}">
          {#if msg.role === 'user'}
            <div class="whitespace-pre-wrap leading-relaxed select-text">{msg.content}</div>
            <div class="mt-1.5 flex items-center justify-end gap-1.5 text-[10px] text-muted-theme select-none opacity-0 group-hover:opacity-100 transition-opacity">
              {#if msg.created_at}
                <span class="mr-1">{formatTime(msg.created_at)}</span>
              {/if}
              <Tooltip text="Quote in reply" position="top">
                <button
                  type="button"
                  onclick={() => handleQuoteSelectionOrText(msg.content)}
                  class="p-1 rounded hover:bg-surface-elevated text-secondary-theme hover:text-primary-theme transition-colors cursor-pointer flex items-center gap-1"
                  aria-label="Quote prompt"
                >
                  <Quote size={11} />
                </button>
              </Tooltip>
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
            {@const turnDurationMs = getTurnDurationMs(absIdx, msg)}
            {@const msgThought = messageThoughts?.[msg.id]}

            {#if msgThought && msgThought.text}
              <div class="mb-3 rounded-lg border border-subtle/80 bg-surface/60 overflow-hidden text-xs">
                <button
                  type="button"
                  onclick={() => {
                    expandedThinkingMap = { ...expandedThinkingMap, [msg.id]: !expandedThinkingMap[msg.id] };
                  }}
                  class="w-full px-3 py-1.5 flex items-center gap-2 text-left text-[11px] text-secondary-theme hover:text-primary-theme hover:bg-surface-hover/60 transition-colors cursor-pointer"
                >
                  <ChevronRight size={13} class="transition-transform {expandedThinkingMap[msg.id] ? 'rotate-90' : ''}" />
                  <Brain size={12} class="text-purple-400" />
                  <span class="font-medium">
                    Thought{msgThought.durationMs ? ` for ${formatDuration(msgThought.durationMs)}` : ""}
                  </span>
                </button>
                {#if expandedThinkingMap[msg.id]}
                  <div class="px-3 pb-2.5 pt-1 border-t border-subtle/60 text-[11px] text-muted-theme whitespace-pre-wrap leading-relaxed max-h-60 overflow-y-auto">
                    {msgThought.text}
                  </div>
                {/if}
              </div>
            {/if}

            {#if toolSteps.length > 0}
              <div class="mb-3">
                <ToolCallGroup steps={toolSteps} isStreaming={false} {turnDurationMs} />
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
                {#if turnDurationMs && turnDurationMs > 0}
                  <span>&bull; {formatDuration(turnDurationMs)}</span>
                {/if}
                {#if msg.token_count > 0}
                  <span>&bull; ~{msg.token_count} tokens</span>
                {/if}
              </div>
              <div class="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                <Tooltip text="Quote selection or message into composer" position="top">
                  <button
                    type="button"
                    onclick={() => handleQuoteSelectionOrText(cleanContent || msg.content)}
                    class="p-1 rounded hover:bg-surface-hover text-secondary-theme hover:text-primary-theme transition-colors cursor-pointer flex items-center gap-1 text-[11px]"
                    aria-label="Quote response"
                  >
                    <Quote size={13} />
                    <span>Quote</span>
                  </button>
                </Tooltip>
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
                {#if onForkSession}
                  <Tooltip text="Fork conversation into new session from this turn" position="top">
                    <button
                      type="button"
                      onclick={() => onForkSession && onForkSession(msg.id)}
                      class="p-1 rounded hover:bg-surface-hover text-secondary-theme hover:text-sky-400 transition-colors cursor-pointer flex items-center gap-1 text-[11px]"
                      aria-label="Fork session from this turn"
                    >
                      <GitFork size={13} />
                      <span>Fork</span>
                    </button>
                  </Tooltip>
                {/if}
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

                {#if !messages.slice(absIdx + 1).some(m => m.role === 'assistant') && !isStreaming}
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
        <div class="w-full min-w-0 bg-bubble-assistant border border-bubble-assistant rounded-2xl rounded-tl-sm px-5 py-4 text-xs text-primary-theme shadow-xs">
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

          {#if streamingThoughtText}
            {@const liveExpanded = expandedThinkingMap["__live__"] ?? false}
            <div class="mb-3 rounded-lg border border-purple-500/30 bg-purple-500/5 overflow-hidden text-xs">
              <button
                type="button"
                onclick={() => {
                  expandedThinkingMap = { ...expandedThinkingMap, __live__: !liveExpanded };
                }}
                class="w-full px-3 py-1.5 flex items-center justify-between gap-2 text-left text-[11px] text-purple-300 hover:bg-purple-500/10 transition-colors cursor-pointer"
              >
                <div class="flex items-center gap-2">
                  <ChevronRight size={13} class="transition-transform {liveExpanded ? 'rotate-90' : ''}" />
                  <Brain size={12} class="text-purple-400 animate-pulse" />
                  <span class="font-medium">Reasoning / Thinking...</span>
                </div>
                {#if streamingThinkingStartedAt}
                  <span class="font-mono text-[10px] opacity-80">live</span>
                {/if}
              </button>
              {#if liveExpanded}
                <div class="px-3 pb-2.5 pt-1 border-t border-purple-500/20 text-[11px] text-secondary-theme whitespace-pre-wrap leading-relaxed max-h-60 overflow-y-auto">
                  {streamingThoughtText}
                </div>
              {/if}
            </div>
          {/if}

          {#if streamingToolCalls && streamingToolCalls.length > 0}
            <div class="mb-3">
              <ToolCallGroup steps={streamingToolCalls} isStreaming={true} {streamingStartedAt} />
            </div>
          {/if}

          {#if streamingText}
            <div class="markdown-body">
              {@html renderMarkdown(streamingText)}
            </div>
          {:else if (!streamingToolCalls || streamingToolCalls.length === 0) && !streamingThoughtText}
            <div class="flex items-center gap-2 text-muted-theme italic">
              <span class="inline-block w-2 h-2 rounded-full bg-accent-theme animate-ping"></span>
              Thinking...
            </div>
          {/if}
        </div>
      </div>
    {/if}
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
    <div class="px-4 sm:px-6">
      <ToolPermissionCard {toolPermission} pendingCount={pendingPermissionCount} onRespond={onToolResponse} />
    </div>
  {/if}

  <!-- Instant Debug Doctor Diagnostic Banner -->
  {#if activeDebugError}
    <div class="px-4 sm:px-6 pt-2">
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
    {activeEngine}
    {dynamicCommands}
    {queuedPrompts}
    {onQueuePrompt}
    {onRemoveQueuedPrompt}
    {onTriggerDebugDoctor}
    {onDebugErrorDetected}
    {onSteerPrompt}
    onOpenDiffModal={() => (isChangesModalOpen = true)}
    onToggleTerminal={() => (showTerminalDrawer = !showTerminalDrawer)}
    onTogglePlanWorkbench={() => (isPlanWorkbenchOpen = true)}
    onOpenMcpModal={onOpenMcpModal}
    onOpenRewindModal={onOpenRewindModal}
    onSendPrompt={(prompt, atts) => {
      isUserScrolledUp = false;
      tick().then(() => scrollToBottom(true));
      onSendPrompt(prompt, atts);
    }}
    onCancelPrompt={() => {
      wasExecutingPlan = false;
      onCancelPrompt();
    }}
  />

  <!-- Terminal Drawer -->
  <TerminalDrawer bind:isOpen={showTerminalDrawer} {workspace} {activeEngine} />

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
