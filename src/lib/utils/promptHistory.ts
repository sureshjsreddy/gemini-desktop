export interface PromptHistoryNavParams {
  key: "ArrowUp" | "ArrowDown";
  inputPrompt: string;
  selectionStart: number;
  selectionEnd: number;
  historyIndex: number;
  promptHistory: string[];
  hasModifiers?: boolean;
}

export interface PromptHistoryNavResult {
  handled: boolean;
  nextPrompt: string;
  nextHistoryIndex: number;
}

/**
 * Resolves Up/Down prompt history navigation matching Antigravity composer behavior:
 * - Never navigates into history if the user currently has draft text (`historyIndex === -1 && inputPrompt.length > 0`).
 * - Starts browsing history on `ArrowUp` only when the prompt text is completely empty.
 * - While actively browsing history (`historyIndex >= 0`), allows moving between older/newer entries and exits back to empty on `ArrowDown`.
 */
export function resolvePromptHistoryNavigation(
  params: PromptHistoryNavParams,
): PromptHistoryNavResult {
  const {
    key,
    inputPrompt,
    selectionStart,
    selectionEnd,
    historyIndex,
    promptHistory,
    hasModifiers = false,
  } = params;

  const unchanged: PromptHistoryNavResult = {
    handled: false,
    nextPrompt: inputPrompt,
    nextHistoryIndex: historyIndex,
  };

  if (hasModifiers || promptHistory.length === 0) {
    return unchanged;
  }

  // Never hijack cursor movement when user has an active selection
  if (selectionStart !== selectionEnd) {
    return unchanged;
  }

  if (key === "ArrowUp") {
    if (historyIndex === -1) {
      // Only enter prompt history if the composer is currently empty
      if (inputPrompt.length > 0) {
        return unchanged;
      }
      return {
        handled: true,
        nextPrompt: promptHistory[promptHistory.length - 1],
        nextHistoryIndex: 0,
      };
    }

    const isFirstLine = !inputPrompt.slice(0, selectionStart).includes("\n");
    if (isFirstLine && historyIndex < promptHistory.length - 1) {
      const nextIndex = historyIndex + 1;
      return {
        handled: true,
        nextPrompt: promptHistory[promptHistory.length - 1 - nextIndex],
        nextHistoryIndex: nextIndex,
      };
    }

    return unchanged;
  }

  // ArrowDown: only step forward/exit when already browsing history
  if (historyIndex < 0) {
    return unchanged;
  }

  const isLastLine = !inputPrompt.slice(selectionEnd).includes("\n");
  if (!isLastLine) {
    return unchanged;
  }

  if (historyIndex > 0) {
    const nextIndex = historyIndex - 1;
    return {
      handled: true,
      nextPrompt: promptHistory[promptHistory.length - 1 - nextIndex],
      nextHistoryIndex: nextIndex,
    };
  }

  return {
    handled: true,
    nextPrompt: "",
    nextHistoryIndex: -1,
  };
}
