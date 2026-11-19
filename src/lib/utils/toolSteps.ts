import type { Message, ToolExecutionStep } from "$lib/types";

/**
 * Resolves a human-friendly title for a tool execution step.
 * Prevents machine identifiers like "call_0" or "tool" from being displayed to users.
 */
export function resolveHumanFriendlyTitle(step: ToolExecutionStep): string {
  const toolName = (step.tool_name || "").toLowerCase();
  const rawTitle = (step.title || "").trim();
  const isMachineId =
    !rawTitle ||
    rawTitle === "tool" ||
    rawTitle.startsWith("call_") ||
    rawTitle.startsWith("tool-") ||
    /^[0-9_\-]+$/.test(rawTitle);

  const p = step.parameters;
  if (p && typeof p === "object") {
    if (typeof p.toolSummary === "string" && p.toolSummary.trim()) {
      return p.toolSummary.trim();
    }
    if (typeof p.summary === "string" && p.summary.trim()) {
      return p.summary.trim();
    }
    if (typeof p.description === "string" && p.description.trim()) {
      return p.description.trim();
    }

    // Command tools
    if (
      toolName.includes("terminal") ||
      toolName.includes("command") ||
      toolName.includes("bash") ||
      toolName.includes("shell") ||
      toolName.includes("exec")
    ) {
      const cmd =
        typeof p.CommandLine === "string"
          ? p.CommandLine
          : typeof p.command === "string"
          ? p.command
          : typeof p.cmd === "string"
          ? p.cmd
          : "";
      if (cmd.trim()) {
        const line = cmd.trim().split("\n")[0].trim();
        return line.length > 55 ? line.slice(0, 52) + "..." : line;
      }
    }

    // File tools
    if (
      toolName.includes("read") ||
      toolName.includes("write") ||
      toolName.includes("edit") ||
      toolName.includes("patch") ||
      toolName.includes("view") ||
      toolName.includes("file")
    ) {
      const path =
        typeof p.Path === "string"
          ? p.Path
          : typeof p.path === "string"
          ? p.path
          : typeof p.file_path === "string"
          ? p.file_path
          : "";
      if (path.trim()) {
        const norm = path.replace(/\\/g, "/").trim();
        const srcIdx = norm.indexOf("/src");
        if (srcIdx >= 0) return norm.slice(srcIdx + 1);
        const knIdx = norm.indexOf("/knowledge");
        if (knIdx >= 0) return norm.slice(knIdx + 1);
        const parts = norm.split("/").filter(Boolean);
        return parts.length > 2 ? `${parts[parts.length - 2]}/${parts[parts.length - 1]}` : norm;
      }
    }

    // Search tools
    if (toolName.includes("search") || toolName.includes("grep") || toolName.includes("find")) {
      const q =
        typeof p.query === "string"
          ? p.query
          : typeof p.Pattern === "string"
          ? p.Pattern
          : typeof p.pattern === "string"
          ? p.pattern
          : "";
      if (q.trim()) {
        return q.length > 45 ? `"${q.slice(0, 42)}..."` : `"${q}"`;
      }
    }
  }

  if (!isMachineId) return rawTitle;

  if (toolName.includes("terminal") || toolName.includes("command")) return "Terminal Command";
  if (toolName.includes("read") || toolName.includes("view")) return "Read File";
  if (toolName.includes("edit") || toolName.includes("write")) return "Edit File";
  if (toolName.includes("search") || toolName.includes("grep")) return "Search Files";
  return step.tool_name ? step.tool_name.replace(/_/g, " ") : "Task Step";
}

/**
 * Extracts structured tool execution steps from an assistant message.
 * Supports modern structured tool_calls_json as well as legacy inline markdown logs.
 */
export function parseToolSteps(msg: Message): ToolExecutionStep[] {
  if (msg.tool_calls_json) {
    try {
      const parsed = JSON.parse(msg.tool_calls_json);
      if (Array.isArray(parsed) && parsed.length > 0) {
        return parsed.map((s) => ({
          ...s,
          title: resolveHumanFriendlyTitle(s),
        }));
      }
    } catch (e) {
      console.warn("Failed to parse tool_calls_json:", e);
    }
  }

  // Fallback: parse legacy markdown blockquotes in existing saved conversations
  if (
    msg.content &&
    (msg.content.includes("**Running tool:**") ||
      msg.content.includes("**Completed:**") ||
      msg.content.includes("**Tool failed:**"))
  ) {
    const steps: ToolExecutionStep[] = [];
    const lines = msg.content.split("\n");
    let currentStep: ToolExecutionStep | null = null;

    for (let i = 0; i < lines.length; i++) {
      const line = lines[i].trim();
      const runMatch = line.match(/>\s*(?:⚙️|\*)\s*\*\*Running tool:\*\*\s*`?([^`]+)`?/i);
      const doneMatch = line.match(/>\s*(?:✅|\*)\s*\*\*Completed:\*\*\s*`?([^`]+)`?/i);
      const failMatch = line.match(/>\s*(?:❌|\*)\s*\*\*Tool failed:\*\*\s*`?([^`:]+)`?(?::\s*(.*))?/i);

      if (runMatch) {
        const title = runMatch[1].replace(/\.{3}$/, "").trim();
        currentStep = {
          id: `legacy-${steps.length}`,
          title,
          tool_name: title,
          summary: `✅ Ran: ${title}`,
          status: "completed", // Legacy messages in DB are already finished
        };
        steps.push(currentStep);
      } else if (doneMatch && !currentStep) {
        const title = doneMatch[1].trim();
        steps.push({
          id: `legacy-${steps.length}`,
          title,
          tool_name: title,
          summary: `✅ Completed: ${title}`,
          status: "completed",
        });
      } else if (failMatch) {
        if (currentStep) {
          currentStep.status = "failed";
          currentStep.summary = `❌ Failed: ${currentStep.title}`;
          if (failMatch[2]) currentStep.error = failMatch[2].trim();
        } else {
          const title = failMatch[1].trim();
          steps.push({
            id: `legacy-${steps.length}`,
            title,
            tool_name: title,
            summary: `❌ Failed: ${title}`,
            status: "failed",
            error: failMatch[2]?.trim(),
          });
        }
      }
    }

    if (steps.length > 0) {
      return steps;
    }
  }

  return [];
}

/**
 * Strips legacy tool execution blockquotes from message content if they have been extracted
 * into collapsible tool cards, preserving the final assistant synthesis.
 */
export function cleanMessageContent(content: string, hasExtractedSteps: boolean): string {
  if (!content) return "";
  if (!hasExtractedSteps) return content;

  return content
    .replace(/>\s*⚙️\s*\*\*Running tool:\*\*.*?(?:\r?\n|$)/gi, "")
    .replace(/>\s*✅\s*\*\*Completed:\*\*.*?(?:\r?\n|$)/gi, "")
    .replace(/>\s*❌\s*\*\*Tool failed:\*\*.*?(?:\r?\n|$)/gi, "")
    .trim();
}

/**
 * Formats a duration in milliseconds into a human-friendly string.
 * Examples:
 * - 450 ms -> "0.5s"
 * - 4,800 ms -> "4.8s"
 * - 24,000 ms -> "24s"
 * - 75,000 ms -> "1m 15s"
 * - 3,600,000 ms -> "1h 0m"
 * - 3,750,000 ms -> "1h 2m"
 */
export function formatDuration(ms: number | undefined | null): string {
  if (ms === undefined || ms === null || isNaN(ms) || ms <= 0) return "";
  const totalSeconds = Math.round(ms / 1000);
  if (totalSeconds < 10) {
    return `${(ms / 1000).toFixed(1)}s`;
  }
  if (totalSeconds < 60) {
    return `${totalSeconds}s`;
  }
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  if (minutes < 60) {
    return seconds > 0 ? `${minutes}m ${seconds}s` : `${minutes}m`;
  }
  const hours = Math.floor(minutes / 60);
  const remMinutes = minutes % 60;
  return `${hours}h ${remMinutes}m`;
}
