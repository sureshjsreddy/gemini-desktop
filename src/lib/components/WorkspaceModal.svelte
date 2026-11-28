<script lang="ts">
  import type {
    Workspace,
    CliModelInfo,
    CliModelDiscoveryResult,
    WorkspaceContextProbe,
  } from "$lib/types";
  import {
    X,
    Plus,
    Trash2,
    Check,
    Bell,
    BellRing,
    FileDiff,
    AlertTriangle,
    RefreshCw,
    Sparkles,
    Shield,
    FolderOpen,
    FolderPlus,
    Copy,
    Search,
    CheckCircle2,
    GitBranch,
    FileText,
    Sliders,
    Cpu,
    Lock,
  } from "lucide-svelte";
  import {
    isNotificationSettingEnabled,
    setNotificationSetting,
    notifyBackgroundTaskComplete,
  } from "$lib/utils/notifications";
  import { dialogManager } from "$lib/dialog.svelte";
  import { engineStore } from "$lib/stores/engineStore.svelte";
  import {
    formatContextLimitLabel,
    getEffectiveModel,
    saveRememberedWorkspaceModel,
    getRememberedWorkspaceModel,
  } from "$lib/utils/engineModels";
  import { APPROVAL_MODES, getModeConfig } from "$lib/utils/approvalModes";
  import {
    normalizeWorkspaceDraft,
    isWorkspaceDraftDirty,
    suggestWorkspaceNameFromPath,
    shouldAutoSyncProfileName,
  } from "$lib/utils/workspaceDraft";
  import { invoke } from "@tauri-apps/api/core";

  interface ModelOptionGroup {
    group: string;
    options: { value: string; label: string }[];
  }

  function isDeprecatedModel(model: string): boolean {
    const m = (model || "").trim().toLowerCase();
    return (
      m.startsWith("gemini-1.5") ||
      m.includes("preview-09-2025") ||
      m === "gemini-1.5-flash-8b" ||
      m === "gemini-1.5-flash" ||
      m === "gemini-1.5-pro"
    );
  }

  function isRetiringVertexModel(model: string): boolean {
    const m = (model || "").trim().toLowerCase();
    return m === "gemini-2.5-flash-lite";
  }

  const MODEL_GROUPS: ModelOptionGroup[] = [
    {
      group: "Auto (Recommended by Gemini CLI)",
      options: [
        { value: "auto", label: "Auto — Let Gemini CLI decide the best model (Recommended)" },
        { value: "auto-gemini-2.5", label: "Auto (Gemini 2.5) — Auto-selects 2.5 Pro / 2.5 Flash" },
      ],
    },
    {
      group: "Gemini 3 / 3.5 / 3.8 (Recommended & Latest)",
      options: [
        { value: "gemini-3.8-flash", label: "gemini-3.8-flash (Latest Preview — Ultra-responsive next-gen Flash)" },
        { value: "gemini-3.5-flash", label: "gemini-3.5-flash (Fast & powerful default in CLI 0.59+)" },
        { value: "gemini-3.5-flash-lite", label: "gemini-3.5-flash-lite (Ultra-fast & lightweight 3.5 — Upgrade target)" },
        { value: "gemini-3.1-pro-preview", label: "gemini-3.1-pro-preview (Deep reasoning, complex coding)" },
        { value: "gemini-3.1-flash-lite", label: "gemini-3.1-flash-lite (Ultra-fast & cost-effective)" },
        { value: "gemini-3-pro-preview", label: "gemini-3-pro-preview (Preview reasoning & coding)" },
        { value: "gemini-3-flash-preview", label: "gemini-3-flash-preview (High speed, fast results)" },
      ],
    },
    {
      group: "Gemini 2.5",
      options: [
        { value: "gemini-2.5-pro", label: "gemini-2.5-pro (Complex tasks, coding & architecture)" },
        { value: "gemini-2.5-flash", label: "gemini-2.5-flash (Balanced speed & performance)" },
        { value: "gemini-2.5-flash-lite", label: "gemini-2.5-flash-lite (Active on AI Studio; Vertex retiring late 2026)" },
      ],
    },
    {
      group: "Gemini 1.5 (Deprecated / Retired Upstream)",
      options: [
        { value: "gemini-1.5-pro", label: "gemini-1.5-pro (Deprecated / Retired by Google)" },
        { value: "gemini-1.5-flash", label: "gemini-1.5-flash (Deprecated / Retired by Google)" },
        { value: "gemini-1.5-flash-8b", label: "gemini-1.5-flash-8b (Deprecated / Retired by Google)" },
      ],
    },
  ];

  const AGY_MODEL_GROUPS: ModelOptionGroup[] = [
    {
      group: "Antigravity Default & Orchestrator",
      options: [
        { value: "auto", label: "Auto — Default Antigravity model" },
        { value: "orchestrator", label: "orchestrator — Multi-Model Coordinator (Planner + Executor)" },
      ],
    },
    {
      group: "Gemini 4 Frontier (Hydrogen / Carbon / Neon)",
      options: [
        { value: "gemini-4-hydrogen-high", label: "gemini-4-hydrogen-high (Gemini 4 Hydrogen High)" },
        { value: "gemini-4-hydrogen-xhigh", label: "gemini-4-hydrogen-xhigh (Gemini 4 Hydrogen Extra High)" },
        { value: "gemini-4-hydrogen-medium", label: "gemini-4-hydrogen-medium (Gemini 4 Hydrogen Medium)" },
        { value: "gemini-4-hydrogen-low", label: "gemini-4-hydrogen-low (Gemini 4 Hydrogen Low)" },
        { value: "gemini-4-carbon", label: "gemini-4-carbon (Gemini 4 Carbon — Fast Frontier)" },
        { value: "gemini-4-neon", label: "gemini-4-neon (Gemini 4 Neon — Balanced Frontier)" },
        { value: "gemini-4-argon", label: "gemini-4-argon (Gemini 4 Argon — Legacy Frontier)" },
      ],
    },
    {
      group: "Gemini 3.8 Flash (Reasoning & Speed)",
      options: [
        { value: "gemini-3.8-flash-high", label: "gemini-3.8-flash-high (Gemini 3.8 Flash High Reasoning)" },
        { value: "gemini-3.8-flash-medium", label: "gemini-3.8-flash-medium (Gemini 3.8 Flash Medium Reasoning)" },
        { value: "gemini-3.8-flash-low", label: "gemini-3.8-flash-low (Gemini 3.8 Flash Low Reasoning)" },
      ],
    },
    {
      group: "Anthropic Claude (Extended Thinking)",
      options: [
        { value: "claude-sonnet-4-6", label: "claude-sonnet-4-6 (Claude Sonnet 4.6 Thinking)" },
        { value: "claude-opus-4-6-thinking", label: "claude-opus-4-6-thinking (Claude Opus 4.6 Thinking)" },
      ],
    },
    {
      group: "Open Models",
      options: [
        { value: "gpt-oss-120b-medium", label: "gpt-oss-120b-medium (GPT-OSS 120B Medium Reasoning)" },
      ],
    },
  ];

  function loadCachedDiscoveredModels(engine: string): CliModelInfo[] {
    if (typeof window === "undefined") return [];
    try {
      const raw = localStorage.getItem(`gemini_desktop_discovered_models_${engine}`);
      if (!raw) return [];
      const parsed = JSON.parse(raw);
      return Array.isArray(parsed) ? parsed : [];
    } catch {
      return [];
    }
  }

  let {
    isOpen = false,
    activeWorkspace = null,
    workspaces = [],
    activeEngine = "gemini",
    onClose,
    onSaveWorkspace,
    onDeleteWorkspace,
  }: {
    isOpen: boolean;
    activeWorkspace?: Workspace | null;
    workspaces: Workspace[];
    activeEngine?: string;
    onClose: () => void;
    onSaveWorkspace: (ws: Workspace, switchToWorkspace?: boolean) => void | Promise<void>;
    onDeleteWorkspace: (id: string) => void | Promise<void>;
  } = $props();

  type WorkspaceTab = "essentials" | "agent" | "files" | "app";
  type PendingDiscardAction =
    | { type: "close" }
    | { type: "new" }
    | { type: "switch"; ws: Workspace }
    | { type: "duplicate"; ws: Workspace }
    | null;

  let activeTab = $state<WorkspaceTab>("essentials");
  let workspaceSearch = $state("");
  let pathInputEl = $state<HTMLInputElement | null>(null);

  let activeDescriptor = $derived(engineStore.getDescriptor(activeEngine));
  let effectiveModelGroups = $derived(
    activeDescriptor.modelGroups && activeDescriptor.modelGroups.length > 0
      ? activeDescriptor.modelGroups
      : activeEngine === "agy"
        ? AGY_MODEL_GROUPS
        : MODEL_GROUPS
  );

  let editingWorkspace: Workspace = $state(normalizeWorkspaceDraft({}));
  let baselineWorkspace: Workspace | null = $state(null);
  let isNewDraft = $state(false);
  let lastSuggestedName = $state("");

  let selectedDropdownValue = $state("auto");
  let customModelInput = $state("");
  let lastSyncedWorkspaceId = $state<string | null>(null);
  let isSaving = $state(false);
  let notificationsEnabled = $state(true);

  let isDiscoveringModels = $state(false);
  let discoveryStatus = $state<{ type: "success" | "info" | "error"; text: string } | null>(null);
  let liveDiscoveredModels = $state<CliModelInfo[]>([]);
  let discoveredEngine = $state<string | null>(null);
  let discoveredVersion = $state<string | null>(null);
  let previousActiveEngine = $state<string | null>(null);

  let pathProbe = $state<WorkspaceContextProbe | null>(null);
  let isProbingPath = $state(false);
  let isBrowsingFolder = $state(false);
  let isCreatingFolder = $state(false);
  let pathProbeTimer: ReturnType<typeof setTimeout> | null = null;
  let pathProbeSeq = 0;

  let backdropPointerDown = false;
  let closeGuardNotice = $state(false);
  let closeGuardTimer: ReturnType<typeof setTimeout> | null = null;
  let pendingDiscardAction = $state<PendingDiscardAction>(null);

  let isSyncingSkills = $state(false);
  let skillsSyncResult = $state<{ success: boolean; message: string; synced_count: number } | null>(null);

  let isDirty = $derived(isWorkspaceDraftDirty(editingWorkspace, baselineWorkspace, isNewDraft));

  let filteredWorkspaces = $derived.by(() => {
    const q = workspaceSearch.trim().toLowerCase();
    if (!q) return workspaces;
    return workspaces.filter(
      (w) =>
        w.name.toLowerCase().includes(q) ||
        w.path.toLowerCase().includes(q) ||
        (w.model || "").toLowerCase().includes(q)
    );
  });

  let availableApprovalModes = $derived(
    APPROVAL_MODES.filter((m) => m.id !== "auto" || activeEngine === "agy")
  );
  let selectedApprovalModeConfig = $derived(
    getModeConfig(editingWorkspace.approval_mode || "default")
  );

  let hasAdvancedAgentConfig = $derived(
    Boolean(
      editingWorkspace.reasoning_effort?.trim() ||
        editingWorkspace.agent_name?.trim() ||
        editingWorkspace.additional_dirs?.trim() ||
        editingWorkspace.sandbox_enabled ||
        editingWorkspace.system_prompt?.trim()
    )
  );

  let canOfferCreateFolder = $derived(
    Boolean(
      pathProbe &&
        !pathProbe.validation.is_valid &&
        !pathProbe.validation.exists &&
        editingWorkspace.path.trim().length > 0 &&
        !(pathProbe.validation.error_message || "").toLowerCase().includes("restricted") &&
        !(pathProbe.validation.error_message || "").toLowerCase().includes("blocked") &&
        !(pathProbe.validation.error_message || "").toLowerCase().includes("root drive") &&
        !(pathProbe.validation.error_message || "").toLowerCase().includes("temporary")
    )
  );

  let canSave = $derived(
    editingWorkspace.name.trim().length > 0 &&
      editingWorkspace.path.trim().length > 0 &&
      !isSaving
  );

  let isEditingInactiveSaved = $derived(
    !isNewDraft && Boolean(activeWorkspace) && editingWorkspace.id !== activeWorkspace?.id
  );

  $effect(() => {
    if (previousActiveEngine === null) {
      previousActiveEngine = activeEngine;
      liveDiscoveredModels = loadCachedDiscoveredModels(activeEngine);
      discoveredEngine = liveDiscoveredModels.length > 0 ? activeEngine : null;
      return;
    }
    if (activeEngine !== previousActiveEngine) {
      previousActiveEngine = activeEngine;
      liveDiscoveredModels = loadCachedDiscoveredModels(activeEngine);
      discoveryStatus = null;
      discoveredEngine = liveDiscoveredModels.length > 0 ? activeEngine : null;
      discoveredVersion = null;
      if (editingWorkspace.id) {
        const remembered = getRememberedWorkspaceModel(editingWorkspace.id, activeEngine);
        if (remembered) {
          editingWorkspace.model = remembered;
          lastSyncedWorkspaceId = null;
        }
      }
    }
  });

  $effect(() => {
    if (isOpen) {
      notificationsEnabled = isNotificationSettingEnabled();
    }
  });

  let wasOpen = false;
  $effect(() => {
    if (isOpen && !wasOpen) {
      wasOpen = true;
      pendingDiscardAction = null;
      closeGuardNotice = false;
      skillsSyncResult = null;
      workspaceSearch = "";
      activeTab = "essentials";
      const target =
        (activeWorkspace && workspaces.find((w) => w.id === activeWorkspace.id)) ||
        activeWorkspace ||
        workspaces[0];
      if (target) {
        loadExistingProfile(target);
      } else {
        initNewDraft();
      }
    } else if (!isOpen) {
      wasOpen = false;
      if (pathProbeTimer) {
        clearTimeout(pathProbeTimer);
        pathProbeTimer = null;
      }
      if (closeGuardTimer) {
        clearTimeout(closeGuardTimer);
        closeGuardTimer = null;
      }
    }
  });

  $effect(() => {
    // Only re-sync dropdown state when editing workspace changes (avoids clobbering manual typing)
    if (editingWorkspace.id !== lastSyncedWorkspaceId) {
      lastSyncedWorkspaceId = editingWorkspace.id;
      const isKnownStatic = effectiveModelGroups.some((g) =>
        g.options.some((o) => o.value === editingWorkspace.model)
      );
      const isKnownLive = liveDiscoveredModels.some((m) => m.id === editingWorkspace.model);
      if (isKnownStatic || isKnownLive) {
        selectedDropdownValue = editingWorkspace.model;
        customModelInput = "";
      } else {
        selectedDropdownValue = "manual_custom";
        customModelInput = editingWorkspace.model || "";
      }
    }
  });

  async function probePathNow(rawPath: string) {
    const seq = ++pathProbeSeq;
    const trimmed = (rawPath || "").trim();
    if (!trimmed) {
      pathProbe = null;
      isProbingPath = false;
      return;
    }
    isProbingPath = true;
    try {
      const res = await invoke<WorkspaceContextProbe>("probe_workspace_context", { path: trimmed });
      if (seq === pathProbeSeq) {
        pathProbe = res;
      }
    } catch {
      if (seq === pathProbeSeq) {
        pathProbe = null;
      }
    } finally {
      if (seq === pathProbeSeq) {
        isProbingPath = false;
      }
    }
  }

  function schedulePathProbe(rawPath: string) {
    if (pathProbeTimer) {
      clearTimeout(pathProbeTimer);
    }
    pathProbeTimer = setTimeout(() => {
      probePathNow(rawPath);
    }, 240);
  }

  function loadExistingProfile(ws: Workspace) {
    const normalized = normalizeWorkspaceDraft(ws);
    editingWorkspace = normalized;
    baselineWorkspace = normalizeWorkspaceDraft(ws);
    isNewDraft = false;
    lastSuggestedName = "";
    lastSyncedWorkspaceId = null;
    skillsSyncResult = null;
    probePathNow(normalized.path);
  }

  async function initNewDraft() {
    let defaultPath = "";
    try {
      defaultPath = await invoke<string>("get_default_workspace_dir");
    } catch {
      defaultPath = "";
    }

    const suggested = suggestWorkspaceNameFromPath(defaultPath);
    const draft = normalizeWorkspaceDraft({
      id: "ws-" + Math.random().toString(36).substring(2, 9),
      name: suggested || "New Workspace",
      path: defaultPath,
      model: "auto",
      approval_mode: "default",
      track_file_changes: false,
      created_at: new Date().toISOString(),
    });
    editingWorkspace = draft;
    baselineWorkspace = normalizeWorkspaceDraft(draft);
    isNewDraft = true;
    lastSuggestedName = suggested;
    lastSyncedWorkspaceId = null;
    skillsSyncResult = null;
    activeTab = "essentials";
    await probePathNow(defaultPath);
    setTimeout(() => {
      pathInputEl?.focus();
    }, 60);
  }

  function initDuplicateDraft(source: Workspace) {
    const cloned = normalizeWorkspaceDraft({
      ...source,
      id: "ws-" + Math.random().toString(36).substring(2, 9),
      name: `${source.name} (Copy)`,
      created_at: new Date().toISOString(),
    });
    editingWorkspace = cloned;
    baselineWorkspace = null; // Cloned draft should immediately register as saveable/unsaved
    isNewDraft = true;
    lastSuggestedName = "";
    lastSyncedWorkspaceId = null;
    skillsSyncResult = null;
    activeTab = "essentials";
    probePathNow(cloned.path);
    setTimeout(() => {
      pathInputEl?.focus();
    }, 60);
  }

  function requestSwitchOrAction(action: Exclude<PendingDiscardAction, null>) {
    if (isDirty) {
      pendingDiscardAction = action;
      return;
    }
    executePendingAction(action);
  }

  function executePendingAction(action: Exclude<PendingDiscardAction, null>) {
    pendingDiscardAction = null;
    closeGuardNotice = false;
    if (action.type === "close") {
      onClose();
    } else if (action.type === "new") {
      initNewDraft();
    } else if (action.type === "switch") {
      loadExistingProfile(action.ws);
    } else if (action.type === "duplicate") {
      initDuplicateDraft(action.ws);
    }
  }

  function triggerCloseGuardFeedback() {
    closeGuardNotice = true;
    if (closeGuardTimer) clearTimeout(closeGuardTimer);
    closeGuardTimer = setTimeout(() => {
      closeGuardNotice = false;
    }, 4500);
  }

  function handleBackdropMouseDown(e: MouseEvent) {
    backdropPointerDown = e.target === e.currentTarget;
  }

  function handleBackdropClick(e: MouseEvent) {
    if (!backdropPointerDown || e.target !== e.currentTarget) {
      backdropPointerDown = false;
      return;
    }
    backdropPointerDown = false;
    // Never dismiss silently when creating a draft or holding unsaved changes
    if (isDirty || isNewDraft) {
      triggerCloseGuardFeedback();
      return;
    }
    onClose();
  }

  function handleAttemptClose() {
    if (dialogManager.isOpen) return;
    if (isDirty) {
      pendingDiscardAction = { type: "close" };
      return;
    }
    onClose();
  }

  function onPathInput(e: Event) {
    const val = (e.target as HTMLInputElement).value;
    editingWorkspace.path = val;
    const nextSuggested = suggestWorkspaceNameFromPath(val);
    if (shouldAutoSyncProfileName(editingWorkspace.name, lastSuggestedName, isNewDraft)) {
      if (nextSuggested) {
        editingWorkspace.name = nextSuggested;
        lastSuggestedName = nextSuggested;
      }
    }
    schedulePathProbe(val);
  }

  async function handleBrowseFolder() {
    if (isBrowsingFolder) return;
    isBrowsingFolder = true;
    try {
      const picked = await invoke<string | null>("pick_workspace_directory", {
        initialPath: editingWorkspace.path || null,
      });
      if (picked) {
        editingWorkspace.path = picked;
        const nextSuggested = suggestWorkspaceNameFromPath(picked);
        if (shouldAutoSyncProfileName(editingWorkspace.name, lastSuggestedName, isNewDraft) && nextSuggested) {
          editingWorkspace.name = nextSuggested;
          lastSuggestedName = nextSuggested;
        }
        await probePathNow(picked);
      }
    } catch (err) {
      await dialogManager.alert(`Could not open folder browser: ${err}`, "Folder Picker Error");
    } finally {
      isBrowsingFolder = false;
    }
  }

  async function handleCreateMissingFolder() {
    const targetPath = editingWorkspace.path.trim();
    if (!targetPath || isCreatingFolder) return;
    isCreatingFolder = true;
    try {
      await invoke("create_workspace_directory", { path: targetPath });
      await probePathNow(targetPath);
    } catch (err) {
      await dialogManager.alert(`Failed to create folder:\n${err}`, "Create Folder Failed");
    } finally {
      isCreatingFolder = false;
    }
  }

  function onSelectModelChange(e: Event) {
    const target = e.target as HTMLSelectElement;
    selectedDropdownValue = target.value;
    if (target.value === "manual_custom") {
      if (!customModelInput.trim()) {
        const isKnown =
          effectiveModelGroups.some((g) => g.options.some((o) => o.value === editingWorkspace.model)) ||
          liveDiscoveredModels.some((m) => m.id === editingWorkspace.model);
        customModelInput = isKnown ? "" : (editingWorkspace.model || "");
      }
      editingWorkspace.model = customModelInput.trim();
    } else {
      editingWorkspace.model = target.value;
    }
  }

  function onCustomModelInput(e: Event) {
    const target = e.target as HTMLInputElement;
    customModelInput = target.value;
    editingWorkspace.model = target.value.trim();
  }

  async function handleSyncSkills() {
    if (!editingWorkspace.path) return;
    isSyncingSkills = true;
    skillsSyncResult = null;
    try {
      const res = await invoke<{ success: boolean; message: string; synced_count: number }>(
        "sync_workspace_skills_for_agy",
        { workspacePath: editingWorkspace.path }
      );
      skillsSyncResult = res;
      await probePathNow(editingWorkspace.path);
    } catch (err: any) {
      skillsSyncResult = {
        success: false,
        message: `Sync failed: ${err}`,
        synced_count: 0,
      };
    } finally {
      isSyncingSkills = false;
    }
  }

  async function handleDiscoverModels() {
    isDiscoveringModels = true;
    discoveryStatus = null;
    try {
      const res = await invoke<CliModelDiscoveryResult>("discover_gemini_models");
      const eng = res.engine || activeEngine;
      discoveredEngine = eng;
      discoveredVersion = res.cli_version || null;
      const engName = eng === "agy" ? "Antigravity CLI (agy)" : "Gemini CLI";
      const verSuffix = res.cli_version
        ? ` (v${res.cli_version.replace(/^v/i, "")})`
        : eng === "agy"
          ? ""
          : " (v0.67+)";

      if (res.supported && res.models.length > 0) {
        liveDiscoveredModels = res.models;
        if (typeof window !== "undefined") {
          try {
            localStorage.setItem(`gemini_desktop_discovered_models_${eng}`, JSON.stringify(res.models));
          } catch {}
        }
        discoveryStatus = {
          type: "success",
          text: `Discovered ${res.models.length} active models directly via ${engName}${verSuffix}.`,
        };
      } else {
        discoveryStatus = {
          type: "info",
          text:
            res.message ||
            `Model discovery returned no models from ${engName}. Standard models remain available.`,
        };
      }
    } catch (e: any) {
      discoveryStatus = {
        type: "error",
        text: `Discovery failed: ${e?.toString()}`,
      };
    } finally {
      isDiscoveringModels = false;
      setTimeout(() => {
        discoveryStatus = null;
      }, 6000);
    }
  }

  async function handleTestToast() {
    await notifyBackgroundTaskComplete(
      "Gemini Desktop",
      "Native Windows Toast Notifications are enabled and working properly!",
      true
    );
  }

  async function handleSave(switchToWorkspace = false) {
    if (!canSave) return;
    if (selectedDropdownValue === "manual_custom") {
      const manualModel = customModelInput.trim();
      if (!manualModel) {
        activeTab = "essentials";
        await dialogManager.alert(
          "Please specify a custom model name or select an option from the list.",
          "Model Name Required"
        );
        return;
      }
      editingWorkspace.model = manualModel;
    } else {
      editingWorkspace.model = selectedDropdownValue;
    }

    isSaving = true;
    try {
      const val = await invoke<{ is_valid: boolean; exists: boolean; error_message?: string }>(
        "validate_workspace_path",
        { path: editingWorkspace.path.trim() }
      );
      if (!val.is_valid) {
        activeTab = "essentials";
        await probePathNow(editingWorkspace.path);
        await dialogManager.alert(
          val.error_message || `The path "${editingWorkspace.path.trim()}" is invalid for a workspace.`,
          val.exists ? "Restricted Directory (EDR Security Protection)" : "Folder Not Found"
        );
        return;
      }

      editingWorkspace.approval_mode = editingWorkspace.approval_mode || "default";
      saveRememberedWorkspaceModel(editingWorkspace.id, activeEngine, editingWorkspace.model);
      await onSaveWorkspace(
        {
          ...editingWorkspace,
          name: editingWorkspace.name.trim(),
          path: editingWorkspace.path.trim(),
          reasoning_effort: editingWorkspace.reasoning_effort?.trim() || undefined,
          agent_name: editingWorkspace.agent_name?.trim() || undefined,
          additional_dirs: editingWorkspace.additional_dirs?.trim() || undefined,
          sandbox_enabled: editingWorkspace.sandbox_enabled ?? false,
        },
        switchToWorkspace
      );
    } catch (err) {
      await dialogManager.alert(`Failed to verify directory: ${err}`, "Validation Error");
    } finally {
      isSaving = false;
    }
  }

  async function handleDelete() {
    if (isNewDraft || workspaces.length <= 1) return;
    const confirmed = await dialogManager.confirm(
      `Delete workspace profile "${editingWorkspace.name}"? Associated sessions and chat history for this workspace will be permanently deleted.`,
      {
        title: "Delete Workspace Profile",
        confirmText: "Delete",
        isDestructive: true,
      }
    );
    if (confirmed) {
      await onDeleteWorkspace(editingWorkspace.id);
    }
  }
</script>

{#if isOpen}
  <div
    class="fixed inset-0 bg-black/60 backdrop-blur-xs z-40 flex items-center justify-center p-4"
    onmousedown={handleBackdropMouseDown}
    onclick={handleBackdropClick}
    onkeydown={(e) => e.key === "Escape" && handleAttemptClose()}
    role="presentation"
  >
    <div
      class="w-full max-w-4xl h-[84vh] max-h-[760px] bg-surface border border-theme-default rounded-2xl shadow-2xl overflow-hidden flex flex-col"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
      role="dialog"
      aria-modal="true"
      aria-label="Workspace Management"
      tabindex="-1"
    >
      <!-- Sticky Top Header -->
      <div class="px-5 py-3.5 border-b border-subtle flex items-center justify-between bg-surface-elevated/50 shrink-0">
        <div class="flex items-center gap-3 min-w-0">
          <div class="p-2 rounded-xl bg-accent-subtle text-accent-theme shrink-0">
            <FolderOpen size={17} />
          </div>
          <div class="min-w-0">
            <div class="flex items-center gap-2">
              <h2 class="text-sm font-semibold text-primary-theme truncate">
                {isNewDraft ? "Create New Workspace" : `Workspace: ${baselineWorkspace?.name || editingWorkspace.name}`}
              </h2>
              {#if isNewDraft}
                <span class="px-2 py-0.5 text-[10px] font-semibold rounded-full bg-sky-500/15 text-sky-400 border border-sky-500/30 shrink-0">
                  New Draft
                </span>
              {/if}
              {#if isDirty}
                <span class="px-2 py-0.5 text-[10px] font-semibold rounded-full bg-amber-500/15 text-amber-300 border border-amber-500/30 shrink-0">
                  Unsaved changes
                </span>
              {/if}
            </div>
            <p class="text-[11px] text-secondary-theme truncate">
              Bind a local project directory to its own model, safety policy, instructions, and chat history.
            </p>
          </div>
        </div>
        <button
          type="button"
          onclick={handleAttemptClose}
          class="p-1.5 text-secondary-theme hover:text-primary-theme hover:bg-surface rounded-lg transition-colors cursor-pointer shrink-0"
          aria-label="Close modal"
        >
          <X size={16} />
        </button>
      </div>

      <!-- Protected Backdrop / Unsaved Discard Guards -->
      {#if closeGuardNotice}
        <div class="px-5 py-2 bg-amber-500/10 border-b border-amber-500/30 flex items-center justify-between text-xs text-amber-200 shrink-0">
          <div class="flex items-center gap-2">
            <Lock size={13} class="text-amber-400 shrink-0" />
            <span>Outside click ignored to protect your workspace edits. Use <strong>Cancel</strong> or <strong>Save</strong> below.</span>
          </div>
          <button
            type="button"
            onclick={() => (closeGuardNotice = false)}
            class="text-[11px] text-amber-300 hover:text-amber-100 underline cursor-pointer"
          >
            Dismiss
          </button>
        </div>
      {/if}

      {#if pendingDiscardAction}
        <div class="px-5 py-2.5 bg-amber-500/15 border-b border-amber-500/40 flex items-center justify-between gap-3 text-xs shrink-0">
          <div class="flex items-center gap-2 text-amber-100">
            <AlertTriangle size={14} class="text-amber-400 shrink-0" />
            <span>
              You have unsaved changes to <strong>{editingWorkspace.name || "this workspace"}</strong>. Discard them?
            </span>
          </div>
          <div class="flex items-center gap-2 shrink-0">
            <button
              type="button"
              onclick={() => (pendingDiscardAction = null)}
              class="px-2.5 py-1 rounded-md bg-surface hover:bg-surface-hover text-primary-theme border border-theme-default text-[11px] font-medium cursor-pointer"
            >
              Keep Editing
            </button>
            <button
              type="button"
              onclick={() => pendingDiscardAction && executePendingAction(pendingDiscardAction)}
              class="px-2.5 py-1 rounded-md bg-rose-500 hover:bg-rose-600 text-white text-[11px] font-semibold cursor-pointer"
            >
              Discard Changes
            </button>
          </div>
        </div>
      {/if}

      <!-- Main Two-Pane Body -->
      <div class="flex-1 flex min-h-0 overflow-hidden">
        <!-- Left Sidebar: Profiles List -->
        <div class="w-64 shrink-0 border-r border-subtle p-3 flex flex-col bg-app/30 min-h-0">
          <button
            type="button"
            onclick={() => requestSwitchOrAction({ type: "new" })}
            class="w-full flex items-center justify-center gap-1.5 px-3 py-2 rounded-lg bg-accent-subtle hover:bg-accent-theme hover:text-white text-accent-theme text-xs font-semibold transition-colors mb-2.5 cursor-pointer shrink-0 border border-accent-theme/20"
          >
            <Plus size={14} />
            <span>New Workspace</span>
          </button>

          {#if workspaces.length > 4}
            <div class="relative mb-2 shrink-0">
              <label for="ws-search" class="sr-only">Filter workspaces</label>
              <Search size={12} class="absolute left-2.5 top-1/2 -translate-y-1/2 text-muted-theme pointer-events-none" />
              <input
                id="ws-search"
                type="text"
                bind:value={workspaceSearch}
                placeholder="Filter workspaces..."
                class="w-full pl-7 pr-2.5 py-1.5 bg-surface border border-theme-default rounded-lg text-[11px] text-primary-theme focus:outline-none focus:border-accent-theme"
              />
            </div>
          {/if}

          <div class="flex-1 overflow-y-auto space-y-1.5 pr-0.5">
            {#if isNewDraft}
              <div class="w-full text-left px-3 py-2 rounded-lg text-xs bg-sky-500/10 text-sky-300 font-medium border border-sky-500/35">
                <div class="flex items-center justify-between gap-1">
                  <span class="truncate">{editingWorkspace.name.trim() || "New Workspace"}</span>
                  <span class="text-[9px] px-1.5 py-0.2 rounded bg-sky-500/20 text-sky-200 uppercase tracking-wider shrink-0">Draft</span>
                </div>
                <div class="text-[10px] text-sky-300/75 font-mono truncate mt-0.5">
                  {editingWorkspace.path.trim() || "Choose folder..."}
                </div>
              </div>
            {/if}

            {#each filteredWorkspaces as ws (ws.id)}
              {@const isSelected = !isNewDraft && editingWorkspace.id === ws.id}
              {@const isCurrentActive = activeWorkspace?.id === ws.id}
              <div
                class="group relative rounded-lg border transition-colors {isSelected
                  ? 'bg-surface-elevated border-accent-theme/40 text-accent-theme'
                  : 'border-transparent hover:bg-surface-elevated/60 text-secondary-theme'}"
              >
                <button
                  type="button"
                  onclick={() => {
                    if (!isSelected) requestSwitchOrAction({ type: "switch", ws });
                  }}
                  class="w-full text-left px-3 py-2 pr-8 rounded-lg text-xs cursor-pointer"
                >
                  <div class="flex items-center gap-1.5">
                    {#if isCurrentActive}
                      <span class="w-1.5 h-1.5 rounded-full bg-emerald-400 shrink-0" title="Currently active workspace"></span>
                    {/if}
                    <span class="truncate font-medium {isSelected ? 'text-primary-theme' : ''}">{ws.name}</span>
                  </div>
                  <div class="text-[10px] text-muted-theme font-mono truncate flex items-center justify-between mt-0.5">
                    <span class="truncate mr-1">{getEffectiveModel(ws.model, activeEngine)}</span>
                    <span class="capitalize text-[9px] shrink-0 opacity-80">{ws.approval_mode || "default"}</span>
                  </div>
                </button>
                <button
                  type="button"
                  onclick={(e) => {
                    e.stopPropagation();
                    requestSwitchOrAction({ type: "duplicate", ws });
                  }}
                  class="absolute right-1.5 top-1/2 -translate-y-1/2 p-1 rounded opacity-0 group-hover:opacity-100 hover:bg-surface text-muted-theme hover:text-primary-theme transition-opacity cursor-pointer"
                  title="Duplicate workspace profile"
                  aria-label={`Duplicate ${ws.name}`}
                >
                  <Copy size={12} />
                </button>
              </div>
            {/each}
          </div>
        </div>

        <!-- Right Pane: Tabbed Editor + Sticky Footer -->
        <div class="flex-1 flex flex-col min-w-0 min-h-0">
          <!-- Sub-Navigation Tabs -->
          <div class="px-5 pt-2.5 border-b border-subtle flex items-center gap-1 bg-surface-elevated/20 shrink-0 overflow-x-auto">
            <button
              type="button"
              onclick={() => (activeTab = "essentials")}
              class="px-3 py-2 text-xs font-medium border-b-2 transition-colors cursor-pointer flex items-center gap-1.5 shrink-0 {activeTab === 'essentials' ? 'border-accent-theme text-accent-theme' : 'border-transparent text-secondary-theme hover:text-primary-theme'}"
            >
              <Cpu size={13} />
              <span>Essentials</span>
            </button>
            <button
              type="button"
              onclick={() => (activeTab = "agent")}
              class="px-3 py-2 text-xs font-medium border-b-2 transition-colors cursor-pointer flex items-center gap-1.5 shrink-0 {activeTab === 'agent' ? 'border-accent-theme text-accent-theme' : 'border-transparent text-secondary-theme hover:text-primary-theme'}"
            >
              <Sliders size={13} />
              <span>Agent & Prompt</span>
              {#if hasAdvancedAgentConfig}
                <span class="w-1.5 h-1.5 rounded-full bg-accent-theme"></span>
              {/if}
            </button>
            <button
              type="button"
              onclick={() => (activeTab = "files")}
              class="px-3 py-2 text-xs font-medium border-b-2 transition-colors cursor-pointer flex items-center gap-1.5 shrink-0 {activeTab === 'files' ? 'border-accent-theme text-accent-theme' : 'border-transparent text-secondary-theme hover:text-primary-theme'}"
            >
              <FileDiff size={13} />
              <span>Files & Skills</span>
              {#if editingWorkspace.track_file_changes || (activeDescriptor.capabilities.supportsSkillSync && pathProbe?.has_gemini_skills)}
                <span class="w-1.5 h-1.5 rounded-full bg-amber-400"></span>
              {/if}
            </button>
            <button
              type="button"
              onclick={() => (activeTab = "app")}
              class="px-3 py-2 text-xs font-medium border-b-2 transition-colors cursor-pointer flex items-center gap-1.5 shrink-0 {activeTab === 'app' ? 'border-accent-theme text-accent-theme' : 'border-transparent text-secondary-theme hover:text-primary-theme'}"
            >
              <Bell size={13} />
              <span>Notifications</span>
            </button>
          </div>

          <!-- Scrollable Tab Body -->
          <div class="flex-1 p-5 overflow-y-auto space-y-4 text-xs">
            {#if activeTab === "essentials"}
              <!-- 1. Working Directory (Pick Folder First) -->
              <div>
                <div class="flex items-center justify-between mb-1">
                  <label for="ws-path" class="block text-primary-theme font-medium">
                    Working Directory (Absolute Path)
                  </label>
                  <span class="text-[10px] text-muted-theme">
                    {activeDescriptor.workspaceInstructionsHint}
                  </span>
                </div>
                <div class="flex items-center gap-2">
                  <input
                    bind:this={pathInputEl}
                    id="ws-path"
                    type="text"
                    value={editingWorkspace.path}
                    oninput={onPathInput}
                    placeholder="C:\Projects\my-repo"
                    class="flex-1 px-3 py-2 bg-app border border-theme-default rounded-lg text-primary-theme font-mono focus:outline-none focus:border-accent-theme"
                  />
                  <button
                    type="button"
                    onclick={handleBrowseFolder}
                    disabled={isBrowsingFolder}
                    class="flex items-center gap-1.5 px-3 py-2 rounded-lg bg-surface-elevated hover:bg-surface-hover border border-theme-default text-primary-theme font-medium transition-colors cursor-pointer disabled:opacity-50 shrink-0"
                  >
                    <FolderOpen size={14} class="text-accent-theme" />
                    <span>{isBrowsingFolder ? "Opening..." : "Browse…"}</span>
                  </button>
                </div>

                <!-- Live Inline Path Validation & Auto-Detected Project Signals -->
                {#if isProbingPath}
                  <div class="mt-2 text-[11px] text-muted-theme flex items-center gap-1.5">
                    <RefreshCw size={11} class="animate-spin" />
                    <span>Checking folder and project context...</span>
                  </div>
                {:else if pathProbe}
                  {#if pathProbe.validation.is_valid}
                    <div class="mt-2 flex flex-wrap items-center gap-1.5">
                      <span class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-medium bg-emerald-500/10 text-emerald-300 border border-emerald-500/30">
                        <CheckCircle2 size={11} />
                        <span>Valid directory</span>
                      </span>
                      {#if pathProbe.has_git}
                        <span class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-medium bg-sky-500/10 text-sky-300 border border-sky-500/30">
                          <GitBranch size={11} />
                          <span>Git repo</span>
                        </span>
                      {:else if !editingWorkspace.track_file_changes}
                        <button
                          type="button"
                          onclick={() => {
                            editingWorkspace.track_file_changes = true;
                          }}
                          class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-medium bg-amber-500/15 hover:bg-amber-500/25 text-amber-300 border border-amber-500/35 transition-colors cursor-pointer"
                          title="No .git directory detected — enable snapshot diffs & 1-click revert"
                        >
                          <FileDiff size={11} />
                          <span>Non-Git folder • Enable File Snapshots?</span>
                        </button>
                      {/if}
                      {#if pathProbe.has_agents_md || pathProbe.has_gemini_md}
                        <span class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-medium bg-purple-500/10 text-purple-300 border border-purple-500/30">
                          <FileText size={11} />
                          <span>{pathProbe.has_agents_md && pathProbe.has_gemini_md ? "AGENTS.md & GEMINI.md" : pathProbe.has_agents_md ? "AGENTS.md" : "GEMINI.md"}</span>
                        </span>
                      {/if}
                      {#each pathProbe.detected_stack as stack}
                        <span class="px-2 py-0.5 rounded-full text-[10px] font-mono bg-surface-elevated text-secondary-theme border border-subtle">
                          {stack}
                        </span>
                      {/each}
                      {#if activeDescriptor.capabilities.supportsSkillSync && pathProbe.has_gemini_skills && !pathProbe.has_agents_skills}
                        <button
                          type="button"
                          onclick={() => (activeTab = "files")}
                          class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-medium bg-purple-500/15 hover:bg-purple-500/25 text-purple-300 border border-purple-500/35 transition-colors cursor-pointer"
                        >
                          <Sparkles size={11} />
                          <span>Sync .gemini/skills for Antigravity</span>
                        </button>
                      {/if}
                    </div>
                  {:else}
                    <div class="mt-2 p-2.5 rounded-lg bg-rose-500/10 border border-rose-500/30 flex items-center justify-between gap-3 text-[11px] text-rose-200">
                      <div class="flex items-start gap-2">
                        <AlertTriangle size={14} class="text-rose-400 shrink-0 mt-0.5" />
                        <span>{pathProbe.validation.error_message || "Invalid workspace directory."}</span>
                      </div>
                      {#if canOfferCreateFolder}
                        <button
                          type="button"
                          onclick={handleCreateMissingFolder}
                          disabled={isCreatingFolder}
                          class="flex items-center gap-1 px-2.5 py-1 rounded-md bg-accent-theme hover:bg-accent-hover text-white font-medium transition-colors cursor-pointer shrink-0 disabled:opacity-50"
                        >
                          <FolderPlus size={12} />
                          <span>{isCreatingFolder ? "Creating..." : "Create Folder"}</span>
                        </button>
                      {/if}
                    </div>
                  {/if}
                {/if}
              </div>

              <!-- 2. Profile Name -->
              <div>
                <label for="ws-name" class="block text-primary-theme font-medium mb-1">
                  Profile Name
                </label>
                <input
                  id="ws-name"
                  type="text"
                  bind:value={editingWorkspace.name}
                  placeholder="e.g. billing-service"
                  class="w-full px-3 py-2 bg-app border border-theme-default rounded-lg text-primary-theme focus:outline-none focus:border-accent-theme"
                />
              </div>

              <!-- 3. Policy Approval Mode (Compact Segmented Control + 1-Line Summary) -->
              <div>
                <div class="flex items-center justify-between mb-1.5">
                  <span id="ws-approval-label" class="text-primary-theme font-medium">Execution Approval Policy</span>
                  <span class="text-[10px] font-medium px-2 py-0.5 rounded-full {selectedApprovalModeConfig.tagClass}">
                    {selectedApprovalModeConfig.label}
                  </span>
                </div>
                <div
                  role="group"
                  aria-labelledby="ws-approval-label"
                  class="flex items-center gap-1 p-1 rounded-xl bg-app border border-subtle"
                >
                  {#each availableApprovalModes as mode (mode.id)}
                    {@const ModeIcon = mode.icon}
                    {@const isModeSelected = (editingWorkspace.approval_mode || "default") === mode.id}
                    <button
                      type="button"
                      aria-pressed={isModeSelected}
                      title={mode.description}
                      onclick={() => (editingWorkspace.approval_mode = mode.id)}
                      class="flex-1 min-w-0 flex items-center justify-center gap-1.5 px-2 py-1.5 rounded-lg text-[11px] font-medium transition-all cursor-pointer {isModeSelected
                        ? 'bg-surface-elevated text-primary-theme border border-accent-theme/50 shadow-xs'
                        : 'text-secondary-theme hover:text-primary-theme hover:bg-surface-elevated/40 border border-transparent'}"
                    >
                      <ModeIcon size={12} class="{mode.iconClass} shrink-0" />
                      <span class="truncate">{mode.shortLabel}</span>
                    </button>
                  {/each}
                </div>
                {#if selectedApprovalModeConfig}
                  {@const SelectedModeIcon = selectedApprovalModeConfig.icon}
                  <p class="mt-1.5 flex items-center gap-1.5 text-[11px] text-muted-theme leading-snug">
                    <SelectedModeIcon size={12} class="{selectedApprovalModeConfig.iconClass} shrink-0" />
                    <span>{selectedApprovalModeConfig.description}</span>
                  </p>
                {/if}
              </div>

              <!-- 4. Preferred Model -->
              <div>
                <div class="flex items-center justify-between mb-1">
                  <label for="ws-model" class="block text-primary-theme font-medium">Preferred Model</label>
                  <button
                    type="button"
                    onclick={handleDiscoverModels}
                    disabled={isDiscoveringModels}
                    class="flex items-center gap-1.5 px-2 py-0.5 text-[11px] font-medium text-accent-theme hover:bg-accent-subtle/30 rounded transition-colors cursor-pointer disabled:opacity-50"
                    title={activeEngine === "agy" ? "Discover active models from Antigravity CLI (agy models)" : "Discover active models from Gemini CLI v0.67+ models command"}
                  >
                    <RefreshCw size={11} class={isDiscoveringModels ? "animate-spin" : ""} />
                    <span>{isDiscoveringModels ? "Querying CLI..." : activeEngine === "agy" ? "Discover Live Models (agy)" : "Discover Live Models (v0.67+)"}</span>
                  </button>
                </div>
                <select
                  id="ws-model"
                  value={selectedDropdownValue}
                  onchange={onSelectModelChange}
                  class="w-full px-3 py-2 bg-app border border-theme-default rounded-lg text-primary-theme focus:outline-none focus:border-accent-theme font-mono"
                >
                  {#if liveDiscoveredModels.length > 0}
                    <optgroup label={`Discovered via ${activeDescriptor.shortName} (Live API)`}>
                      {#each liveDiscoveredModels as lm}
                        {@const ctxLabel = formatContextLimitLabel(lm.input_token_limit)}
                        <option value={lm.id}>
                          {lm.id} {ctxLabel ? `(${ctxLabel})` : ""} {lm.is_default ? "— Default" : ""}
                        </option>
                      {/each}
                    </optgroup>
                  {/if}
                  {#each effectiveModelGroups as group}
                    <optgroup label={group.group}>
                      {#each group.options as opt}
                        <option value={opt.value}>{opt.label}</option>
                      {/each}
                    </optgroup>
                  {/each}
                  <optgroup label="Manual / Custom Model">
                    <option value="manual_custom">Manual (Specify any available model...)</option>
                  </optgroup>
                </select>

                {#if selectedDropdownValue === "manual_custom"}
                  <div class="mt-2.5 p-2.5 bg-app border border-accent-subtle rounded-lg">
                    <label for="ws-custom-model" class="block text-[11px] font-medium text-accent-theme mb-1">
                      Custom Model Name / Identifier:
                    </label>
                    <input
                      id="ws-custom-model"
                      type="text"
                      value={customModelInput}
                      oninput={onCustomModelInput}
                      placeholder={activeEngine === "agy" ? "e.g. gemini-4-argon, gemini-3.8-flash-high, or custom model" : "e.g. gemini-3.8-flash, gemini-3.5-flash-lite, or custom model"}
                      class="w-full px-2.5 py-1.5 bg-surface border border-theme-default rounded text-primary-theme font-mono text-xs focus:outline-none focus:border-accent-theme"
                    />
                    <span class="text-[10px] text-muted-theme mt-1 block">
                      Passed directly to {activeEngine === "agy" ? "Antigravity CLI (agy)" : "Gemini CLI"} via ACP protocol.
                    </span>
                  </div>
                {/if}

                {#if discoveryStatus}
                  <div class="mt-2.5 p-2 rounded-lg text-xs {discoveryStatus.type === 'success' ? 'bg-emerald-500/10 text-emerald-300 border border-emerald-500/30' : discoveryStatus.type === 'error' ? 'bg-rose-500/10 text-rose-300 border border-rose-500/30' : 'bg-surface border border-subtle text-secondary-theme'}">
                    {discoveryStatus.text}
                  </div>
                {/if}

                {#if isDeprecatedModel(editingWorkspace.model)}
                  <div class="mt-2.5 p-3 bg-amber-500/10 border border-amber-500/30 rounded-lg flex items-start gap-2.5 text-xs text-amber-200">
                    <AlertTriangle size={16} class="text-amber-400 shrink-0 mt-0.5" />
                    <div class="flex-1">
                      <div class="font-semibold text-amber-300">Deprecated Model Selected</div>
                      <p class="text-[11px] text-amber-200/90 mt-0.5 leading-relaxed">
                        <code class="font-mono text-amber-300">{editingWorkspace.model}</code> has been retired upstream by Google and API calls will fail. Upgrade to <strong>gemini-3.8-flash</strong> or <strong>gemini-3.5-flash-lite</strong>.
                      </p>
                      <div class="flex items-center gap-2 mt-2">
                        <button
                          type="button"
                          onclick={() => {
                            editingWorkspace.model = "gemini-3.8-flash";
                            selectedDropdownValue = "gemini-3.8-flash";
                            customModelInput = "";
                          }}
                          class="px-2.5 py-1 text-[11px] font-medium bg-amber-500/25 hover:bg-amber-500/40 text-amber-100 rounded border border-amber-500/50 transition-colors cursor-pointer"
                        >
                          Migrate to gemini-3.8-flash
                        </button>
                        <button
                          type="button"
                          onclick={() => {
                            editingWorkspace.model = "gemini-3.5-flash-lite";
                            selectedDropdownValue = "gemini-3.5-flash-lite";
                            customModelInput = "";
                          }}
                          class="px-2.5 py-1 text-[11px] font-medium bg-surface-elevated hover:bg-surface-hover text-secondary-theme hover:text-primary-theme rounded border border-subtle transition-colors cursor-pointer"
                        >
                          Migrate to gemini-3.5-flash-lite
                        </button>
                      </div>
                    </div>
                  </div>
                {:else if isRetiringVertexModel(editingWorkspace.model)}
                  <div class="mt-2.5 p-2.5 bg-blue-500/10 border border-blue-500/30 rounded-lg flex items-start gap-2 text-xs text-blue-200">
                    <AlertTriangle size={15} class="text-blue-400 shrink-0 mt-0.5" />
                    <div class="flex-1 text-[11px] leading-relaxed">
                      <span class="font-medium text-blue-300">Upcoming Transition:</span>
                      <span class="text-blue-200/90 ml-1">
                        gemini-2.5-flash-lite remains available on AI Studio, but Vertex AI is retiring older 2.5 checkpoints. Consider upgrading to <strong>gemini-3.5-flash-lite</strong>.
                      </span>
                    </div>
                  </div>
                {/if}
              </div>

            {:else if activeTab === "agent"}
              {#if activeDescriptor.capabilities.supportsReasoningEffort || activeDescriptor.capabilities.supportsAgentPersona}
                <div class="grid grid-cols-1 sm:grid-cols-2 gap-3 p-3.5 bg-app/70 border border-purple-500/25 rounded-xl">
                  <div>
                    <label for="ws-reasoning-effort" class="block text-primary-theme font-medium mb-1">
                      Reasoning Effort (<code class="font-mono text-[10px] text-purple-400">--effort</code>)
                    </label>
                    <select
                      id="ws-reasoning-effort"
                      bind:value={editingWorkspace.reasoning_effort}
                      class="w-full px-2.5 py-1.5 bg-surface border border-theme-default rounded-lg text-primary-theme font-mono text-xs focus:outline-none focus:border-accent-theme"
                    >
                      <option value="">Default (Model Standard)</option>
                      <option value="low">low</option>
                      <option value="medium">medium</option>
                      <option value="high">high</option>
                      <option value="xhigh">xhigh</option>
                      <option value="max">max</option>
                    </select>
                  </div>
                  <div>
                    <label for="ws-agent-persona" class="block text-primary-theme font-medium mb-1">
                      Agent Persona (<code class="font-mono text-[10px] text-purple-400">--agent</code>)
                    </label>
                    <input
                      id="ws-agent-persona"
                      type="text"
                      bind:value={editingWorkspace.agent_name}
                      placeholder="Optional (e.g. architect, reviewer)"
                      class="w-full px-2.5 py-1.5 bg-surface border border-theme-default rounded-lg text-primary-theme font-mono text-xs focus:outline-none focus:border-accent-theme"
                    />
                  </div>
                </div>
              {/if}

              <div class="grid grid-cols-1 sm:grid-cols-2 gap-3 p-3.5 bg-app border border-subtle rounded-xl">
                <div>
                  <label for="ws-additional-dirs" class="block text-primary-theme font-medium mb-1">
                    Additional Directories (<code class="font-mono text-[10px] text-secondary-theme">{activeEngine === "agy" ? "--add-dir" : "--include-directories"}</code>)
                  </label>
                  <input
                    id="ws-additional-dirs"
                    type="text"
                    bind:value={editingWorkspace.additional_dirs}
                    placeholder="e.g. ../shared; D:\CommonLibs"
                    class="w-full px-2.5 py-1.5 bg-surface border border-theme-default rounded-lg text-primary-theme font-mono text-xs focus:outline-none focus:border-accent-theme"
                  />
                  <span class="text-[10px] text-muted-theme mt-1 block">Semicolon or comma-separated extra workspace paths.</span>
                </div>

                <div class="flex items-center justify-between px-3 py-2 rounded-lg bg-surface/60 border border-subtle">
                  <div class="pr-3">
                    <div class="flex items-center gap-1.5 font-medium text-primary-theme text-xs">
                      <Shield size={13} class="text-teal-400" />
                      <span>OS Sandbox (<code class="font-mono text-[10px]">--sandbox</code>)</span>
                    </div>
                    <p class="text-[10px] text-muted-theme mt-0.5">
                      Isolates shell execution inside CLI sandbox boundaries.
                    </p>
                  </div>
                  <button
                    type="button"
                    role="switch"
                    aria-checked={editingWorkspace.sandbox_enabled ?? false}
                    aria-label="Toggle OS Sandbox"
                    onclick={() => {
                      editingWorkspace.sandbox_enabled = !editingWorkspace.sandbox_enabled;
                    }}
                    class="relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none {editingWorkspace.sandbox_enabled ? 'bg-teal-500' : 'bg-slate-400/40 dark:bg-slate-700'}"
                  >
                    <span
                      aria-hidden="true"
                      class="pointer-events-none inline-block h-4 w-4 transform rounded-full bg-white shadow-md ring-0 transition duration-200 ease-in-out {editingWorkspace.sandbox_enabled ? 'translate-x-4' : 'translate-x-0'}"
                    ></span>
                  </button>
                </div>
              </div>

              <div>
                <label for="ws-prompt" class="block text-primary-theme font-medium mb-1">
                  Custom System Instructions (Optional)
                </label>
                <textarea
                  id="ws-prompt"
                  bind:value={editingWorkspace.system_prompt}
                  rows="6"
                  placeholder="e.g. Always write production C# code following Microsoft coding conventions..."
                  class="w-full px-3 py-2 bg-app border border-theme-default rounded-lg text-primary-theme focus:outline-none focus:border-accent-theme resize-y"
                ></textarea>
                <span class="text-[11px] text-muted-theme mt-1 block">
                  Appended to workspace sessions alongside repository <code class="font-mono">AGENTS.md</code> / <code class="font-mono">GEMINI.md</code> files.
                </span>
              </div>

            {:else if activeTab === "files"}
              <!-- Session File Changes & Snapshots Tracking -->
              <div class="p-4 bg-app border border-subtle rounded-xl space-y-2.5">
                <div class="flex items-center justify-between">
                  <div class="flex items-center gap-2">
                    <FileDiff size={15} class="text-amber-400" />
                    <span class="font-semibold text-primary-theme">Track Session File Changes & Snapshots</span>
                  </div>
                  <div class="flex items-center gap-2.5">
                    <span class="text-xs font-mono font-semibold transition-colors duration-150 {editingWorkspace.track_file_changes ? 'text-emerald-400' : 'text-muted-theme'}">
                      {editingWorkspace.track_file_changes ? "ON" : "OFF"}
                    </span>
                    <button
                      type="button"
                      role="switch"
                      aria-checked={editingWorkspace.track_file_changes ?? false}
                      aria-label="Toggle Track Session File Changes"
                      onclick={() => {
                        editingWorkspace.track_file_changes = !editingWorkspace.track_file_changes;
                      }}
                      class="relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none focus-visible:ring-2 focus-visible:ring-emerald-500 {editingWorkspace.track_file_changes ? 'bg-emerald-500' : 'bg-slate-400/40 dark:bg-slate-700'}"
                    >
                      <span
                        aria-hidden="true"
                        class="pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow-md ring-0 transition duration-200 ease-in-out {editingWorkspace.track_file_changes ? 'translate-x-5' : 'translate-x-0'}"
                      ></span>
                    </button>
                  </div>
                </div>
                <p class="text-[11px] text-muted-theme leading-relaxed">
                  Recommended for folders not under Git source control. Snapshots files before agent edits, calculates visual line diffs (+/-), and enables 1-click restore. Keep disabled for Git repositories to ensure maximum file operation speed.
                </p>
              </div>

              <!-- Antigravity Skills Compatibility Sync Card -->
              {#if activeDescriptor.capabilities.supportsSkillSync}
                <div class="p-4 bg-app border border-subtle rounded-xl space-y-2.5">
                  <div class="flex items-center justify-between gap-3">
                    <div class="flex items-center gap-2">
                      <Sparkles size={15} class="text-purple-400" />
                      <span class="font-semibold text-primary-theme">Antigravity Skills Compatibility Sync</span>
                    </div>
                    <button
                      type="button"
                      onclick={handleSyncSkills}
                      disabled={isSyncingSkills || !editingWorkspace.path.trim()}
                      class="px-3 py-1.5 text-xs rounded-lg bg-purple-600 hover:bg-purple-500 text-white font-medium transition-colors cursor-pointer flex items-center gap-1.5 disabled:opacity-50 shrink-0"
                    >
                      {#if isSyncingSkills}
                        <RefreshCw size={12} class="animate-spin" />
                        <span>Syncing...</span>
                      {:else}
                        <Sparkles size={12} />
                        <span>Sync to .agents/skills</span>
                      {/if}
                    </button>
                  </div>
                  <p class="text-[11px] text-muted-theme leading-relaxed">
                    Copies workspace skills from <code class="font-mono text-secondary-theme">.gemini/skills/</code> to <code class="font-mono text-secondary-theme">.agents/skills/</code> so Antigravity CLI recognizes them natively.
                  </p>
                  {#if skillsSyncResult}
                    <div class="p-2.5 rounded-lg text-xs {skillsSyncResult.success ? 'bg-purple-500/10 text-purple-200 border border-purple-500/25' : 'bg-rose-500/10 text-rose-300 border border-rose-500/25'}">
                      {skillsSyncResult.message}
                    </div>
                  {/if}
                </div>
              {:else}
                <div class="p-4 bg-app/50 border border-subtle rounded-xl text-[11px] text-muted-theme">
                  Switch active CLI engine to <strong>Antigravity CLI (agy)</strong> to sync <code class="font-mono">.gemini/skills/</code> into <code class="font-mono">.agents/skills/</code>.
                </div>
              {/if}

            {:else}
              <!-- Global App Preferences (Explicitly separated from per-workspace settings) -->
              <div class="p-4 bg-app border border-subtle rounded-xl space-y-2.5">
                <div class="flex items-center justify-between">
                  <div class="flex items-center gap-2">
                    <Bell size={15} class="text-accent-theme" />
                    <div>
                      <span class="font-semibold text-primary-theme block">Desktop Toast Notifications</span>
                      <span class="text-[10px] text-accent-theme font-medium">Global app preference (applies across all workspaces)</span>
                    </div>
                  </div>
                  <div class="flex items-center gap-2.5">
                    <span class="text-xs font-mono font-semibold transition-colors duration-150 {notificationsEnabled ? 'text-emerald-400' : 'text-muted-theme'}">
                      {notificationsEnabled ? "ON" : "OFF"}
                    </span>
                    <button
                      type="button"
                      role="switch"
                      aria-checked={notificationsEnabled}
                      aria-label="Toggle Desktop Toast Notifications"
                      onclick={() => {
                        notificationsEnabled = !notificationsEnabled;
                        setNotificationSetting(notificationsEnabled);
                      }}
                      class="relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none focus-visible:ring-2 focus-visible:ring-emerald-500 {notificationsEnabled ? 'bg-emerald-500' : 'bg-slate-400/40 dark:bg-slate-700'}"
                    >
                      <span
                        aria-hidden="true"
                        class="pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow-md ring-0 transition duration-200 ease-in-out {notificationsEnabled ? 'translate-x-5' : 'translate-x-0'}"
                      ></span>
                    </button>
                  </div>
                </div>
                <p class="text-[11px] text-muted-theme leading-relaxed">
                  Receive native Windows toast notifications when background turns finish or require tool permission while you are working in other apps.
                </p>
                <div class="pt-1">
                  <button
                    type="button"
                    onclick={handleTestToast}
                    class="px-3 py-1.5 text-[11px] rounded-lg bg-surface hover:bg-surface-elevated text-secondary-theme hover:text-primary-theme border border-theme-default transition-colors flex items-center gap-1.5 cursor-pointer"
                  >
                    <BellRing size={12} class="text-accent-theme" />
                    <span>Test Windows Toast</span>
                  </button>
                </div>
              </div>
            {/if}
          </div>

          <!-- Sticky Bottom Footer (Always Visible) -->
          <div class="px-5 py-3 border-t border-subtle bg-surface-elevated/40 flex items-center justify-between gap-3 shrink-0 text-xs">
            <div class="flex items-center gap-2">
              {#if !isNewDraft}
                <button
                  type="button"
                  onclick={handleDelete}
                  disabled={workspaces.length <= 1}
                  title={workspaces.length <= 1 ? "At least one workspace profile must remain" : "Delete this workspace profile"}
                  class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-rose-400 hover:bg-rose-500/10 transition-colors cursor-pointer disabled:opacity-40 disabled:cursor-not-allowed"
                >
                  <Trash2 size={14} />
                  <span>Delete</span>
                </button>
                <button
                  type="button"
                  onclick={() => requestSwitchOrAction({ type: "duplicate", ws: editingWorkspace })}
                  class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-secondary-theme hover:text-primary-theme hover:bg-surface-hover border border-theme-default transition-colors cursor-pointer"
                >
                  <Copy size={13} />
                  <span>Duplicate</span>
                </button>
              {/if}
            </div>

            <div class="flex items-center gap-2">
              <button
                type="button"
                onclick={handleAttemptClose}
                class="px-3 py-1.5 rounded-lg text-secondary-theme hover:text-primary-theme hover:bg-surface-hover border border-theme-default transition-colors cursor-pointer"
              >
                {isDirty ? "Discard & Close" : "Cancel"}
              </button>

              {#if isEditingInactiveSaved}
                <button
                  type="button"
                  onclick={() => handleSave(true)}
                  disabled={!canSave}
                  class="flex items-center gap-1.5 px-3.5 py-1.5 rounded-lg font-medium border border-accent-theme/40 text-accent-theme bg-accent-subtle hover:bg-accent-theme/20 transition-colors cursor-pointer disabled:opacity-50"
                >
                  <Check size={14} />
                  <span>Save & Switch</span>
                </button>
              {/if}

              <button
                type="button"
                onclick={() => handleSave(isNewDraft)}
                disabled={!canSave}
                class="flex items-center gap-1.5 px-4 py-1.5 rounded-lg font-semibold transition-colors shadow-xs {canSave
                  ? 'bg-accent-theme hover:bg-accent-hover text-white cursor-pointer'
                  : 'bg-slate-600 text-slate-400 cursor-not-allowed opacity-60'}"
                title={!canSave ? "Enter a workspace name and folder path to enable saving" : ""}
              >
                <Check size={14} />
                <span>
                  {isSaving
                    ? "Saving…"
                    : isNewDraft
                      ? "Create & Open Workspace"
                      : "Save Changes"}
                </span>
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
{/if}
