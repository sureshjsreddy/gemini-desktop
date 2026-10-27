<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import type {
    Workspace,
    Session,
    Message,
    PromptTemplate,
    SearchResult,
    ToolPermissionPayload,
    GeminiEnvStatus,
    WorkspaceFileEntry,
    UpdateInfo,
    ApprovalMode,
    SessionStreamState,
    ParsedDebugError,
    FileSnippetResult,
    ToolCallPayload,
    PlanTaskItem,
  } from "$lib/types";
  import { parseDebugError } from "$lib/utils/debugParser";
  import { notifyBackgroundTaskComplete, setupNotificationActionListener } from "$lib/utils/notifications";
  import Sidebar from "$lib/components/Sidebar.svelte";
  import ChatView from "$lib/components/ChatView.svelte";
  import SolutionExplorer from "$lib/components/SolutionExplorer.svelte";
  import SearchModal from "$lib/components/SearchModal.svelte";
  import WorkspaceModal from "$lib/components/WorkspaceModal.svelte";
  import TemplatesModal from "$lib/components/TemplatesModal.svelte";
  import ThemeModal from "$lib/components/ThemeModal.svelte";
  import McpModal from "$lib/components/McpModal.svelte";
  import CliSetupModal from "$lib/components/CliSetupModal.svelte";
  import RewindModal from "$lib/components/RewindModal.svelte";
  import UpdateNotificationBanner from "$lib/components/UpdateNotificationBanner.svelte";
  import { themeManager } from "$lib/theme.svelte";
  import { dialogManager } from "$lib/dialog.svelte";
  import { modalManager } from "$lib/modal.svelte";
  import { getAppVersion, APP_VERSION_FALLBACK } from "$lib/version";

  const LAST_WORKSPACE_KEY = "gemini_desktop_last_workspace_id";

  // Reactive State (Svelte 5 Runes)
  let appVersion = $state(APP_VERSION_FALLBACK);
  let workspaces = $state<Workspace[]>([]);
  let activeWorkspace = $state<Workspace | null>(null);
  let sessions = $state<Session[]>([]);
  let activeSessionId = $state<string | null>(null);
  let activeSession = $derived<Session | null>(
    sessions.find((s) => s.id === activeSessionId) ?? null
  );
  let messages = $state<Message[]>([]);
  let pendingUserMessages = $state<Record<string, Message>>({});
  let promptTemplates = $state<PromptTemplate[]>([]);
  let workspaceFiles = $state<WorkspaceFileEntry[]>([]);
  let updateInfo = $state<UpdateInfo | null>(null);
  let isCheckingUpdate = $state(false);
  let approvalMode: ApprovalMode = $derived(activeWorkspace?.approval_mode || "default");

  let streamStates = $state<Record<string, SessionStreamState>>({});

  let activeStreamState = $derived.by<SessionStreamState>(() => {
    const sid = activeSessionId;
    if (sid && streamStates[sid]) {
      return streamStates[sid];
    }
    return { streamingText: "", isStreaming: false, toolPermission: null };
  });

  let isStreaming = $derived(activeStreamState.isStreaming);
  let streamingText = $derived(activeStreamState.streamingText);
  let toolPermission = $derived(activeStreamState.toolPermission);
  let streamingToolCalls = $derived(activeStreamState.toolCalls || []);

  let generatingSessionIds = $derived(
    new Set(
      Object.entries(streamStates)
        .filter(([_, s]) => s.isStreaming)
        .map(([id]) => id)
    )
  );

  let envStatus: GeminiEnvStatus | null = $state(null);

  const SIDEBAR_OPEN_KEY = "gemini_sidebar_open";
  const EXPLORER_OPEN_KEY = "gemini_explorer_open";

  function getInitialSidebarOpen(): boolean {
    if (typeof window === "undefined") return false;
    try {
      const saved = localStorage.getItem(SIDEBAR_OPEN_KEY);
      return saved !== null ? saved === "true" : false;
    } catch {
      return false;
    }
  }

  function getInitialExplorerOpen(): boolean {
    if (typeof window === "undefined") return false;
    try {
      const saved = localStorage.getItem(EXPLORER_OPEN_KEY);
      return saved !== null ? saved === "true" : false;
    } catch {
      return false;
    }
  }

  // Panels & Drawers (default to collapsed; state persisted across app restarts)
  let showTerminalDrawer = $state(false);
  let showSolutionExplorer = $state(getInitialExplorerOpen());
  let showSidebar = $state(getInitialSidebarOpen());

  $effect(() => {
    if (typeof window !== "undefined") {
      try {
        localStorage.setItem(SIDEBAR_OPEN_KEY, String(showSidebar));
      } catch (e) {
        console.warn("Failed to persist sidebar state:", e);
      }
    }
  });

  $effect(() => {
    if (typeof window !== "undefined") {
      try {
        localStorage.setItem(EXPLORER_OPEN_KEY, String(showSolutionExplorer));
      } catch (e) {
        console.warn("Failed to persist explorer state:", e);
      }
    }
  });

  let chatViewRef = $state<ReturnType<typeof ChatView> | null>(null);
  let solutionExplorerRef = $state<ReturnType<typeof SolutionExplorer> | null>(null);

  let unlistenChunk: UnlistenFn | null = null;
  let unlistenTool: UnlistenFn | null = null;
  let unlistenToolCall: UnlistenFn | null = null;
  let unlistenError: UnlistenFn | null = null;
  let unlistenModeUpdate: UnlistenFn | null = null;
  let unlistenStderr: UnlistenFn | null = null;
  let unlistenDebugDoctor: UnlistenFn | null = null;
  let unlistenSteer: UnlistenFn | null = null;
  let unlistenRewind: UnlistenFn | null = null;

  // Instant Debug Doctor State
  let activeDebugError = $state<ParsedDebugError | null>(null);
  let activeFileSnippet = $state<FileSnippetResult | null>(null);
  let dismissedErrorSignature = $state<string | null>(null);
  let rewindTargetMessageId = $state<string | null>(null);

  onMount(async () => {
    // 0. Initialize theme & dynamic app version
    themeManager.init();
    setupNotificationActionListener();
    getAppVersion().then((ver) => {
      if (ver) appVersion = ver;
    });

    // 1. Check Gemini environment status
    try {
      envStatus = await invoke<GeminiEnvStatus>("check_gemini_env");
    } catch (e) {
      console.warn("Could not check gemini environment:", e);
    }

    // 2. Load Workspaces
    try {
      workspaces = await invoke<Workspace[]>("get_workspaces");
      if (workspaces.length > 0) {
        let initialWs = workspaces[0];
        if (typeof window !== "undefined") {
          const lastWorkspaceId = localStorage.getItem(LAST_WORKSPACE_KEY);
          if (lastWorkspaceId) {
            const matched = workspaces.find((w) => w.id === lastWorkspaceId);
            if (matched) {
              initialWs = matched;
            }
          }
        }
        await selectWorkspace(initialWs);
      }
    } catch (e) {
      console.error("Failed to load workspaces:", e);
    }

    // 3. Load Templates
    try {
      promptTemplates = await invoke<PromptTemplate[]>("get_prompts");
    } catch (e) {
      console.error("Failed to load prompt templates:", e);
    }

    // 4. Listen to Tauri backend ACP events
    unlistenChunk = await listen<{ session_id: string; delta: string; is_done: boolean }>(
      "acp-chunk",
      (event) => {
        const { session_id, delta, is_done } = event.payload;
        if (!session_id) return;

        if (!streamStates[session_id]) {
          streamStates[session_id] = {
            streamingText: "",
            isStreaming: true,
            toolPermission: null,
          };
        }

        // When a turn is canceled due to mid-flight steering, ignore the cancellation completion event
        if (streamStates[session_id].isSteering && is_done) {
          streamStates[session_id].isSteering = false;
          streamStates[session_id].isStreaming = true;
          return;
        }

        let cleanDelta = delta || "";
        if (cleanDelta.includes("*(Prompt cancelled)*") && (streamStates[session_id]?.steeringHints?.length || streamStates[session_id]?.isSteering)) {
          cleanDelta = "";
        }
        if (cleanDelta.includes("[MODE_UPDATE]")) {
          cleanDelta = cleanDelta.replace(/\[MODE_UPDATE\]\s*[a-zA-Z0-9_]*/g, "");
        }

        if (cleanDelta) {
          const current = streamStates[session_id].streamingText;
          if (!current) {
            streamStates[session_id].streamingText = cleanDelta;
          } else if (cleanDelta.startsWith(current)) {
            // Full replacement if cleanDelta is the full accumulated response
            streamStates[session_id].streamingText = cleanDelta;
          } else if (!current.endsWith(cleanDelta)) {
            streamStates[session_id].streamingText += cleanDelta;
          }
        }
        streamStates[session_id].isStreaming = !is_done;

        if (is_done) {
          finishStreamingForSession(session_id);
        }
      }
    );

    unlistenModeUpdate = await listen<{ sessionId?: string; mode?: string }>(
      "acp-mode-update",
      (event) => {
        const mode = event.payload?.mode;
        if (mode && activeWorkspace) {
          const normalizedMode: ApprovalMode =
            mode === "autoEdit" ? "auto_edit" :
            mode === "yolo" ? "yolo" :
            mode === "plan" ? "plan" : "default";
          if (activeWorkspace.approval_mode !== normalizedMode) {
            activeWorkspace = { ...activeWorkspace, approval_mode: normalizedMode };
          }
        }
      }
    );

    unlistenTool = await listen<ToolPermissionPayload>(
      "acp-tool-permission",
      async (event) => {
        const payload = event.payload;
        const sessionId = payload.session_id;

        // Resolve effective approval mode specifically for the target session's workspace
        const targetSession = sessions.find((s) => s.id === sessionId);
        const targetWorkspace = targetSession
          ? workspaces.find((w) => w.id === targetSession.workspace_id)
          : activeWorkspace;
        const targetApprovalMode: ApprovalMode = targetWorkspace?.approval_mode || approvalMode || "default";

        // 1. YOLO Mode: Auto-approve all tools immediately without showing permission prompt
        if (targetApprovalMode === "yolo") {
          try {
            await invoke("respond_tool_permission", {
              requestId: payload.request_id,
              allowed: true,
            });
          } catch (e) {
            console.error("Failed to auto-approve tool in YOLO mode:", e);
          }
          return;
        }

        // 2. Auto-Edit Mode: Auto-approve file edits and safe inspection tools (reads, searches)
        if (targetApprovalMode === "auto_edit") {
          const kind = (payload.kind || "").toLowerCase();
          const name = (payload.tool_name || "").toLowerCase();
          const isSafe =
            kind === "edit" ||
            kind === "read" ||
            kind === "search" ||
            kind === "think" ||
            kind === "fetch" ||
            name.includes("write") ||
            name.includes("edit") ||
            name.includes("replace") ||
            name.includes("patch") ||
            name.includes("create") ||
            name.includes("read") ||
            name.includes("search") ||
            name.includes("grep") ||
            name.includes("view") ||
            name.includes("list") ||
            name.includes("glob") ||
            name.includes("find");

          if (isSafe) {
            try {
              await invoke("respond_tool_permission", {
                requestId: payload.request_id,
                allowed: true,
              });
            } catch (e) {
              console.error("Failed to auto-approve safe tool in auto_edit mode:", e);
            }
            return;
          }
        }

        // 3. Ask Mode (default) or unsafe terminal command in Auto-Edit mode: Present interactive confirmation UI for the target session
        if (sessionId) {
          if (!streamStates[sessionId]) {
            streamStates[sessionId] = {
              streamingText: "",
              isStreaming: true,
              toolPermission: null,
            };
          }
          streamStates[sessionId].toolPermission = payload;
          notifyBackgroundTaskComplete(
            "Action Required",
            `Gemini is requesting permission for: ${payload.tool_name || "Tool execution"}`
          );
        }
      }
    );

    unlistenToolCall = await listen<ToolCallPayload>(
      "acp-tool-call",
      (event) => {
        const payload = event.payload;
        const sessionId = payload.session_id;
        if (!sessionId) return;

        if (!streamStates[sessionId]) {
          streamStates[sessionId] = {
            streamingText: "",
            isStreaming: true,
            toolPermission: null,
            toolCalls: [],
          };
        }

        if (!streamStates[sessionId].toolCalls) {
          streamStates[sessionId].toolCalls = [];
        }

        const list = streamStates[sessionId].toolCalls!;
        const existingIdx = list.findIndex(
          (t) => t.id === payload.tool_call_id || (payload.tool_call_id !== "tool" && t.id === payload.tool_call_id)
        );

        const now = Date.now();
        if (existingIdx >= 0) {
          const prev = list[existingIdx];
          const duration = prev.created_at ? now - new Date(prev.created_at).getTime() : undefined;
          list[existingIdx] = {
            ...prev,
            status: payload.status as any,
            title: payload.title || prev.title,
            tool_name: payload.tool_name || prev.tool_name,
            summary: payload.summary || prev.summary,
            error: payload.error || prev.error,
            raw_output: payload.raw_output || prev.raw_output,
            parameters: payload.parameters || prev.parameters,
            duration_ms: duration ?? prev.duration_ms,
          };
        } else {
          list.push({
            id: payload.tool_call_id || `tool-${Date.now()}-${Math.random().toString(36).slice(2)}`,
            title: payload.title || payload.tool_name || "Task Step",
            tool_name: payload.tool_name || payload.title || "tool",
            summary: payload.summary,
            status: payload.status as any,
            error: payload.error,
            raw_output: payload.raw_output,
            parameters: payload.parameters,
            created_at: new Date(now).toISOString(),
          });
        }
        streamStates[sessionId].toolCalls = [...list];
        streamStates = { ...streamStates };
      }
    );

    unlistenError = await listen<any>("acp-error", (event) => {
      const payloadStr = JSON.stringify(event.payload || "");
      const targetSid =
        event.payload?.sessionId ||
        event.payload?.session_id ||
        (activeSessionId && streamStates[activeSessionId]?.isStreaming
          ? activeSessionId
          : Object.keys(streamStates).find((id) => streamStates[id]?.isStreaming)) ||
        activeSessionId;

      if (payloadStr.toLowerCase().includes("cancel")) {
        console.warn("Ignored cancellation signal:", event.payload);
        if (targetSid) {
          finishStreamingForSession(targetSid);
        }
        return;
      }
      console.error("ACP Error:", event.payload);
      if (targetSid) {
        if (streamStates[targetSid]) {
          streamStates[targetSid].streamingText += `\n\n**Error:** ${payloadStr}`;
        }
        finishStreamingForSession(targetSid);
      }
    });

    unlistenStderr = await listen<string>("acp-stderr", (event) => {
      const line = event.payload;
      if (!line) return;
      if (
        line.includes("Retrying with backoff") ||
        line.includes("503") ||
        line.includes("429") ||
        line.includes("high demand")
      ) {
        const sid =
          (activeSessionId && streamStates[activeSessionId]?.isStreaming
            ? activeSessionId
            : Object.keys(streamStates).find((id) => streamStates[id]?.isStreaming)) ||
          activeSessionId;
        if (sid && streamStates[sid] && streamStates[sid].isStreaming) {
          if (!streamStates[sid].streamingText.includes("high demand, retrying")) {
            streamStates[sid].streamingText += "\n\n> ⏳ *Model is currently experiencing high demand, retrying with backoff...*\n\n";
          }
        }
      }
    });

    // 4.5. Instant Debug Doctor (Global OS Hotkey Ctrl+Alt+D & Window Focus)
    try {
      unlistenDebugDoctor = await listen("instant-debug-doctor", () => {
        checkClipboardForDebugError(true);
      });
    } catch (e) {
      console.warn("Could not setup instant-debug-doctor event listener:", e);
    }
    window.addEventListener("focus", handleWindowFocus);

    window.addEventListener("keydown", handleGlobalShortcuts);
    window.addEventListener("message", handleVsMessage);

    // 5. Check for updates on WinGet in the background (3s delay)
    setTimeout(() => {
      handleCheckUpdate(false);
    }, 3000);
  });

  function handleWindowFocus() {
    checkClipboardForDebugError(false);
  }

  async function checkClipboardForDebugError(force = false) {
    let clipText = "";
    try {
      clipText = await invoke<string>("get_clipboard_text");
    } catch {
      try {
        if (typeof navigator !== "undefined" && navigator.clipboard) {
          clipText = await navigator.clipboard.readText();
        }
      } catch {}
    }

    if (!clipText || !clipText.trim()) {
      if (force) {
        await dialogManager.alert(
          "No debug exception or stack trace detected on the clipboard.\n\nIn Visual Studio, click 'Copy details' on the Exception Helper dialog (or copy a stack trace from the Output window), then click Debug Doctor or press Ctrl+Alt+D.",
          "Debug Doctor"
        );
      }
      return;
    }

    const parsed = parseDebugError(clipText);
    if (!parsed) {
      if (force) {
        await dialogManager.alert(
          "The clipboard does not contain a recognized exception or stack trace.\n\nIn Visual Studio, click 'Copy details' on the Exception Helper dialog to copy the structured exception payload.",
          "Debug Doctor"
        );
      }
      return;
    }

    const signature = `${parsed.exceptionType}:${parsed.filePath || ""}:${parsed.lineNumber || ""}`;
    if (!force && signature === dismissedErrorSignature) {
      return;
    }

    await loadAndDisplayDebugError(parsed);
  }

  async function loadAndDisplayDebugError(parsed: ParsedDebugError) {
    activeDebugError = parsed;
    activeFileSnippet = null;

    if (parsed.filePath && parsed.lineNumber) {
      try {
        const snippet = await invoke<FileSnippetResult>("read_file_snippet", {
          filePath: parsed.filePath,
          targetLine: parsed.lineNumber,
          contextLines: 15,
          workspacePath: activeWorkspace?.path,
        });
        activeFileSnippet = snippet;
      } catch (e) {
        console.warn("Failed to load source snippet for debug exception:", e);
      }
    }
  }

  function handleDiagnoseDebugError(prompt: string) {
    activeDebugError = null;
    activeFileSnippet = null;
    handleSendPrompt(prompt);
  }

  function handlePrefillDebugError(prompt: string) {
    activeDebugError = null;
    activeFileSnippet = null;
    window.dispatchEvent(new CustomEvent("gemini-prefill-prompt", { detail: prompt }));
  }

  function handleDismissDebugError() {
    if (activeDebugError) {
      dismissedErrorSignature = `${activeDebugError.exceptionType}:${activeDebugError.filePath || ""}:${activeDebugError.lineNumber || ""}`;
    }
    activeDebugError = null;
    activeFileSnippet = null;
  }

  function handleVsMessage(event: MessageEvent) {
    const msg = event?.data;
    if (!msg || msg.type !== "event") return;
    if (msg.event === "vs-context-action" && msg.payload) {
      const payload = msg.payload;
      if (payload.action === "auto_prompt" && payload.prompt) {
        handleSendPrompt(payload.prompt);
      } else if (payload.action === "selection_context" && payload.context) {
        const ctx = payload.context;
        const note = ctx.SelectedText
          ? `Selected code from ${ctx.FileName} (lines ${ctx.StartLine}-${ctx.EndLine}):\n\`\`\`${ctx.Language || ""}\n${ctx.SelectedText}\n\`\`\`\n`
          : `Active file: ${ctx.FileName}\n`;
        window.dispatchEvent(new CustomEvent("gemini-prefill-prompt", { detail: note }));
      }
    }
  }

  async function handleCheckUpdate(manual = false) {
    if (isCheckingUpdate) return;
    isCheckingUpdate = true;
    try {
      const info = await invoke<UpdateInfo>("check_app_update");
      if (info && info.update_available) {
        const dismissed = localStorage.getItem("gemini_dismissed_update_version");
        if (manual || dismissed !== info.latest_version) {
          updateInfo = info;
        }
      } else if (manual) {
        await dialogManager.alert(
          `Gemini Desktop v${info?.current_version || appVersion} is already up to date with the latest WinGet release!`,
          "Up to Date"
        );
      }
    } catch (e) {
      console.warn("Update check failed:", e);
      if (manual) {
        await dialogManager.alert(`Could not check for updates: ${e}`, "Check Failed");
      }
    } finally {
      isCheckingUpdate = false;
    }
  }

  onDestroy(() => {
    if (unlistenChunk) unlistenChunk();
    if (unlistenTool) unlistenTool();
    if (unlistenToolCall) unlistenToolCall();
    if (unlistenError) unlistenError();
    if (unlistenModeUpdate) unlistenModeUpdate();
    if (unlistenStderr) unlistenStderr();
    if (unlistenDebugDoctor) unlistenDebugDoctor();
    if (unlistenSteer) (unlistenSteer as any)();
    if (unlistenRewind) (unlistenRewind as any)();
    window.removeEventListener("focus", handleWindowFocus);
    window.removeEventListener("keydown", handleGlobalShortcuts);
    window.removeEventListener("message", handleVsMessage);
  });

  function handleGlobalShortcuts(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key === "n") {
      e.preventDefault();
      handleNewSession();
    } else if ((e.ctrlKey || e.metaKey) && e.key === "k") {
      e.preventDefault();
      modalManager.toggle("search");
    } else if ((e.ctrlKey || e.metaKey) && e.key === "m") {
      e.preventDefault();
      modalManager.toggle("mcp");
    } else if ((e.ctrlKey || e.metaKey) && (e.key === "`" || e.key === "~")) {
      e.preventDefault();
      showTerminalDrawer = !showTerminalDrawer;
    } else if ((e.ctrlKey || e.metaKey) && (e.key === "b" || e.key === "B")) {
      e.preventDefault();
      showSidebar = !showSidebar;
    } else if ((e.ctrlKey || e.metaKey) && e.altKey && (e.key === "l" || e.key === "L")) {
      e.preventDefault();
      showSolutionExplorer = !showSolutionExplorer;
    } else if ((e.ctrlKey || e.metaKey) && e.altKey && (e.key === "d" || e.key === "D")) {
      e.preventDefault();
      checkClipboardForDebugError(true);
    } else if ((e.ctrlKey || e.metaKey) && e.key === ";") {
      e.preventDefault();
      if (!showSolutionExplorer) showSolutionExplorer = true;
      setTimeout(() => {
        solutionExplorerRef?.focusSearch();
      }, 50);
    }
  }

  async function loadWorkspaceFiles(wsId: string) {
    try {
      workspaceFiles = await invoke<WorkspaceFileEntry[]>("list_workspace_files", { workspaceId: wsId });
    } catch (e) {
      console.warn("Failed to list workspace files:", e);
      workspaceFiles = [];
    }
  }

  async function refreshWorkspaceFiles() {
    if (activeWorkspace) {
      await loadWorkspaceFiles(activeWorkspace.id);
    }
  }

  async function loadSessionsForWorkspace(wsId: string) {
    const loaded = await invoke<Session[]>("get_sessions", { workspaceId: wsId });
    sessions = loaded;
    if (loaded.length > 0) {
      await selectSession(loaded[0]);
    } else {
      activeSessionId = null;
      messages = [];
    }
  }

  async function selectWorkspace(ws: Workspace) {
    activeWorkspace = ws;
    if (typeof window !== "undefined") {
      try {
        localStorage.setItem(LAST_WORKSPACE_KEY, ws.id);
      } catch (e) {
        console.warn("Failed to persist last workspace id:", e);
      }
    }
    await loadSessionsForWorkspace(ws.id);
    await loadWorkspaceFiles(ws.id);
  }

  async function selectSession(session: Session) {
    const targetId = session.id;
    activeSessionId = targetId;
    const loadedMessages = await invoke<Message[]>("get_session_messages", { sessionId: targetId });
    if (activeSessionId === targetId) {
      const pending = pendingUserMessages[targetId];
      if (pending && !loadedMessages.some((m) => m.role === "user" && m.content === pending.content)) {
        messages = [...loadedMessages, pending];
      } else {
        messages = loadedMessages;
      }
    }
  }

  async function handleNewSession(): Promise<Session | null> {
    if (!activeWorkspace) return null;
    const newSession = await invoke<Session>("create_session", {
      workspaceId: activeWorkspace.id,
      title: "New Conversation",
    });
    sessions = [newSession, ...sessions];
    activeSessionId = newSession.id;
    messages = [];
    return sessions.find((s) => s.id === newSession.id) ?? newSession;
  }

  async function handleRenameSession(session: Session) {
    const currentSession = sessions.find((s) => s.id === session.id) ?? session;
    const newTitle = await dialogManager.prompt("Enter new title for conversation:", currentSession.title, {
      title: "Rename Conversation",
      placeholder: "New conversation title...",
    });
    if (!newTitle || !newTitle.trim() || newTitle.trim() === currentSession.title) return;
    const trimmed = newTitle.trim();
    const now = new Date().toISOString();
    sessions = sessions.map((s) =>
      s.id === session.id ? { ...s, title: trimmed, updated_at: now } : s
    );
    await invoke("rename_session", { sessionId: session.id, title: trimmed });
  }

  async function handleDeleteSession(session: Session) {
    const currentSession = sessions.find((s) => s.id === session.id) ?? session;
    const confirmed = await dialogManager.confirm(`Delete conversation "${currentSession.title}"? This cannot be undone.`, {
      title: "Delete Conversation",
      confirmText: "Delete",
      isDestructive: true,
    });
    if (!confirmed) return;
    await invoke("delete_session", { sessionId: session.id });
    delete pendingUserMessages[session.id];
    try {
      localStorage.removeItem(`gemini_plan_tasks_${session.id}`);
    } catch {}
    sessions = sessions.filter((s) => s.id !== session.id);
    if (activeSessionId === session.id) {
      if (sessions.length > 0) {
        await selectSession(sessions[0]);
      } else {
        activeSessionId = null;
        messages = [];
      }
    }
  }

  async function handleSteerPrompt(hint: string) {
    if (!activeSessionId) return;
    const sid = activeSessionId;
    const stream = streamStates[sid];
    if (stream) {
      const current = stream.steeringHints || [];
      stream.steeringHints = [...current, hint];
      stream.isSteering = true;
      stream.streamingText += `\n\n---\n> 🎯 **Steering Injected:** *"${hint}"*\n\n`;
      streamStates = { ...streamStates };
    }
    try {
      await invoke("steer_prompt", { sessionId: sid, hint });
    } catch (e: any) {
      console.error("Failed to send steering hint:", e);
      if (streamStates[sid]) {
        streamStates[sid].isSteering = false;
        streamStates = { ...streamStates };
      }
    }
  }

  async function handleRewound(
    targetMessageId: string,
    rewoundCount: number,
    restoredFiles: string[],
    prevPrompt?: string | null
  ) {
    if (!activeSessionId) return;
    const loaded = await invoke<Message[]>("get_session_messages", { sessionId: activeSessionId });
    messages = loaded;

    if (prevPrompt) {
      window.dispatchEvent(new CustomEvent("gemini-prefill-prompt", { detail: prevPrompt }));
    }
    if (restoredFiles.length > 0) {
      await refreshWorkspaceFiles();
    }
    await dialogManager.alert(
      `Rewound ${rewoundCount} message(s)${restoredFiles.length > 0 ? ` and restored ${restoredFiles.length} file(s) in workspace.` : "."}`,
      "Turn Rewound"
    );
  }

  async function handleExecutePlan(targetMode: "auto_edit" | "default", tasks?: PlanTaskItem[]) {
    if (!activeWorkspace) return;
    const updatedWs: Workspace = {
      ...activeWorkspace,
      approval_mode: targetMode,
    };
    await handleSaveWorkspace(updatedWs);

    if (!tasks || tasks.length === 0) {
      await dialogManager.alert("Please add at least one task to the plan before executing.", "No Tasks in Plan");
      return;
    }

    const pendingTasks = tasks.filter((t) => !t.completed);
    const completedTasks = tasks.filter((t) => t.completed);

    let prompt = `Plan approved. Proceed with implementation of the approved tasks in ${targetMode === "auto_edit" ? "Auto-Edit" : "Ask"} mode:\n\n`;
    if (pendingTasks.length > 0) {
      prompt += `Tasks to implement:\n${pendingTasks.map((t) => `- [ ] ${t.text}`).join("\n")}\n\n`;
    }
    if (completedTasks.length > 0) {
      prompt += `Already completed / skip:\n${completedTasks.map((t) => `- [x] ${t.text}`).join("\n")}\n\n`;
    }
    prompt += `Please begin implementing the pending tasks step-by-step.`;

    await handleSendPrompt(prompt);
  }

  async function handleSendPrompt(prompt: string) {
    if (!activeWorkspace) return;

    let targetSession = activeSession;
    if (!targetSession) {
      targetSession = await handleNewSession();
    }
    if (!targetSession) return;

    const targetSessionId = targetSession.id;
    const targetWorkspaceId = activeWorkspace.id;
    const targetModel = activeWorkspace.model;
    const now = new Date().toISOString();

    // Synchronously update session title in both Sidebar (sessions) and Header (derived activeSession)
    // before any async IPC call so switching chats during streaming never reverts the name.
    let summaryToPersist: string | null = null;
    if (
      !targetSession.title ||
      targetSession.title === "New Conversation" ||
      targetSession.title === "New Chat"
    ) {
      const cleanPrompt = prompt.trim().replace(/\s+/g, " ");
      const summary = cleanPrompt.substring(0, 32) + (cleanPrompt.length > 32 ? "..." : "");
      summaryToPersist = summary;
      sessions = sessions.map((s) =>
        s.id === targetSessionId ? { ...s, title: summary, updated_at: now } : s
      );
    }

    // Add user message to UI immediately
    const userMsg: Message = {
      id: "msg-" + (typeof crypto !== "undefined" && crypto.randomUUID ? crypto.randomUUID() : (Date.now() + "-" + Math.random().toString(36).slice(2))),
      session_id: targetSessionId,
      role: "user",
      content: prompt,
      token_count: 0,
      created_at: now,
    };
    pendingUserMessages[targetSessionId] = userMsg;
    if (activeSessionId === targetSessionId) {
      messages = [...messages, userMsg];
    }

    // Initialize session stream state
    streamStates[targetSessionId] = {
      streamingText: "",
      isStreaming: true,
      toolPermission: null,
    };

    if (summaryToPersist) {
      try {
        await invoke("rename_session", { sessionId: targetSessionId, title: summaryToPersist });
      } catch (e) {
        console.error("Failed to rename session:", e);
      }
    }

    try {
      await invoke("send_prompt", {
        sessionId: targetSessionId,
        workspaceId: targetWorkspaceId,
        prompt,
        model: targetModel,
        approvalMode,
        messageId: userMsg.id,
      });
    } catch (err: any) {
      if (streamStates[targetSessionId]) {
        streamStates[targetSessionId].streamingText = `**Error starting prompt:** ${err?.toString() || err}`;
      }
      finishStreamingForSession(targetSessionId);
    } finally {
      delete pendingUserMessages[targetSessionId];
    }
  }

  async function finishStreamingForSession(sessionId: string) {
    const stream = streamStates[sessionId];
    if (!stream) return;

    let finalText = stream.streamingText.trim();
    const toolCalls = stream.toolCalls && stream.toolCalls.length > 0 ? stream.toolCalls : undefined;
    stream.isStreaming = false;
    delete streamStates[sessionId];
    delete pendingUserMessages[sessionId];
    streamStates = { ...streamStates };

    if (!finalText && (!toolCalls || toolCalls.length === 0)) {
      finalText = "*(Completed with no additional text response)*";
    }

    const assistantMsg: Message = {
      id: "msg-" + (typeof crypto !== "undefined" && crypto.randomUUID ? crypto.randomUUID() : (Date.now() + "-" + Math.random().toString(36).slice(2))),
      session_id: sessionId,
      role: "assistant",
      content: finalText,
      tool_calls_json: toolCalls ? JSON.stringify(toolCalls) : undefined,
      token_count: Math.ceil(finalText.length / 4),
      created_at: new Date().toISOString(),
    };
    await invoke("save_message", { msg: assistantMsg });

    // Notify user via native Windows toast if window is backgrounded / minimized
    const wsName = activeWorkspace?.name || "Gemini Desktop";
    const cleanSnippet = finalText.replace(/[#*`_>]/g, "").trim();
    const snippet = cleanSnippet.length > 90 ? `${cleanSnippet.slice(0, 87)}...` : cleanSnippet;
    notifyBackgroundTaskComplete(`${wsName} — Response Complete`, snippet);

    // Reload latest persisted messages from SQLite to ensure 100% ID synchronization across turns
    try {
      const loaded = await invoke<Message[]>("get_session_messages", { sessionId });
      if (activeSessionId === sessionId && loaded.length > 0) {
        messages = loaded;
      } else if (activeSessionId === sessionId) {
        messages = [...messages, assistantMsg];
      }
    } catch {
      if (activeSessionId === sessionId) {
        messages = [...messages, assistantMsg];
      }
    }
  }

  async function handleCancelPrompt() {
    if (!activeSessionId) return;
    const sid = activeSessionId;
    if (streamStates[sid]) {
      streamStates[sid].toolPermission = null;
    }
    try {
      await invoke("cancel_prompt", { requestId: 1, sessionId: sid });
    } catch (e) {
      console.warn("Cancel signal error:", e);
    }
    finishStreamingForSession(sid);
  }

  async function handleToolResponse(requestId: number, optionId?: string, allowed: boolean = true) {
    if (activeSessionId && streamStates[activeSessionId]) {
      streamStates[activeSessionId].toolPermission = null;
    }
    try {
      await invoke("respond_tool_permission", { requestId, optionId, allowed });
    } catch (e) {
      console.error("Failed to send tool response:", e);
    }
  }

  async function handleExport(format: string) {
    if (!activeSession) return;
    try {
      const exported = await invoke<string>("export_session", {
        sessionId: activeSession.id,
        format,
      });

      // Download file to user's computer
      const blob = new Blob([exported], { type: "text/plain;charset=utf-8" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `${activeSession.title.replace(/[^a-zA-Z0-9]/g, "_")}.${format}`;
      a.click();
      URL.revokeObjectURL(url);
    } catch (e) {
      await dialogManager.alert("Failed to export chat: " + e, "Export Failed");
    }
  }

  async function handleSearch(query: string): Promise<SearchResult[]> {
    return await invoke<SearchResult[]>("search_history", { query });
  }

  async function handleSaveWorkspace(ws: Workspace) {
    const isNew = !workspaces.some((w) => w.id === ws.id);
    const oldMode = activeWorkspace?.approval_mode;
    await invoke("save_workspace", { workspace: ws });
    workspaces = await invoke<Workspace[]>("get_workspaces");
    if (isNew) {
      await selectWorkspace(ws);
    } else if (activeWorkspace?.id === ws.id) {
      activeWorkspace = ws;
      await loadWorkspaceFiles(ws.id);
      if (oldMode !== ws.approval_mode) {
        const anyStreaming = Object.values(streamStates).some((s) => s.isStreaming);
        if (!anyStreaming) {
          try {
            await invoke("restart_gemini_session");
          } catch (e) {
            console.warn("Failed to reset session on mode switch:", e);
          }
        }
      }
    }
    modalManager.close();
  }

  async function handleDeleteWorkspace(id: string) {
    await invoke("delete_workspace", { id });
    workspaces = await invoke<Workspace[]>("get_workspaces");
    if (activeWorkspace?.id === id) {
      if (workspaces.length > 0) {
        await selectWorkspace(workspaces[0]);
      } else {
        activeWorkspace = null;
        sessions = [];
        activeSessionId = null;
        messages = [];
        workspaceFiles = [];
        if (typeof window !== "undefined") {
          try {
            localStorage.removeItem(LAST_WORKSPACE_KEY);
          } catch (e) {
            // ignore
          }
        }
      }
    }
    modalManager.close();
  }
</script>

<div class="flex h-screen w-screen bg-app overflow-hidden select-none">
  <!-- Left Navigation Sidebar -->
  <Sidebar
    isOpen={showSidebar}
    {workspaces}
    {activeWorkspace}
    {sessions}
    {activeSession}
    {generatingSessionIds}
    {envStatus}
    {appVersion}
    onToggle={() => (showSidebar = !showSidebar)}
    onSelectWorkspace={selectWorkspace}
    onSelectSession={selectSession}
    onNewSession={handleNewSession}
    onRenameSession={handleRenameSession}
    onDeleteSession={handleDeleteSession}
    onOpenSearch={() => modalManager.open("search")}
    onOpenTemplates={() => modalManager.open("templates")}
    onOpenWorkspaceModal={() => modalManager.open("workspace")}
    onOpenThemeModal={() => modalManager.open("theme")}
    onOpenMcpModal={() => modalManager.open("mcp")}
    onOpenCliSetup={() => modalManager.open("cli-setup")}
    onToggleTerminal={() => (showTerminalDrawer = !showTerminalDrawer)}
    onToggleExplorer={() => (showSolutionExplorer = !showSolutionExplorer)}
    onCheckUpdate={() => handleCheckUpdate(true)}
    {isCheckingUpdate}
  />

  <!-- Main Chat Surface -->
  <div class="flex-1 flex flex-col min-w-0 h-full overflow-hidden">
    {#if updateInfo}
      <UpdateNotificationBanner {updateInfo} onDismiss={() => (updateInfo = null)} />
    {/if}
    <ChatView
      bind:this={chatViewRef}
      workspace={activeWorkspace}
      session={activeSession}
      {workspaceFiles}
      {messages}
      {isStreaming}
      {streamingText}
      {streamingToolCalls}
      streamingSteeringHints={activeStreamState.steeringHints || []}
      {toolPermission}
      {showSidebar}
      {approvalMode}
      onToggleSidebar={() => (showSidebar = !showSidebar)}
      bind:showTerminalDrawer
      bind:showSolutionExplorer
      {activeDebugError}
      {activeFileSnippet}
      onDiagnoseDebugError={handleDiagnoseDebugError}
      onPrefillDebugError={handlePrefillDebugError}
      onDismissDebugError={handleDismissDebugError}
      onTriggerDebugDoctor={() => checkClipboardForDebugError(true)}
      onDebugErrorDetected={(err) => loadAndDisplayDebugError(err)}
      onSendPrompt={handleSendPrompt}
      onCancelPrompt={handleCancelPrompt}
      onSteerPrompt={handleSteerPrompt}
      onExecutePlan={handleExecutePlan}
      onToolResponse={handleToolResponse}
      onExport={handleExport}
      onOpenMcpModal={() => modalManager.open("mcp")}
      onOpenRewindModal={(targetId?: string) => {
        rewindTargetMessageId = targetId ?? null;
        modalManager.open("rewind");
      }}
    />
    <RewindModal
    workspace={activeWorkspace}
    session={activeSession}
    {messages}
    selectedMessageId={rewindTargetMessageId}
    onRewound={handleRewound}
  />
</div>

  <!-- Right Visual Studio 2022 Workspace Explorer -->
  <SolutionExplorer
    bind:this={solutionExplorerRef}
    workspace={activeWorkspace}
    {workspaceFiles}
    isOpen={showSolutionExplorer}
    onToggle={() => (showSolutionExplorer = !showSolutionExplorer)}
    onRefresh={refreshWorkspaceFiles}
    onInsertMention={(relPath) => {
      chatViewRef?.insertFileMention(relPath);
    }}
    onAttachItem={(relPath, isDir, name) => {
      chatViewRef?.attachItem(relPath, isDir, name);
    }}
    onAttachMultiple={(items) => {
      chatViewRef?.attachMultiple(items);
    }}
  />

  <!-- Modals Consolidated with modalManager -->
  <SearchModal
    isOpen={modalManager.isOpen("search")}
    onClose={() => modalManager.close()}
    onSearch={handleSearch}
    onSelectResult={async (res) => {
      const foundSession = sessions.find((s) => s.id === res.session_id);
      if (foundSession) {
        await selectSession(foundSession);
      }
    }}
  />

  <WorkspaceModal
    isOpen={modalManager.isOpen("workspace")}
    {activeWorkspace}
    {workspaces}
    onClose={() => modalManager.close()}
    onSaveWorkspace={handleSaveWorkspace}
    onDeleteWorkspace={handleDeleteWorkspace}
  />

  <TemplatesModal
    isOpen={modalManager.isOpen("templates")}
    templates={promptTemplates}
    onClose={() => modalManager.close()}
    onSelectTemplate={(prompt) => {
      handleSendPrompt(prompt);
    }}
  />

  <ThemeModal
    isOpen={modalManager.isOpen("theme")}
    onClose={() => modalManager.close()}
  />

  <McpModal
    isOpen={modalManager.isOpen("mcp")}
    {activeWorkspace}
    onClose={() => modalManager.close()}
  />

  <CliSetupModal
    isOpen={modalManager.isOpen("cli-setup")}
    {envStatus}
    {activeWorkspace}
    onClose={() => modalManager.close()}
    onEnvStatusUpdated={(newStatus) => {
      envStatus = newStatus;
    }}
  />
</div>
