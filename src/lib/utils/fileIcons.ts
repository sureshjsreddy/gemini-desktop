export type FileIconType = "folder" | "code" | "text" | "database" | "terminal" | "image";

export interface FileIconMeta {
  type: FileIconType;
  color: string;
  badge: string;
}

export interface FileLikeItem {
  name?: string;
  isDir?: boolean;
  is_dir?: boolean;
  extension?: string;
}

/**
 * Returns Visual Studio 2022 style icon metadata (type, color, badge)
 * based on file name, directory status, and extension.
 */
export function getFileIconMeta(item: FileLikeItem): FileIconMeta {
  const isDirectory = Boolean(item.isDir || item.is_dir);
  if (isDirectory) {
    return {
      type: "folder",
      color: "#f59e0b", // Classic VS yellow/amber
      badge: "",
    };
  }

  const name = item.name || "";
  const lowerName = name.toLowerCase();
  const ext = (item.extension || (name.includes(".") ? name.split(".").pop() : "") || "").toLowerCase();

  // Specific file names
  if (lowerName === "cargo.toml" || lowerName === "cargo.lock") {
    return { type: "code", color: "#f97316", badge: "CRG" };
  }
  if (lowerName === "package.json") {
    return { type: "code", color: "#eab308", badge: "{}" };
  }
  if (lowerName === "gemini.md" || lowerName === "agents.md") {
    return { type: "text", color: "#38bdf8", badge: "AI" };
  }
  if (lowerName.startsWith(".git")) {
    return { type: "text", color: "#f97316", badge: "GIT" };
  }

  // Extensions
  switch (ext) {
    case "cs":
      return { type: "code", color: "#a855f7", badge: "C#" };
    case "rs":
      return { type: "code", color: "#f97316", badge: "RS" };
    case "ts":
    case "tsx":
      return { type: "code", color: "#3b82f6", badge: "TS" };
    case "js":
    case "jsx":
    case "mjs":
      return { type: "code", color: "#eab308", badge: "JS" };
    case "svelte":
      return { type: "code", color: "#ff3e00", badge: "SV" };
    case "html":
    case "htm":
      return { type: "code", color: "#f97316", badge: "<>" };
    case "css":
    case "scss":
    case "sass":
    case "less":
      return { type: "code", color: "#06b6d4", badge: "#" };
    case "json":
      return { type: "code", color: "#fbbf24", badge: "{}" };
    case "md":
    case "markdown":
      return { type: "text", color: "#38bdf8", badge: "MD" };
    case "yaml":
    case "yml":
      return { type: "text", color: "#c084fc", badge: "YML" };
    case "toml":
    case "ini":
    case "conf":
    case "cfg":
      return { type: "text", color: "#fb923c", badge: "CFG" };
    case "sql":
    case "db":
    case "sqlite":
      return { type: "database", color: "#14b8a6", badge: "SQL" };
    case "sh":
    case "bash":
    case "bat":
    case "cmd":
    case "ps1":
      return { type: "terminal", color: "#22c55e", badge: ">_" };
    case "png":
    case "jpg":
    case "jpeg":
    case "gif":
    case "svg":
    case "ico":
    case "webp":
      return { type: "image", color: "#c084fc", badge: "IMG" };
    default:
      return { type: "text", color: "#94a3b8", badge: "" };
  }
}

/**
 * Convenience helper returning just the accent color for a file or extension.
 */
export function getFileIconColor(ext?: string, name?: string): string {
  return getFileIconMeta({ name, extension: ext }).color;
}
