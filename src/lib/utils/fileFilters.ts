/**
 * Set of directory and cache folder names routinely ignored during file searches and tree navigation.
 */
export const IGNORED_NAMES: ReadonlySet<string> = new Set([
  "node_modules",
  "target",
  "build",
  "dist",
  ".svelte-kit",
  ".git",
  ".vscode",
  ".idea",
  "__pycache__",
  ".next",
  ".turbo",
  "vendor",
]);

/**
 * Determines whether a given relative path contains hidden components (e.g. `.git`)
 * or directories present in the ignored names set.
 */
export function isHiddenOrIgnored(relPath: string): boolean {
  const parts = relPath.replace(/\\/g, "/").split("/");
  for (const p of parts) {
    if (!p) continue;
    if (p.startsWith(".")) return true;
    if (IGNORED_NAMES.has(p.toLowerCase())) return true;
  }
  return false;
}

/**
 * Filter helper: dotfiles/dotfolders are hidden unless showHidden is true.
 */
export function isNodeVisible(nameOrItem: string | { name: string }, showHidden: boolean): boolean {
  if (showHidden) return true;
  const name = typeof nameOrItem === "string" ? nameOrItem : nameOrItem.name;
  return !name.startsWith(".");
}
