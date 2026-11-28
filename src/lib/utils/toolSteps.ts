import type {
  Message,
  PlanStepStatus,
  ToolExecutionStep,
  TurnMilestoneStep,
} from "$lib/types";

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
          // Historical saved messages represent completed turns: normalize dangling states
          status: s.status === "in_progress" || s.status === "pending" ? ("completed" as const) : s.status,
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

export type ToolCategory = "explore" | "edit" | "command" | "web" | "other";

export interface ProgressPhaseNode {
  id: string;
  label: string;
  status: PlanStepStatus;
}

export interface LiveWorkFocus {
  verb: string;
  target: string;
  category: ToolCategory;
  exploreBurstCount: number;
}

export interface ImpactChip {
  id: string;
  kind: "file" | "command_ok" | "command_err" | "explore_summary";
  label: string;
  detail?: string;
}

export interface TurnReceiptSummary {
  headline: string;
  hasFailure: boolean;
  editedFiles: string[];
  commandCount: number;
  failedCount: number;
  exploreCount: number;
  milestoneTotal: number;
  milestoneDone: number;
}

/**
 * Removes legacy decorative emojis so components can render crisp Lucide icons and typography.
 */
export function stripToolEmojiPrefix(text: string): string {
  return text
    .replace(/^(?:✅|⚙️|❌|✏️|📖|🔍|⚡|🛠|🧰)\s*/u, "")
    .trim();
}

export function normalizePlanStatus(rawStatus: unknown, completedFlag?: unknown): PlanStepStatus {
  if (completedFlag === true) return "completed";
  const s = typeof rawStatus === "string" ? rawStatus.trim().toLowerCase() : "";
  if (s === "completed" || s === "complete" || s === "done" || s === "success") {
    return "completed";
  }
  if (s === "in_progress" || s === "running" || s === "active" || s === "working") {
    return "in_progress";
  }
  if (s === "failed" || s === "error") {
    return "failed";
  }
  return "pending";
}

/**
 * Normalizes ACP `sessionUpdate: "plan"` entries or TODO arrays into ordered 4-state milestones.
 */
export function normalizeAcpPlanEntries(rawEntries: unknown): TurnMilestoneStep[] {
  if (!Array.isArray(rawEntries)) return [];
  const items: TurnMilestoneStep[] = [];
  for (let i = 0; i < rawEntries.length; i++) {
    const entry = rawEntries[i];
    if (!entry || typeof entry !== "object") continue;
    const obj = entry as Record<string, unknown>;
    const rawTitle = obj.content ?? obj.title ?? obj.text ?? obj.description ?? "";
    const title = String(rawTitle).trim();
    if (!title) continue;
    items.push({
      id: obj.id ? String(obj.id) : `acp-plan-${i}`,
      title,
      status: normalizePlanStatus(obj.status, obj.completed),
      priority: typeof obj.priority === "string" ? obj.priority : undefined,
    });
  }
  return items;
}

export function isTodoMetaStep(step: ToolExecutionStep): boolean {
  const name = (step.tool_name || "").toLowerCase();
  return (
    name.includes("todo") ||
    name.includes("update_steps") ||
    name.includes("task_plan") ||
    name.includes("write_plan")
  );
}

/**
 * Detects whether a tool step represents structured milestone planning (`update_steps`, `write_todos`, etc.).
 */
export function extractTodoMilestonesFromToolStep(
  step: ToolExecutionStep,
): TurnMilestoneStep[] | null {
  if (!isTodoMetaStep(step)) return null;
  const p = step.parameters;
  if (!p || typeof p !== "object") return null;
  const candidate = p.todos ?? p.steps ?? p.tasks ?? p.entries;
  const parsed = normalizeAcpPlanEntries(candidate);
  return parsed.length > 0 ? parsed : null;
}

export function classifyToolStep(step: ToolExecutionStep): ToolCategory {
  const name = `${step.tool_name || ""} ${step.title || ""}`.toLowerCase();
  if (
    name.includes("run_command") ||
    name.includes("terminal") ||
    name.includes("shell") ||
    name.includes("bash") ||
    name.includes("exec") ||
    name.includes("command")
  ) {
    return "command";
  }
  if (
    name.includes("edit") ||
    name.includes("write") ||
    name.includes("create") ||
    name.includes("patch") ||
    name.includes("replace") ||
    name.includes("delete") ||
    name.includes("revert")
  ) {
    return "edit";
  }
  if (
    name.includes("read") ||
    name.includes("view") ||
    name.includes("grep") ||
    name.includes("search") ||
    name.includes("find") ||
    name.includes("list") ||
    name.includes("glob")
  ) {
    return "explore";
  }
  if (name.includes("web") || name.includes("fetch") || name.includes("browser")) {
    return "web";
  }
  return "other";
}

function shortenDisplayPath(raw: string): string {
  const norm = raw.replace(/\\/g, "/").trim().replace(/\/+$/, "");
  if (!norm) return "";
  const parts = norm.split("/").filter(Boolean);
  return parts[parts.length - 1] || norm;
}

export function extractStepFilePath(step: ToolExecutionStep): string | null {
  const p = step.parameters;
  if (p && typeof p === "object") {
    const cand =
      p.Path ??
      p.path ??
      p.filePath ??
      p.file_path ??
      p.target_path ??
      p.targetPath ??
      p.filename ??
      p.file;
    if (typeof cand === "string" && cand.trim()) {
      return shortenDisplayPath(cand);
    }
  }
  const friendly = stripToolEmojiPrefix(resolveHumanFriendlyTitle(step));
  if (friendly.includes("/") || friendly.includes("\\") || /\.[a-z0-9]{1,8}$/i.test(friendly)) {
    const cleaned = friendly
      .replace(/^(?:writing to|editing|creating|updating|patching|reading|viewing)\s+/i, "")
      .trim();
    return shortenDisplayPath(cleaned);
  }
  return null;
}

/**
 * Resolves either explicit ACP milestone steps OR auto-derived calm phase steps when none exist.
 */
export function deriveProgressPhases(
  steps: ToolExecutionStep[],
  milestones: TurnMilestoneStep[],
  isStreaming: boolean,
): ProgressPhaseNode[] {
  if (milestones.length > 0) {
    return milestones.map((m) => ({
      id: m.id,
      label: m.title,
      status:
        !isStreaming && (m.status === "in_progress" || m.status === "pending")
          ? "completed"
          : m.status,
    }));
  }

  const actionable = steps.filter((s) => !isTodoMetaStep(s));
  if (actionable.length <= 1) {
    return [];
  }

  const orderedCategories: ToolCategory[] = [];
  for (const s of actionable) {
    const cat = classifyToolStep(s);
    if (cat === "other") continue;
    if (!orderedCategories.includes(cat)) {
      orderedCategories.push(cat);
    }
  }

  if (orderedCategories.length <= 1) {
    return [];
  }

  const activeStep = isStreaming
    ? [...actionable].reverse().find((s) => s.status === "in_progress" || s.status === "pending")
    : undefined;
  const activeCat = activeStep ? classifyToolStep(activeStep) : null;

  const phaseLabels: Record<ToolCategory, { active: string; done: string }> = {
    explore: { active: "Reviewing codebase", done: "Reviewed codebase" },
    edit: { active: "Updating files", done: "Updated files" },
    command: { active: "Verifying & running", done: "Ran verification" },
    web: { active: "Checking references", done: "Checked references" },
    other: { active: "Working", done: "Completed" },
  };

  return orderedCategories.map((cat) => {
    const catSteps = actionable.filter((s) => classifyToolStep(s) === cat);
    const hasFailed = catSteps.some((s) => s.status === "failed");
    const hasRunning =
      isStreaming &&
      (cat === activeCat || catSteps.some((s) => s.status === "in_progress" || s.status === "pending"));

    let status: PlanStepStatus = "completed";
    if (hasRunning) {
      status = "in_progress";
    } else if (hasFailed) {
      status = "failed";
    }

    return {
      id: `phase-${cat}`,
      label: status === "in_progress" ? phaseLabels[cat].active : phaseLabels[cat].done,
      status,
    };
  });
}

/**
 * Produces the calm single-line live focus indicator while the turn is running.
 */
export function buildLiveWorkFocus(
  steps: ToolExecutionStep[],
  isStreaming: boolean,
): LiveWorkFocus | null {
  if (!isStreaming) return null;

  const actionable = steps.filter((s) => !isTodoMetaStep(s));
  if (actionable.length === 0) return null;

  const activeStep = [...actionable]
    .reverse()
    .find((s) => s.status === "in_progress" || s.status === "pending");

  if (!activeStep) {
    return {
      verb: "Preparing response",
      target: "",
      category: "other",
      exploreBurstCount: 0,
    };
  }

  const cat = classifyToolStep(activeStep);
  const cleanTarget = stripToolEmojiPrefix(resolveHumanFriendlyTitle(activeStep));
  const exploreCount = actionable.filter((s) => classifyToolStep(s) === "explore").length;

  switch (cat) {
    case "explore": {
      const targetFile = extractStepFilePath(activeStep) || cleanTarget;
      return {
        verb: exploreCount > 1 ? `Reviewing files (${exploreCount} checked)` : "Reviewing",
        target: targetFile,
        category: cat,
        exploreBurstCount: exploreCount,
      };
    }
    case "edit":
      return {
        verb: "Editing",
        target: extractStepFilePath(activeStep) || cleanTarget,
        category: cat,
        exploreBurstCount: 0,
      };
    case "command":
      return {
        verb: "Running",
        target: cleanTarget,
        category: cat,
        exploreBurstCount: 0,
      };
    case "web":
      return {
        verb: "Checking",
        target: cleanTarget,
        category: cat,
        exploreBurstCount: 0,
      };
    default:
      return {
        verb: "Working on",
        target: cleanTarget,
        category: cat,
        exploreBurstCount: 0,
      };
  }
}

/**
 * Extracts concise clickable impact pills (unique edited files + verification status).
 */
export function extractImpactChips(steps: ToolExecutionStep[]): {
  chips: ImpactChip[];
  editedFiles: string[];
  extraFileCount: number;
} {
  const actionable = steps.filter((s) => !isTodoMetaStep(s));
  const editedFiles: string[] = [];
  let commandOk = 0;
  let commandErr = 0;

  for (const s of actionable) {
    const cat = classifyToolStep(s);
    if (cat === "edit" && s.status !== "failed") {
      const name = extractStepFilePath(s);
      if (name && !editedFiles.includes(name)) {
        editedFiles.push(name);
      }
    } else if (cat === "command") {
      if (s.status === "failed") {
        commandErr++;
      } else if (s.status === "completed") {
        commandOk++;
      }
    }
  }

  const maxFilesShown = 3;
  const shownFiles = editedFiles.slice(0, maxFilesShown);
  const extraFileCount = Math.max(0, editedFiles.length - maxFilesShown);

  const chips: ImpactChip[] = shownFiles.map((fileName) => ({
    id: `file-${fileName}`,
    kind: "file",
    label: fileName,
  }));

  if (commandErr > 0) {
    chips.push({
      id: "cmd-err",
      kind: "command_err",
      label: `${commandErr} command ${commandErr === 1 ? "issue" : "issues"}`,
    });
  } else if (commandOk > 0) {
    chips.push({
      id: "cmd-ok",
      kind: "command_ok",
      label: `${commandOk} ${commandOk === 1 ? "command ran" : "commands ran"}`,
    });
  }

  return { chips, editedFiles, extraFileCount };
}

/**
 * Generates the calm, scannable 1-line Work Receipt shown after a turn finishes.
 */
export function buildTurnReceiptSummary(
  steps: ToolExecutionStep[],
  milestones: TurnMilestoneStep[],
): TurnReceiptSummary {
  const actionable = steps.filter((s) => !isTodoMetaStep(s));
  const { editedFiles } = extractImpactChips(actionable);
  const failedCount = actionable.filter((s) => s.status === "failed").length;
  const exploreCount = actionable.filter((s) => classifyToolStep(s) === "explore").length;
  const commandCount = actionable.filter((s) => classifyToolStep(s) === "command").length;
  const milestoneTotal = milestones.length;
  const milestoneDone = milestones.filter((m) => m.status === "completed").length;
  const hasFailedMilestone = milestones.some((m) => m.status === "failed");
  const hasFailure = failedCount > 0 || hasFailedMilestone;

  const parts: string[] = [];

  if (hasFailure) {
    if (failedCount > 0) {
      parts.push(`${failedCount} ${failedCount === 1 ? "step failed" : "steps failed"}`);
    } else {
      parts.push("Attention needed");
    }
    if (editedFiles.length > 0) {
      parts.push(
        editedFiles.length === 1
          ? `Updated ${editedFiles[0]}`
          : `Updated ${editedFiles.length} files`,
      );
    }
  } else {
    if (milestoneTotal > 0) {
      parts.push(`${milestoneDone}/${milestoneTotal} tasks completed`);
    }
    if (editedFiles.length > 0) {
      parts.push(
        editedFiles.length === 1
          ? `Updated ${editedFiles[0]}`
          : `Updated ${editedFiles.length} files`,
      );
    }
    if (commandCount > 0) {
      parts.push(
        commandCount === 1 ? "Ran 1 command" : `Ran ${commandCount} commands`,
      );
    }
    if (parts.length === 0 && exploreCount > 0) {
      parts.push(
        exploreCount === 1
          ? "Reviewed 1 item"
          : `Reviewed ${exploreCount} files & searches`,
      );
    }
    if (parts.length === 0 && actionable.length > 0) {
      parts.push(
        actionable.length === 1
          ? stripToolEmojiPrefix(resolveHumanFriendlyTitle(actionable[0]))
          : `Completed ${actionable.length} actions`,
      );
    }
  }

  return {
    headline: parts.slice(0, 2).join(" • ") || "Completed",
    hasFailure,
    editedFiles,
    commandCount,
    failedCount,
    exploreCount,
    milestoneTotal,
    milestoneDone,
  };
}
