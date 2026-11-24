# Changelog

All notable changes to Gemini Desktop are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.8.1] - 2026-11-24

### Fixed & Enhanced
- **Dynamic MCP Server & Plugin Tool Aggregation in ACP Mode (`mcp.rs`, `session.rs`)**:
  - Fixed a critical Agent Client Protocol (ACP) session bootstrapping gap where `mcpServers: []` was hardcoded in `session/new`, causing Antigravity CLI (`agy acp`) to drop all configured session MCP servers.
  - Implemented `get_active_mcp_servers_for_session` to dynamically aggregate and format active MCP servers across enabled Antigravity plugins, global configuration, and workspace configuration into the official ACP `mcpServers` array.
  - Resolved plugin sidecar module resolution failures by resolving relative path arguments in plugin manifests to absolute paths against the plugin root and automatically injecting the plugin directory as `cwd`.
  - Added namespaced plugin server registration (`<plugin>_<server>`) alongside direct aliases to ensure compatibility with skill definitions and corporate multi-agent workflows.
  - Preserved workspace precedence and explicit tool suppression (`disabled: true`).
  - Added `get_session_mcp_servers` Tauri IPC command for frontend introspection and automated test harnesses.

- **AKBO AI Knowledge Base & Memory Dreaming MCP Preset (`McpModal.svelte`)**:
  - Added a 1-click **AKBO AI Knowledge & Dreaming (MCP)** preset (`npx -y akbo-cli mcp`) to `PRESETS` in `McpModal.svelte`.
  - Enables zero-token AST Codebase Knowledge Graph (CKG) search, blast-radius calculation, runtime observation recording, and out-of-band memory consolidation ("dreaming") across Gemini CLI and Antigravity CLI sessions.

- **Architecture Standard & Automated Verification**:
  - Updated [PROD-MCP-001.yaml](file:///c:/AIFiles/gemini-desktop/knowledge/product/PROD-MCP-001.yaml) (v1.5.0) to standardize ACP multi-layer MCP server aggregation and plugin path resolution.
  - Added unit test suites verifying workspace server filtering, disabled tool suppression, and plugin argument resolution (`test_get_active_mcp_servers_for_session_workspace_and_filtering`, `test_plugin_mcp_args_resolution_and_cwd_injection`).

---

## [0.8.0] - 2026-11-22

### Added & Enhanced
- **Unified Command Palette (`Ctrl+P` / `Ctrl+Shift+P`, `CommandPaletteModal.svelte`)**:
  - Global hotkeys `Ctrl+P` and `Ctrl+Shift+P` plus a dedicated search button in `Sidebar.svelte` for fast keyboard-driven navigation.
  - Interactive fuzzy search across slash tools (`/plan`, `/diff`, `/terminal`, `/mcp`, `/compact`, `/setup`, `/settings`, `/doctor`), active conversation switching, and instant theme selection (`Dark`, `Light`, `System`).
  - Integrated with `modalManager` (`src/lib/modal.svelte.ts`) with keyboard arrow navigation, active item selection, and backdrop dismissal.

- **Conversation Forking & Branching from Any Turn (`fork_session`)**:
  - Fork any conversation from any assistant response in `ChatView.svelte` into a new independent session without altering or losing the original discussion history.
  - Added `fork_session` in `database.rs` and exposed Tauri command `fork_session` in `commands/db.rs` copying messages up to the chosen turn with new UUIDs and custom branch titles (`Original Session (Fork)`).
  - Added comprehensive backend unit tests (`test_fork_session`, `test_fork_session_entire_history_and_custom_title`).

- **Direct File & Screenshot Drag-and-Drop Attachment (`ChatView.svelte`, `ChatPromptComposer.svelte`)**:
  - Interactive HTML5 drag-and-drop zone with animated glassmorphism dashed overlay.
  - Automatically processes dropped image files (`.png`, `.jpg`, `.webp`, `.gif`, `.svg`) via `save_clipboard_image` into `.gemini/attachments/` and generates preview thumbnail chips.
  - Resolves dropped workspace source code files and documents relative to the active workspace into standard `@path` references.

- **Tool Execution Tree Multi-Toggle & Error Filtering (`ToolCallGroup.svelte`, `ToolCallNode.svelte`)**:
  - Added filter pills: **"All (N)"** vs **"Errors Only (M)"** to isolate failed tool calls immediately.
  - Added **"Expand All / Collapse All"** button (`ChevronsUpDown` / `ChevronsDownUp`) to expand or collapse all step parameters, raw stdout, and error traces simultaneously.
  - Propagated `forceExpanded` property to `ToolCallNode.svelte` while preserving manual per-card click toggling.

- **Session Context Token & Ceiling Meter**:
  - Live token consumption meter in the chat header tracking active tokens against the 1M ceiling.
  - Visual warning badge and compact suggestion when approaching 750,000+ tokens.

- **Clipboard Image & Screenshot Pasting**:
  - Paste screenshots directly (`Win+Shift+S` or image copy via `Ctrl+V`) into composer.
  - Backend `save_clipboard_image` command saves images into `.gemini/attachments/` and renders preview thumbnail chips.

- **Prompt History Navigation**:
  - `↑` and `↓` arrow navigation in `ChatPromptComposer.svelte` cycling through past sent prompts while preserving active in-progress drafts.

- **Markdown Sanitization & Safe Code Block Copy**:
  - Integrated `DOMPurify` to sanitize rendered HTML and prevent XSS.
  - Replaced inline `onclick` string handlers on code block copy buttons with safe delegated event listeners in `ChatView.svelte`.

---

## [0.7.7] - 2026-11-21

### Added & Enhanced
- **Standalone Portable Executable Distribution (`.github/workflows/release.yml`, `README.md`)**:
  - Re-introduced automated publishing of `GeminiDesktop-Portable.exe` in the GitHub Actions release workflow.
  - Generates and attaches the zero-install standalone Windows executable (`GeminiDesktop-Portable.exe`) alongside official WiX MSI (`.msi`) and NSIS (`-setup.exe`) installers.
  - Implemented resilient release asset upload step with automatic retries in CI/CD pipeline.
  - Updated download and installation instructions in `README.md`.

---

## [0.7.6] - 2026-11-21

### Added & Enhanced
- **Adaptive Fluid Width & Visible Area Scaling (`ChatView.svelte`, `ChatPromptComposer.svelte`)**:
  - Replaced the fixed `max-w-3xl` (768px) message constraint with a fluid, responsive container (`w-full max-w-4xl lg:max-w-5xl xl:max-w-6xl 2xl:max-w-7xl mx-auto`).
  - Conversation blocks, code blocks, diffs, tables, and tool progress trees now seamlessly expand to utilize the full visible canvas on larger screens and when collateral navigation panels (Left Sidebar and Right Solution Explorer) are collapsed.
  - Automatically re-adjusts between reading comfort (`896px` / `1024px`) when sidebars are open and expanded width (`1152px` / `1280px`) when panels are collapsed.
  - Sized user prompt bubbles proportionally (`max-w-xl sm:max-w-2xl lg:max-w-3xl`) right-aligned to maintain balanced typography, while assistant messages utilize `w-full min-w-0` across the conversation column.
  - Synchronized width scaling across the entire chat surface:
    - **Prompt Composer** (`ChatPromptComposer.svelte`): Scaled to `max-w-4xl lg:max-w-5xl xl:max-w-6xl 2xl:max-w-7xl mx-auto`.
    - **Plan Mode Workbench** (`PlanWorkbench.svelte`): Wrapped in coordinated responsive width container.
    - **ACP Tool Permission Intercept Card** (`ToolPermissionCard.svelte`): Centered and aligned to fluid container.
    - **Instant Debug Doctor Diagnostic Banner** (`DebugDoctorBanner.svelte`): Expanded to match conversation width.
    - **Empty State Starter Cards** (`ChatView.svelte`): Increased grid width to `max-w-xl lg:max-w-2xl xl:max-w-3xl` for improved card proportions.

---

## [0.7.5] - 2026-11-20

### Added
- **True Wall-Clock Turn Duration & Human-Friendly Telemetry**:
  - Replaced isolated tool subprocess runtime summation with accurate wall-clock turn duration ($\Delta t = t_{\text{assistant}} - t_{\text{user}}$) for conversation history, accurately reflecting total thinking, searching, and synthesis time.
  - Added live elapsed timer (`started_at`) that increments reactively every second during active streaming with a pulse indicator (`Executing tasks (2/4 completed)... 1m 15s`).
  - Added `formatDuration(ms)` helper in `toolSteps.ts` formatting durations concisely across thresholds: `< 10s` (e.g. `0.4s`, `4.8s`), `10s - 59s` (e.g. `24s`, `58s`), `60s+` (e.g. `2m 15s`, `3m`), and `60m+` (e.g. `1h 12m`).
  - Integrated turn duration metrics directly into the assistant message footer alongside token usage (`3:45 PM • 2m 15s • ~1,450 tokens`).
- **Antigravity-Inspired Hierarchical Tool Execution Progress Tree (`ToolCallGroup.svelte`)**:
  - Multi-step tool runs are now dynamically grouped into an expandable progress tree component inspired by modern agentic IDEs.
  - While an agent is working, completed steps are tucked away into an accordion header (`[▶] Executing tasks (N/M completed)...`) displaying only the currently active working step underneath to eliminate visual clutter.
  - When all tasks finish, steps collapse into a single compact header (`[▶] N steps completed • 2m 15s`) with a one-click toggleable tree view (`View tree` / `Hide tree`).
  - Individual tool cards in `ToolCallNode.svelte` now display human-readable duration badges (`formatDuration(step.duration_ms)`).

---

## [0.7.4] - 2026-11-20

### Fixed & Optimized
- **Antigravity CLI (`agy`) ACP Turn Completion & Hanging Fix**:
  - Resolved an issue where ACP turns with `agy-cli` occasionally remained stuck showing "working" after completion.
  - Added resilient JSON-RPC ID parsing supporting both numeric and string integer IDs (`as_u64()` and string parsing).
  - Enhanced the ACP fallback result detector in `acp_client.rs` to explicitly match `val.pointer("/result/stopReason")` and fall back to `get_active_local_session()` if `take_prompt_session(id)` misses, guaranteeing that `agy` turn completions (which send `stopReason: "end_turn"` without redundant content fields) always emit `is_done: true`.
  - Added `has_pending_prompt_for_session` to accurately track in-flight prompt turns and avoid spurious session cancellations on idle engines.
- **Steering Response Completion & Clean Guiding**:
  - Fixed a frontend race condition in `+page.svelte` where genuine steering response completions were trapped and swallowed as prompt cancellations, causing an infinite "is work completed?" cycle.
  - Conditioned user steering prompt generation in `session.rs` so plan task classifications (`ADD_TASK`, `MODIFY_TASK`, `CANCEL_TASK`) are only applied in Plan Mode, eliminating steering prompt poisoning during standard chat/code turns.

### Changed & Refactored
- **Prompt Composer Toolbar Decluttering (`ChatPromptComposer.svelte`)**:
  - Removed duplicate Policy Approval Mode badge from the input bar (already displayed prominently in the top header and configured in workspace settings), eliminating horizontal wrapping on narrower windows.
  - Moved standalone *Debug Doctor* button into the **Attach** quick menu (keeping `Ctrl+Alt+D`, `/doctor`, and automatic clipboard paste detection fully functional), giving the working directory chip and send controls a clean, uncluttered layout.
- **Empty State Interactive Onboarding (`ChatView.svelte`)**:
  - Replaced static text with dynamic CLI engine identification (`Antigravity CLI (agy)` vs `Gemini CLI`).
  - Added 3 one-click interactive starter prompt cards:
    - 🔍 **Architecture**: *"Explain the architecture, folder structure, and tech stack of this workspace"*
    - ⚡ **Git Changes**: *"Inspect git status, summarize uncommitted changes, and identify what needs attention"*
    - 📝 **Feature Plan**: *"/plan Create an implementation plan for my next feature or refactoring task"*
- **Header Bar Streamlining (`ChatView.svelte`)**:
  - Integrated "Compress Context" into the **Export & Actions** dropdown menu with its `/compress` shortcut, decluttering the top header bar and keeping it focused on chat title, active model, and session diff badges.
- **Hierarchical Tool Execution Tree & Progress Stack (`ToolCallGroup.svelte`)**:
  - Replaced the flat vertical stack of individual tool cards with a collapsible hierarchical tree component inspired by modern agentic IDEs.
  - While an agent is working, completed steps are tucked into a compact accordion header (`[▶] Executing tasks (N/M completed)...`) while displaying only the single active running step underneath with a live spinner.
  - When execution finishes, all steps collapse into a single clean summary (`[▶] N steps completed (X.Xs)`), leaving the assistant's final Markdown response clean and fully readable without terminal card clutter.
  - Interactive "View tree" / "Hide tree" toggle allows users to expand the tree hierarchy with tree-branch connectors, status checkmarks, and duration badges at any time.
- **Semantic Human-Friendly Task Summaries (`resolve_human_friendly_tool_title`)**:
  - Eliminated robotic machine identifiers (`call_0`, `call_1`, `tool`) from tool cards.
  - Dynamically extracts semantic user intent from `parameters` (`toolSummary`, `CommandLine`, `Path`, `query`), formatting natural action titles like `Run cargo test --lib`, `Read knowledge/index.yaml`, and `Edited src/acp_client.rs`.

---

## [0.7.3] - 2026-11-18

### Added
- **Antigravity CLI (`agy`) Plugins & Extensions Hub (`PluginsModal.svelte`)**:
  - Full native GUI management for Antigravity agent plugins and modular extension bundles.
  - **Installed Plugins (`Manage`)**: Live queries installed plugins via `list_agy_plugins` (`agy plugin list --json`), displaying skill counts, MCP server attachments, and active rules, with live 1-click Enable / Disable switches (`toggle_agy_plugin`), Explorer folder access, and uninstallation.
  - **Explore Catalog**: Curated directory of official and partner plugins (`android-cli-plugin`, `chrome-devtools-plugin`, `firebase`, `science`, `google-antigravity-sdk`) with 1-click installation and custom target install (`install_agy_plugin`).
  - **Custom Plugin Studio (`+ Create Custom`)**: Interactive scaffolding wizard to generate and activate custom plugin bundles in `%USERPROFILE%\.gemini\config\plugins\<name>\` (`create_custom_plugin`), pre-configuring `plugin.json`, `README.md`, skills, MCP tools, agent rules (`rules/AGENTS.md`), and specialist subagents.
  - **Dual-Engine Awareness**: Seamlessly displays an educational banner when `gemini` CLI is active, informing users that plugins are an `agy` agentic feature without disruptive conversion prompts.
- **Backend Rust Plugin IPC Commands**:
  - Implemented `list_agy_plugins`, `toggle_agy_plugin`, `install_agy_plugin`, `uninstall_agy_plugin`, `create_custom_plugin`, and `open_plugin_directory` in `src-tauri/src/commands/plugins.rs` with full unit test coverage.

### Changed & Refactored
- **Sidebar UX Architecture Redesign (Pinned Chat Hero + Bottom Settings Hub)**:
  - Eliminated the 9-button vertical stack in `Sidebar.svelte` that pushed recent conversations below the fold on standard and laptop screens.
  - Assigned primary vertical height (`flex-1 min-h-0 overflow-y-auto`) to **Recent Chats**, keeping 10–15+ conversations immediately visible without scrolling.
  - Placed compact high-frequency chat actions (`+ New Chat` and `Search History`) directly beneath the Workspace Selector.
  - Reorganized secondary tools into a compact bottom toolbar:
    - Side-by-side Developer Drawer toggles: `Terminal` (`Ctrl+\``) and `Workspace Explorer` (`Ctrl+Alt+L`).
    - Unified **Tools & Settings** launcher opening a sleek upward-floating popover for `Prompt Library`, `MCP Servers`, `Plugins & Extensions`, `CLI & Auth Setup`, `CLI Settings`, and `Theme & Colors`.

---

## [0.7.2] - 2026-11-17

### Added
- **Interactive `/` Tools & Slash Commands Autocomplete Palette**:
  - Introduced an interactive `/` tools and commands autocomplete popover (`ChatSlashPopover.svelte`) in the chat prompt composer.
  - Dynamically adapts available tools based on the active CLI engine (`gemini` vs `agy`):
    - **Antigravity CLI (`agy`)**: Surfaces `/plugins` (browse marketplace plugins, custom skills, and extensions).
    - **Gemini CLI (`gemini`)**: Surfaces `/compress` (prompt context compression).
    - **Core Desktop Tools**: Surfaces `/plan` (Plan Mode Workbench), `/diff` (Visual Diff & File Changes), `/terminal` (Integrated Terminal Drawer), `/mcp` (MCP Server Manager), `/doctor` (Instant Debug Doctor), `/compact` (Session Context Compaction), `/tools` (List Agent Capabilities), `/model` (Model Parameters), `/settings` (Global CLI Settings), `/setup` (CLI Environment & Auth Setup), `/rewind` (Turn Rewind), and `/clear` (Clear Messages).
  - Full keyboard navigation with `↑`/`↓` arrow keys, `Enter`/`Tab` execution, and `Esc` dismissal, with instant GUI action dispatch and ACP command expansion.

### Fixed & Optimized
- **Zero-Process-Spawn CLI Diagnostics & Process Optimization**:
  - Eliminated process flooding (`cmd.exe`, `where.exe`, `powershell.exe`) previously spawned every time the CLI Setup or Settings modals were opened.
  - Implemented thread-safe in-memory cache `CLI_PATH_CACHE` with `invalidate_cli_path_cache()` in `process_manager.rs`, checking well-known directories and in-memory `PATH` without spawning external shell processes.
  - Replaced PowerShell child process execution with native Win32 Registry reading (`HKCU\Environment\Path`) using `windows-sys` (`RegOpenKeyExW` / `RegQueryValueExW`) for instantaneous path validation.
  - Replaced `cmd.exe /c where.exe gcloud` with direct disk inspection of Google Cloud configuration files.
  - Applied Windows `CREATE_NO_WINDOW` (flag `0x08000000`) across all child process executions to prevent console window flashing.
  - Cached environment checks in `CliSetupModal.svelte` to prevent redundant invocations when engine state has not changed.
- **Dual-Engine MCP Server Routing & Diagnostics**:
  - Documented and verified configuration isolation between Gemini CLI (`%USERPROFILE%/.gemini/settings.json`) and Antigravity CLI (`%USERPROFILE%/.gemini/config/mcp_config.json`).
  - Verified Playwright MCP browser automation and tool schema generation across both engines.

---

## [0.7.1] - 2026-11-16

### Fixed
- **Engine-Aware Dynamic Model Discovery Branding**:
  - Enhanced `discover_gemini_models` Tauri IPC command to return `engine` and `cli_version` metadata alongside discovered models.
  - Resolved issue in `WorkspaceModal` where discovering models under Antigravity CLI (`agy`) displayed hardcoded Gemini CLI version notifications (`via Gemini CLI (v0.67+)`).
  - Discovery notification banner, button title/label (`Discover Live Models (agy)` vs `Discover Live Models (v0.67+)`), and dropdown optgroup headers now dynamically adapt to the active CLI engine.
  - Automatically resets discovered live models state when switching between `gemini` and `agy` engines.

---

## [0.7.0] - 2026-11-12

### Added
- **Dual-Engine CLI Runtime Architecture**:
  - Full support for switching between official Gemini CLI (`gemini`) and Antigravity CLI (`agy`) at application level with persistent SQLite storage.
  - Cross-engine model sanitization fallback (`sanitize_model_for_engine`) preventing incompatible model crashes.
  - Windows Job Objects supervision with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` applied across both engines.
  - Configuration isolation between `~/.gemini/settings.json` and `~/.gemini/antigravity-cli/settings.json`.
  - Seamless 1-click workspace skill synchronization from `.gemini/skills/` to `.agents/skills/`.

---

## [0.6.0] - 2026-11-07

### Added
- **Dynamic Model Discovery (Gemini CLI v0.67+ Alignment)**:
  - Implemented `discover_gemini_models` Rust Tauri command invoking `gemini models --output-format json` (with `--list-models` and plaintext fallback for pre-v0.67 installations).
  - Queries active environment credentials and extracts model display names, descriptions, and input/output token limits.
  - Added interactive "Discover Live Models (v0.67+)" button with refresh animation in `WorkspaceModal` to dynamically populate available models.
- **Model Context Protocol (MCP) Server Transparency in Tool Permissions**:
  - Enhanced ACP client JSON-RPC parser to extract source MCP server metadata (`/params/toolCall/server`, `/params/server`).
  - Added dedicated `🔌 MCP: <server>` badge to `ToolPermissionCard` allowing users to distinguish MCP-driven system calls from built-in CLI operations before approving permissions.
- **Rate-Limit 429 Awareness & Countdown Notices**:
  - Added real-time regex parsing on ACP stream stderr to detect HTTP 429 rate limit responses and `cooling down for (\d+)s` intervals.
  - Surfaced live in-stream countdown notices warning users of temporary rate limits instead of displaying silent timeouts or generic failures.
- **Automated Verification**:
  - Expanded Cargo unit test suite to 90 passing tests covering JSON array, JSON object, and plaintext fallback parsing for dynamic model discovery.

### Changed
- **Gemini Model Routing & Lifecycle Alignment**:
  - Deprecated legacy Gemini 1.5 series across model selectors, default options, and documentation per Google AI Studio deprecation schedule.
  - Aligned model presets with Gemini 3.5 / 3.8 and active Gemini 2.5 series (`gemini-3.8-flash`, `gemini-3.5-flash`, `gemini-3.5-flash-lite`, `gemini-2.5-pro`, `gemini-2.5-flash`).
  - Updated deprecated model error handling to surface actionable upgrade instructions.

---

## [0.5.2] - 2026-11-05

### Fixed
- **AI Conventional Commit Generation via Stdin Streaming**:
  - Switched Gemini CLI commit prompt generation from command line arguments (`-p <diff>`) to standard input streaming (`stdin.write_all`), completely eliminating the Windows 8,191-character command line buffer overflow limit on large Git diffs.
  - Added direct Node.js execution of `gemini.js` when running on Windows with npm batch wrapper scripts (`gemini.cmd`), preventing `cmd.exe` escape errors and quote mangling.
- **AI Refusal & Excuse Filtering**:
  - Added `is_valid_commit_message` validation in the Rust backend to intercept and reject LLM refusal messages (e.g., *"no git diff was included in your message, and direct shell execution is not enabled"* or apologetic excuses).
  - Automatically falls back to deterministic heuristic commit message synthesis when LLM generation is unavailable or rejected.
  - Added frontend detection in `GitChangesWindow` to block model refusal phrases and surface an actionable error banner prompting the user to stage or save changes.
- **Synthetic Git Diffs for Untracked Files**:
  - Automatically reads and synthesizes Git diff hunks for newly created untracked files (`--- /dev/null +++ b/<path>`), allowing Gemini CLI to generate accurate commit messages even when changes consist purely of untracked files.

---

## [0.5.1] - 2026-11-03

### Fixed
- **Universal Git Undo & Revert Compatibility**:
  - Resolved `pathspec did not match any file(s) known to git` error when discarding newly added (`[A]`) files by unstaging from Git index and purging safely from disk.
  - Replaced `git restore` with universal `git reset HEAD` and `git checkout HEAD` to ensure full compatibility with older Git CLI versions (< 2.23).
  - Hardened `git_undo_all` using universal `git reset --hard HEAD` and `git clean -fd` to cleanly discard all modifications, deletions, and untracked files.
- **CRLF vs. LF Line Ending Normalization**:
  - Fixed false-positive whole-file deletions in visual diff where all lines appeared deleted and re-added due to Windows carriage return (`\r\n`) mismatches against Git LF baselines.
- **Whitespace & Indentation Suppression**:
  - Integrated `-w` (`--ignore-all-space`) and `--ignore-cr-at-eol` into Git diff execution so that whitespace, indentation, and formatting changes are cleanly ignored, surfacing only genuine code modifications.
  - Added user-friendly notice card in `SessionChangesModal` when file differences are purely formatting or line endings.

---

## [0.5.0] - 2026-11-02

### Added
- **Remote Git Synchronization (Pull, Push & Commit-and-Push)**:
  - Implemented `git_pull` in Rust backend with `GIT_TERMINAL_PROMPT=0` to prevent hanging on missing credentials and gracefully handle merge conflicts and auth failures.
  - Implemented `git_push` with automatic upstream branch configuration fallback (`git push -u origin <branch>`).
  - Added dedicated `[↓ Pull]` and `[↑ Push]` top bar buttons with live unpulled (`↓behind`) and unpushed (`↑ahead`) commit badges and animated status states.
  - Added 1-click `[Commit & Push]` composite action in the commit row for atomic check-in and remote synchronization in a single developer turn.
- **Context-Aware AI Conventional Commit Generation via Gemini CLI**:
  - Enhanced `git_generate_commit_message` to dispatch to the local Gemini CLI (`gemini-3.8-flash`) using active chat turns, draft prompts, and issue/story references (`PROJ-104`, `#42`).
  - Integrated resilient local heuristic fallback (`synthesize_heuristic_commit_message`) when Gemini CLI is unavailable or offline.
- **Defensive Toolbar Layout for Long Branch Names**:
  - Implemented `shrink-0` flexbox styling on action buttons (`Pull`, `Push`, `Refresh`, `More options`) ensuring they never compress or get displaced.
  - Added bounded width and ellipsis truncation (`min-w-0 max-w-[125px] sm:max-w-[150px] truncate`) on active branch tags with full native hover tooltip inspection.

---

## [0.4.1] - 2026-11-01

### Added
- **Upstream Ahead / Behind Git Sync Indicators**:
  - Implemented `get_git_ahead_behind` in Rust backend tracking unpushed (`↑`) and unpulled (`↓`) commits against upstream tracking branches (`HEAD...@{upstream}`).
  - Added colored glyph counters directly on the active branch pill in the Git Changes top bar.
- **Interactive Working Tree Undo in Diff Modal**:
  - Enhanced `SessionChangesModal` with customizable modal title, description, and action button labels.
  - Wired direct "Undo Changes" action (`git_undo_change`) from the Git diff comparison modal with confirmation dialogs.
  - Automatically hidden whole-session "Revert All" button when viewing individual Git diff comparisons.
- **Visible Error Feedback for Git Staging & Discard Operations**:
  - Added immediate UI error feedback banners alerting users if Git commands fail due to index locks or permission restrictions.

### Changed
- **Multi-Line Commit Message Ergonomics**:
  - Updated commit textarea keydown handler in `GitChangesWindow` to require `Ctrl+Enter` / `Cmd+Enter` for commit submission, enabling natural `Enter` keypresses for multi-line commit descriptions and bulleted summaries.

---

## [0.4.0] - 2026-10-31

### Added
- **Visual Studio Git Changes Tool Window**:
  - Full-featured check-in tool window accessible via tab switching on the right dock or keyboard shortcut `Ctrl+Shift+G`.
  - Staged Changes and Changes (unstaged) partitioned sections with collapsible accordions and item counters.
  - 1-click `[+]` Stage All / `[-]` Unstage All and individual Stage/Unstage quick action buttons.
  - Color-coded change type glyph badges matching Visual Studio 2022:
    - `[A]` Added (emerald)
    - `[M]` Modified (sky)
    - `[D]` Deleted (rose with strikethrough font styling)
    - `[U]` Untracked (olive/emerald)
    - `[R]` Renamed (amber)
    - `[!]` Merge Conflict (red)
  - Hover tooltips clarifying status definitions and file extension icons matching Solution Explorer.
  - Visual Studio commit priority logic: commits only staged files if staged items exist (`git commit -m`), or all tracked working tree changes if none are staged (`git commit -a -m`). Dynamic commit button labels (`Commit Staged (N)` vs `Commit All (N)`).
  - 1-click `[✨ AI Generate]` Conventional Commit message generator analyzing active diffs.
  - Line-by-line unified diff comparisons against `HEAD` (for staged files) or unmodified baselines (for unstaged files).
  - Right-click context menu with status header, system default app opener, Windows Notepad launcher, File Explorer reveal, and path copy actions.
  - Reactive auto-refresh synchronization on turn completion, mutating tool updates, and window focus events.
- **Automated Verification Suite**:
  - Added unit tests for Git porcelain status parsing, staged/unstaged partitioning, and whitespace handling.
  - Expanded test coverage to 76 automated Cargo unit tests.

### Fixed
- **Session File Changes Revert Resurrection**:
  - Implemented `reverted_files: HashSet<String>` in `SnapshotManifest` to prevent message history re-scans from resurrecting reverted files as phantom "deleted" items with 0 lines or "created" items.
  - Reverting newly created files deletes them from disk and cleanly eliminates them from the changes window.
  - Reverting pre-existing modified files restores baseline snapshot content without re-tracking as newly created.
  - "Revert All" cleanly drops the changes counter to 0, hides the header badge, and closes/resets the changes modal.
- **Per-Conversation Snapshot Isolation**:
  - Strictly isolated snapshots and manifests into `%LOCALAPPDATA%\GeminiDesktop\snapshots\<session_id>\manifest.json` per conversation.
- **Subsequent Turn Re-edits**:
  - Automatically un-reverts and establishes a fresh tracking baseline when the agent modifies a previously reverted file in a subsequent turn.

---

## [0.3.0] - 2026-10-30

### Added
- **Global Gemini CLI Settings GUI**: Full visual configuration for model defaults, telemetry, output limits, and system prompt templates.
- **Session Context Compaction**: Continuation and recovery standard for long-running conversation turns approaching 1M token ceilings.
- **Git-Aware Change Tracking**: Zero-overhead change tracking integrating with Git HEAD baselines.

---

## [0.2.0] - 2026-10-15

### Added
- **Interactive Plan Mode Workbench**: Structured task extraction, checklist tracking, and step-by-step progress sync.
- **Windows Job Objects Process Supervision**: Kernel-level child process termination (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`) to eliminate zombie background processes.
- **Native Notifications & Toast Alerts**: AppUserModelID-based Windows toast notifications for long-running background completions.
- **Local-First SQLite Persistence with FTS5**: Millisecond full-text search across conversations, prompt templates, and workspace profiles.
