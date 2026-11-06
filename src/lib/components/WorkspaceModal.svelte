<script lang="ts">
  import type { Workspace, ApprovalMode, WorkspacePathValidation, CliModelInfo, CliModelDiscoveryResult } from "$lib/types";
  import { X, Plus, Trash2, Check, Bell, BellRing, FileDiff, AlertTriangle, RefreshCw } from "lucide-svelte";
  import { isNotificationSettingEnabled, setNotificationSetting, notifyBackgroundTaskComplete } from "$lib/utils/notifications";
  import { dialogManager } from "$lib/dialog.svelte";
  import { invoke } from "@tauri-apps/api/core";

  const APPROVAL_MODES: {
    value: ApprovalMode;
    label: string;
    description: string;
  }[] = [
    {
      value: "default",
      label: "Ask Permission (Default)",
      description: "Requires explicit user confirmation before every file edit and command execution",
    },
    {
      value: "auto_edit",
      label: "Auto-Edit",
      description: "Auto-approves safe file edits; interactive prompt for shell and terminal commands",
    },
    {
      value: "yolo",
      label: "YOLO (Full Autonomous)",
      description: "Auto-approves all tool actions including terminal execution and edits without prompts",
    },
    {
      value: "plan",
      label: "Plan Mode",
      description: "Read-only planning and analysis; file edits and terminal commands are disallowed",
    },
  ];

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

  let {
    isOpen = false,
    activeWorkspace = null,
    workspaces = [],
    onClose,
    onSaveWorkspace,
    onDeleteWorkspace,
  }: {
    isOpen: boolean;
    activeWorkspace?: Workspace | null;
    workspaces: Workspace[];
    onClose: () => void;
    onSaveWorkspace: (ws: Workspace) => void;
    onDeleteWorkspace: (id: string) => void;
  } = $props();

  let editingWorkspace: Workspace = $state({
    id: "",
    name: "",
    path: "C:\\",
    model: "auto",
    approval_mode: "default",
    track_file_changes: false,
    system_prompt: "",
    created_at: new Date().toISOString(),
  });

  let selectedDropdownValue = $state("auto");
  let customModelInput = $state("");
  let lastSyncedWorkspaceId = $state<string | null>(null);
  let isSaving = $state(false);
  let notificationsEnabled = $state(true);

  let isDiscoveringModels = $state(false);
  let discoveryStatus = $state<{ type: "success" | "info" | "error"; text: string } | null>(null);
  let liveDiscoveredModels = $state<CliModelInfo[]>([]);

  async function handleDiscoverModels() {
    isDiscoveringModels = true;
    discoveryStatus = null;
    try {
      const res = await invoke<CliModelDiscoveryResult>("discover_gemini_models");
      if (res.supported && res.models.length > 0) {
        liveDiscoveredModels = res.models;
        discoveryStatus = {
          type: "success",
          text: `Discovered ${res.models.length} active models directly via Gemini CLI (v0.67+).`,
        };
      } else {
        discoveryStatus = {
          type: "info",
          text: res.message || "Model discovery returned no models. Standard models remain available.",
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

  $effect(() => {
    if (isOpen) {
      notificationsEnabled = isNotificationSettingEnabled();
    }
  });

  async function handleTestToast() {
    await notifyBackgroundTaskComplete(
      "Gemini Desktop",
      "Native Windows Toast Notifications are enabled and working properly!",
      true
    );
  }

  let canSave = $derived(
    editingWorkspace.name.trim().length > 0 &&
    editingWorkspace.path.trim().length > 0 &&
    !isSaving
  );

  let wasOpen = false;
  $effect(() => {
    if (isOpen && !wasOpen) {
      wasOpen = true;
      const target = (activeWorkspace && workspaces.find((w) => w.id === activeWorkspace.id)) || activeWorkspace || workspaces[0];
      if (target) {
        editingWorkspace = {
          ...target,
          approval_mode: target.approval_mode || "default",
          track_file_changes: target.track_file_changes ?? false,
        };
      } else {
        startNew();
      }
    } else if (!isOpen) {
      wasOpen = false;
    }
  });

  $effect(() => {
    // Only re-sync dropdown state when editing workspace changes (avoids clobbering manual typing)
    if (editingWorkspace.id !== lastSyncedWorkspaceId) {
      lastSyncedWorkspaceId = editingWorkspace.id;
      const isKnown = MODEL_GROUPS.some((g) =>
        g.options.some((o) => o.value === editingWorkspace.model)
      );
      if (isKnown) {
        selectedDropdownValue = editingWorkspace.model;
        customModelInput = "";
      } else {
        selectedDropdownValue = "manual_custom";
        customModelInput = editingWorkspace.model || "";
      }
    }
  });

  function onSelectModelChange(e: Event) {
    const target = e.target as HTMLSelectElement;
    selectedDropdownValue = target.value;
    if (target.value === "manual_custom") {
      if (!customModelInput.trim()) {
        const isKnown = MODEL_GROUPS.some((g) =>
          g.options.some((o) => o.value === editingWorkspace.model)
        );
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

  async function startNew() {
    let defaultPath = "";
    try {
      defaultPath = await invoke<string>("get_default_workspace_dir");
    } catch {
      defaultPath = "";
    }

    editingWorkspace = {
      id: "ws-" + Math.random().toString(36).substring(2, 9),
      name: "New Workspace",
      path: defaultPath,
      model: "auto",
      approval_mode: "default",
      track_file_changes: false,
      system_prompt: "",
      created_at: new Date().toISOString(),
    };
  }

  async function handleSave() {
    if (!canSave) return;
    if (selectedDropdownValue === "manual_custom") {
      const manualModel = customModelInput.trim();
      if (!manualModel) {
        dialogManager.alert("Please specify a custom model name or select an option from the list.", "Model Name Required");
        return;
      }
      editingWorkspace.model = manualModel;
    } else {
      editingWorkspace.model = selectedDropdownValue;
    }

    isSaving = true;
    try {
      const val = await invoke<WorkspacePathValidation>("validate_workspace_path", { path: editingWorkspace.path.trim() });
      if (!val.is_valid) {
        await dialogManager.alert(
          val.error_message || `The path "${editingWorkspace.path.trim()}" is invalid for a workspace.`,
          val.exists ? "Restricted Directory (EDR Security Protection)" : "Folder Not Found"
        );
        return;
      }

      editingWorkspace.approval_mode = editingWorkspace.approval_mode || "default";
      onSaveWorkspace({ ...editingWorkspace });
    } catch (err) {
      await dialogManager.alert(`Failed to verify directory: ${err}`, "Validation Error");
    } finally {
      isSaving = false;
    }
  }
</script>

{#if isOpen}
  <div
    class="fixed inset-0 bg-black/60 backdrop-blur-xs z-50 flex items-center justify-center p-4"
    onclick={onClose}
    onkeydown={(e) => e.key === "Escape" && onClose()}
    role="presentation"
  >
    <div
      class="w-full max-w-2xl bg-surface border border-theme-default rounded-2xl shadow-2xl overflow-hidden flex flex-col max-h-[85vh]"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
      role="dialog"
      aria-modal="true"
      tabindex="-1"
    >
      <!-- Header -->
      <div class="px-5 py-4 border-b border-subtle flex items-center justify-between bg-surface-elevated/40">
        <div>
          <h2 class="text-sm font-semibold text-primary-theme">Workspace Profiles</h2>
          <p class="text-xs text-secondary-theme">Configure directory contexts, models, and custom prompts.</p>
        </div>
        <button onclick={onClose} class="p-1 text-secondary-theme hover:text-primary-theme rounded" aria-label="Close modal">
          <X size={16} />
        </button>
      </div>

      <!-- Content -->
      <div class="flex-1 flex overflow-hidden">
        <!-- Workspaces List -->
        <div class="w-1/3 border-r border-subtle p-3 space-y-1.5 overflow-y-auto">
          <button
            onclick={startNew}
            class="w-full flex items-center gap-1.5 px-3 py-2 rounded-lg bg-accent-subtle hover:bg-accent-theme hover:text-white text-accent-theme text-xs font-medium transition-colors mb-2"
          >
            <Plus size={14} />
            <span>Add Workspace</span>
          </button>

          {#each workspaces as ws}
            <button
              onclick={() => (editingWorkspace = { ...ws, approval_mode: ws.approval_mode || "default" })}
              class="w-full text-left px-3 py-2 rounded-lg text-xs transition-colors {editingWorkspace.id === ws.id ? 'bg-surface-elevated text-accent-theme font-medium border border-subtle' : 'text-secondary-theme hover:bg-surface-elevated/50'}"
            >
              <div class="truncate">{ws.name}</div>
              <div class="text-[10px] text-muted-theme font-mono truncate flex items-center justify-between">
                <span class="truncate mr-1">{ws.model}</span>
                <span class="capitalize text-[9px] shrink-0 opacity-75">{ws.approval_mode || "default"}</span>
              </div>
            </button>
          {/each}
        </div>

        <!-- Form Details -->
        <div class="flex-1 p-5 overflow-y-auto space-y-4 text-xs">
          <div>
            <label for="ws-name" class="block text-primary-theme font-medium mb-1">Profile Name</label>
            <input
              id="ws-name"
              type="text"
              bind:value={editingWorkspace.name}
              class="w-full px-3 py-2 bg-app border border-theme-default rounded-lg text-primary-theme focus:outline-none focus:border-accent-theme"
            />
          </div>

          <div>
            <label for="ws-path" class="block text-primary-theme font-medium mb-1">Working Directory (Absolute Path)</label>
            <input
              id="ws-path"
              type="text"
              bind:value={editingWorkspace.path}
              class="w-full px-3 py-2 bg-app border border-theme-default rounded-lg text-primary-theme font-mono focus:outline-none focus:border-accent-theme"
            />
            <span class="text-[11px] text-muted-theme mt-1 block">Gemini CLI will run inside this directory and auto-read GEMINI.md.</span>
          </div>

          <div>
            <div class="flex items-center justify-between mb-1">
              <label for="ws-model" class="block text-primary-theme font-medium">Preferred Model</label>
              <button
                type="button"
                onclick={handleDiscoverModels}
                disabled={isDiscoveringModels}
                class="flex items-center gap-1.5 px-2 py-0.5 text-[11px] font-medium text-accent-theme hover:bg-accent-subtle/30 rounded transition-colors cursor-pointer disabled:opacity-50"
                title="Discover active models from Gemini CLI v0.67+ models command"
              >
                <RefreshCw size={11} class={isDiscoveringModels ? "animate-spin" : ""} />
                <span>{isDiscoveringModels ? "Querying CLI..." : "Discover Live Models (v0.67+)"}</span>
              </button>
            </div>
            <select
              id="ws-model"
              value={selectedDropdownValue}
              onchange={onSelectModelChange}
              class="w-full px-3 py-2 bg-app border border-theme-default rounded-lg text-primary-theme focus:outline-none focus:border-accent-theme font-mono"
            >
              {#if liveDiscoveredModels.length > 0}
                <optgroup label="Discovered via Gemini CLI (Live API)">
                  {#each liveDiscoveredModels as lm}
                    <option value={lm.id}>
                      {lm.id} {lm.input_token_limit ? `(${Math.round(lm.input_token_limit / 1000)}k ctx)` : ""} {lm.is_default ? "— Default" : ""}
                    </option>
                  {/each}
                </optgroup>
              {/if}
              {#each MODEL_GROUPS as group}
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
                  placeholder="e.g. gemini-3.8-flash, gemini-3.5-flash-lite, or custom model"
                  class="w-full px-2.5 py-1.5 bg-surface border border-theme-default rounded text-primary-theme font-mono text-xs focus:outline-none focus:border-accent-theme"
                />
                <span class="text-[10px] text-muted-theme mt-1 block">
                  Passed directly to Gemini CLI via ACP protocol.
                </span>
              </div>
            {/if}
            <span class="text-[11px] text-muted-theme mt-1.5 block">
              Auto dynamically balances speed and complexity. Manual lets you select or enter any model.
            </span>

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

          <div>
            <label for="ws-approval-mode" class="block text-primary-theme font-medium mb-1">Policy Approval Mode</label>
            <select
              id="ws-approval-mode"
              bind:value={editingWorkspace.approval_mode}
              class="w-full px-3 py-2 bg-app border border-theme-default rounded-lg text-primary-theme focus:outline-none focus:border-accent-theme font-sans"
            >
              {#each APPROVAL_MODES as m}
                <option value={m.value}>{m.label}</option>
              {/each}
            </select>
            <span class="text-[11px] text-muted-theme mt-1.5 block">
              {APPROVAL_MODES.find((m) => m.value === (editingWorkspace.approval_mode || "default"))?.description}
            </span>
          </div>

          <div>
            <label for="ws-prompt" class="block text-primary-theme font-medium mb-1">Custom System Instructions (Optional)</label>
            <textarea
              id="ws-prompt"
              bind:value={editingWorkspace.system_prompt}
              rows="3"
              placeholder="e.g. Always write production C# code following Microsoft coding conventions..."
              class="w-full px-3 py-2 bg-app border border-theme-default rounded-lg text-primary-theme focus:outline-none focus:border-accent-theme resize-none"
            ></textarea>
          </div>

          <!-- Session File Changes & Snapshots Tracking (Optional Non-Git Feature) -->
          <div class="p-3 bg-app border border-subtle rounded-lg space-y-2">
            <div class="flex items-center justify-between">
              <div class="flex items-center gap-2">
                <FileDiff size={14} class="text-amber-400" />
                <span class="font-medium text-primary-theme">Track Session File Changes & Snapshots</span>
              </div>
              <div class="flex items-center gap-2.5">
                <span class="text-xs font-mono font-semibold transition-colors duration-150 {editingWorkspace.track_file_changes ? 'text-emerald-400' : 'text-muted-theme'}">
                  {editingWorkspace.track_file_changes ? 'ON' : 'OFF'}
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
            <p class="text-[11px] text-muted-theme">
              Recommended for folders not under Git source control. Snapshots files before agent edits, calculates visual line diffs (+/-), and enables 1-click restore. Keep disabled for Git repositories to ensure maximum file operation speed.
            </p>
          </div>

          <!-- Windows Desktop Toast Notifications -->
          <div class="p-3 bg-app border border-subtle rounded-lg space-y-2">
            <div class="flex items-center justify-between">
              <div class="flex items-center gap-2">
                <Bell size={14} class="text-accent-theme" />
                <span class="font-medium text-primary-theme">Desktop Toast Notifications</span>
              </div>
              <div class="flex items-center gap-2.5">
                <span class="text-xs font-mono font-semibold transition-colors duration-150 {notificationsEnabled ? 'text-emerald-400' : 'text-muted-theme'}">
                  {notificationsEnabled ? 'ON' : 'OFF'}
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
            <p class="text-[11px] text-muted-theme">
              Receive native Windows toast notifications when background turns finish or require tool permission while you are working in other apps.
            </p>
            <div class="pt-1">
              <button
                type="button"
                onclick={handleTestToast}
                class="px-2.5 py-1 text-[11px] rounded bg-surface hover:bg-surface-elevated text-secondary-theme hover:text-primary-theme border border-theme-default transition-colors flex items-center gap-1.5 cursor-pointer"
              >
                <BellRing size={12} class="text-accent-theme" />
                <span>Test Windows Toast</span>
              </button>
            </div>
          </div>

          <div class="pt-2 flex items-center justify-between border-t border-subtle">
            <button
              onclick={async () => {
                const confirmed = await dialogManager.confirm("Delete this workspace profile? This cannot be undone.", {
                  title: "Delete Workspace Profile",
                  confirmText: "Delete",
                  isDestructive: true,
                });
                if (confirmed) {
                  onDeleteWorkspace(editingWorkspace.id);
                }
              }}
              class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-rose-400 hover:bg-rose-500/10 transition-colors cursor-pointer"
            >
              <Trash2 size={14} />
              <span>Delete</span>
            </button>

            <button
              onclick={handleSave}
              disabled={!canSave}
              class="flex items-center gap-1.5 px-4 py-1.5 rounded-lg font-semibold transition-colors shadow-xs {canSave ? 'bg-accent-theme hover:bg-accent-hover text-white cursor-pointer' : 'bg-slate-600 text-slate-400 cursor-not-allowed opacity-60'}"
              title={!canSave ? 'Enter a workspace name and folder path to enable saving' : ''}
            >
              <Check size={14} />
              <span>{isSaving ? 'Saving…' : 'Save Workspace'}</span>
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
{/if}
