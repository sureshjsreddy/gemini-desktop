import type { ContextUsageStats, TurnUsageStats } from "$lib/types";

export const DEFAULT_CONTEXT_WINDOW = 1_000_000;

/**
 * Merges an incoming `acp-usage-update` into prior session context usage state.
 * Preserves existing non-zero `usedTokens` and `contextWindow` when interim
 * turn-start `usage_update` notifications emit `used: 0` or omit `size`.
 */
export function mergeContextUsage(
  prev: ContextUsageStats | null | undefined,
  incoming: { usedTokens?: number; contextWindow?: number; percentage?: number }
): ContextUsageStats {
  const rawUsed = incoming.usedTokens && incoming.usedTokens > 0 ? incoming.usedTokens : 0;
  const rawWindow = incoming.contextWindow && incoming.contextWindow > 0 ? incoming.contextWindow : 0;

  const usedTokens = rawUsed > 0 ? rawUsed : (prev?.usedTokens || 0);
  const contextWindow = rawWindow > 0 ? rawWindow : (prev?.contextWindow || 0);
  const percentage =
    contextWindow > 0
      ? Math.min(100, Math.max(0, (usedTokens / contextWindow) * 100))
      : (incoming.percentage || prev?.percentage || 0);

  return {
    usedTokens,
    contextWindow,
    percentage,
    ...(prev?.lastTurnUsage ? { lastTurnUsage: prev.lastTurnUsage } : {}),
  };
}

/**
 * Attaches cumulative per-turn API usage metadata without altering context window occupancy.
 */
export function mergeTurnUsage(
  prev: ContextUsageStats | null | undefined,
  turnUsage: TurnUsageStats
): ContextUsageStats {
  return {
    usedTokens: prev?.usedTokens || 0,
    contextWindow: prev?.contextWindow || 0,
    percentage: prev?.percentage || 0,
    lastTurnUsage: turnUsage,
  };
}

/**
 * Formats token counts concisely for the chat header meter (e.g. `24693` -> `24.7k`, `1048576` -> `1.0M`).
 */
export function formatTokenCount(num: number): string {
  if (num >= 1_000_000) {
    return (num / 1_000_000).toFixed(1) + "M";
  }
  if (num >= 1_000) {
    return (num / 1_000).toFixed(1) + "k";
  }
  return Math.max(0, Math.round(num)).toString();
}

/**
 * Estimates fallback token count from persisted messages when live ACP context telemetry is unavailable.
 */
export function estimateSessionFallbackTokens(
  messages: ReadonlyArray<{ token_count?: number; content?: string }>
): number {
  return messages.reduce((acc, m) => {
    const count =
      m.token_count && m.token_count > 0
        ? m.token_count
        : Math.ceil((m.content?.length || 0) / 4);
    return acc + count;
  }, 0);
}
