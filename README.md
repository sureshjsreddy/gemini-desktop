# 🚀 Gemini Desktop

[![Release](https://img.shields.io/github/v/release/sureshjsreddy/gemini-desktop?style=flat-square&color=38bdf8)](https://github.com/sureshjsreddy/gemini-desktop/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg?style=flat-square)](LICENSE)
[![Tauri v2](https://img.shields.io/badge/Tauri-v2-blue?style=flat-square&logo=tauri)](https://v2.tauri.app)
[![Svelte 5](https://img.shields.io/badge/Svelte-5-FF3E00?style=flat-square&logo=svelte)](https://svelte.dev)
[![Rust](https://img.shields.io/badge/Rust-2021-black?style=flat-square&logo=rust)](https://www.rust-lang.org)
[![Tailwind CSS v4](https://img.shields.io/badge/Tailwind-v4-38bdf8?style=flat-square&logo=tailwindcss)](https://tailwindcss.com)

A modern, blazing-fast native desktop application for interacting with the **Gemini CLI** via the **Agent Client Protocol (ACP)** over standard I/O (JSON-RPC). Built from the ground up with **Tauri v2**, **Rust**, and **SvelteKit / Svelte 5**.

---

## ✨ Features

- **🔗 Universal Code Location, Line Goto & Resource Link Navigation**
  - **Interactive File Link Pills**: Click any code reference (e.g. `api/main.py:179`, `file.ts:42`, `PROJECT_ANALYSIS.md`) directly within the chat to immediately open it in your preferred editor.
  - **Line & Column Goto**: Seamlessly parses `:line`, `:line:col`, `:line-line2`, and `#Lline`, launching VS Code (`code`), Cursor (`cursor`), or Windsurf (`windsurf`) via `-r -g` jumping directly to the targeted line without console flashing.
  - **Fuzzy Workspace Search**: Automatically locates referenced files across subdirectories (e.g. `backend/api/main.py`) when relative paths omit parent folders.
  - **Antigravity Brain Artifact Resolution**: Opens internal session reports and deliverables stored in `.gemini/antigravity/brain/...` without repository pollution.
  - **DOMPurify Link Target Protection**: Protected `data-path` embedding guarantees clicks are never dropped due to HTML sanitizer URI filtering.
  - **Interactive "Create & Open"**: Automatically offers to scaffold missing directories and files on demand when clicking references to uncreated files.

- **⚡ Native Agent Client Protocol (ACP) & Resilient Engine Runtime**
  - Full bidirectional JSON-RPC streaming directly over the child CLI process `stdin`/`stdout`.
  - Resilient JSON-RPC ID parsing and `stopReason: "end_turn"` completion handling across Gemini CLI and Antigravity CLI (`agy`), eliminating hanging "working" states.
  - Non-blocking real-time model steering mid-flight with mode-aware steering prompts.
  - Interactive **Tool Execution Permission Intercepts** — review and approve tool/command execution before changes occur on your machine.

- **🎛️ Unified Command Palette (`Ctrl+P` / `Ctrl+Shift+P`)**
  - Instant fuzzy discovery and execution of navigation targets, tools, and actions across the workspace.
  - Quick-switch between active conversations, trigger slash actions (`/plan`, `/diff`, `/terminal`, `/mcp`, `/compact`, `/setup`, `/settings`, `/doctor`), and toggle color themes (`Dark`, `Light`, `System`).
  - Accessible via global hotkeys `Ctrl+P` / `Ctrl+Shift+P` and sidebar search button with full keyboard arrow navigation.

- **🌿 Conversation Forking & Branching from Any Turn**
  - Branch any active discussion from any assistant response into an independent session without modifying original conversation history.
  - Dedicated `Fork` action button on each turn generates an atomic cloned session in SQLite with all preceding context and prompt history intact.

- **🌲 Hierarchical Tool Execution Progress Tree, Multi-Toggle & Error Filtering**
  - Tucks multi-step agent tool executions into an expandable accordion tree (`ToolCallGroup.svelte`) displaying only the single active running task with a live spinner during execution.
  - Automatically collapses completed steps into a single compact header (`[▶] N steps completed • 2m 15s`) with a one-click toggleable tree view, keeping the assistant's final response clean and readable.
  - **Error Filter Pill**: Toggle between "All Steps" and "Errors Only" to instantly isolate failed executions and stack traces.
  - **Batch Multi-Toggle**: One-click "Expand All / Collapse All" to inspect parameters, raw stdout, and errors across all steps simultaneously.
  - Semantic human-friendly task titles (`resolveHumanFriendlyTitle`) extracted dynamically from task parameters (`Run cargo test`, `Read src/lib.rs`, `Search files`).
  - True wall-clock turn duration tracking ($\Delta t = t_{\text{assistant}} - t_{\text{user}}$) with live elapsed seconds/minutes during streaming and human-readable formatting (`formatDuration`: `0.4s`, `24s`, `2m 15s`, `1h 12m`).
  - Comprehensive turn metadata in the message footer (`3:45 PM • 2m 15s • ~1,450 tokens`).

- **📎 Drag-and-Drop, Screenshot Pasting & Context Attachments**
  - **Drag-and-Drop Drop Zone**: Drag and drop code files, documents, or screenshots directly onto the chat canvas with an interactive dashed drop zone overlay.
  - **Clipboard Screenshot Paste**: Paste screenshots directly (`Win+Shift+S` or `Ctrl+V`) into composer; saved automatically to `.gemini/attachments/` with thumbnail preview chips.
  - Inline `@` mention autocomplete popover with fuzzy search across workspace files and directories.
  - One-click Git context presets: `@git:diff` (working tree changes), `@git:staged` (index changes), and `@git:status`.
  - Removable badge chips row with individual and "Clear All" dismiss controls.

- **📊 Context Window & Cumulative Token Meter Gauge**
  - Live session token consumption meter in the chat header tracking active tokens against the 1M ceiling.
  - Visual color indicator and warning notification when approaching 750,000+ tokens with one-click recommendation to compact context.

- **💬 Prompt History Navigation & Sanitized Markdown**
  - `↑` and `↓` arrow navigation in `ChatPromptComposer.svelte` cycling through past sent prompts while preserving active in-progress drafts.
  - XSS-safe markdown rendering powered by `DOMPurify` with event-delegated code block copy buttons.

- **🎨 Multi-Theme System & Syntax Highlighting**
  - 4 tailored themes: **Platinum Dark** (default), **Gemini Aurora**, **Cyber Emerald**, and **Platinum Light** (`platinum-white`).
  - Full theme-adaptive `highlight.js` syntax highlighting palette tuned for readable light and dark contrast.
  - Modernized theme-adaptive tool execution authorization banners.

- **📁 Workspace & Directory Profiles**
  - Configure individual workspaces bound to local project directories (`GEMINI.md` context is automatically loaded).
  - Customize preferred models and custom system instructions per workspace profile.

- **💬 Multi-Session Chat & History**
  - Seamlessly switch between multiple conversation threads per workspace.
  - Rename, export (Markdown/JSON), and delete sessions with ease.

- **🔍 Full-Text Search (SQLite FTS5)**
  - Embedded local-first SQLite database.
  - Instant full-text search across previous conversations, code snippets, and responses.

- **📚 Reusable Prompt Templates**
  - Built-in prompt library for common developer workflows (Architecture Analysis, Test Generation, Code Refactoring, Documentation, Dead Code Detection).
  - Add your own custom prompt templates with category filtering.

- **🖥️ Integrated Workspace Terminal Drawer (`Ctrl+` `)**
  - Toggleable PowerShell terminal drawer docked directly inside the desktop window (`Ctrl+` ` or status toggle).
  - Automatically executes commands inside the active workspace root with exit codes, execution duration, and full stdout/stderr capture.
  - Interactive history recall (`↑`/`↓`), clear commands (`clear`/`cls`), and quick-action shortcuts (`git status`, `git diff --stat`, `dir`).
  - Height adjustment controls (standard drawer vs. expanded 65vh view).
  - 1-click **"Reload Gemini"** environment sync action (`restart_gemini_session`) to reload modified `.env` values into the AI session immediately.

- **🌱 Workspace `.env` & `.env.local` Auto-Injection**
  - Native parsing and automatic injection of `.env` and `.env.local` variables located in the active workspace root.
  - Automatically passes secrets, custom tokens, and configuration variables directly to both Gemini CLI ACP processes and terminal commands.

- **🔒 Custom App Dialogs & Accessible Tooltips**
  - Sleek, non-blocking theme-adaptive modal dialogs (`alert`, `confirm`, `prompt`) replacing webview-freezing browser popups.
  - Full keyboard accessibility, destructive action warnings, and clean input prompts.
  - Universal tooltip framework with rich contextual guidance across all icon actions, sidebar controls, and tool intercept triggers.

- **🛡️ Process Management & Safety**
  - Supervised with native **Windows Job Objects** (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`).
  - Zero-process-spawn environment diagnostics using native Win32 Registry APIs (`RegOpenKeyExW`) and in-memory executable path caching (`CLI_PATH_CACHE`), eliminating `cmd.exe`/`where.exe` child process forks when opening settings.
  - Windows `CREATE_NO_WINDOW` execution flags preventing console window flashing.
  - Guarantees child CLI processes are terminated cleanly on exit without leaving orphaned background tasks.
  - Fallback mock / offline mode when the CLI is in standby.

- **🔌 Model Context Protocol (MCP) & Plugin Tool Aggregation**
  - Connect AI agents to local and remote tools, databases, and services via Model Context Protocol (MCP 2.0).
  - Dynamic multi-layer server aggregation across enabled Antigravity plugins, global settings (`mcp_config.json`), and workspace profiles (`.agents/mcp_config.json`).
  - Automatic plugin argument path resolution and execution scoping, ensuring sidecar scripts and bundled MCP tools launch seamlessly in ACP mode.
  - Built-in 1-click presets for **AKBO Knowledge & Dreaming** (`akbo-cli`), Local Filesystem, GitHub Docker, Postgres, Memory, and Brave Search.
  - Dual guided-form and monospaced raw JSON editors with live status toggle switches (`disabled: true`).

- **🧩 Antigravity Plugins Catalog & AKBO Central Knowledge Pool**
  - Native GUI for discovering, installing, enabling, configuring, and scaffolding Antigravity CLI (`agy`) plugins.
  - Built-in 1-click installation for **AKBO Knowledge Oracle & Central Memory Pool** (`akbo`) bundling AST Codebase Knowledge Graph (CKG) tools, agent invariant rules, and automated post-turn memory consolidation ("dreaming").
  - **Central Knowledge Pool GUI Configuration**: Customize central memory repository paths (`~/.gemini/config/akbo_pool.json`) and toggle `--stage-only` proposal review mode directly in `PluginsModal.svelte`.
  - Clean lifecycle uninstallation and disk directory cleanup.

---

## ⌨️ Keyboard Shortcuts

| Shortcut | Action | Scope |
| :--- | :--- | :--- |
| **`Ctrl + P`** / **`Ctrl + Shift + P`** | Open Unified Command Palette | Global |
| **`Ctrl + ` `** / **`Ctrl + ~`** | Toggle Integrated Workspace Terminal Drawer | Global |
| **`Ctrl + M`** | Toggle MCP Server Manager | Global |
| **`Ctrl + K`** | Open Full-Text SQLite FTS5 Search | Global |
| **`Ctrl + N`** | Start New Chat Session | Global |
| **`↑` / `↓`** | Navigate Prompt History (on first/last line) | Prompt Textarea |
| **`/`** | Trigger Tools & Slash Commands Autocomplete Palette | Prompt Textarea |
| **`@`** | Trigger Context Mention Autocomplete (Files, Dirs, Git) | Prompt Textarea |
| **`Enter`** | Send Prompt / Execute Terminal Command | Prompt / Terminal |
| **`Shift + Enter`** | Insert Newline in Prompt | Prompt Textarea |
| **`Esc`** | Close Open Modals / Autocomplete Popover | Global |

## 📥 Installation & Downloads

Pre-built Windows binaries are automatically generated on every release:

1. Visit the [**Latest Releases**](https://github.com/sureshjsreddy/gemini-desktop/releases/latest) page.
2. Download either:
   - **`GeminiDesktop-Setup.exe`**: Standard Windows installer (NSIS).
   - **`Gemini Desktop_x64_en-US.msi`**: Windows MSI installer.
   - **`GeminiDesktop-Portable.exe`**: Portable standalone executable (no installation required).
3. Run the installer or portable binary.

### Prerequisites
- **Operating System**: Windows 10/11 (64-bit).
- **Gemini CLI**: Ensure `gemini` is installed and accessible in your system `PATH`.

---

## 🛠️ Tech Stack

| Layer | Technology |
| :--- | :--- |
| **Desktop Framework** | [Tauri v2](https://v2.tauri.app) |
| **Backend Language** | [Rust](https://www.rust-lang.org) (2021 Edition) |
| **Frontend Framework** | [SvelteKit 2](https://kit.svelte.dev) + [Svelte 5](https://svelte.dev) (Runes) |
| **Language** | [TypeScript](https://www.typescriptlang.org) |
| **Styling** | [Tailwind CSS v4](https://tailwindcss.com) + [Lucide Icons](https://lucide.dev) |
| **Markdown & Syntax** | `marked`, `highlight.js` |
| **Database** | Embedded SQLite with FTS5 via `rusqlite` |

---

## 💻 Local Development Setup

### 1. Prerequisites
- **Node.js**: `v20` or later
- **Rust**: Latest stable toolchain ([rustup.rs](https://rustup.rs))
- **Tauri CLI**: Installed via npm or cargo

### 2. Clone & Install
```bash
git clone https://github.com/sureshjsreddy/gemini-desktop.git
cd gemini-desktop

# Install frontend dependencies
npm install
```

### 3. Run Development Server
```bash
npm run tauri dev
```
This boots Vite with hot-module reloading on `localhost:1420` and spawns the native Tauri desktop window.

### 4. Code Quality & Verification
```bash
# Type check Svelte & TypeScript
npm run check

# Run Rust unit tests
npm --prefix src-tauri test
# or
cargo test --manifest-path src-tauri/Cargo.toml
```

### 5. Build for Production
```bash
npm run tauri build
```
Compiled binaries and installers will be output to `src-tauri/target/release/bundle/`.

---

## 🤖 CI / CD

Automated builds and releases are managed via **GitHub Actions** (`.github/workflows/release.yml`):
- Pushing any tag matching `v*` (e.g. `v0.1.0`) triggers a build on `windows-latest`.
- Compiles the SvelteKit frontend and Rust release binary with LTO optimizations.
- Automatically publishes the release and attaches `.exe` installers, `.msi` packages, and standalone portable executables (`GeminiDesktop-Portable.exe`) to GitHub Releases.
- Manual triggers are also supported via `workflow_dispatch` in the GitHub Actions tab.

---

## 📄 License

This project is licensed under the [MIT License](LICENSE).
