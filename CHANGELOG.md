# Changelog

All notable changes to Gemini Desktop are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
