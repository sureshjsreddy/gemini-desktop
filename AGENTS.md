# 🤖 Gemini Desktop — AI Agent Workspace Instructions

> **ATTENTION ALL AI AGENTS (Antigravity, Cursor, Claude Code, Windsurf, Copilot, Gemini CLI)**  
> This workspace enforces an automated **AI-Native Knowledge Base System** located in [`knowledge/`](file:///c:/AIFiles/gemini-desktop/knowledge/).

---

## 1. Mandatory Pre-Execution Protocol (Strict Invariant)

> [!IMPORTANT]
> **MANDATORY FIRST TOOL ACTION**:
> On **ANY** task (including bug investigation, general questions, architecture review, refactoring, or writing code/tests), your **VERY FIRST tool call MUST be `view_file` on [`knowledge/index.yaml`](file:///c:/AIFiles/gemini-desktop/knowledge/index.yaml)**.
> 
> **DO NOT** perform speculative keyword `grep_search`, `find_by_name`, or open application code files until you have consulted `knowledge/index.yaml` and loaded the relevant Knowledge Units.

1. **Query Master Index First**: Call `view_file` on [`knowledge/index.yaml`](file:///c:/AIFiles/gemini-desktop/knowledge/index.yaml) to identify the Knowledge Units matching your role and domain.
2. **Retrieve Persona Knowledge Units**:
   - **Frontend & UI Tasks** (Svelte 5 runes, components, modals, Tailwind v4 styling): Inspect [`DEV-SVELTE-001`](file:///c:/AIFiles/gemini-desktop/knowledge/development/DEV-SVELTE-001.yaml), [`PROD-MODEL-001`](file:///c:/AIFiles/gemini-desktop/knowledge/product/PROD-MODEL-001.yaml), [`PROD-TOOL-001`](file:///c:/AIFiles/gemini-desktop/knowledge/product/PROD-TOOL-001.yaml), [`PROD-WS-001`](file:///c:/AIFiles/gemini-desktop/knowledge/product/PROD-WS-001.yaml), [`PROD-MCP-001`](file:///c:/AIFiles/gemini-desktop/knowledge/product/PROD-MCP-001.yaml), [`PROD-ATTACH-001`](file:///c:/AIFiles/gemini-desktop/knowledge/product/PROD-ATTACH-001.yaml), [`PROD-EXPLORER-001`](file:///c:/AIFiles/gemini-desktop/knowledge/product/PROD-EXPLORER-001.yaml), [`PROD-TERM-001`](file:///c:/AIFiles/gemini-desktop/knowledge/product/PROD-TERM-001.yaml), [`PROD-SETUP-001`](file:///c:/AIFiles/gemini-desktop/knowledge/product/PROD-SETUP-001.yaml), and [`DEV-TEST-001`](file:///c:/AIFiles/gemini-desktop/knowledge/development/DEV-TEST-001.yaml).
   - **Backend & System Tasks** (Tauri v2 commands, Rust IPC, process supervisor, SQLite FTS5): Inspect [`ARCH-SYS-001`](file:///c:/AIFiles/gemini-desktop/knowledge/architecture/ARCH-SYS-001.yaml), [`ARCH-ACP-001`](file:///c:/AIFiles/gemini-desktop/knowledge/architecture/ARCH-ACP-001.yaml), [`ARCH-PROC-001`](file:///c:/AIFiles/gemini-desktop/knowledge/architecture/ARCH-PROC-001.yaml), [`ARCH-DATA-001`](file:///c:/AIFiles/gemini-desktop/knowledge/architecture/ARCH-DATA-001.yaml), [`DEV-RUST-001`](file:///c:/AIFiles/gemini-desktop/knowledge/development/DEV-RUST-001.yaml), [`PROD-SETUP-001`](file:///c:/AIFiles/gemini-desktop/knowledge/product/PROD-SETUP-001.yaml), and [`DEV-TEST-001`](file:///c:/AIFiles/gemini-desktop/knowledge/development/DEV-TEST-001.yaml).
   - **Architecture & Packaging Decisions** (CI/CD, release automation, process lifecycle, persistence): Inspect [`ARCH-SYS-001`](file:///c:/AIFiles/gemini-desktop/knowledge/architecture/ARCH-SYS-001.yaml), [`ADR-001`](file:///c:/AIFiles/gemini-desktop/knowledge/decisions/ADR-001.yaml), [`ADR-002`](file:///c:/AIFiles/gemini-desktop/knowledge/decisions/ADR-002.yaml), [`ADR-003`](file:///c:/AIFiles/gemini-desktop/knowledge/decisions/ADR-003.yaml), [`ADR-004`](file:///c:/AIFiles/gemini-desktop/knowledge/decisions/ADR-004.yaml), and [`PROD-SETUP-001`](file:///c:/AIFiles/gemini-desktop/knowledge/product/PROD-SETUP-001.yaml).
3. **Enforce Directives**: Strictly follow all `rule`, `do`, and `dont` fields in loaded Knowledge Units before taking any action.

---

## 2. Core Project Invariants

- **Desktop Framework**: Built with **Tauri v2** (Rust) and **SvelteKit 2** (Svelte 5 runes + `@sveltejs/adapter-static`).
- **Process Supervision**: All spawned Gemini CLI and child processes MUST be governed by Windows Job Objects with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` to prevent zombie processes on application exit.
- **Protocol**: Gemini CLI interactions use the **Agent Client Protocol (ACP)** JSON-RPC over `stdin`/`stdout`.
- **Tool Intercept Safety**: Any tool execution requested by an agent must trigger an interactive confirmation UI for user authorization before execution.
- **Persistence**: All user sessions, messages, and prompt templates reside in local-first SQLite (`gemini_desktop.db`) using FTS5 virtual tables for millisecond text search.
- **Model Standard**: Model selection follows official Gemini CLI documentation: Auto (Gemini 3 / 2.5), Gemini 3 / 3.5 / 3.8 (`gemini-3.8-flash`, `gemini-3.5-flash`, `gemini-3.5-flash-lite`, `gemini-3.1-pro-preview`, `gemini-3.1-flash-lite`, `gemini-3-pro-preview`, `gemini-3-flash-preview`), Gemini 2.5 (`gemini-2.5-pro`, `gemini-2.5-flash`, `gemini-2.5-flash-lite`), Legacy 1.5, and Manual custom model input.
- **Verification Gate**: Before pushing commits or completing tasks, all changes must pass `npm run check` (0 errors, 0 warnings), `cargo test --lib` (all unit tests passing), and `npm run tauri build` (compiling the native Windows desktop binary and packaging `.msi`/`-setup.exe` installers).
- **Release Build Standard**: Always run `npm run tauri build` to compile and package the native Windows desktop application and installers (`gemini-desktop.exe`, `.msi`, and `-setup.exe`). Never stop at only `npm run build`, which only compiles frontend static assets into `build/` and does NOT produce desktop executables.
