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

- **⚡ Native Agent Client Protocol (ACP) & Resilient Engine Runtime**
  - Full bidirectional JSON-RPC streaming directly over the child CLI process `stdin`/`stdout`.
  - Resilient JSON-RPC ID parsing and `stopReason: "end_turn"` completion handling across Gemini CLI and Antigravity CLI (`agy`), eliminating hanging "working" states.
  - Non-blocking real-time model steering mid-flight with mode-aware steering prompts.
  - Interactive **Tool Execution Permission Intercepts** — review and approve tool/command execution before changes occur on your machine.

- **🌲 Hierarchical Tool Execution Progress Tree & Wall-Clock Telemetry**
  - Tucks multi-step agent tool executions into an expandable accordion tree (`ToolCallGroup.svelte`) displaying only the single active running task with a live spinner during execution.
  - Automatically collapses completed steps into a single compact header (`[▶] N steps completed • 2m 15s`) with a one-click toggleable tree view, keeping the assistant's final response clean and readable.
  - Semantic human-friendly task titles (`resolveHumanFriendlyTitle`) extracted dynamically from task parameters (`Run cargo test`, `Read src/lib.rs`, `Search files`).
  - True wall-clock turn duration tracking ($\Delta t = t_{\text{assistant}} - t_{\text{user}}$) with live elapsed seconds/minutes during streaming and human-readable formatting (`formatDuration`: `0.4s`, `24s`, `2m 15s`, `1h 12m`).
  - Comprehensive turn metadata in the message footer (`3:45 PM • 2m 15s • ~1,450 tokens`).

- **📐 Fluid Responsive Chat Canvas & Dynamic Visible Area Sizing**
  - Conversation viewport dynamically adapts its width based on visible screen estate and panel states.
  - Smoothly expands up to `1280px` (`max-w-7xl`) when collateral navigation drawers (Left Sidebar and Right Solution Explorer) are collapsed or on high-resolution/ultrawide displays, giving complex code snippets, diffs, tables, and tool logs maximum breathing room.
  - Automatically contracts to an ergonomic reading width (`896px` / `1024px`) when sidebars are opened, maintaining optimal typography line lengths.
  - Harmoniously synchronizes width across Assistant responses, User prompts, Plan Mode Workbench, ACP Permission Cards, and the bottom Prompt Composer.

- **💬 Streamlined Typing Experience & Interactive Starter Cards**
  - Decluttered prompt composer toolbar with dedicated context attachment (`@`), working directory tracking, and clean submit controls.
  - Interactive onboarding empty-state cards for one-click repository architecture exploration, uncommitted git diff review, and step-by-step feature planning.
  - Integrated "Compress Context" action inside the Export & Actions menu to keep the top header focused on conversation context.

- **🎯 Official Gemini Model Selection**
  - **Auto (Gemini 3)**: Dynamically balances between Pro and Flash models based on task complexity.
  - **Auto (Gemini 2.5)**: Dynamic selection across Gemini 2.5 series.
  - **Gemini 3 / 3.5 / 3.8 Series**: `gemini-3.8-flash` (latest preview — ultra-responsive next-gen Flash), `gemini-3.5-flash` (powerful default), `gemini-3.5-flash-lite` (lightweight), `gemini-3.1-pro-preview`, `gemini-3.1-flash-lite`, `gemini-3-pro-preview`, `gemini-3-flash-preview`.
  - **Gemini 2.5 Series**: `gemini-2.5-pro`, `gemini-2.5-flash`, and `gemini-2.5-flash-lite`.
  - **Manual / Custom Entry**: Enter any custom model identifier, fine-tuned endpoint, or preview string.

- **🧩 Antigravity CLI (`agy`) Plugins & Extensions Hub**
  - Native desktop manager for Antigravity agent plugins and extension bundles.
  - **Installed View**: Live management of active plugins with capability badges (skills, MCP tools, and rules), 1-click enable/disable switches, and Explorer folder access.
  - **Explore Catalog**: Curated directory of official and community extensions (`android-cli-plugin`, `chrome-devtools-plugin`, `firebase`, `science`, `google-antigravity-sdk`) plus custom ID/GitHub target installation.
  - **Custom Plugin Studio**: Scaffolding wizard to generate new custom plugins in `%USERPROFILE%\.gemini\config\plugins\<name>\` complete with skills, MCP configs, and rules.
  - **Dual-Engine Graceful Guidance**: Informative banner when `gemini` CLI is active explaining plugin requirements without unwanted prompts.

- **📐 Ergonomic Pinned Chat Sidebar Architecture**
  - Re-architected left sidebar layout prioritizing conversation history: assigns primary vertical height (`flex-1 min-h-0`) to **Recent Chats**, keeping 10–15+ chats instantly accessible.
  - Clean top action bar for `+ New Chat` and `Search History` (`Ctrl+K`).
  - Integrated bottom toolbar featuring side-by-side Developer Panes (`Terminal` `Ctrl+\`` and `Workspace Explorer` `Ctrl+Alt+L`) alongside a unified **Tools & Settings** launcher.

- **🧰 Dynamic `/` Tools & Slash Commands Autocomplete**
  - Instant tool discovery palette triggered by typing `/` in the prompt composer.
  - Dynamically filters available tools and actions based on the active CLI engine:
    - **Antigravity CLI (`agy`)**: Access Antigravity plugins and extensions (`/plugins`).
    - **Gemini CLI (`gemini`)**: Access prompt context compression (`/compress`).
    - **Unified Desktop Actions**: Instant 1-click execution for Plan Workbench (`/plan`), Session Diff (`/diff`), Terminal Drawer (`/terminal`), MCP Manager (`/mcp`), Debug Doctor (`/doctor`), Context Compaction (`/compact`), Capabilities (`/tools`), Model Parameters (`/model`), Settings (`/settings`), Setup (`/setup`), Rewind (`/rewind`), and Clear (`/clear`).
  - Full keyboard navigation (`↑`/`↓` selection, `Enter`/`Tab` execution, `Esc` dismiss).

- **🔌 Model Context Protocol (MCP) Server Integration (`Ctrl+M`)**
  - Native dual-scope configuration management: **Workspace-scoped** (`<workspace>/.gemini/settings.json`) or **Global** (`~/.gemini/settings.json`).
  - 1-click popular presets: 🐙 **GitHub (Docker)**, 📁 **Local Filesystem**, 🐘 **PostgreSQL**, 🔍 **Brave Web Search**, and 🧠 **Knowledge Graph Memory**.
  - Dual editing experience: guided visual form or monospaced raw JSON editor.
  - Safe non-destructive persistence: strictly preserves all existing non-MCP keys and settings.

- **📎 File, Folder & Git Context Attachments**
  - Inline `@` mention autocomplete popover with fuzzy search across workspace files and directories.
  - One-click Git context presets: `@git:diff` (working tree changes), `@git:staged` (index changes), and `@git:status`.
  - Removable badge chips row with individual and "Clear All" dismiss controls.
  - Interactive file & folder search picker modal.

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

---

## ⌨️ Keyboard Shortcuts

| Shortcut | Action | Scope |
| :--- | :--- | :--- |
| **`Ctrl + ` `** / **`Ctrl + ~`** | Toggle Integrated Workspace Terminal Drawer | Global |
| **`Ctrl + M`** | Toggle MCP Server Manager | Global |
| **`Ctrl + K`** | Open Full-Text SQLite FTS5 Search | Global |
| **`Ctrl + N`** | Start New Chat Session | Global |
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
