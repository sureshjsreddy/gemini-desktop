import type { GitFileItem, GitStatusResult } from "../types.ts";

export interface GitStatusBadgeMeta {
  letter: string;
  label: string;
  title: string;
  badgeClass: string;
  textColor: string;
  dotColor: string;
}

const STATUS_PRIORITY: Record<string, number> = {
  conflict: 6,
  modified: 5,
  added: 4,
  renamed: 3,
  untracked: 2,
  deleted: 1,
};

/**
 * Normalizes file path separators and strips leading/trailing slashes.
 */
function normPath(raw: string): string {
  return raw.replace(/\\/g, "/").replace(/^\/+|\/+$/g, "");
}

/**
 * Converts a Git repo-relative file path into a workspace-relative path.
 * Handles both standard workspace-at-repo-root and monorepo-subdirectory workspaces.
 */
export function toWorkspaceRelativeGitPath(
  gitRelPath: string,
  workspacePath?: string,
  repoRoot?: string
): string | null {
  const cleanGitPath = normPath(gitRelPath);
  if (!cleanGitPath) return null;
  if (!workspacePath || !repoRoot) return cleanGitPath;

  const wsNorm = normPath(workspacePath).toLowerCase();
  const rootNorm = normPath(repoRoot).toLowerCase();

  if (!wsNorm || !rootNorm || wsNorm === rootNorm) {
    return cleanGitPath;
  }

  if (wsNorm.startsWith(rootNorm + "/")) {
    const subPrefix = normPath(workspacePath).slice(normPath(repoRoot).length + 1);
    const subPrefixLower = subPrefix.toLowerCase() + "/";
    if (cleanGitPath.toLowerCase().startsWith(subPrefixLower)) {
      return cleanGitPath.slice(subPrefix.length + 1);
    }
    return null;
  }

  return cleanGitPath;
}

/**
 * Builds fast lookup maps for Workspace Explorer rows and folder indicators
 * from the active GitStatusResult without extra backend calls.
 */
export function buildGitStatusMaps(
  gitStatus: GitStatusResult | null | undefined,
  workspacePath?: string
): {
  fileStatusMap: Map<string, GitFileItem>;
  dirtyDirs: Set<string>;
} {
  const fileStatusMap = new Map<string, GitFileItem>();
  const dirtyDirs = new Set<string>();

  if (!gitStatus || !gitStatus.is_git_repo) {
    return { fileStatusMap, dirtyDirs };
  }

  const allItems = [...(gitStatus.unstaged || []), ...(gitStatus.staged || [])];

  for (const item of allItems) {
    const wsRel = toWorkspaceRelativeGitPath(item.path, workspacePath, gitStatus.repo_root);
    if (!wsRel) continue;

    const existing = fileStatusMap.get(wsRel);
    const nextPriority = STATUS_PRIORITY[item.status] || 0;
    const prevPriority = existing ? STATUS_PRIORITY[existing.status] || 0 : -1;

    if (!existing || nextPriority > prevPriority) {
      fileStatusMap.set(wsRel, item);
    }

    const parts = wsRel.split("/");
    for (let i = 1; i < parts.length; i++) {
      dirtyDirs.add(parts.slice(0, i).join("/"));
    }
  }

  return { fileStatusMap, dirtyDirs };
}

/**
 * Returns theme-readable status badge metadata for Git status values.
 */
export function getGitStatusBadge(status: string): GitStatusBadgeMeta {
  switch (status) {
    case "added":
      return {
        letter: "A",
        label: "Added",
        title: "Added — New file in Git index",
        badgeClass: "bg-emerald-500/20 text-emerald-500 border-emerald-500/40",
        textColor: "text-emerald-500",
        dotColor: "bg-emerald-500",
      };
    case "modified":
      return {
        letter: "M",
        label: "Modified",
        title: "Modified — File has changes",
        badgeClass: "bg-sky-500/20 text-sky-500 border-sky-500/40",
        textColor: "text-primary-theme",
        dotColor: "bg-sky-500",
      };
    case "deleted":
      return {
        letter: "D",
        label: "Deleted",
        title: "Deleted — File was removed",
        badgeClass: "bg-rose-500/20 text-rose-500 border-rose-500/40",
        textColor: "text-rose-500 line-through opacity-75",
        dotColor: "bg-rose-500",
      };
    case "renamed":
      return {
        letter: "R",
        label: "Renamed",
        title: "Renamed — File was renamed or moved",
        badgeClass: "bg-amber-500/20 text-amber-500 border-amber-500/40",
        textColor: "text-amber-500",
        dotColor: "bg-amber-500",
      };
    case "conflict":
      return {
        letter: "!",
        label: "Conflict",
        title: "Conflict — Merge conflict",
        badgeClass: "bg-red-500/25 text-red-500 border-red-500/50",
        textColor: "text-red-500 font-semibold",
        dotColor: "bg-red-500",
      };
    case "untracked":
    default:
      return {
        letter: "U",
        label: "Untracked",
        title: "Untracked — New file on disk, not yet in Git",
        badgeClass: "bg-emerald-500/15 text-emerald-500 border-emerald-500/35",
        textColor: "text-emerald-500",
        dotColor: "bg-emerald-500",
      };
  }
}

/**
 * Splits a relative file path into file name and parent directory.
 */
export function splitGitFilePath(fullPath: string): { name: string; dir: string } {
  const normalized = fullPath.replace(/\\/g, "/");
  const lastSlash = normalized.lastIndexOf("/");
  if (lastSlash === -1) {
    return { name: normalized, dir: "" };
  }
  return {
    name: normalized.slice(lastSlash + 1),
    dir: normalized.slice(0, lastSlash),
  };
}
