import test from "node:test";
import assert from "node:assert/strict";

import {
  sanitizeModelForEngine,
  getEffectiveModel,
  formatContextLimitLabel,
} from "../src/lib/utils/engineModels.ts";
import { extractTasksFromMarkdown } from "../src/lib/utils/planParser.ts";
import { parseDebugError } from "../src/lib/utils/debugParser.ts";
import { isHiddenOrIgnored, isNodeVisible } from "../src/lib/utils/fileFilters.ts";

test("engineModels: cross-engine sanitization & fallback labels", () => {
  assert.equal(sanitizeModelForEngine("gemini-2.5-pro", "agy"), "auto");
  assert.equal(sanitizeModelForEngine("gemini-3.8-flash", "agy"), "auto");
  assert.equal(sanitizeModelForEngine("claude-sonnet-4-6", "agy"), "claude-sonnet-4-6");

  assert.equal(sanitizeModelForEngine("claude-sonnet-4-6", "gemini"), "auto");
  assert.equal(sanitizeModelForEngine("gemini-4", "gemini"), "auto");
  assert.equal(sanitizeModelForEngine("gemini-4-argon", "gemini"), "auto");
  assert.equal(getEffectiveModel("gemini-4", "gemini"), "auto (fallback)");
  assert.equal(sanitizeModelForEngine("gemini-3.8-flash", "gemini"), "gemini-3.8-flash");

  // Unknown future engine preserves custom models cleanly
  assert.equal(sanitizeModelForEngine("claude-opus-4-6", "claude"), "claude-opus-4-6");
  assert.equal(getEffectiveModel("gemini-2.5-pro", "agy"), "auto (fallback)");
  assert.equal(getEffectiveModel("auto", "gemini"), "auto");
  assert.equal(getEffectiveModel("", "agy"), "auto");
  assert.equal(getEffectiveModel("orchestrator", "agy"), "orchestrator");
  assert.equal(getEffectiveModel("orchestrator", "gemini"), "auto (fallback)");
  assert.equal(getEffectiveModel("gemini-3.8-flash", "gemini"), "gemini-3.8-flash");
  assert.equal(formatContextLimitLabel(1048576), "1049k ctx");
  assert.equal(formatContextLimitLabel(null), "");
});

test("planParser: extracts checkboxes and skips fenced code blocks", () => {
  const md = [
    "## Backend Refactor",
    "- [ ] Extract EngineRegistry trait",
    "- [x] Wire shared CLI builder",
    "```markdown",
    "- [ ] Fake checkbox inside code fence",
    "```",
  ].join("\n");

  const tasks = extractTasksFromMarkdown(md);
  assert.equal(tasks.length, 2);
  assert.equal(tasks[0].text, "Extract EngineRegistry trait");
  assert.equal(tasks[0].completed, false);
  assert.equal(tasks[0].category, "Backend Refactor");
  assert.equal(tasks[1].completed, true);
});

test("debugParser: parses stack trace error frame", () => {
  const sample = `System.NullReferenceException: Object reference not set
   at MyApp.Program.Main() in C:\\src\\Program.cs:line 42`;
  const parsed = parseDebugError(sample);
  assert.ok(parsed);
  assert.equal(parsed.fileName, "Program.cs");
  assert.equal(parsed.lineNumber, 42);
});

test("fileFilters: filters ignored and hidden paths cleanly", () => {
  assert.equal(isHiddenOrIgnored("node_modules/pkg/index.js"), true);
  assert.equal(isHiddenOrIgnored("src/.git/HEAD"), true);
  assert.equal(isHiddenOrIgnored("src/lib/types.ts"), false);
  assert.equal(isNodeVisible(".env", false), false);
  assert.equal(isNodeVisible(".env", true), true);
  assert.equal(isNodeVisible("README.md", false), true);
});
