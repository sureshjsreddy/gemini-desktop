import type { Workspace } from "$lib/types";

/**
 * Normalizes optional workspace fields into consistent defaults for editing and dirty comparison.
 */
export function normalizeWorkspaceDraft(ws: Partial<Workspace>): Workspace {
  return {
    id: ws.id || "",
    name: ws.name ?? "",
    path: ws.path ?? "",
    model: ws.model?.trim() || "auto",
    system_prompt: ws.system_prompt ?? "",
    approval_mode: ws.approval_mode || "default",
    track_file_changes: ws.track_file_changes ?? false,
    reasoning_effort: ws.reasoning_effort ?? "",
    sandbox_enabled: ws.sandbox_enabled ?? false,
    agent_name: ws.agent_name ?? "",
    additional_dirs: ws.additional_dirs ?? "",
    created_at: ws.created_at || new Date().toISOString(),
  };
}

/**
 * Returns true if the current workspace draft differs from its saved baseline or has user edits on a new draft.
 */
export function isWorkspaceDraftDirty(
  draft: Workspace,
  baseline: Workspace | null,
  isNewDraft: boolean
): boolean {
  if (!baseline) {
    return isNewDraft && (draft.name.trim().length > 0 || draft.path.trim().length > 0);
  }
  const a = normalizeWorkspaceDraft(draft);
  const b = normalizeWorkspaceDraft(baseline);
  return (
    a.name !== b.name ||
    a.path !== b.path ||
    a.model !== b.model ||
    a.approval_mode !== b.approval_mode ||
    Boolean(a.track_file_changes) !== Boolean(b.track_file_changes) ||
    (a.reasoning_effort || "") !== (b.reasoning_effort || "") ||
    Boolean(a.sandbox_enabled) !== Boolean(b.sandbox_enabled) ||
    (a.agent_name || "") !== (b.agent_name || "") ||
    (a.additional_dirs || "") !== (b.additional_dirs || "") ||
    (a.system_prompt || "") !== (b.system_prompt || "")
  );
}

/**
 * Extracts a clean display name from a local workspace directory path (e.g. `C:\repos\my-app` -> `my-app`).
 */
export function suggestWorkspaceNameFromPath(dirPath: string): string {
  const cleaned = (dirPath || "").trim().replace(/[\\/]+$/, "");
  if (!cleaned || /^[a-zA-Z]:$/.test(cleaned)) return "";
  const parts = cleaned.split(/[\\/]/).filter(Boolean);
  const last = parts[parts.length - 1] || "";
  if (!last || /^[a-zA-Z]:$/.test(last)) return "";
  return last;
}

/**
 * Determines whether changing the folder path should automatically update the workspace profile name.
 */
export function shouldAutoSyncProfileName(
  currentName: string,
  lastSuggestedName: string,
  isNewDraft: boolean
): boolean {
  if (!isNewDraft) return false;
  const trimmed = (currentName || "").trim();
  return trimmed === "" || trimmed === "New Workspace" || (lastSuggestedName !== "" && trimmed === lastSuggestedName);
}
