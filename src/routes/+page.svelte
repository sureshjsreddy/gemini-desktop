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
  } from "$lib/types";
  import Sidebar from "$lib/components/Sidebar.svelte";
  import ChatView from "$lib/components/ChatView.svelte";
  import SolutionExplorer from "$lib/components/SolutionExplorer.svelte";
  import SearchModal from "$lib/components/SearchModal.svelte";
  import WorkspaceModal from "$lib/components/WorkspaceModal.svelte";
  import TemplatesModal from "$lib/components/TemplatesModal.svelte";
  import ThemeModal from "$lib/components/ThemeModal.svelte";
  import McpModal from "$lib/components/McpModal.svelte";
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
  let activeSession = $state<Session | null>(null);
  let messages = $state<Message[]>([]);
  let promptTemplates = $state<PromptTemplate[]>([]);
  let workspaceFiles = $state<WorkspaceFileEntry[]>([]);
  let updateInfo = $state<UpdateInfo | null>(null);
  let isCheckingUpdate = $state(false);
  let approvalMode: ApprovalMode = $derived(activeWorkspace?.approval_mode || "default");

  let streamStates = $state<Record<string, SessionStreamState>>({});

  let activeStreamState = $derived.by<SessionStreamState>(() => {
    const sid = activeSession?.id;
    if (sid && streamStates[sid]) {
      return streamStates[sid];
    }
    return { streamingText: "", isStreaming: false, toolPermission: null };
  });

  let isStreaming = $derived(activeStreamState.isStreaming);
  let streamingText = $derived(activeStreamState.streamingText);
  let toolPermission = $derived(activeStreamState.toolPermission);

  let generatingSessionIds = $derived(
    new Set(
      Object.entries(streamStates)
        .filter(([_, s]) => s.isStreaming)
        .map(([id]) => id)
    )
  );

  let envStatus: GeminiEnvStatus | null = $state(null);

  // Panels & Drawers
  let showTerminalDrawer = $state(false);
  let showSolutionExplorer = $state(true);
  let showSidebar = $state(true);

  let chatViewRef = $state<ReturnType<typeof ChatView> | null>(null);
  let solutionExplorerRef = $state<ReturnType<typeof SolutionExplorer> | null>(null);

  let unlistenChunk: UnlistenFn | null = null;
  let unlistenTool: UnlistenFn | null = null;
  let unlistenError: UnlistenFn | null = null;
  let unlistenModeUpdate: UnlistenFn | null = null;
  let unlistenStderr: UnlistenFn | null = null;

  onMount(async () => {
    // 0. Initialize theme & dynamic app version
    themeManager.init();
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

        let cleanDelta = delta || "";
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

        // 1. YOLO Mode: Auto-approve all tools immediately without showing permission prompt
        if (approvalMode === "yolo") {
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
        if (approvalMode === "auto_edit") {
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
        }
      }
    );

    unlistenError = await listen<any>("acp-error", (event) => {
      const payloadStr = JSON.stringify(event.payload || "");
      if (payloadStr.toLowerCase().includes("cancel")) {
        console.warn("Ignored cancellation signal:", event.payload);
        if (activeSession?.id) {
          finishStreamingForSession(activeSession.id);
        }
        return;
      }
      console.error("ACP Error:", event.payload);
      if (activeSession?.id) {
        if (streamStates[activeSession.id]) {
          streamStates[activeSession.id].streamingText += `\n\n**Error:** ${payloadStr}`;
        }
        finishStreamingForSession(activeSession.id);
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
        const sid = activeSession?.id;
        if (sid && streamStates[sid] && streamStates[sid].isStreaming) {
          if (!streamStates[sid].streamingText.includes("high demand, retrying")) {
            streamStates[sid].streamingText += "\n\n> ⏳ *Model is currently experiencing high demand, retrying with backoff...*\n\n";
          }
        }
      }
    });

    window.addEventListener("keydown", handleGlobalShortcuts);

    // 5. Check for updates on WinGet in the background (3s delay)
    setTimeout(() => {
      handleCheckUpdate(false);
    }, 3000);
  });

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
    if (unlistenError) unlistenError();
    if (unlistenModeUpdate) unlistenModeUpdate();
    if (unlistenStderr) unlistenStderr();
    window.removeEventListener("keydown", handleGlobalShortcuts);
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
    sessions = await invoke<Session[]>("get_sessions", { workspaceId: wsId });
    if (sessions.length > 0) {
      await selectSession(sessions[0]);
    } else {
      activeSession = null;
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
    activeSession = session;
    messages = await invoke<Message[]>("get_session_messages", { sessionId: session.id });
  }

  async function handleNewSession() {
    if (!activeWorkspace) return;
    const newSession = await invoke<Session>("create_session", {
      workspaceId: activeWorkspace.id,
      title: "New Conversation",
    });
    sessions = [newSession, ...sessions];
    await selectSession(newSession);
  }

  async function handleRenameSession(session: Session) {
    const newTitle = await dialogManager.prompt("Enter new title for conversation:", session.title, {
      title: "Rename Conversation",
      placeholder: "New conversation title...",
    });
    if (!newTitle || !newTitle.trim() || newTitle === session.title) return;
    const trimmed = newTitle.trim();
    await invoke("rename_session", { sessionId: session.id, title: trimmed });
    session.title = trimmed;
    sessions = [...sessions];
  }

  async function handleDeleteSession(session: Session) {
    const confirmed = await dialogManager.confirm(`Delete conversation "${session.title}"? This cannot be undone.`, {
      title: "Delete Conversation",
      confirmText: "Delete",
      isDestructive: true,
    });
    if (!confirmed) return;
    await invoke("delete_session", { sessionId: session.id });
    sessions = sessions.filter((s) => s.id !== session.id);
    if (activeSession?.id === session.id) {
      if (sessions.length > 0) {
        await selectSession(sessions[0]);
      } else {
        activeSession = null;
        messages = [];
      }
    }
  }

  async function handleSendPrompt(prompt: string) {
    if (!activeWorkspace) return;

    if (!activeSession) {
      await handleNewSession();
    }
    if (!activeSession) return;

    const targetSessionId = activeSession.id;

    // Auto-update title if it's default
    if (activeSession.title === "New Conversation") {
      const summary = prompt.substring(0, 32) + (prompt.length > 32 ? "..." : "");
      await invoke("rename_session", { sessionId: targetSessionId, title: summary });
      activeSession.title = summary;
      sessions = [...sessions];
    }

    // Add user message to UI immediately
    const userMsg: Message = {
      id: "temp-" + Date.now(),
      session_id: targetSessionId,
      role: "user",
      content: prompt,
      token_count: 0,
      created_at: new Date().toISOString(),
    };
    messages = [...messages, userMsg];

    // Initialize session stream state
    streamStates[targetSessionId] = {
      streamingText: "",
      isStreaming: true,
      toolPermission: null,
    };

    try {
      await invoke("send_prompt", {
        sessionId: targetSessionId,
        workspaceId: activeWorkspace.id,
        prompt,
        model: activeWorkspace.model,
        approvalMode,
      });
    } catch (err: any) {
      if (streamStates[targetSessionId]) {
        streamStates[targetSessionId].streamingText = `**Error starting prompt:** ${err?.toString() || err}`;
      }
      finishStreamingForSession(targetSessionId);
    }
  }

  async function finishStreamingForSession(sessionId: string) {
    const stream = streamStates[sessionId];
    if (!stream) return;

    let finalText = stream.streamingText.trim();
    stream.isStreaming = false;
    delete streamStates[sessionId];
    streamStates = { ...streamStates };

    if (!finalText) {
      finalText = "*(Completed with no additional text response)*";
    }

    const assistantMsg: Message = {
      id: "msg-" + Date.now(),
      session_id: sessionId,
      role: "assistant",
      content: finalText,
      token_count: Math.ceil(finalText.length / 4),
      created_at: new Date().toISOString(),
    };
    await invoke("save_message", { msg: assistantMsg });

    // If the user is currently viewing this session, update visible messages immediately
    if (activeSession?.id === sessionId) {
      messages = [...messages, assistantMsg];
    }
  }

  async function handleCancelPrompt() {
    if (!activeSession) return;
    const sid = activeSession.id;
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
    if (activeSession?.id && streamStates[activeSession.id]) {
      streamStates[activeSession.id].toolPermission = null;
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
        activeSession = null;
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
      {toolPermission}
      {showSidebar}
      {approvalMode}
      onToggleSidebar={() => (showSidebar = !showSidebar)}
      bind:showTerminalDrawer
      bind:showSolutionExplorer
      onSendPrompt={handleSendPrompt}
      onCancelPrompt={handleCancelPrompt}
      onToolResponse={handleToolResponse}
      onExport={handleExport}
      onOpenMcpModal={() => modalManager.open("mcp")}
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
</div>
