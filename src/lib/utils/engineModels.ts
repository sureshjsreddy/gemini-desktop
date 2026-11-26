/**
 * Pure model resolution, sanitization, and per-engine model persistence helpers.
 * Intentionally dependency-free so unit tests can execute directly under Node.js.
 */

const WORKSPACE_MODEL_MAP_PREFIX = "gemini_desktop_ws_model_map_";

/**
 * Returns whether a model string is compatible with the specified CLI engine.
 */
export function sanitizeModelForEngine(model: string | undefined, engine: string | undefined): string {
  const raw = (model || "").trim();
  if (!raw || raw.toLowerCase() === "auto") return "auto";

  const eng = (engine || "gemini").trim().toLowerCase();
  const lower = raw.toLowerCase();

  if (eng === "agy") {
    const isGeminiCliSpecific =
      lower.startsWith("gemini-2.5") ||
      lower.startsWith("gemini-1.5") ||
      lower.startsWith("gemini-3.5") ||
      lower.startsWith("gemini-3.1") ||
      lower.startsWith("gemini-3-") ||
      lower === "gemini-3.8-flash";
    return isGeminiCliSpecific ? "auto" : raw;
  }

  if (eng === "gemini") {
    const isAgySpecific =
      lower.startsWith("claude-") ||
      lower.startsWith("gpt-oss-") ||
      lower === "gemini-4" ||
      lower.startsWith("gemini-4-") ||
      lower === "orchestrator" ||
      lower.endsWith("-high") ||
      lower.endsWith("-xhigh") ||
      lower.endsWith("-medium") ||
      lower.endsWith("-low") ||
      lower.endsWith("-max");
    return isAgySpecific ? "auto" : raw;
  }

  return raw;
}

/**
 * Computes human-readable effective model string shown in workspace headers and badges.
 */
export function getEffectiveModel(model: string | undefined, engine: string | undefined): string {
  const raw = (model || "").trim();
  if (!raw || raw.toLowerCase() === "auto") return "auto";
  const sanitized = sanitizeModelForEngine(raw, engine);
  return sanitized === "auto" && raw.toLowerCase() !== "auto" ? "auto (fallback)" : sanitized;
}

/**
 * Formats token context window numbers concisely (e.g. `1048576` -> `1049k`, `65536` -> `66k`).
 */
export function formatContextLimitLabel(inputTokens?: number | null): string {
  if (!inputTokens || inputTokens <= 0) return "";
  return `${Math.round(inputTokens / 1000)}k ctx`;
}

/**
 * Persists a workspace's chosen model per engine so switching CLIs preserves each engine's model choice.
 */
export function saveRememberedWorkspaceModel(workspaceId: string, engineId: string, model: string): void {
  if (typeof window === "undefined" || !workspaceId || !engineId) return;
  const trimmed = (model || "").trim();
  if (!trimmed) return;
  try {
    const key = `${WORKSPACE_MODEL_MAP_PREFIX}${workspaceId}`;
    const raw = localStorage.getItem(key);
    const map: Record<string, string> = raw ? JSON.parse(raw) : {};
    map[engineId] = trimmed;
    localStorage.setItem(key, JSON.stringify(map));
  } catch {
    // Ignore storage errors
  }
}

/**
 * Loads a previously saved model for `(workspaceId, engineId)`, if any.
 */
export function getRememberedWorkspaceModel(workspaceId: string, engineId: string): string | null {
  if (typeof window === "undefined" || !workspaceId || !engineId) return null;
  try {
    const raw = localStorage.getItem(`${WORKSPACE_MODEL_MAP_PREFIX}${workspaceId}`);
    if (!raw) return null;
    const map = JSON.parse(raw);
    const val = typeof map?.[engineId] === "string" ? map[engineId].trim() : "";
    return val || null;
  } catch {
    return null;
  }
}
