# My Daily Standup — PR Sequence & Contribution Guide

> **For agentic workers:** This is the **master sequencing document**, not a task-level implementation plan. Before starting any PR below, write a detailed TDD implementation plan for *that PR alone* using `superpowers:writing-plans`, save it to `docs/superpowers/plans/<date>-pr-NN-<slug>.md`, then execute it with `superpowers:subagent-driven-development`. Checkboxes here track PR-level completion only.

**Repo:** https://github.com/Wasif0410/My-Daily-Standup
**Spec:** `docs/spec.md` (the Product and Technical Specification, v0.1)
**Goal:** Ship a local-first Windows desktop planning companion that reads an Obsidian vault, runs private voice standups against on-demand local models, and keeps commitments visible as lightweight sticky-note windows.

**Architecture:** Two hard-separated tiers. A **lightweight tier** (Tauri shell, sticky-note windows, system tray, SQLite) runs whenever the app is enabled and never loads a model. A **session tier** (llama.cpp, whisper.cpp, Sherpa-ONNX) spawns as sidecar processes only during an explicit AI session and is fully terminated afterward. Rust owns all state, file access, and process lifecycle; the LLM only produces text and structured proposals that Rust validates before anything is persisted.

**Tech Stack:** Tauri 2 · React 19 + TypeScript · Vite · Rust · rusqlite (bundled SQLite) · zustand · Vitest + Testing Library · llama.cpp · whisper.cpp · Sherpa-ONNX

---

## Global Constraints

Every PR's requirements implicitly include this section. A PR that violates any line here is rejected regardless of whether its tests pass.

- **Windows-first.** The MVP targets Windows 11. macOS/Linux are Wave 8 only. Never add a platform-specific dependency without a `cfg` guard.
- **Local-first.** All core functionality must work with no internet connection after models are downloaded. No feature may hard-depend on a network call.
- **No telemetry, no remote logging, no accounts.** Not behind a flag, not opt-out. Absent.
- **The LLM must not be loaded merely because sticky notes are visible.** Any board interaction (complete, uncomplete, edit, move, add, delete) that starts an inference process is a bug.
- **Idle shutdown default: 5 minutes.** Configurable to: immediate / 5 min / 15 min / manual. The default favors resource conservation.
- **No Obsidian write without explicit per-change approval.** The user must see the affected file, the proposed diff, and the reason, with approve / edit / reject.
- **Excluded vault folders are never indexed, searched, or sent to a model.** Enforced in the indexer, not the UI.
- **Sidecars bind to localhost only, on a randomly selected port, and reject remote connections.** They terminate when the parent session ends.
- **Raw audio is processed in memory and deleted after transcription** unless the user explicitly enables retention.
- **Never execute file operations from unvalidated model text.** All structured output passes a Rust-side schema validator first.
- **Obsidian is the long-term source of truth.** SQLite holds operational state only (current tasks, window geometry, settings, session metadata).

---

## Hardware Profiles

One installer, one binary. The profile is detected at runtime and overridable in Settings — **there is no separate "laptop build" and "GPU build."**

| Profile | Minimum machine | LLM (Q4_K_M) | Whisper | Context | Session footprint |
|---|---|---|---|---|---|
| **Lightweight** | 8 GB RAM, no dGPU | Qwen3-1.7B (~1.1 GB) | tiny | 2k | ~2 GB RAM |
| **Balanced** | 16 GB RAM, iGPU or entry dGPU | Qwen3-4B (~2.5 GB) | base | 4k | ~3.7 GB RAM |
| **High Quality** | **16 GB RAM + ≥8 GB VRAM** | Qwen3-8B (~5.0 GB) | small | 8k | ~6 GB VRAM + ~1 GB RAM |

**VRAM is the gate for High Quality, not system RAM.** When the model is fully offloaded, weights and KV cache live in VRAM; system RAM holds only the app plus memory-mapped GGUF pages, which are reclaimable page cache rather than committed memory. 16 GB of system RAM is sufficient.

**Always-on tier footprint (every profile):** Tauri shell + 5 board windows + SQLite ≈ **under 1 GB**, with zero model memory. This is the number that has to stay true for §26 to hold.

**Prefill dominates latency, so context size matters more than model size.** At ~250 tok/s CPU prefill, a 3k prompt costs ~12s and an 8k prompt costs ~32s. The tiered vault map (PR 24) is what keeps CPU-only machines usable, not just what improves answer quality.

**On an 8 GB VRAM card, run Whisper on CPU.** The LLM plus KV cache takes ~6 GB of the 8 GB; Whisper base transcribes 10s of audio on CPU in 1–2s anyway, and the headroom is better spent on context.

**Never trust WMI for VRAM.** `Win32_VideoController.AdapterRAM` is a 32-bit field and reports 4,095 MB for any card with 4 GB or more — verified on an RTX 3070 Ti that actually has 8,192 MB. Use DXGI's `DXGI_ADAPTER_DESC.DedicatedVideoMemory`, falling back to `nvidia-smi`.

**Backends are a build-time choice, not a runtime one.** llama.cpp compiles separately for CPU, CUDA, Vulkan, and ROCm. Bundle **CPU + Vulkan** (one GPU backend covering NVIDIA, AMD, and Intel); offer CUDA as an optional download for NVIDIA users. CUDA is ~10–30% faster but its runtime DLLs add hundreds of megabytes to the installer.

---

## Workflow

### Branching

`main` is always releasable. One branch per PR, cut fresh from the latest `main`:

```
<type>/pr-<NN>-<short-slug>
```

Examples: `chore/pr-01-repo-docs`, `feat/pr-11-priority-board`, `feat/pr-23-llama-lifecycle`.

Types: `feat`, `fix`, `chore`, `docs`, `refactor`, `test`, `ci`.

### Commits

[Conventional Commits](https://www.conventionalcommits.org/). Commit frequently inside a PR — every red→green→refactor cycle.

```
feat(boards): persist sticky window geometry on move
test(obsidian): add fixture for nested checkbox parsing
fix(inference): reap orphaned llama.cpp process on crash
```

### Merge strategy

**Squash merge only.** Each PR becomes exactly one commit on `main`, and its squash message is the PR title. Delete the branch on merge. This keeps `main` bisectable at PR granularity.

Configure once in **Settings → General → Pull Requests**: allow squash merging only; uncheck merge commits and rebase merging; check "Automatically delete head branches".

### Review gate

1. I open the PR with a filled-in description (see template below).
2. CI runs. If it's red, I push fixes before asking for anything.
3. I post a summary of what changed and what to look at.
4. **You approve or reject.** Nothing merges without your explicit approval.
5. You click squash-merge. I pull `main` and cut the next branch.

**Strictly sequential.** Each PR assumes every earlier PR is merged. No parallel branches — this plan's interfaces are chained, and concurrent work will conflict.

### Branch protection (set up after the bootstrap commit)

Settings → Branches → Add rule for `main`:
- Require a pull request before merging
- Require approvals: **0** — GitHub forbids a PR author from approving their own PR, and on a solo repo the author is always you, so requiring 1 would deadlock every merge. The review gate is manual, not enforced by GitHub.
- Require status checks to pass (add the CI jobs once PR 3 lands)
- Do not allow bypassing the above settings

### PR description template

Lands in `.github/pull_request_template.md` in PR 1:

```markdown
## What

<!-- One paragraph. What does this PR do? -->

## Why

<!-- Which spec section / plan PR does this implement? Link it. -->

## How to test

<!-- Exact commands, plus manual steps if UI is involved. -->

## Checklist

- [ ] Tests written before implementation (TDD)
- [ ] `npm test` and `cargo test` pass locally
- [ ] `npm run lint` and `cargo clippy -- -D warnings` clean
- [ ] No new network calls
- [ ] No inference process started by a non-session code path
- [ ] Docs updated if behavior changed
```

### Definition of Done (every PR)

A PR is not ready for your review until all of these hold:

- The deliverable works end-to-end and is demonstrable, not just unit-tested.
- Tests were written first and fail without the implementation.
- CI is green.
- No `TODO`, no commented-out code, no placeholder strings shipped to `main`.
- The PR touches only its stated scope. Unrelated cleanup goes in its own PR.
- If the PR changes user-visible behavior, `README.md` or `docs/` reflects it.

---

## Milestones

| Tag | After PR | What you can do |
|---|---|---|
| `v0.1.0` | 17 | Use it as a full non-AI desktop planner with persistent sticky boards |
| `v0.2.0` | 21 | Promote real tasks out of your Obsidian vault onto the boards |
| `v0.3.0` | 27 | Run a typed local-LLM standup that proposes and saves a daily plan |
| `v0.4.0` | 31 | Run the whole standup by voice — MVP feature-complete |
| `v1.0.0-rc` | 37 | Evening/weekly/monthly reviews with approved Obsidian writeback |

Each milestone is a usable product. You can stop at any tag and still have something worth running.

---

## PR 0 — Bootstrap (direct push to `main`, not a PR)

The repo is empty and has no default branch, so there is nothing to open a PR *against*. This one commit is pushed straight to `main`, then branch protection goes on and every later change is a PR.

**What this gives the app:** Creates the repository itself and establishes `main`, so every later change can arrive as a reviewable pull request.

```bash
git init
git branch -M main
git remote add origin https://github.com/Wasif0410/My-Daily-Standup.git
```

Contents: `README.md` (name, one-sentence pitch, "under construction"), `LICENSE` (MIT, © 2026 Wasif Saeed), `.gitignore` (Node, Rust, Tauri, OS junk, `/models`, `*.db`).

```bash
git add -A
git commit -m "chore: initial commit"
git push -u origin main
```

Then enable branch protection per the settings above.

> **Prerequisite before PR 2:** Rust is not installed on this machine. Install via https://rustup.rs plus the **Visual Studio Build Tools** with the "Desktop development with C++" workload — Tauri cannot build on Windows without the MSVC toolchain. Verify with `cargo --version` and `rustc --version`.

---

# Wave 0 — Foundation (PR 1–3)

### - [ ] PR 1 — Project docs & governance
**Branch:** `docs/pr-01-repo-docs`
**Depends on:** PR 0
**What this gives the app:** Writes down what is being built and the rules for building it, so the design lives in the repo rather than in one person's head.

Establishes the paperwork so every later PR has a home to update.

**Adds:** `docs/spec.md` (the full product spec), this file, `CONTRIBUTING.md` (branch naming, conventional commits, TDD expectation, local setup incl. Rust prerequisite), `SECURITY.md` (private disclosure via GitHub Security Advisories; explicit statement that the app makes no outbound connections), `CODE_OF_CONDUCT.md` (Contributor Covenant 2.1), `.github/pull_request_template.md`, `.github/ISSUE_TEMPLATE/{bug_report,feature_request}.yml`, expanded `README.md` with the two-tier architecture diagram and roadmap table.

**DoD:** Every doc renders correctly on GitHub. README links resolve. Repo description and topics set (`tauri`, `obsidian`, `local-llm`, `productivity`, `rust`).
**Test:** Manual — read every file on github.com.

---

### - [ ] PR 2 — Tauri 2 + React + TypeScript scaffold
**Branch:** `feat/pr-02-tauri-scaffold`
**Depends on:** PR 1
**What this gives the app:** Turns the project into something you can actually double-click and see. A window opens with the app's name in it.

The app compiles, launches, and shows a window. Nothing else.

**Creates:** `package.json`, `tsconfig.json`, `vite.config.ts`, `index.html`, `src/main.tsx`, `src/app/App.tsx`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, `src-tauri/src/main.rs`, `src-tauri/src/lib.rs`, `src-tauri/capabilities/default.json`, `src-tauri/build.rs`.

**Key decisions to encode:** app identifier `com.wasif.dailystandup`; product name `My Daily Standup`; window `width: 900, height: 700, visible: true, resizable: true`; capabilities file grants the **minimum** permission set — do not ship the default permissive template.

**Interfaces produced:** an invokable Rust command `greet(name: String) -> String` used purely to prove the IPC bridge works; removed in PR 6.

**DoD:** `npm run tauri dev` opens a window rendering "My Daily Standup". `npm run tauri build` produces an `.msi` in `src-tauri/target/release/bundle/`.
**Test:** `cargo test` (one canary), manual launch.

---

### - [ ] PR 3 — CI pipeline & test harness
**Branch:** `ci/pr-03-pipeline`
**Depends on:** PR 2
**What this gives the app:** Puts a robot in charge of checking every change. From here on, a mistake gets caught automatically before it can reach `main`.

Every later PR's green checkmark comes from here, so this lands before any real code.

**Creates:** `.github/workflows/ci.yml` with jobs — `lint` (eslint + prettier --check), `typecheck` (`tsc --noEmit`), `test-web` (vitest run), `rust` (`cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`), `build` (`tauri build` on `windows-latest`, artifact uploaded). Also `.eslintrc.cjs`, `.prettierrc`, `vitest.config.ts`, `src/test/setup.ts`, `rustfmt.toml`, `clippy.toml`, and a Dependabot config for `cargo`, `npm`, and `github-actions`.

**Caching:** `Swatinem/rust-cache` for the Rust jobs, `actions/setup-node` cache for npm. Without this the build job takes 15+ minutes per push.

**DoD:** All five jobs green on the PR. Add them as required status checks in branch protection after merge.
**Test:** CI itself is the test. Include one deliberately trivial `vitest` and one `#[test]` so the runners have something to execute.

---

# Wave 1 — Data layer (PR 4–7)

### - [ ] PR 4 — SQLite setup & migration runner
**Branch:** `feat/pr-04-sqlite-migrations`
**Depends on:** PR 3
**What this gives the app:** Gives the app a memory. Somewhere to keep your tasks that survives closing the app, restarting the machine, or crashing.

**Creates:** `src-tauri/src/storage/mod.rs`, `storage/db.rs`, `storage/migrations.rs`, `src-tauri/migrations/001_initial.sql`.

Uses `rusqlite` with the `bundled` feature so no system SQLite is required. Migrations are versioned SQL files applied in order, tracked with `PRAGMA user_version` — no migration framework dependency.

DB lives at the Tauri app data dir (`app_handle.path().app_data_dir()`), file `standup.db`. Enable `PRAGMA foreign_keys = ON` and `journal_mode = WAL` on every connection.

`001_initial.sql` creates the `tasks` table mirroring the spec's `Task` model: `id TEXT PRIMARY KEY`, `title`, `description`, `horizon`, `status`, `parent_task_id`, `source_type`, `source_file`, `source_line`, `area`, `project`, `priority`, `scheduled_date`, `period_start`, `period_end`, `due_date`, `completed_at`, `progress_current`, `progress_target`, `progress_unit`, `blocker`, `notes`, `rollover_count NOT NULL DEFAULT 0`, `created_at`, `updated_at`. Indexes on `(horizon, scheduled_date)`, `(status)`, `(parent_task_id)`.

**Interfaces produced:** `Db::open(path: &Path) -> Result<Db>`, `Db::open_in_memory() -> Result<Db>` (tests), `run_migrations(conn: &Connection) -> Result<()>`.

**DoD:** Fresh DB migrates to latest. Re-running migrations is a no-op. In-memory DB works for tests.
**Test:** Rust tests using `Db::open_in_memory()` — migration applies, is idempotent, `user_version` advances, foreign keys enforced.

---

### - [ ] PR 5 — Task repository
**Branch:** `feat/pr-05-task-repository`
**Depends on:** PR 4
**What this gives the app:** Teaches the app to actually use that memory — add a task, read it back, change it, delete it.

Pure Rust data access, no Tauri coupling, so it is fully unit-testable.

**Creates:** `src-tauri/src/storage/task.rs` (the `Task` struct + `TaskHorizon`/`TaskStatus` enums with serde), `storage/task_repo.rs`.

**Interfaces produced:**
```rust
fn create(&self, input: NewTask) -> Result<Task>
fn get(&self, id: &str) -> Result<Option<Task>>
fn update(&self, id: &str, patch: TaskPatch) -> Result<Task>
fn delete(&self, id: &str) -> Result<()>
fn list_by_horizon(&self, h: TaskHorizon) -> Result<Vec<Task>>
fn list_for_date(&self, date: NaiveDate) -> Result<Vec<Task>>
fn list_for_period(&self, start: NaiveDate, end: NaiveDate) -> Result<Vec<Task>>
fn children_of(&self, parent_id: &str) -> Result<Vec<Task>>
```
IDs are UUIDv4 strings, generated in Rust — never supplied by the frontend or a model. `updated_at` is set by the repository on every write, never by the caller.

**DoD:** Full CRUD round-trips. Deleting a parent nulls children's `parent_task_id` rather than cascading (a child task outliving its parent is valid).
**Test:** Rust unit tests per method against in-memory DB, including the parent-delete behavior and that `created_at` is immutable across updates.

---

### - [ ] PR 6 — Tauri command layer & shared types
**Branch:** `feat/pr-06-command-layer`
**Depends on:** PR 5
**What this gives the app:** Connects the Rust engine to the screen. Until now the two halves could not talk about real data; after this they can.

Bridges Rust to the frontend with types that cannot silently drift apart.

**Creates:** `src-tauri/src/commands/mod.rs`, `commands/tasks.rs`, `src/types/task.ts`, `src/lib/ipc.ts`.
**Modifies:** `src-tauri/src/lib.rs` (register handlers), `capabilities/default.json`. **Removes** the `greet` canary from PR 2.

Commands: `task_create`, `task_get`, `task_update`, `task_delete`, `task_list_by_horizon`, `task_list_for_date`, `task_list_for_period`.

`src/lib/ipc.ts` wraps `invoke` in typed functions so no component ever calls `invoke` with a raw string. Errors come back as a tagged `{ kind, message }` shape, not stringified panics.

**Interfaces produced:** `createTask(input: NewTask): Promise<Task>` and siblings, plus the `Task` TS interface mirroring the spec's model exactly (camelCase in TS, snake_case in Rust, bridged via `#[serde(rename_all = "camelCase")]`).

**DoD:** A temporary dev button creates and lists a task through the real IPC path.
**Test:** Rust tests on command functions; Vitest tests on `ipc.ts` with `invoke` mocked, asserting argument shapes.

---

### - [ ] PR 7 — Progress & rollover engine
**Branch:** `feat/pr-07-progress-engine`
**Depends on:** PR 6
**What this gives the app:** Teaches the app arithmetic: how far along a goal is, and how many times you have pushed a task to tomorrow.

The spec's calculation rules, isolated in one pure module so a model never does arithmetic.

**Creates:** `src-tauri/src/domain/progress.rs`, `domain/rollover.rs`.

Rules to implement exactly:
- Progress types: subtask-completion, numeric target, percentage, binary, manual.
- Completing a child updates the parent's computed progress but **never** auto-completes the parent unless its completion rule is satisfied.
- Rescheduling a task increments `rollover_count` by exactly 1. Editing the title, adding a blocker, or completing does not.
- Weekly completion rate = completed ÷ planned for the period, excluding cancelled tasks.

**Interfaces produced:**
```rust
fn compute_progress(task: &Task, children: &[Task]) -> Progress
fn reschedule(repo: &TaskRepo, id: &str, to: NaiveDate) -> Result<Task>
fn period_stats(tasks: &[Task]) -> PeriodStats  // planned, completed, rate, carried, cancelled
```

**DoD:** Every rule above has a test that fails if the rule is inverted.
**Test:** Table-driven Rust unit tests. No DB needed for `compute_progress`/`period_stats` — they take slices.

---

# Wave 2 — Sticky-note boards (PR 8–14)

### - [ ] PR 8 — Frontend store & data hooks
**Branch:** `feat/pr-08-store`
**Depends on:** PR 7
**What this gives the app:** Makes the interface feel instant. Ticking a checkbox updates immediately instead of waiting on the database.

**Creates:** `src/stores/taskStore.ts` (zustand), `src/features/boards/hooks/useTasks.ts`, `src/features/boards/hooks/useBoardActions.ts`.

Optimistic updates: mutate local state immediately, call IPC, roll back and surface a toast on failure. Board interaction must feel instant.

**Interfaces produced:** `useTasks(filter)`, `useBoardActions()` returning `{ complete, uncomplete, editTitle, moveToDate, promote, remove }`.

**DoD:** Store round-trips through mocked IPC; rollback path proven.
**Test:** Vitest — optimistic apply, success commit, failure rollback.

---

### - [ ] PR 9 — Sticky window manager
**Branch:** `feat/pr-09-window-manager`
**Depends on:** PR 8
**What this gives the app:** Real sticky notes appear on your desktop as separate windows, and they remember where you left them.

The core of the lightweight tier: real frameless desktop windows that survive the main window closing.

**Creates:** `src-tauri/src/windows/mod.rs`, `windows/board_windows.rs`, `src-tauri/migrations/002_board_windows.sql`, `src/features/boards/BoardRoot.tsx`.

Each board is a separate `WebviewWindow` created with `decorations: false`, `transparent: true`, `skip_taskbar: true`, routed by a URL query param (`?board=priority`). Geometry (x, y, width, height, monitor, visible, collapsed, opacity, always_on_top, locked) persists to a `board_windows` table on move/resize, debounced 500ms, and is restored on startup.

**Interfaces produced:** `open_board(kind: BoardKind)`, `close_board(kind)`, `restore_boards()`, `save_board_geometry(kind, geom)`.

**DoD:** Open a board, drag it, resize it, quit the app, relaunch → the board reappears in the same place on the same monitor. Closing the main window leaves boards running.
**Test:** Rust tests on the geometry persistence layer; manual verification for the window behavior itself.

---

### - [ ] PR 10 — Board shell component & theme
**Branch:** `feat/pr-10-board-shell`
**Depends on:** PR 9
**What this gives the app:** Gives the boards their look — dark, minimal, controls that stay hidden until you hover.

The shared visual chassis every board renders inside.

**Creates:** `src/components/BoardShell.tsx`, `src/components/BoardHeader.tsx`, `src/styles/theme.css`, `src/styles/tokens.css`.

Per the spec's visual direction: dark charcoal background, light text, thin colored top border (accent per board), soft rounded corners, compact typography, section dividers. **Controls are hidden until pointer hover.** Custom drag region via `data-tauri-drag-region` on the header. Collapse-to-titlebar toggle.

CSS custom properties for accent, opacity, and font size so PR 15 can drive them from settings without touching components.

**DoD:** Storybook-free visual check — a board renders with hover-revealed controls and collapses to its title bar.
**Test:** Vitest + Testing Library — controls hidden by default, appear on hover, collapse toggles content visibility.

---

### - [ ] PR 11 — Priority Tasks board
**Branch:** `feat/pr-11-priority-board`
**Depends on:** PR 10
**What this gives the app:** Your first working board: the important things that span more than a single day.

First real board. Long-lived important items grouped by area.

**Creates:** `src/features/boards/PriorityBoard.tsx`, `src/features/boards/components/TaskRow.tsx`, `components/AreaGroup.tsx`.

Renders tasks where `horizon != 'daily'` and `priority >= threshold`, grouped by `area`. `TaskRow` is the shared row primitive used by every later board — inline edit on double-click, checkbox, hover actions.

**Interfaces produced:** `<TaskRow task onComplete onEdit onMove onDelete />`.

**DoD:** Board shows real tasks from SQLite. Completing one persists and survives restart. **Confirm no inference process exists** — Task Manager shows no sidecar.
**Test:** Vitest — grouping, empty state, inline edit commit/cancel, checkbox fires the right action.

---

### - [ ] PR 12 — Weekly Tasks board & task interactions
**Branch:** `feat/pr-12-weekly-board`
**Depends on:** PR 11
**What this gives the app:** The week's commitments on screen, plus every way you would want to change a task — complete it, edit it, move it, block it.

**Creates:** `src/features/boards/WeeklyBoard.tsx`, `src/features/boards/components/QuickAdd.tsx`, `src/features/boards/components/TaskContextMenu.tsx`.

Implements the spec's full interaction set: complete, uncomplete, edit text, move to another day, promote daily→weekly, move weekly→another week, add blocker, add comment, delete/archive. Right-click context menu plus keyboard shortcuts.

Grouped by project, current period only (`period_start`/`period_end` covering today).

**DoD:** Every interaction in §6.6 works and persists. Moving a task to another day increments `rollover_count` exactly once (verify in DB).
**Test:** Vitest for the menu and each action's dispatch; Rust test asserting the rollover increment via the real reschedule path.

---

### - [ ] PR 13 — Weekly Progress board
**Branch:** `feat/pr-13-weekly-progress-board`
**Depends on:** PR 12
**What this gives the app:** A visible record of your week. Finished work stays on screen, dimmed rather than deleted, so you can see what the week actually held.

**Creates:** `src/features/boards/WeeklyProgressBoard.tsx`, `components/DaySection.tsx`.

Day-by-day view of the current week. Completed items stay visible but dimmed and struck through — the board is a record of the week, not a disappearing list. Today's section is highlighted. Week start day comes from settings (default Monday).

**DoD:** Matches the spec's example layout. Completing a task on the Weekly Tasks board updates this board live (shared store, no refetch).
**Test:** Vitest — correct day bucketing across a week boundary, completed styling applied, week-start setting respected.

---

### - [ ] PR 14 — Monthly Progress board
**Branch:** `feat/pr-14-monthly-board`
**Depends on:** PR 13
**What this gives the app:** The monthly view: progress bars showing how far along each commitment is, rather than a list of every task.

**Creates:** `src/features/boards/MonthlyBoard.tsx`, `components/ProgressBar.tsx`.

Outcome-focused, not task-focused. Renders each monthly commitment with its numeric progress (`12 / 20 applications`) and a bar. Bar uses block characters or a styled div — must stay legible at low opacity and small font size.

Progress values come from PR 7's `compute_progress`, never computed in the component.

**DoD:** Monthly commitments show correct roll-up from completed weekly/daily children.
**Test:** Vitest on the bar's percentage rendering incl. 0%, 100%, and over-target; integration test that a completed daily task moves the monthly number.

---

# Wave 3 — Desktop shell (PR 15–17)

### - [ ] PR 15 — Window behaviors
**Branch:** `feat/pr-15-window-behaviors`
**Depends on:** PR 14
**What this gives the app:** Control over how the notes behave — always on top, see-through, locked in place, pinned to one monitor.

**Creates:** `src-tauri/src/windows/behaviors.rs`, `src/features/boards/components/BoardMenu.tsx`.

Always-on-top, desktop-level mode, lock position, click-through when locked (`set_ignore_cursor_events`), adjustable opacity, adjustable font size, pin to a specific monitor, independent per-board visibility, light/dark theme, compact/expanded mode. All persisted to `board_windows`.

**Careful:** click-through must only activate while locked, and there must be an escape hatch (tray menu → Unlock All Boards) or the user can permanently lose access to a board.

**DoD:** Every behavior in §6.7 toggles, persists, and restores after reboot. The unlock escape hatch works.
**Test:** Rust tests on persistence; manual matrix for the OS-level behaviors.

---

### - [ ] PR 16 — System tray & quick add
**Branch:** `feat/pr-16-system-tray`
**Depends on:** PR 15
**What this gives the app:** A tray icon, so the app is one click away without a window taking up space.

**Creates:** `src-tauri/src/tray.rs`, `src/features/quick-add/QuickAddWindow.tsx`.

Tray menu exactly as §6.8: Open Boards, Start Daily Standup, Start Evening Check-In, Plan My Week, Monthly Review, Quick Add Task, Pause Reminders, Settings, Quit.

AI entries are present but **disabled with a "coming soon" tooltip** until Wave 5 — they must not silently do nothing. Quick Add opens a small always-on-top input window; adding a task **must not** start any model.

Adds `tauri-plugin-autostart` for launch-at-login.

**DoD:** Tray survives main window close. Quick Add creates a task and the boards update. Quit terminates everything cleanly.
**Test:** Rust test on menu construction; manual for tray interaction.

---

### - [ ] PR 17 — Settings & reminders → **tag `v0.1.0`**
**Branch:** `feat/pr-17-settings-reminders`
**Depends on:** PR 16
**What this gives the app:** Settings you can change and reminders that nudge you morning and evening. **This is the first version genuinely worth using every day.**

**Creates:** `src-tauri/src/storage/settings.rs`, `migrations/003_settings.sql`, `src/features/settings/SettingsWindow.tsx`, `settings/sections/{General,StickyNotes,Planning}.tsx`, `src-tauri/src/reminders.rs`.

Settings sections per §18 — only General, Sticky Notes, and Planning here; Obsidian/AI/Voice sections are added by the PRs that introduce those subsystems.

Reminders: morning and evening notification at configurable times via `tauri-plugin-notification`, with Pause Reminders honored. Clicking a reminder opens the app — it does **not** auto-start a model.

**DoD:** Settings persist across restart. A reminder fires at the configured time. **This is a shippable non-AI desktop planner.**
**Test:** Rust tests on settings get/set with defaults and on reminder scheduling arithmetic; Vitest on the settings forms.

**After merge:** `git tag v0.1.0 && git push --tags`, and cut a GitHub release with the built `.msi`.

---

# Wave 4 — Obsidian read integration (PR 18–21)

### - [ ] PR 18 — Vault selection & folder scoping
**Branch:** `feat/pr-18-vault-config`
**Depends on:** PR 17
**What this gives the app:** Lets the app point at your Obsidian vault — and lets you decide which folders it must never look at.

**Creates:** `src-tauri/src/obsidian/mod.rs`, `obsidian/config.rs`, `migrations/004_vault.sql`, `src/features/settings/sections/Obsidian.tsx`.

Folder picker for the vault root. Include/exclude lists with the spec's suggested defaults pre-filled as *suggestions the user confirms*: `Private/`, `Journal/`, `Medical/`, `Financial/`. Read-only mode toggle, defaulting to **on**. Configurable target folders for daily/weekly/monthly notes.

Exclusion is enforced by a single `is_indexable(path) -> bool` function that every later Obsidian code path must call. Centralizing it is the whole point — a second exclusion check somewhere else is how a private note eventually leaks into a prompt.

**Interfaces produced:** `VaultConfig { root, include, exclude, read_only, daily_folder, weekly_folder, monthly_folder }`, `is_indexable(&VaultConfig, &Path) -> bool`.

**DoD:** Selecting a vault persists it. Excluded paths return false, including nested children and case variations.
**Test:** Rust tests over a temp fixture vault — exclusion of nested paths, glob edge cases, symlink refusal.

---

### - [ ] PR 19 — Markdown parser
**Branch:** `feat/pr-19-markdown-parser`
**Depends on:** PR 18
**What this gives the app:** Teaches the app to read your notes: the checkboxes, the tags, the priorities, the links between them.

Pure parsing library, zero I/O, so it can be tested exhaustively against fixtures.

**Creates:** `src-tauri/src/obsidian/parser.rs`, `obsidian/frontmatter.rs`, `tests/fixtures/vault/**` (realistic sample notes).

Parses per §9.1: YAML frontmatter (`serde_yaml`), note title, headings, markdown checkboxes with line numbers, wikilinks, `parent` relationships, tags, `status`, `priority`, due dates, `last_updated`, callouts.

Also extracts a **`summary_line`** — the one-line description that feeds the vault map in PR 24. Resolution order, first hit wins: the `desired_outcome` frontmatter field (§9.6) → the first non-empty prose line after the H1 → the title alone. Truncate to 100 characters at a word boundary. Fully deterministic; no model involved.

**Interfaces produced:**
```rust
fn parse_note(content: &str, path: &Path) -> Result<ParsedNote>
struct ParsedNote { title, summary_line: String, frontmatter, headings, tasks: Vec<ParsedTask>, wikilinks, tags }
struct ParsedTask { text, checked, line: usize, heading_path: Vec<String> }
```

**Careful:** malformed YAML must degrade to "no frontmatter", never error the whole scan. One broken note cannot break indexing.

**DoD:** Parses the spec's example note correctly, including `parent: "[[Projects]]"` and `- [ ] Schedule a dental cleaning...` at the right line number.
**Test:** Fixture-driven Rust tests — nested checkboxes, indented tasks, `- [x]`/`- [X]`/`- [-]`, CRLF line endings, notes with no frontmatter, malformed YAML, unicode.

---

### - [ ] PR 20 — Vault indexer & file watcher
**Branch:** `feat/pr-20-vault-indexer`
**Depends on:** PR 19
**What this gives the app:** Builds a fast index of your vault and keeps it current as you edit in Obsidian, so nothing has to be re-read from scratch.

**Creates:** `src-tauri/src/obsidian/indexer.rs`, `obsidian/watcher.rs`, `migrations/005_vault_index.sql`.

Walks the vault (honoring `is_indexable`), parses each note, writes a lightweight index across three tables:

- `notes` — path, title, **`summary_line`**, frontmatter fields (status, priority, parent, due), mtime, hash
- `note_tasks` — note_path, line, text, checked
- **`note_links`** — an edge list (`from_path`, `to_path`, `kind`) built from wikilinks and `parent:` frontmatter

The edge table is what makes the vault a real graph rather than a flat list. It costs almost nothing to populate and buys three things: rendering the hierarchy in PR 24's map, **neighbor expansion** (pulling in a note's parent summary alongside the note itself), and §5.5's "identify neglected areas" as a graph query. Ancestor and descendant traversal is a recursive CTE — roughly eight lines of SQL.

Incremental — reindex a file only when mtime or hash changed.

File watcher via `notify`, debounced 1s, triggering targeted reindex. Full scan runs off the UI thread with progress events.

**Interfaces produced:** `index_vault(&VaultConfig) -> Result<IndexStats>`, `reindex_file(path)`, `start_watcher(app)`, `stop_watcher()`.

**DoD:** Indexing a real vault completes and reports counts. Editing a note in Obsidian updates the index within ~2s. Excluded folders produce zero rows — verify by querying the DB directly. A recursive CTE returns the correct ancestor chain for a nested note.
**Test:** Rust tests over a temp vault — initial index, incremental no-op, file added/modified/deleted, ancestor/descendant traversal, a link to a non-existent note (must not error), a link cycle (must terminate), and an explicit test asserting excluded files never appear in *any* of the three tables.

---

### - [ ] PR 21 — Relevance ranking & promote-to-board → **tag `v0.2.0`**
**Branch:** `feat/pr-21-ranking-promote`
**Depends on:** PR 20
**What this gives the app:** The app can now tell you which vault tasks matter most today, and you can pull one onto a board with a link back to the note it came from.

**Creates:** `src-tauri/src/obsidian/ranking.rs`, `src/features/vault/VaultBrowser.tsx`, `src/features/vault/components/SourceBadge.tsx`.

Deterministic scoring per §9.3 — project priority + due-date urgency + active-status weight + weekly/monthly connection + rollover count + recent mentions. Weights are named constants in one place, documented, and tunable. **No embeddings.**

UI: browse ranked candidate tasks, promote one onto a board. Promotion copies text and sets `source_type: 'obsidian'`, `source_file`, `source_line`. Every promoted task shows a source badge; clicking it opens the note via the `obsidian://open?path=` URI.

**DoD:** Ranking is stable and explainable — each candidate shows its score breakdown, matching §9.4's transparency example. Promoted tasks keep working source links.
**Test:** Rust table-driven tests on scoring (a priority-9 overdue task outranks a priority-3 one; ordering is deterministic for equal scores); Vitest on the badge and promote flow.

**After merge:** tag `v0.2.0`.

---

# Wave 5 — Local LLM text standup (PR 22–27)

### - [ ] PR 22 — Model registry, hardware detection & download
**Branch:** `feat/pr-22-model-registry`
**Depends on:** PR 21
**What this gives the app:** The app works out what your computer can handle and downloads a language model that fits it.

**Creates:** `src-tauri/src/inference/mod.rs`, `inference/registry.rs`, `inference/hardware.rs`, `inference/benchmark.rs`, `inference/download.rs`, `models/catalog.json`, `src/features/settings/sections/AI.tsx`.

Detects CPU cores, total RAM, and GPU/VRAM, then recommends a profile from the Hardware Profiles table above.

**VRAM detection must use DXGI's `DXGI_ADAPTER_DESC.DedicatedVideoMemory`, with `nvidia-smi` as a fallback. Do not use WMI** — `Win32_VideoController.AdapterRAM` is a 32-bit field and reports 4,095 MB for an 8 GB RTX 3070 Ti. Write the regression test against that exact case.

Catalog lists GGUF models with size, license, min RAM, and min VRAM — a Qwen3 model is the suggested default (Apache 2.0), but the app must support **any** configured GGUF, never hard-code one.

**Backend acquisition:** CPU and Vulkan llama.cpp backends ship with the installer; CUDA is an optional post-install download offered only when an NVIDIA GPU is detected. Backend binaries follow the same checksum-verified download path as models.

**Empirical profile selection:** after detection, run a **20-token benchmark** against the recommended model and pick the final profile from measured tokens/sec rather than from spec-sheet heuristics. Heuristics mispredict constantly across laptop thermal profiles and driver versions; a five-second measurement does not. Cache the result so it runs once, not per session.

Downloads with progress, resume, and **SHA-256 verification before the file is accepted**. Models land in the app data dir, never in the repo. `.gitignore` already excludes `/models`.

**Interfaces produced:** `detect_hardware() -> HardwareProfile`, `benchmark(model, backend) -> TokensPerSecond`, `recommend_models(&HardwareProfile) -> Vec<ModelSpec>`, `download_model(spec, progress_cb) -> Result<PathBuf>`, `download_backend(kind) -> Result<PathBuf>`, `installed_models() -> Vec<InstalledModel>`.

**DoD:** Hardware detected correctly on the dev machine (Ryzen 5 9600X / 32 GB / RTX 3070 Ti 8 GB → High Quality). A model downloads, verifies, and a corrupted download is rejected and deleted. The benchmark produces a stable tokens/sec figure across runs.
**Test:** Rust tests with a local HTTP fixture server — checksum mismatch rejected, interrupted download resumes, disk-full path handled. A VRAM-detection test asserting an 8 GB card reports 8 GB and not 4,095 MB. A profile-selection test asserting a machine with 16 GB RAM and 8 GB VRAM qualifies for High Quality.

---

### - [ ] PR 23 — llama.cpp sidecar lifecycle
**Branch:** `feat/pr-23-llama-lifecycle`
**Depends on:** PR 22
**What this gives the app:** The AI starts only when you ask for it and shuts down completely when you are done. **This is the promise the entire product rests on.**

**The single most important PR in the project.** §26 says the boards are always available but inference is always on demand — this is where that becomes true or doesn't.

**Creates:** `src-tauri/src/inference/llama.rs`, `inference/process.rs`, `inference/health.rs`, `src-tauri/binaries/README.md` (how to vendor the sidecar binary).

Spawns `llama-server` as a Tauri sidecar on a **random free localhost port**, bound to `127.0.0.1` only, with `--host 127.0.0.1`. Waits for a health check before reporting ready. Terminates on session end, on idle timeout (default 5 min, configurable per §7.4), and on app quit. Detects and reaps orphaned processes from a previous crash by recording the PID in SQLite at spawn.

**Backend selection and GPU offload:** picks the backend binary chosen in PR 22 and computes `--n-gpu-layers` from detected VRAM minus a safety margin, rather than hard-coding a layer count. If the GPU-offloaded spawn fails — driver mismatch, VRAM exhausted, another process holding memory — **fall back to the CPU backend automatically and tell the user what happened.** A failed offload must degrade to slow, never to broken.

**Interfaces produced:**
```rust
async fn start(model: &InstalledModel, cfg: &InferenceConfig) -> Result<LlamaSession>
async fn LlamaSession::complete(&self, prompt: &str) -> Result<String>
async fn LlamaSession::stream(&self, prompt: &str) -> impl Stream<Item = Result<String>>
async fn LlamaSession::shutdown(self) -> Result<()>
fn reap_orphans() -> Result<usize>
```

Also covers §17.1: if the model fails to start, show a clear error with diagnostic detail, offer a retry with a smaller model, leave all existing tasks intact, and keep manual sticky-note functionality fully working.

**DoD:** Start a session → `llama-server` visible in Task Manager with memory allocated. Shut down → **process gone and both RAM and VRAM released, verified in Task Manager and `nvidia-smi`.** Kill the app mid-session → the orphan is reaped on next launch. Port is never a fixed number. A deliberately corrupted model path produces the error UI, not a hang or a crash. Forcing an impossible `--n-gpu-layers` falls back to CPU with a visible message.
**Test:** Rust integration tests — spawn/shutdown round-trip, idle timeout fires, shutdown is idempotent, orphan reaping, GPU-offload failure falls back to CPU. Gate the tests that need a real model behind an env var so CI stays fast.

---

### - [ ] PR 24 — Vault map & tiered context builder
**Branch:** `feat/pr-24-vault-map-context`
**Depends on:** PR 23
**What this gives the app:** Gives the AI a map of your goals instead of your whole vault, so it knows where to look without having to read everything.

Per §9.2, **never send the whole vault** — a 500-note vault is ~260k tokens, which is roughly 33 GB of KV cache and ~9 minutes of prefill. Instead the model gets a small **map** of what exists plus a small set of ranked excerpts, and can request more.

**Creates:** `src-tauri/src/inference/map.rs`, `inference/context.rs`, `inference/prompt.rs`, `prompts/daily-standup.md`, `prompts/evening-review.md`, `prompts/weekly-planning.md`, `prompts/monthly-review.md`.

**Tier 1 — the vault map (~1,500 token budget, always present).** Active goals and projects only, rendered from `notes` + `note_links` as an indented tree with priority, status, and `summary_line`. Roughly 12–15 tokens per line, so ~20 goals and ~90 child notes fit the budget. Capped by count *and* tokens; overflow drops the lowest-ranked goals first.

```
VAULT MAP — active projects

Job Search [p9, ongoing] — Find a senior dev role by December
  → Fall 2026.md ......... application tracker, 12/20 submitted
  → Resume.md ............ master resume + per-company variants

Health [p8, ongoing] — Clear the backlog of overdue appointments
  → Dental.md ............ routine cleaning, not scheduled
```

**Tier 2 — ranked excerpts (~2,000 token budget).** Top-N candidates from PR 21's scorer with their surrounding note context, plus current weekly/monthly commitments, yesterday's incomplete tasks, upcoming due dates, and repeatedly-deferred tasks per §11.2.

**Tier 3 — on demand.** Fetch a single note by path for PR 27's expansion loop.

The division of labor matters: **the map tells the model what exists; the ranker tells it what is urgent.** Dropping the ranker would force the model to infer priority from the map, which is exactly the judgment §3.6 says must stay deterministic.

Prompts live in editable markdown files with a `{{variable}}` substitution layer, so users can customize them later without a rebuild.

**Interfaces produced:**
```rust
fn build_map(cfg: &VaultConfig, budget: usize) -> Result<VaultMap>
fn build_context(session_kind, budget: TokenBudget) -> Result<SessionContext>
fn fetch_note(path: &str) -> Result<NoteExcerpt>   // Tier 3, used by PR 27
fn render_prompt(template, &SessionContext) -> String
```

**Careful:** the map is built from indexed notes only, so excluded folders are absent by construction — the model never learns they exist, rather than being filtered after the fact. Assert this in code; it is the last checkpoint before text reaches a model.

**DoD:** The rendered map for a real vault fits its budget and reads like the example above. Total prompt stays under the profile's context size. Nothing from an excluded folder appears anywhere.
**Test:** Rust tests — map respects both count and token caps; overflow drops lowest-ranked goals first; a note with no `summary_line` degrades to title-only; a vault with a link cycle renders without hanging; a decoy test filling an excluded folder with distinctive content asserts none of it reaches the rendered prompt.

---

### - [ ] PR 25 — Standup session state machine & chat UI
**Branch:** `feat/pr-25-standup-session`
**Depends on:** PR 24
**What this gives the app:** An actual standup conversation you can type. The app asks the questions, in order, and keeps control of the conversation.

**Creates:** `src-tauri/src/session/mod.rs`, `session/state_machine.rs`, `src/features/standup/StandupWindow.tsx`, `standup/components/{MessageList,Composer,StageIndicator,ContextPanel}.tsx`.

Stages are driven by **Rust**, not the model: Context → Previous progress → Current priorities → Blockers → Capacity → Proposed commitments → Approval → Save & close. The model generates the language for each stage; it cannot skip, reorder, or invent stages.

UI shows streaming responses, a stage indicator, the retrieved Obsidian context with sources, and the §7.5 loading sequence ("Starting local assistant… Loading language model… Ready."). Typed input only in this PR — voice arrives in Wave 6.

**Interfaces produced:** `start_session(kind) -> SessionId`, `send_message(session, text)`, `advance_stage(session)`, `end_session(session)` (which shuts down inference).

**DoD:** A full typed standup runs end to end and the model process terminates when the window closes.
**Test:** Rust tests on stage transitions incl. illegal transitions rejected; Vitest on the chat UI with a mocked stream.

---

### - [ ] PR 26 — Structured output, validation & approval
**Branch:** `feat/pr-26-structured-approval`
**Depends on:** PR 25
**What this gives the app:** Everything the AI suggests gets checked by real code and shown to you for approval before a single task is saved.

**Creates:** `src-tauri/src/session/proposal.rs`, `session/validator.rs`, `src/features/standup/components/ApprovalPanel.tsx`.

The model returns the §11.5 JSON shape. Rust deserializes it with strict serde types and validates: titles non-empty and under a length cap, `horizon` a known variant, `parentTaskId` referencing an existing task, `sourceFile` inside the vault and not excluded, no duplicate proposals. Invalid output triggers **one** repair round-trip with the validation errors appended to the prompt; a second failure surfaces an error rather than guessing.

Approval UI: every proposed task can be individually approved, edited, or rejected. Nothing is written to SQLite until approval. Approved tasks become real tasks and the boards refresh.

**DoD:** A standup produces a plan you approve, and those tasks appear on the boards. **Deliberately malformed model JSON never reaches the database.**
**Test:** Rust tests feeding adversarial payloads — missing fields, wrong types, `sourceFile` pointing at `Private/`, path traversal (`../../etc/passwd`), a parent ID that doesn't exist, 10,000-character titles. Each must be rejected.

---

### - [ ] PR 27 — Bounded context expansion → **tag `v0.3.0`**
**Branch:** `feat/pr-27-context-expansion`
**Depends on:** PR 26
**What this gives the app:** Lets the AI ask to see a specific note, or ask you a question, when what it has is not enough to answer well.

Lets the model say "I need to look at that note" or "I don't know where you track this" — **without tool-calling.** Small quantized models are unreliable at tool-use protocols, and every tool call is another 5–10s round trip.

**Creates:** `src-tauri/src/session/expansion.rs`. **Modifies:** `session/proposal.rs`, `session/validator.rs`, `src/features/standup/components/MessageList.tsx`.

Extends PR 26's validated envelope with two optional fields:

```json
{
  "needs_context": ["Health/Dental.md"],
  "question_for_user": "I don't see a note for travel — where do you track that?"
}
```

Rust validates each requested path (exists, inside the vault, passes `is_indexable`), fetches it via PR 24's `fetch_note`, appends it, and re-prompts. **Capped at 2 expansion rounds** so a request loop cannot spiral into a minute of latency. Neighbor expansion comes free from `note_links` — fetching `Dental.md` also pulls its parent `Health.md` summary.

`question_for_user` costs nothing extra: it just renders in the chat and waits for a reply.

This works with a weak model because it is only JSON output — not a protocol the model has to execute correctly.

**Interfaces produced:** `expand_context(session, requests: Vec<String>) -> Result<SessionContext>`, `ExpansionBudget { max_rounds: 2, max_notes_per_round: 3 }`.

**DoD:** Asking about something outside the retrieved set causes the model to request the right note and answer correctly on the second pass. Expansion never exceeds 2 rounds. A request for an excluded path is refused and the refusal is invisible to the model — it is not told the file exists.
**Test:** Rust tests — a request for `Private/Secrets.md` is refused; path traversal refused; a nonexistent path refused without erroring the session; the round cap holds when the model requests context every turn; neighbor expansion pulls the parent summary.

**After merge:** tag `v0.3.0`.

---

# Wave 6 — Voice (PR 28–31)

### - [ ] PR 28 — Audio capture, push-to-talk & VAD
**Branch:** `feat/pr-28-audio-capture`
**Depends on:** PR 27
**What this gives the app:** Your microphone, on a push-to-talk key, with nothing written to disk.

**Creates:** `src-tauri/src/audio/mod.rs`, `audio/capture.rs`, `audio/vad.rs`, `src/features/settings/sections/Voice.tsx`.

`cpal` for microphone capture, device enumeration, and level metering. Push-to-talk via `tauri-plugin-global-shortcut`. Simple energy-based VAD to trim silence.

**Audio stays in memory as `Vec<f32>` and is zeroed after transcription.** No temp files unless the user explicitly enables retention. The mic is only opened during an active session — never in the background (§16.1).

**Interfaces produced:** `list_input_devices()`, `start_capture(device) -> CaptureHandle`, `CaptureHandle::stop() -> AudioBuffer`, `trim_silence(&AudioBuffer) -> AudioBuffer`.

**DoD:** Hold the shortcut, speak, release → a buffer of the right duration and sample rate. Mic indicator is off outside a session.
**Test:** Rust tests on VAD with synthetic signals (silence, speech-like noise, clipping) and on buffer zeroing after use.

---

### - [ ] PR 29 — whisper.cpp transcription
**Branch:** `feat/pr-29-whisper`
**Depends on:** PR 28
**What this gives the app:** What you say becomes text, transcribed on your own machine and nowhere else.

**Creates:** `src-tauri/src/audio/whisper.rs`, extends `inference/registry.rs` with Whisper models.

Runs whisper.cpp on the captured buffer, resampling to 16kHz mono. Whisper model selection (tiny/base/small) follows the hardware profile from PR 22. Loaded on session start, unloaded on session end alongside the LLM.

**On cards with ≤8 GB VRAM, run Whisper on CPU.** The LLM plus KV cache already takes ~6 GB of 8 GB at the High Quality profile, and Whisper base transcribes 10s of audio on CPU in 1–2s. Make this the automatic default when detected VRAM headroom is under 2 GB.

**Fallback is mandatory:** if transcription fails or no mic exists, the composer stays fully usable for typing. §17.2 — voice enhances the product, it must never be required.

**Interfaces produced:** `transcribe(&AudioBuffer, &WhisperModel) -> Result<String>`.

**DoD:** Speaking a sentence produces accurate text in the composer. Unplugging the mic mid-session degrades to typing with a clear message, not a crash.
**Test:** Rust test transcribing a short committed WAV fixture and asserting keyword presence; failure-path test asserting the typed fallback stays enabled.

---

### - [ ] PR 30 — Sherpa-ONNX text-to-speech
**Branch:** `feat/pr-30-tts`
**Depends on:** PR 29
**What this gives the app:** The assistant talks back out loud, with a voice generated locally.

**Creates:** `src-tauri/src/audio/tts.rs`, `audio/playback.rs`, `audio/segmentation.rs`.

Sentence segmentation so speech starts before the full response finishes generating. Playback via `rodio`. Voice selection, speed control, auto-play toggle, and a **Stop Speaking** button (§12.4 — full interruption is deliberately out of scope for the MVP).

Per §12.3 the assistant summarizes rather than reading long task lists aloud — enforce this in the prompt, and cap spoken length.

**Interfaces produced:** `synthesize(text, voice) -> AudioBuffer`, `speak(stream)`, `stop_speaking()`.

**DoD:** The assistant's response is spoken locally with no network access. Stop Speaking halts playback immediately. TTS unloads with the session.
**Test:** Rust tests on sentence segmentation (abbreviations, decimals, ellipses shouldn't split wrongly) and on the stop signal cancelling the queue.

---

### - [ ] PR 31 — Onboarding wizard → **tag `v0.4.0` (MVP)**
**Branch:** `feat/pr-31-onboarding`
**Depends on:** PR 30
**What this gives the app:** A first-run walkthrough that takes a new user from install to their first standup without ever opening Settings.

**Creates:** `src/features/onboarding/OnboardingWizard.tsx` and one step component per §5.1 stage.

All twelve steps: explain local processing → pick vault → read-only first → scan → show discovered structure → exclude folders → detect hardware → recommend models → download after confirmation → test mic and voice → place boards → choose daily-only or all horizons.

The hardware step surfaces PR 22's benchmark result plainly — measured tokens/sec, the selected profile, and the expected turn latency — so the §7.5 startup tradeoff is set as an expectation before first use rather than discovered as a surprise. Offer the CUDA backend download here when an NVIDIA GPU is present.

**DoD:** A fresh install walks a new user from zero to a working first standup without touching Settings. Re-runnable from Settings.
**Test:** Vitest on step navigation, back/forward state retention, and the skip paths.

**After merge:** tag `v0.4.0` and cut a release. **This is the MVP** — check it against every line of §23's acceptance criteria before tagging.

---

# Wave 7 — Reviews & Obsidian writeback (PR 32–37)

### - [ ] PR 32 — Evening check-in
**Branch:** `feat/pr-32-evening-checkin`
**Depends on:** PR 31
**What this gives the app:** An end-of-day check-in that asks what happened, and what to do with whatever did not.

Shorter session per §5.3. Distinguishes the five outcomes the spec names: still important / blocked externally / too large / no longer wanted / recurring avoidance. Reschedule, backlog, delegate, or drop each unfinished task. Language stays non-judgmental (§10.3).

**Creates:** `src-tauri/src/session/evening.rs`, `src/features/evening/EveningWindow.tsx`.
**DoD:** Unfinished tasks are triaged and the outcome persists with correct rollover accounting.
**Test:** Rust tests — each outcome produces the right status and rollover delta.

---

### - [ ] PR 33 — Weekly planning & retrospective
**Branch:** `feat/pr-33-weekly-review`
**Depends on:** PR 32
**What this gives the app:** A weekly review built on real numbers — planned versus completed — and a realistic plan for the week ahead.

Per §5.4, using PR 7's `period_stats` for all numbers — **the model never calculates completion rates.**

**Creates:** `src-tauri/src/session/weekly.rs`, `src/features/weekly-review/WeeklyReviewWindow.tsx`.
**DoD:** A weekly session reviews real stats, sets milestones, and refreshes the weekly boards.
**Test:** Rust tests on stat computation across a week boundary; snapshot test on the generated review structure.

---

### - [ ] PR 34 — Monthly planning & retrospective
**Branch:** `feat/pr-34-monthly-review`
**Depends on:** PR 33
**What this gives the app:** A monthly review that compares what you actually did against the long-term goals in your vault.

Per §5.5 — compares completed work to long-term goals, surfaces neglected areas, sets a limited number of measurable monthly commitments, seeds initial weekly milestones.

**Creates:** `src-tauri/src/session/monthly.rs`, `src/features/monthly-review/MonthlyReviewWindow.tsx`.
**DoD:** A monthly session produces commitments with measurable targets that the Monthly board renders.
**Test:** Rust tests on neglected-area detection and commitment→milestone seeding.

---

### - [ ] PR 35 — Obsidian writer (append-only review notes)
**Branch:** `feat/pr-35-obsidian-writer`
**Depends on:** PR 34
**What this gives the app:** Your standups and reviews get written back into Obsidian — but only after you approve the exact change.

**Creates:** `src-tauri/src/obsidian/writer.rs`, `obsidian/diff.rs`, `src/features/approval/DiffApproval.tsx`.

Writes daily/weekly/monthly notes into the **dedicated folders only** (§9.5), in the exact formats from §13.1–13.3. Every write shows a diff preview with the file, the change, and the reason before it happens. Original project notes are untouched by this PR.

**Interfaces produced:** `propose_write(target, content) -> WriteProposal`, `apply_write(proposal) -> Result<()>` — and `apply_write` must be unreachable without an approved proposal.

**DoD:** A completed standup writes a correctly formatted daily note after you approve the diff. Rejecting writes nothing. Read-only mode blocks it entirely.
**Test:** Rust tests over a temp vault — write lands in the right folder with the right frontmatter; rejection is a no-op; a proposal targeting a path outside the approved folders is refused; read-only mode refuses all writes.

---

### - [ ] PR 36 — Conflict detection, backups & source-task updates
**Branch:** `feat/pr-36-write-conflicts`
**Depends on:** PR 35
**What this gives the app:** Protects your notes. Detects if you edited a file first, backs up before touching anything, and refuses to overwrite newer work.

**Creates:** `src-tauri/src/obsidian/conflict.rs`, `obsidian/backup.rs`.

Per §17.3: record mtime and hash when a proposal is created; re-check immediately before writing. If the file changed, show a conflict and preserve both versions rather than overwriting. Back up any existing file before modifying it.

Adds the one case where original notes may change: ticking a source checkbox (`- [ ]` → `- [x]`) at a recorded line, gated behind the "may update original tasks" setting, defaulting to **off**, and verified by matching the line's text before editing — never by line number alone, since the file may have shifted.

**DoD:** Editing a note in Obsidian between proposal and approval produces a conflict warning, not a lost edit. Checkbox writeback ticks the right line and refuses when the line text no longer matches.
**Test:** Rust tests — mtime change detected, hash change detected, backup created before modify, checkbox writeback with a shifted line refuses rather than corrupting.

---

### - [ ] PR 37 — Contextual AI helpers → **tag `v1.0.0-rc`**
**Branch:** `feat/pr-37-ai-helpers`
**Depends on:** PR 36
**What this gives the app:** Small AI helpers on a single task — break this down, help me unblock this — without starting a whole session.

The remaining §14 actions that may start AI after confirmation, invoked from a task's context menu rather than from a full session: **Break This Task Down**, **Help Me Resolve This Blocker**, **Summarize Progress**, **Suggest Priorities**.

**Creates:** `src-tauri/src/session/helpers.rs`, `src/features/boards/components/AiHelperMenu.tsx`, `prompts/helpers/*.md`.

Each is a single-shot request reusing PR 23's lifecycle and PR 26's validation and approval path — no new inference machinery. Every one shows a confirmation first ("This will start the local model, ~8s") because a board interaction must never silently spawn a process.

**DoD:** Each helper produces an approvable proposal and shuts the model down afterward per the idle-timeout setting. Cancelling at the confirmation dialog starts nothing.
**Test:** Rust tests that each helper routes through the same validator as PR 26; Vitest asserting the confirmation dialog gates the spawn call.

**After merge:** tag `v1.0.0-rc` and validate every §23 acceptance criterion.

---

# Wave 8 — Cross-platform & community (post-1.0, not yet planned)

Deliberately unplanned until v1.0-rc ships and real usage reveals what actually matters. Expected content per §22 Phase 6: macOS support, Linux support, signed installers, automated releases, contributor documentation, a model adapter interface, theme and prompt customization, and an accessibility review.

Write this wave's PR sequence after the RC, not before.

---

## Risks to watch while executing

| Risk | Where it bites | Guard |
|---|---|---|
| Sidecar binaries bloat the repo | PR 23, 29, 30 | Never commit binaries. Vendor at build time via a script; document in `binaries/README.md`. |
| Windows Smart App Control blocks unsigned sidecars | PR 23, 29, 30 | **Confirmed real on 2026-08-20:** Smart App Control blocked `rustdoc.exe`, `rustfmt.exe`, and cargo build scripts on the dev machine, failing release builds outright. It judges on *reputation*, not signatures, so freshly-built zero-reputation binaries are exactly what it rejects — the same profile as a bundled `llama.cpp`, `whisper.cpp`, or Sherpa-ONNX sidecar. It ships enabled by default on many Windows 11 installs and has **no allowlist**; disabling it is irreversible without a system reset, so "turn off your security feature" is not an acceptable install step. Treat code-signing the sidecars as a shipping requirement, not a nice-to-have, and detect-and-explain the failure rather than letting a session hang. |
| Model process leaks memory between sessions | PR 23 | Task Manager **and `nvidia-smi`** check is part of PR 23's DoD, repeated at every later voice/session PR. |
| Excluded folders leak into a prompt | PR 18, 20, 24, 27 | Four independent guards, including PR 24's decoy-content test and PR 27's refusal of excluded paths in `needs_context`. |
| VRAM misdetected, wrong profile chosen | PR 22 | Use DXGI/`nvidia-smi`, never WMI `AdapterRAM`. Regression test pinned to the 8 GB-reports-as-4,095 MB case, plus an empirical benchmark that overrides the heuristic. |
| Context expansion spirals into latency | PR 27 | Hard cap of 2 rounds and 3 notes per round, enforced in Rust, not requested of the model. |
| Vault map grows past its budget on a large vault | PR 24 | Capped by token count *and* node count; overflow drops lowest-ranked goals first, with a test. |
| Window management fights the OS | PR 9, 15 | Keep behaviors in `behaviors.rs` behind a trait so platform quirks stay isolated in Wave 8. |
| Scope creep inside a PR | Everywhere | The DoD line "touches only its stated scope." Spin extras into new issues. |
| CI build times balloon | PR 3 onward | Rust cache from day one; gate model-dependent tests behind an env var. |

---

## Executing this plan

Before each PR: I write a task-level TDD plan for that PR, you glance at it, then I implement, open the PR, and hand it to you for approval. After merge I pull `main` and cut the next branch. One PR in flight at a time.
