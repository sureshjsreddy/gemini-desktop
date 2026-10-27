import type { PlanTaskItem } from "$lib/types";

/**
 * Parses markdown text to extract implementation plan tasks (checkboxes or numbered steps).
 */
export function extractTasksFromMarkdown(md: string, existingTasks: PlanTaskItem[] = []): PlanTaskItem[] {
  if (!md || !md.trim()) return [];

  const lines = md.split("\n");
  const extracted: PlanTaskItem[] = [];
  let currentCategory = "Implementation Steps";
  let inCodeBlock = false;

  for (let i = 0; i < lines.length; i++) {
    const rawLine = lines[i];
    const line = rawLine.trim();

    // Skip fenced code blocks
    if (line.startsWith("```")) {
      inCodeBlock = !inCodeBlock;
      continue;
    }
    if (inCodeBlock) {
      continue;
    }

    // Heading category
    if (line.startsWith("### ") || line.startsWith("## ")) {
      currentCategory = line.replace(/^#+\s*/, "").replace(/[#*`_]/g, "").trim();
      continue;
    }

    // Checkbox tasks (- [ ] or - [x] or * [ ])
    const checkMatch = line.match(/^[-*]\s*\[([ xX])\]\s*(.*)$/);
    if (checkMatch && checkMatch[2].trim().length > 2) {
      const taskText = checkMatch[2].trim();
      const isCompleted = checkMatch[1].toLowerCase() === "x";
      const existing = existingTasks.find(
        (t) => t.text.toLowerCase() === taskText.toLowerCase()
      );

      extracted.push({
        id: existing?.id || `task-${extracted.length}-${taskText.slice(0, 16).replace(/[^a-zA-Z0-9]/g, "") || "item"}`,
        text: taskText,
        completed: existing !== undefined ? existing.completed : isCompleted,
        category: currentCategory,
      });
      continue;
    }

    // Step items (e.g. "Step 1: ...", "1. Step 1: ...", "**Step 1:** ...", "- Step 1: ...")
    const stepMatch = line.match(/^(?:[-*]|\d+[\.)])?\s*\*{0,2}(?:Step\s+\d+:?)\*{0,2}\s*[:\-–—]?\s*(.*)$/i);
    if (stepMatch && stepMatch[1].trim().length > 3) {
      const taskText = stepMatch[1].trim().replace(/^[:\-–—*`_]+\s*/, "").trim();
      if (taskText.length > 2) {
        const existing = existingTasks.find(
          (t) => t.text.toLowerCase() === taskText.toLowerCase()
        );

        extracted.push({
          id: existing?.id || `task-${extracted.length}-${taskText.slice(0, 16).replace(/[^a-zA-Z0-9]/g, "") || "item"}`,
          text: taskText,
          completed: existing !== undefined ? existing.completed : false,
          category: currentCategory,
        });
      }
    }
  }

  return extracted;
}

/**
 * Synchronizes completion states of existing tasks based on markdown text.
 * Checks for `- [x]` or `* [x]` items in the text and marks matching tasks as completed.
 */
export function syncTasksCompletionFromMarkdown(tasks: PlanTaskItem[], md: string): PlanTaskItem[] {
  if (!md || tasks.length === 0) return tasks;

  const lines = md.split("\n");
  const completedTexts: string[] = [];
  let inCodeBlock = false;

  for (const line of lines) {
    const trimmed = line.trim();
    if (trimmed.startsWith("```")) {
      inCodeBlock = !inCodeBlock;
      continue;
    }
    if (inCodeBlock) continue;

    const match = trimmed.match(/^[-*]\s*\[[xX]\]\s*(.*)$/);
    if (match && match[1].trim()) {
      completedTexts.push(match[1].trim().toLowerCase());
    }
  }

  if (completedTexts.length === 0) return tasks;

  let changed = false;
  const updated = tasks.map((t) => {
    if (t.completed) return t;
    const taskTextLower = t.text.toLowerCase();
    const isCompleted = completedTexts.some(
      (ct) => ct === taskTextLower || ct.includes(taskTextLower) || taskTextLower.includes(ct)
    );
    if (isCompleted) {
      changed = true;
      return { ...t, completed: true };
    }
    return t;
  });

  return changed ? updated : tasks;
}
