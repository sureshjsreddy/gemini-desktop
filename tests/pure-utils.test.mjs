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
import { buildMcpPresets } from "../src/lib/utils/mcpPresets.ts";
import {
  mergeContextUsage,
  mergeTurnUsage,
  formatTokenCount,
  estimateSessionFallbackTokens,
} from "../src/lib/utils/contextUsage.ts";
import {
  normalizeWorkspaceDraft,
  isWorkspaceDraftDirty,
  suggestWorkspaceNameFromPath,
  shouldAutoSyncProfileName,
} from "../src/lib/utils/workspaceDraft.ts";
import {
  normalizeAuthDraft,
  isAuthDraftDirty,
  buildUserEnvComparison,
  evaluateConnectionDiagnostics,
} from "../src/lib/utils/cliSetup.ts";
import { resolvePromptHistoryNavigation } from "../src/lib/utils/promptHistory.ts";
import {
  normalizeAcpPlanEntries,
  extractTodoMilestonesFromToolStep,
  deriveProgressPhases,
  buildLiveWorkFocus,
  extractImpactChips,
  buildTurnReceiptSummary,
  stripToolEmojiPrefix,
} from "../src/lib/utils/toolSteps.ts";
import {
  toWorkspaceRelativeGitPath,
  buildGitStatusMaps,
  getGitStatusBadge,
  splitGitFilePath,
} from "../src/lib/utils/gitStatus.ts";

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

test("mcpPresets: AKBO preset pins akbo-cli@3.2.0 and respects workspacePath", () => {
  const presets = buildMcpPresets("C:\\repos\\sample-app");
  const akbo = presets.find((p) => p.id === "akbo");
  assert.ok(akbo, "akbo preset should exist");
  assert.equal(akbo.config.command, "npx");
  assert.deepEqual(akbo.config.args, ["-y", "akbo-cli@3.2.0", "mcp"]);
  assert.ok(!akbo.config.args.some((a) => a.includes("@latest") || a.includes("3.1.0")));

  const fsPreset = presets.find((p) => p.id === "filesystem");
  assert.ok(fsPreset);
  assert.equal(fsPreset.config.args[2], "C:\\repos\\sample-app");

  const defaultFsPreset = buildMcpPresets().find((p) => p.id === "filesystem");
  assert.equal(defaultFsPreset?.config.args[2], "C:\\projects");
});

test("contextUsage: merges live context window updates without clobbering on zero or turn totals", () => {
  // 1. Initial turn-start notification with used=0, size=1048576
  const initial = mergeContextUsage(null, { usedTokens: 0, contextWindow: 1048576, percentage: 0 });
  assert.equal(initial.usedTokens, 0);
  assert.equal(initial.contextWindow, 1048576);

  // 2. Step completes with real context occupancy 24693
  const step1 = mergeContextUsage(initial, { usedTokens: 24693, contextWindow: 1048576 });
  assert.equal(step1.usedTokens, 24693);
  assert.equal(step1.contextWindow, 1048576);
  assert.ok(step1.percentage > 2.3 && step1.percentage < 2.4);

  // 3. Turn end reports cumulative multi-step API total (e.g. 3.8M) via mergeTurnUsage;
  // context window occupancy MUST remain 24693, never jumping to 3.8M
  const afterTurnEnd = mergeTurnUsage(step1, {
    inputTokens: 3200000,
    outputTokens: 15000,
    cachedReadTokens: 600000,
    thoughtTokens: 4000,
    totalTokens: 3815000,
  });
  assert.equal(afterTurnEnd.usedTokens, 24693);
  assert.equal(afterTurnEnd.contextWindow, 1048576);
  assert.equal(afterTurnEnd.lastTurnUsage?.totalTokens, 3815000);

  // 4. Next turn start emits used=0 placeholder; prior usedTokens (24693) and lastTurnUsage are preserved
  const turn2Start = mergeContextUsage(afterTurnEnd, { usedTokens: 0, contextWindow: 1048576 });
  assert.equal(turn2Start.usedTokens, 24693);
  assert.equal(turn2Start.contextWindow, 1048576);
  assert.equal(turn2Start.lastTurnUsage?.totalTokens, 3815000);

  // 5. Compaction reduces context window occupancy to a smaller non-zero value
  const compacted = mergeContextUsage(turn2Start, { usedTokens: 4200, contextWindow: 0 });
  assert.equal(compacted.usedTokens, 4200);
  assert.equal(compacted.contextWindow, 1048576);

  // 6. Formatting & fallback estimation
  assert.equal(formatTokenCount(3800), "3.8k");
  assert.equal(formatTokenCount(1048576), "1.0M");
  assert.equal(
    estimateSessionFallbackTokens([
      { token_count: 0, content: "Hello world!" }, // 12 chars -> 3 tokens
      { token_count: 150, content: "ignored" },
    ]),
    153
  );
});

test("workspaceDraft: normalizes fields, tracks dirty state accurately, and auto-syncs folder names", () => {
  const baseline = normalizeWorkspaceDraft({
    id: "ws-1",
    name: "gemini-desktop",
    path: "C:\\AIFiles\\gemini-desktop",
    model: "auto",
  });

  // Unchanged copy must not be dirty even if optional booleans/strings were undefined
  const untouched = normalizeWorkspaceDraft({ ...baseline, additional_dirs: undefined, sandbox_enabled: undefined });
  assert.equal(isWorkspaceDraftDirty(untouched, baseline, false), false);

  // Modifying a single option marks dirty
  assert.equal(
    isWorkspaceDraftDirty({ ...untouched, sandbox_enabled: true }, baseline, false),
    true
  );

  // New draft with empty name/path is not dirty yet; with user input is dirty
  const blankDraft = normalizeWorkspaceDraft({ id: "ws-new", name: "", path: "" });
  assert.equal(isWorkspaceDraftDirty(blankDraft, null, true), false);
  assert.equal(
    isWorkspaceDraftDirty({ ...blankDraft, path: "C:\\Projects\\api" }, null, true),
    true
  );

  // Folder name suggestion & auto-sync rules
  assert.equal(suggestWorkspaceNameFromPath("C:\\repos\\billing-service\\"), "billing-service");
  assert.equal(suggestWorkspaceNameFromPath("C:\\"), "");
  assert.equal(shouldAutoSyncProfileName("New Workspace", "", true), true);
  assert.equal(shouldAutoSyncProfileName("old-name", "old-name", true), true);
  assert.equal(shouldAutoSyncProfileName("Custom Profile", "old-name", true), false);
  assert.equal(shouldAutoSyncProfileName("New Workspace", "", false), false);
});

test("cliSetup: dirty check, User Env comparison, and engine-aware diagnostics", () => {
  const base = normalizeAuthDraft({
    authMode: "vertex_ai",
    googleCloudProject: "corp-proj-1",
    googleCloudLocation: "us-central1",
    agyAdcAuth: false,
  });
  assert.equal(isAuthDraftDirty({ ...base, googleCloudProject: " corp-proj-1 " }, base), false);
  assert.equal(isAuthDraftDirty({ ...base, googleCloudProject: "corp-proj-2" }, base), true);
  assert.equal(isAuthDraftDirty({ ...base, agyAdcAuth: true }, base), true);

  // User Env comparison highlights unsynced and mismatched vars without touching secrets
  const plan = buildUserEnvComparison(
    {
      activeEngine: "agy",
      authMode: "vertex_ai",
      googleCloudProject: "corp-proj-2",
      googleCloudLocation: "us-central1",
      agyAdcAuth: true,
    },
    [
      { name: "GOOGLE_CLOUD_PROJECT", value: "corp-proj-1", isSet: true },
      { name: "GOOGLE_CLOUD_LOCATION", value: "us-central1", isSet: true },
      { name: "GOOGLE_GENAI_USE_VERTEXAI", value: undefined, isSet: false },
      { name: "AGY_ADC_AUTH", value: undefined, isSet: false },
    ],
  );
  assert.equal(plan.hasPendingSync, true);
  assert.equal(plan.detectedUserProject, "corp-proj-1");
  assert.equal(plan.rows.find((r) => r.name === "GOOGLE_CLOUD_PROJECT")?.state, "differs");
  assert.equal(plan.rows.find((r) => r.name === "GOOGLE_CLOUD_LOCATION")?.state, "synced");
  assert.equal(plan.rows.find((r) => r.name === "AGY_ADC_AUTH")?.state, "unset_in_windows");

  // Diagnostics for agy should reflect keyring OAuth / agy model instead of gemini-3.8-flash
  const agyDiag = evaluateConnectionDiagnostics({
    activeEngine: "agy",
    effectiveEnvStatus: { installed: true, version: "1.6.0", details: "OK" },
    enginesStatus: {
      activeEngine: "agy",
      geminiInstalled: true,
      agyInstalled: true,
    },
    authMode: "vertex_ai",
    apiKey: "",
    googleCloudProject: "",
    googleAppCredentials: "",
    agyAdcAuth: false,
    gcloudStatus: null,
    testModel: "gemini-3.8-flash",
    testResult: null,
  });
  assert.equal(agyDiag.binary.status, "ok");
  assert.equal(agyDiag.auth.detail, "Antigravity OAuth (Keyring)");
  assert.equal(agyDiag.model.detail, "auto (agy) (Ready)");

  // AGY_ADC_AUTH unset in Windows and unchecked must leave syncRequest.agyAdcAuth undefined
  const unsetAdcPlan = buildUserEnvComparison(
    {
      activeEngine: "agy",
      authMode: "vertex_ai",
      googleCloudProject: "my-proj",
      googleCloudLocation: "us-central1",
      agyAdcAuth: false,
      includeUseVertexFlag: false,
    },
    [],
  );
  assert.equal(unsetAdcPlan.syncRequest.agyAdcAuth, undefined);
  assert.equal(unsetAdcPlan.syncRequest.googleGenaiUseVertexai, undefined);

  // Enterprise ADC requires GCP project when agyAdcAuth=true
  const adcMissingProj = evaluateConnectionDiagnostics({
    activeEngine: "agy",
    effectiveEnvStatus: { installed: true, version: "1.6.0", details: "OK" },
    enginesStatus: { activeEngine: "agy", geminiInstalled: true, agyInstalled: true },
    authMode: "vertex_ai",
    apiKey: "",
    googleCloudProject: "",
    googleAppCredentials: "",
    agyAdcAuth: true,
    gcloudStatus: null,
    testModel: "auto",
    testResult: null,
  });
  assert.equal(adcMissingProj.auth.status, "warn");
  assert.equal(adcMissingProj.auth.detail, "Enterprise ADC (Set GCP Project)");
});

test("promptHistory: ArrowUp/ArrowDown only navigates history when starting from an empty composer", () => {
  const history = ["first prompt", "second prompt", "multi\nline prompt"];

  // Active draft text (even single line or whitespace) must NOT enter prompt history
  const withDraft = resolvePromptHistoryNavigation({
    key: "ArrowUp",
    inputPrompt: "draft in progress",
    selectionStart: 5,
    selectionEnd: 5,
    historyIndex: -1,
    promptHistory: history,
  });
  assert.equal(withDraft.handled, false);
  assert.equal(withDraft.nextHistoryIndex, -1);
  assert.equal(withDraft.nextPrompt, "draft in progress");

  const withSpaces = resolvePromptHistoryNavigation({
    key: "ArrowUp",
    inputPrompt: " ",
    selectionStart: 1,
    selectionEnd: 1,
    historyIndex: -1,
    promptHistory: history,
  });
  assert.equal(withSpaces.handled, false);

  // Empty composer enters history cleanly on ArrowUp
  const enter = resolvePromptHistoryNavigation({
    key: "ArrowUp",
    inputPrompt: "",
    selectionStart: 0,
    selectionEnd: 0,
    historyIndex: -1,
    promptHistory: history,
  });
  assert.equal(enter.handled, true);
  assert.equal(enter.nextHistoryIndex, 0);
  assert.equal(enter.nextPrompt, "multi\nline prompt");

  // On line 2 of recalled multi-line prompt, ArrowUp lets cursor move up inside the prompt first
  const insideMultiLine = resolvePromptHistoryNavigation({
    key: "ArrowUp",
    inputPrompt: enter.nextPrompt,
    selectionStart: enter.nextPrompt.length,
    selectionEnd: enter.nextPrompt.length,
    historyIndex: 0,
    promptHistory: history,
  });
  assert.equal(insideMultiLine.handled, false);

  // Once cursor is on line 1, ArrowUp moves to older entry
  const stepOlder = resolvePromptHistoryNavigation({
    key: "ArrowUp",
    inputPrompt: enter.nextPrompt,
    selectionStart: 2,
    selectionEnd: 2,
    historyIndex: 0,
    promptHistory: history,
  });
  assert.equal(stepOlder.handled, true);
  assert.equal(stepOlder.nextHistoryIndex, 1);
  assert.equal(stepOlder.nextPrompt, "second prompt");

  // ArrowDown steps forward and then clears back to empty
  const stepNewer = resolvePromptHistoryNavigation({
    key: "ArrowDown",
    inputPrompt: stepOlder.nextPrompt,
    selectionStart: stepOlder.nextPrompt.length,
    selectionEnd: stepOlder.nextPrompt.length,
    historyIndex: 1,
    promptHistory: history,
  });
  assert.equal(stepNewer.handled, true);
  assert.equal(stepNewer.nextHistoryIndex, 0);
  assert.equal(stepNewer.nextPrompt, "multi\nline prompt");

  const exitToEmpty = resolvePromptHistoryNavigation({
    key: "ArrowDown",
    inputPrompt: stepNewer.nextPrompt,
    selectionStart: stepNewer.nextPrompt.length,
    selectionEnd: stepNewer.nextPrompt.length,
    historyIndex: 0,
    promptHistory: history,
  });
  assert.equal(exitToEmpty.handled, true);
  assert.equal(exitToEmpty.nextHistoryIndex, -1);
  assert.equal(exitToEmpty.nextPrompt, "");

  // Shift+ArrowUp (selection) must never hijack cursor
  const shiftSel = resolvePromptHistoryNavigation({
    key: "ArrowUp",
    inputPrompt: "",
    selectionStart: 0,
    selectionEnd: 0,
    historyIndex: 0,
    promptHistory: history,
    hasModifiers: true,
  });
  assert.equal(shiftSel.handled, false);

  // ArrowDown on empty composer outside history does nothing
  const downAtRest = resolvePromptHistoryNavigation({
    key: "ArrowDown",
    inputPrompt: "",
    selectionStart: 0,
    selectionEnd: 0,
    historyIndex: -1,
    promptHistory: history,
  });
  assert.equal(downAtRest.handled, false);

  // ArrowDown on line 1 of recalled multi-line prompt moves cursor within text before exiting
  const downInsideMulti = resolvePromptHistoryNavigation({
    key: "ArrowDown",
    inputPrompt: "multi\nline prompt",
    selectionStart: 1,
    selectionEnd: 1,
    historyIndex: 0,
    promptHistory: history,
  });
  assert.equal(downInsideMulti.handled, false);
});

test("turnProgress: Living Now Capsule narrative, ACP milestones, and 1-Line Work Receipt", () => {
  assert.equal(stripToolEmojiPrefix("✅ Ran: cargo test"), "Ran: cargo test");

  // 1. ACP plan entries retain 4 states without losing in_progress
  const acpPlan = normalizeAcpPlanEntries([
    { id: "1", content: "Inspect contracts", status: "completed" },
    { id: "2", content: "Update UI capsule", status: "in_progress" },
    { id: "3", content: "Build installers", status: "pending" },
  ]);
  assert.equal(acpPlan.length, 3);
  assert.equal(acpPlan[0].status, "completed");
  assert.equal(acpPlan[1].status, "in_progress");
  assert.equal(acpPlan[2].status, "pending");

  // 2. Auto-derived phases & burst-grouped live focus when no explicit ACP plan exists
  const steps = [
    { id: "t1", title: "src/lib/types.ts", tool_name: "read_file", status: "completed", parameters: { Path: "C:\\repo\\src\\lib\\types.ts" } },
    { id: "t2", title: "src/lib/utils/toolSteps.ts", tool_name: "read_file", status: "completed", parameters: { Path: "C:\\repo\\src\\lib\\utils\\toolSteps.ts" } },
    { id: "t3", title: "ToolCallGroup.svelte", tool_name: "edit_file", status: "completed", parameters: { Path: "C:\\repo\\src\\lib\\components\\ToolCallGroup.svelte" } },
    { id: "t4", title: "npm run check", tool_name: "run_command", status: "in_progress", parameters: { CommandLine: "npm run check" } },
  ];

  const derivedPhases = deriveProgressPhases(steps, [], true);
  assert.deepEqual(
    derivedPhases.map((p) => `${p.label}:${p.status}`),
    [
      "Reviewed codebase:completed",
      "Updated files:completed",
      "Verifying & running:in_progress",
    ],
  );

  const liveFocus = buildLiveWorkFocus(steps, true);
  assert.equal(liveFocus?.verb, "Running");
  assert.equal(liveFocus?.target, "npm run check");

  const impact = extractImpactChips(steps);
  assert.deepEqual(impact.editedFiles, ["ToolCallGroup.svelte"]);
  assert.equal(impact.chips[0].kind, "file");

  // 3. Finished turn collapses into crisp 1-Line Work Receipt
  const completedSteps = steps.map((s) => ({ ...s, status: "completed" }));
  const receipt = buildTurnReceiptSummary(completedSteps, []);
  assert.equal(receipt.hasFailure, false);
  assert.equal(receipt.headline, "Updated ToolCallGroup.svelte • Ran 1 command");

  const failedSteps = [
    ...completedSteps.slice(0, 3),
    { id: "t4", title: "npm run check", tool_name: "run_command", status: "failed", error: "Type error" },
  ];
  const failedReceipt = buildTurnReceiptSummary(failedSteps, []);
  assert.equal(failedReceipt.hasFailure, true);
  assert.equal(failedReceipt.headline, "1 step failed • Updated ToolCallGroup.svelte");

  // 4. Extracts structured TODO milestones emitted as tool_call when ACP plan update isn't used
  const todoExtracted = extractTodoMilestonesFromToolStep({
    id: "todo-1",
    title: "update_steps",
    tool_name: "update_steps",
    status: "completed",
    parameters: { steps: [{ title: "Step A", status: "done" }, { title: "Step B", status: "in_progress" }] },
  });
  assert.equal(todoExtracted?.length, 2);
  assert.equal(todoExtracted?.[0].status, "completed");
  assert.equal(todoExtracted?.[1].status, "in_progress");

  // 5. Single-step turn skips redundant phase stepper strip & supports milestone-only receipts
  const singleStepPhases = deriveProgressPhases([steps[0]], [], true);
  assert.equal(singleStepPhases.length, 0);

  const milestoneOnlyReceipt = buildTurnReceiptSummary([], acpPlan.map((m) => ({ ...m, status: "completed" })));
  assert.equal(milestoneOnlyReceipt.headline, "3/3 tasks completed");
  assert.equal(milestoneOnlyReceipt.milestoneDone, 3);
  assert.equal(milestoneOnlyReceipt.milestoneTotal, 3);
});

test("gitStatus: maps repo-relative paths, monorepo subpaths, and priority badges cleanly", () => {
  // 1. Standard repo-root workspace
  assert.equal(
    toWorkspaceRelativeGitPath("src/lib/App.svelte", "C:\\repo", "C:\\repo"),
    "src/lib/App.svelte",
  );

  // 2. Monorepo subdirectory workspace maps nested files and ignores sibling packages
  assert.equal(
    toWorkspaceRelativeGitPath("packages/web/src/App.svelte", "C:\\repo\\packages\\web", "C:\\repo"),
    "src/App.svelte",
  );
  assert.equal(
    toWorkspaceRelativeGitPath("packages/api/main.rs", "C:\\repo\\packages\\web", "C:\\repo"),
    null,
  );

  // 3. Builds file & ancestor folder maps with conflict > modified > untracked priority
  const maps = buildGitStatusMaps(
    {
      is_git_repo: true,
      repo_name: "repo",
      repo_root: "C:\\repo",
      active_branch: "main",
      staged: [{ path: "src/App.svelte", status: "added", is_staged: true }],
      unstaged: [
        { path: "src/App.svelte", status: "modified", is_staged: false },
        { path: "src/new/util.ts", status: "untracked", is_staged: false },
      ],
    },
    "C:\\repo",
  );

  assert.equal(maps.fileStatusMap.get("src/App.svelte")?.status, "modified");
  assert.equal(maps.fileStatusMap.get("src/new/util.ts")?.status, "untracked");
  assert.equal(maps.dirtyDirs.has("src"), true);
  assert.equal(maps.dirtyDirs.has("src/new"), true);
  assert.equal(maps.dirtyDirs.has("other"), false);

  // 4. Non-git or null status returns empty maps cleanly, and conflict overrides modified priority
  const emptyMaps = buildGitStatusMaps(null, "C:\\repo");
  assert.equal(emptyMaps.fileStatusMap.size, 0);
  assert.equal(emptyMaps.dirtyDirs.size, 0);

  const conflictMaps = buildGitStatusMaps(
    {
      is_git_repo: true,
      repo_name: "repo",
      active_branch: "main",
      staged: [{ path: "src/merge.ts", status: "modified", is_staged: true }],
      unstaged: [{ path: "src/merge.ts", status: "conflict", is_staged: false }],
    },
    "C:\\repo",
  );
  assert.equal(conflictMaps.fileStatusMap.get("src/merge.ts")?.status, "conflict");

  // 5. Badge and path split helpers
  assert.equal(getGitStatusBadge("conflict").letter, "!");
  assert.equal(getGitStatusBadge("untracked").letter, "U");
  assert.equal(getGitStatusBadge("deleted").letter, "D");
  assert.equal(getGitStatusBadge("renamed").letter, "R");
  assert.equal(getGitStatusBadge("added").letter, "A");
  assert.equal(getGitStatusBadge("modified").letter, "M");
  assert.deepEqual(splitGitFilePath("src/lib/types.ts"), { name: "types.ts", dir: "src/lib" });
  assert.deepEqual(splitGitFilePath("README.md"), { name: "README.md", dir: "" });
});
