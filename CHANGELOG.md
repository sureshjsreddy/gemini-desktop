# Changelog

All notable changes to Gemini Desktop are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
