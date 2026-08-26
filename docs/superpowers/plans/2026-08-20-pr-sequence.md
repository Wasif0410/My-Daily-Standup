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

- **Windows-first.** The MVP targets Windows 11. macOS/Linux are Wave 9 only. Never add a platform-specific dependency without a `cfg` guard.
- **Local-first.** All core functionality must work with no internet connection after models are downloaded. No feature may hard-depend on a network call.
- **No telemetry, no remote logging, no accounts.** Not behind a flag, not opt-out. Absent.
- **The LLM must not be loaded merely because sticky notes are visible.** Any board interaction (complete, uncomplete, edit, move, add, delete, expand a day, record how long something took) that starts an inference process is a bug.
- **Idle shutdown default: 5 minutes.** Configurable to: immediate / 5 min / 15 min / manual. The default favors resource conservation.
- **No Obsidian write without explicit per-change approval.** The user must see the affected file, the proposed diff, and the reason, with approve / edit / reject.
- **Excluded vault folders are never indexed, searched, or sent to a model.** Enforced in the indexer, not the UI.
- **Sidecars bind to localhost only, on a randomly selected port, and reject remote connections.** They terminate when the parent session ends.
- **Raw audio is processed in memory and deleted after transcription** unless the user explicitly enables retention.
- **Never execute file operations from unvalidated model text.** All structured output passes a Rust-side schema validator first.
- **Obsidian is the long-term source of truth once it is connected.** From Wave 7 onward SQLite holds operational state only (current tasks, window geometry, settings, session metadata). Before Wave 7 there is no vault, so SQLite is the only source there is — which is why the standup in Wave 4 is built to read it directly rather than to wait for a vault.

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

**Prefill dominates latency, so context size matters more than model size.** At ~250 tok/s CPU prefill, a 3k prompt costs ~12s and an 8k prompt costs ~32s. The tiered context map (PR 25) is what keeps CPU-only machines usable, not just what improves answer quality.

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

Examples: `chore/pr-01-repo-docs`, `feat/pr-12-priority-board`, `feat/pr-24-llama-lifecycle`.

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
| `v0.1.0` | 18 | A full non-AI desktop planner: persistent sticky boards, an expandable Monday-to-Sunday week, priorities, named sections, and per-task time tracking |
| `v0.2.0` | 28 | Run a typed local-LLM standup that reads your own boards and proposes a daily plan you approve |
| `v0.3.0` | 32 | Run the whole standup by voice — MVP feature-complete |
| `v0.4.0` | 35 | Close the loop with evening, weekly, and monthly reviews built on real numbers |
| `v1.0.0-rc` | 38 | Point the app at your Obsidian vault: promote real vault tasks onto the boards, and write reviews back after approving the diff |

Each milestone is a usable product. You can stop at any tag and still have something worth running.

---

## Execution order

PR numbers are **identities, not positions.** A PR keeps the number it was given the day it was written, because that number is cited in commit messages, branch names, other documents, and half the cross-references in this file — renumbering to restore a tidy sequence would silently invalidate all of them. The waves are what got resequenced when Obsidian moved to the end, so the waves are what carry the ordering. Read this table, not the PR numbers, to know what comes next.

| Wave | Name | PRs | Status |
|---|---|---|---|
| 0 | Foundation | 1–3 | Done |
| 1 | Data layer | 4–7 | Done |
| 2 | Sticky-note boards | 8–15, 15a–15c | Done |
| 3 | Desktop shell | 16, 17, 17a, 18 | 18 is next |
| 4 | Local LLM text standup | 23–28 | |
| 5 | Voice | 29–32 | |
| 6 | Reviews | 33–35 | |
| 7 | Obsidian read integration | 19–22 | |
| 8 | Obsidian writeback | 36–38 | |
| 9 | Cross-platform & community | post-1.0 | Not yet planned |

**Why Obsidian moved to the end.** Every AI capability now ships before any vault code does. The standup was originally sequenced behind the vault because the vault was assumed to be where the model's context came from. It isn't any more — Waves 2 and 3 grew commitments, priorities, sections, rollover counts, and logged durations, which is a richer and more structured picture of the user's week than a folder of markdown ever was. Waiting on the vault would have meant holding the whole AI product hostage to an integration that turns out to be an enrichment rather than a foundation.

The lettered PRs (15a–15c, 17a) are work that shipped on `main` without having been predicted here. They are recorded in their wave for the sake of an honest history; they take letters rather than numbers so that no existing identity shifts.

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

# Wave 2 — Sticky-note boards (PR 8–15, 15a–15c)

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

### - [ ] PR 11 — Recording how long tasks took
**Branch:** `feat/pr-11-time-logging`
**Depends on:** PR 10
**What this gives the app:** A place to record how long each task actually took, so the week can be totalled at the end of it.

**Creates:** `src-tauri/migrations/003_time_spent.sql`, `src-tauri/src/domain/time_spent.rs`. **Modifies:** `storage/task.rs`, `storage/task_repo.rs`, `commands/mod.rs`, `src/types/task.ts`, `src/stores/taskStore.ts`.

**A logged duration, not a stopwatch.** The user types or picks how long something took, usually when marking it done. Nothing runs in the background.

That is a deliberate choice rather than a simplification. A stopwatch requires remembering to start it, remembering to stop it, and working in uninterrupted blocks. What it actually produces is forgotten starts and timers left running overnight, and every total built on that becomes untrustworthy. A duration entered from memory is approximate, but the user knows it is approximate.

**One column, not a table.** A task belongs to a single day, so summing a period means summing the tasks scheduled inside it. There is no work spanning a week boundary that needs splitting, and therefore no reason for per-session rows.

```sql
ALTER TABLE tasks ADD COLUMN time_spent_minutes INTEGER
    CHECK (time_spent_minutes IS NULL OR time_spent_minutes >= 0);
```

**Rules to implement exactly:**

- **Duration is optional and never blocks completion.** A task with no recorded time contributes nothing to the total; it does not count as zero and it does not nag.
- **Recording is one gesture.** Presets — 15m, 30m, 1h, 2h — plus a free-text field. If logging takes more than a few seconds it stops happening, and the recap becomes worthless.
- **Editable at any time**, not only at completion.
- **Period totals sum `time_spent_minutes` across tasks scheduled in the period**, grouped by area and project for the weekly recap.
- **Implausible values are questioned, not rejected.** Eighteen hours on one task is more likely a typo than a marathon, but the user may be right — surface it in the evening check-in rather than refusing the input.

**Interfaces produced:**
```rust
fn set_time_spent(repo, conn, id: &str, minutes: Option<i64>) -> Result<Task>
fn minutes_in_period(tasks: &[Task]) -> i64
fn minutes_by_area(tasks: &[Task]) -> Vec<(String, i64)>
fn minutes_by_project(tasks: &[Task]) -> Vec<(String, i64)>
fn implausible_durations(tasks: &[Task], threshold_minutes: i64) -> Vec<&Task>
```

Extends PR 7's `PeriodStats` with `minutes_tracked`, so the weekly review draws every number from one place.

**DoD:** A duration can be set, changed, and cleared, and survives a restart. Completing a task without recording one works and leaves the total unaffected. A negative value is rejected by the schema.
**Test:** Rust — set, change, and clear round-trip; negative rejected; period totals ignore tasks with no duration rather than treating them as zero; by-area grouping handles tasks with no area; an implausible value is flagged but still stored. Frontend — presets dispatch the right value, free-text parses "1h 30m" and "90".

---

### - [x] PR 12 — Priority Tasks board & the shared task row
**Branch:** `feat/pr-12-priority-board`
**Depends on:** PR 11
**What this gives the app:** Your first working board, and the task row every other board reuses — priority, a checkbox, and how long the task took.

First real board. Long-lived important items grouped by area.

**Creates:** `src/features/boards/PriorityBoard.tsx`, `src/features/boards/components/TaskRow.tsx`, `components/PriorityBadge.tsx`, `components/DurationField.tsx`, `components/AreaGroup.tsx`.

Renders tasks where `horizon != 'daily'` and `priority >= threshold`, grouped by `area`.

**`TaskRow` is the primitive every later board builds on**, so its anatomy is settled here rather than four times over:

```text
P8  ☑  Complete onboarding task           35m
P5  ☐  Schedule dental appointment           —
```

| Element | Behaviour |
|---|---|
| **Priority badge** | The 0–10 value, so importance is visible without reading titles. Colour-graded, but never colour *alone* — the number is always present, since colour-blind users and low-opacity boards both defeat hue. |
| **Checkbox** | Complete / uncomplete. Optimistic, per PR 8. |
| **Time spent** | How long it took, or `—` when nothing was recorded. Click to set or change it: presets plus free text. |
| **Title** | Inline edit on double-click. |
| **Hover actions** | Move, delete. Hidden until hover, per §6.1. |

A board is 340px wide by default, so the row has to stay legible when narrow — the duration collapses before the title truncates.

**Interfaces produced:** `<TaskRow task onComplete onEdit onMove onDelete onSetTimeSpent />`.

**DoD:** Board shows real tasks from SQLite. Completing one persists and survives restart. A duration can be set from the row and is visible afterwards. **Confirm no inference process exists** — Task Manager shows no sidecar.
**Test:** Vitest — grouping, empty state, inline edit commit/cancel, checkbox dispatch, priority rendered as a number not only a colour, duration formatted for none / minutes / hours, presets and free text both dispatch.

---

### - [x] PR 13 — Weekly Tasks board & task interactions
**Branch:** `feat/pr-13-weekly-board`
**Depends on:** PR 12
**What this gives the app:** The week's commitments on screen, plus every way you would want to change a task — complete it, edit it, move it, block it.

**Creates:** `src/features/boards/WeeklyBoard.tsx`, `src/features/boards/components/QuickAdd.tsx`, `src/features/boards/components/TaskContextMenu.tsx`.

Implements the spec's full interaction set: complete, uncomplete, edit text, move to another day, promote daily→weekly, move weekly→another week, add blocker, add comment, delete/archive. Right-click context menu plus keyboard shortcuts.

Grouped by project, current period only (`period_start`/`period_end` covering today).

**DoD:** Every interaction in §6.6 works and persists. Moving a task to another day increments `rollover_count` exactly once (verify in DB).
**Test:** Vitest for the menu and each action's dispatch; Rust test asserting the rollover increment via the real reschedule path.

---

### - [x] PR 14 — Weekly Progress board (expandable Monday–Sunday)
**Branch:** `feat/pr-14-weekly-progress-board`
**Depends on:** PR 13
**What this gives the app:** The whole week at a glance — seven collapsible days, each showing how much got done and how long it took, opening to reveal the tasks inside.

**Creates:** `src/features/boards/WeeklyProgressBoard.tsx`, `components/DaySection.tsx`, `components/DaySummary.tsx`.

**Every day is listed, Monday through Sunday, even when empty.** A week with three empty days *is* the information — showing only the busy days hides the shape of the week.

Each day header summarises its contents without being opened, so a collapsed board still answers "how did the week go":

```text
WEEKLY PROGRESS                    2026-W34

▸ Monday      2/3    1h 45m
▾ Tuesday     1/2      35m
▸ Wednesday   0/2         —
▸ Thursday    0/0         —
```

Expanding a day reveals its rows, each carrying priority, completion, and how long the task took:

```text
▾ Tuesday     1/2      35m

  P8  ☑  Complete onboarding task           35m
  P5  ☐  Schedule dental appointment           —
```

**Today is expanded by default** and visually distinguished. Expand and collapse choices persist per board, so a user who works one day at a time is not re-collapsing six days every launch.

Completed items stay visible but dimmed and struck through — a record of the week, not a list that empties as work is done. Week start day comes from settings (default Monday).

**DoD:** All seven days render on an empty week. A day's summary matches its contents when collapsed. Today is expanded on first open. Collapse state survives a restart. Completing a task on the Weekly Tasks board updates this board live.
**Test:** Vitest — day bucketing across a week boundary, empty days still rendered, summary counts and durations correct, collapse state persisted, week-start setting respected, today highlighted.

---

### - [x] PR 15 — Monthly Progress board
**Branch:** `feat/pr-15-monthly-board`
**Depends on:** PR 14
**What this gives the app:** The monthly view: progress bars showing how far along each commitment is, rather than a list of every task.

**Creates:** `src/features/boards/MonthlyBoard.tsx`, `components/ProgressBar.tsx`.

Outcome-focused, not task-focused. Renders each monthly commitment with its numeric progress (`12 / 20 applications`) and a bar. Bar uses block characters or a styled div — must stay legible at low opacity and small font size.

Progress values come from PR 7's `compute_progress`, never computed in the component.

**DoD:** Monthly commitments show correct roll-up from completed weekly/daily children.
**Test:** Vitest on the bar's percentage rendering incl. 0%, 100%, and over-target; integration test that a completed daily task moves the monthly number.

---

> **PR 15a–15c were not in the original sequence.** They came out of using the boards daily and finding that the plan had described the data correctly and the reading of it badly. They are recorded here because a plan that only lists what was predicted is not a history of the project.
>
> They are filed with the boards because that is what they change, but they **landed after Wave 3's shipped PRs** — 16, 17, and 17a were already on `main` when this work started, which is how a per-board setting from PR 16 was there to reveal that PR 10's type scale could not respond to it. Wave membership here is about subject matter, not chronology.

### - [x] PR 15a — Board text scaling & day panels
**Branch:** `feat/pr-15a-board-text-scale` · shipped as GitHub PR #23
**Depends on:** PR 15
**What this gives the app:** Makes the per-board font size setting actually do something, and turns the week's days from a list of lines into panels you can tell apart at a glance.

**Modifies:** `src/styles/tokens.css`, `src/styles/theme.css`, `src/features/boards/components/DaySummary.tsx`. **Creates:** `src/features/boards/daySummary.ts`.

**The size setting was unreachable, and the reason was in the tokens.** PR 10's type scale was written in `rem`, which resolves against the root font size of the document — one value shared by every board window. A per-board setting cannot move it. The fix is to express the scale relative to the board's own container instead, so the per-board CSS custom property is the thing the text is actually sized from. This is the failure mode a design-token layer is supposed to prevent, and it took a real setting to reveal it.

**Each day gets a completion percentage** alongside its `2/3` count. The fraction answers "how many", the percentage answers "how did the day go", and at a glance the second question is the one being asked. Days with nothing planned report nothing rather than 0% — an empty day is not a failed one.

**DoD:** Changing one board's font size changes only that board and the change survives a restart. Every day renders as a distinct panel with its count, duration, and percentage.
**Test:** Vitest — the percentage is computed from planned versus completed and omitted for an empty day; the summary keeps its count and duration alongside it.

---

### - [x] PR 15b — Sections: user-named task groups
**Branch:** `feat/pr-15b-board-sections` · shipped as GitHub PRs #24, #25, #26
**Depends on:** PR 15a
**What this gives the app:** Lets you name the groups on a board yourself — "Job Search", "House" — instead of living with whatever grouping the app decided on.

**Creates:** `src-tauri/migrations/006_board_sections.sql`, `migrations/007_sections_are_task_groups.sql`, `src-tauri/src/storage/section.rs`, `src-tauri/src/commands/sections.rs`, `src/features/sections/{BoardSections,SectionList,SectionCard}.tsx`, `src/stores/sectionStore.ts`, `src/features/boards/components/TaskGroup.tsx`, `src/features/boards/grouping.ts`, `src/types/section.ts`.

**A section is a name for a group of tasks, not a container that owns them.** It maps onto a field the task already has: `tasks.area` on the Priority board, `tasks.project` on Weekly Tasks. Nothing is duplicated, and a task promoted or rescheduled elsewhere keeps its grouping without any section bookkeeping following it around.

That mapping is what makes the two destructive operations behave sensibly:

| Operation | Behaviour | Why |
|---|---|---|
| **Rename** | Rewrites `area`/`project` across every task in the group, in one transaction | A rename is a rename. Half-renamed tasks scattered across two spellings is the worst outcome available, so it is all-or-nothing. |
| **Delete** | Unfiles the tasks — clears the field, leaves the tasks | Deleting a label must never delete work. The user removing "House" from the board means the heading is wrong, not that the errands are cancelled. |

**Weekly Progress and Monthly Progress refuse sections.** They group by day and by commitment respectively, and neither is a thing the user gets to name — Tuesday is Tuesday, and a monthly commitment is already a task with its own title. Offering a section control there would suggest a grouping that cannot exist. The refusal lives in the section commands rather than in the UI, so a board added later cannot acquire sections by accident.

It arrived in three passes: free-written note cards first (#24), then the add control moved into the board header and the per-section accent rules dropped (#25), then the cards became real task groups (#26). Migration 007 is named `sections_are_task_groups` because that is the correction it makes.

**DoD:** A section can be created, renamed, and deleted on Priority and Weekly Tasks. A rename moves every task in the group. A delete leaves the tasks on the board, ungrouped. The two progress boards expose no section control and reject the command if one is sent.
**Test:** Rust — rename rewrites all matching rows and rolls back whole on failure; delete nulls the field and deletes no task; a section create against a progress board is rejected. Vitest — section list rendering, inline rename commit and cancel, delete confirmation, tasks reappear ungrouped after a delete.

---

### - [x] PR 15c — Priority editable from the badge
**Branch:** `feat/pr-15c-priority-badge-edit` · shipped as GitHub PR #26
**Depends on:** PR 15b
**What this gives the app:** Lets you change a task's priority by clicking the number on it, and the board reorders itself immediately.

**Modifies:** `src/features/boards/components/PriorityBadge.tsx`, `TaskRow.tsx`, `src/stores/taskStore.ts`.

PR 12 made the badge readable and left it read-only, which put the most frequently changed field on the boards behind the context menu. Priority is now settable 1–10 from the badge itself, and every board that sorts by it re-sorts on the change rather than waiting for a reload — otherwise the number says one thing and the position says another.

**DoD:** Clicking a badge sets a priority, it persists, and the board re-sorts in place.
**Test:** Vitest — the picker dispatches the new value, the badge still renders the number and not only a colour, and the surrounding list order updates.

---

# Wave 3 — Desktop shell (PR 16, 17, 17a, 18)

### - [x] PR 16 — Window behaviors
**Branch:** `feat/pr-16-window-behaviors`
**Depends on:** PR 15
**What this gives the app:** Control over how the notes behave — always on top, see-through, locked in place, pinned to one monitor.

**Creates:** `src-tauri/src/windows/behaviors.rs`, `src/features/boards/components/BoardMenu.tsx`.

Always-on-top, desktop-level mode, lock position, click-through when locked (`set_ignore_cursor_events`), adjustable opacity, adjustable font size, pin to a specific monitor, independent per-board visibility, light/dark theme, compact/expanded mode. All persisted to `board_windows`.

**Careful:** click-through must only activate while locked, and there must be an escape hatch (tray menu → Unlock All Boards) or the user can permanently lose access to a board.

**DoD:** Every behavior in §6.7 toggles, persists, and restores after reboot. The unlock escape hatch works.
**Test:** Rust tests on persistence; manual matrix for the OS-level behaviors.

---

### - [x] PR 17 — System tray & quick add
**Branch:** `feat/pr-17-system-tray`
**Depends on:** PR 16
**What this gives the app:** A tray icon, so the app is one click away without a window taking up space.

**Creates:** `src-tauri/src/tray.rs`, `src/features/quick-add/QuickAddWindow.tsx`.

Tray menu exactly as §6.8: Open Boards, Start Daily Standup, Start Evening Check-In, Plan My Week, Monthly Review, Quick Add Task, Pause Reminders, Settings, Quit.

AI entries are present but **disabled with a "coming soon" tooltip** until Wave 4 — they must not silently do nothing. Quick Add opens a small always-on-top input window; adding a task **must not** start any model.

Adds `tauri-plugin-autostart` for launch-at-login.

**DoD:** Tray survives main window close. Quick Add creates a task and the boards update. Quit terminates everything cleanly.
**Test:** Rust test on menu construction; manual for tray interaction.

---

### - [x] PR 17a — Board window lifecycle & close policy
**Branch:** `fix/pr-17a-board-window-lifecycle` · shipped as GitHub PRs #21, #22
**Depends on:** PR 17
**What this gives the app:** Boards that open and close the way you expect them to, from wherever you asked — the tray, the main window, the board's own close button, or Alt+F4.

**Creates:** `src-tauri/src/windows/dispatch.rs`, `windows/close_policy.rs`, `src/components/ErrorBoundary.tsx`. **Modifies:** `src-tauri/src/windows/board_windows.rs`, `commands/boards.rs`, `commands/tray.rs`, `src/features/boards/BoardRoot.tsx`.

Two faults, one root. PR 9 created board windows and PR 17 added a second way to ask for one, and the two paths did not agree about what "close" means or about which thread they were on.

**Window creation has to happen on the main thread.** A board opened from a tray-menu callback or from an async command handler is not on it, and the resulting window came up blank rather than failing loudly. The fix is an explicit dispatch: every route that opens or closes a board posts the request to the main thread and awaits the result, so there is exactly one code path regardless of who asked. A blank window is now impossible by construction rather than by care.

**Closing needs a policy, not a handler.** A board can be dismissed four ways — the board's own close button, Alt+F4, the system window menu, and the taskbar — and the OS routes them differently. Handling three of them and forgetting the fourth is what happened, and the symptom was a board that was gone from the screen but still marked visible in `board_windows`, so it never came back on restart. `close_policy.rs` names the outcome for each route in one place: dismissing a board hides it and records that, it does not destroy state, and reopening restores it.

An `ErrorBoundary` wraps the board root so a render failure inside one board shows an error in that board rather than painting an empty window with no explanation.

**DoD:** Every board opens from the tray, from the main window, and after a previous close, on every route, with content rendered. All four dismissal routes leave `board_windows` agreeing with what is on screen. Reopening a closed board shows the board, not a blank frame.
**Test:** Rust — the dispatcher runs creation on the main thread; each of the four close routes maps to the intended policy; visibility persisted on close matches what was rendered. Vitest — the error boundary catches a throwing board and renders the fallback rather than nothing.

---

### - [ ] PR 18 — Settings & reminders → **tag `v0.1.0`**
**Branch:** `feat/pr-18-settings-reminders`
**Depends on:** PR 17a
**What this gives the app:** Settings you can change and reminders that nudge you morning and evening. **This is the first version genuinely worth using every day.**

**Creates:** `src-tauri/src/storage/settings.rs`, `migrations/008_settings.sql`, `src/features/settings/SettingsWindow.tsx`, `settings/sections/{General,StickyNotes,Planning}.tsx`, `src-tauri/src/reminders.rs`.

**The schema is at 007.** Waves 2 and 3 spent more migrations than this plan predicted — `004_ui_state`, `005_board_appearance`, and PR 15b's `006_board_sections` and `007_sections_are_task_groups` — so this PR's migration is **008**, not the `003` written here originally, and every later migration number in this file has been moved up to match. Migration numbers are positions in a sequence, unlike PR numbers; a duplicate is a runtime failure, not a documentation nit.

Settings sections per §18 — only General, Sticky Notes, and Planning here; Obsidian/AI/Voice sections are added by the PRs that introduce those subsystems.

Reminders: morning and evening notification at configurable times via `tauri-plugin-notification`, with Pause Reminders honored. Clicking a reminder opens the app — it does **not** auto-start a model.

**DoD:** Settings persist across restart. A reminder fires at the configured time. **This is a shippable non-AI desktop planner.**
**Test:** Rust tests on settings get/set with defaults and on reminder scheduling arithmetic; Vitest on the settings forms.

**After merge:** `git tag v0.1.0 && git push --tags`, and cut a GitHub release with the built `.msi`.

---

# Wave 4 — Local LLM text standup (PR 23–28)

The standup runs on the app's own data. There is no vault at this point in the sequence and the wave does not need one — Waves 1 through 3 built monthly commitments, weekly milestones, named sections, priorities, due dates, rollover counts, and logged durations, all in SQLite and all more structured than the markdown the model was originally going to be handed.

### - [ ] PR 23 — Model registry, hardware detection & download
**Branch:** `feat/pr-23-model-registry`
**Depends on:** PR 18
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

### - [ ] PR 24 — llama.cpp sidecar lifecycle
**Branch:** `feat/pr-24-llama-lifecycle`
**Depends on:** PR 23
**What this gives the app:** The AI starts only when you ask for it and shuts down completely when you are done. **This is the promise the entire product rests on.**

**The single most important PR in the project.** §26 says the boards are always available but inference is always on demand — this is where that becomes true or doesn't.

**Creates:** `src-tauri/src/inference/llama.rs`, `inference/process.rs`, `inference/health.rs`, `src-tauri/binaries/README.md` (how to vendor the sidecar binary).

Spawns `llama-server` as a Tauri sidecar on a **random free localhost port**, bound to `127.0.0.1` only, with `--host 127.0.0.1`. Waits for a health check before reporting ready. Terminates on session end, on idle timeout (default 5 min, configurable per §7.4), and on app quit. Detects and reaps orphaned processes from a previous crash by recording the PID in SQLite at spawn.

**Backend selection and GPU offload:** picks the backend binary chosen in PR 23 and computes `--n-gpu-layers` from detected VRAM minus a safety margin, rather than hard-coding a layer count. If the GPU-offloaded spawn fails — driver mismatch, VRAM exhausted, another process holding memory — **fall back to the CPU backend automatically and tell the user what happened.** A failed offload must degrade to slow, never to broken.

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

### - [ ] PR 25 — Commitment map & tiered context builder
**Branch:** `feat/pr-25-context-builder`
**Depends on:** PR 24
**What this gives the app:** Gives the AI a map of your commitments instead of everything you have ever written down, so it knows where to look without having to read all of it.

Per §9.2, **never send everything** — the reason is arithmetic, not taste. A 500-note vault is ~260k tokens, roughly 33 GB of KV cache and ~9 minutes of prefill, and a year of accumulated tasks gets there too. So the model gets a small **map** of what exists plus a small set of ranked detail, and can ask for more.

**The tiered budget was always the point; the vault was one way to fill it.** This PR was originally written to source all three tiers from indexed Obsidian notes, and was sequenced after the vault for that reason. It no longer is. The architecture below is unchanged — three tiers, hard token budgets, deterministic ranking, on-demand expansion — but every tier is fed from SQLite, which by the end of Wave 3 holds monthly commitments, weekly milestones, user-named sections, priorities, statuses, due dates, rollover counts, and logged durations. That is a *better* input than markdown prose: it is already typed, already scored, and already the thing the user manipulates every day. When Obsidian lands in Wave 7 it becomes an **additional source feeding these same tiers** — a vault branch in the map, more candidates for the ranker, notes as one more thing Tier 3 can fetch. It does not replace anything here, and nothing in this PR should be written as though it were temporary.

**Creates:** `src-tauri/src/inference/map.rs`, `inference/context.rs`, `inference/prompt.rs`, `prompts/daily-standup.md`, `prompts/evening-review.md`, `prompts/weekly-planning.md`, `prompts/monthly-review.md`.

**Tier 1 — the commitment map (~1,500 token budget, always present).** Active work only, rendered from `tasks` as an indented tree with three levels: the user's named **sections** (PR 15b) as the headings, the month's commitment for each one on that heading line, and the weekly milestones indented beneath. Every line carries priority and status. Roughly 12–15 tokens per line, so ~20 headings and ~90 children fit the budget. Capped by count *and* tokens; overflow drops the lowest-ranked sections first.

```
COMMITMENT MAP — active

Job Search [p9, in progress] — 12/20 applications this month
  Fall 2026 applications ....... weekly, 3/5 done
  Rewrite resume ............... weekly, blocked
  Referral follow-ups .......... weekly, not started

Health [p8, in progress] — clear the overdue appointments
  Schedule dental cleaning ..... daily, deferred 4×
  Book eye test ................ weekly, not started
```

Sections are what make the tree legible rather than a flat priority-ordered dump. The user named them, so they are the grouping the user already thinks in — the model gets the shape of the week for free instead of inferring categories from task titles.

**Tier 2 — ranked context (~2,000 token budget).** All of it already in SQLite: current weekly and monthly commitments, yesterday's incomplete tasks, upcoming due dates, and repeatedly-deferred tasks per §11.2. Each item carries its own detail — blocker text, progress numbers, time logged, `rollover_count` — because that detail is what turns "this is on your list" into "this has moved four times and has a blocker on it."

**Tier 3 — on demand.** Fetch one task's subtree for PR 28's expansion loop: the task, its children, its notes, and its blocker. A subtree rather than a single row, because the interesting question is almost never about one task in isolation.

The division of labor is unchanged and still matters: **the map tells the model what exists; the ranker tells it what is urgent.** Dropping the ranker would force the model to infer priority from the map, which is exactly the judgment §3.6 says must stay deterministic.

Prompts live in editable markdown files with a `{{variable}}` substitution layer, so users can customize them later without a rebuild.

**Interfaces produced:**
```rust
fn build_map(repo: &TaskRepo, budget: usize) -> Result<CommitmentMap>
fn build_context(session_kind, budget: TokenBudget) -> Result<SessionContext>
fn fetch_task_subtree(id: &str) -> Result<TaskSubtree>   // Tier 3, used by PR 28
fn render_prompt(template, &SessionContext) -> String
```

`SessionContext` is the seam Wave 7 widens: a tier holds a list of context items, not a list of tasks, so PR 25 needs no change when vault-sourced items start arriving alongside task-sourced ones.

**Careful:** the budgets are enforced in the builder, not hoped for in the template. A prompt that renders 6k tokens on a 2k-context Lightweight profile is a truncated prompt, and a truncated prompt is a model answering a question it was never fully asked.

**DoD:** The rendered map for a real board set fits its budget and reads like the example above. Total prompt stays under the profile's context size on every hardware profile.
**Test:** Rust tests — map respects both count and token caps; overflow drops lowest-ranked sections first; a task with no section renders ungrouped rather than being dropped; a deep parent chain renders without hanging; a task tree far larger than the budget still produces a prompt under the Lightweight profile's context size; `fetch_task_subtree` returns children, notes, and blocker for a task and an empty subtree for a leaf.

---

### - [ ] PR 26 — Standup session state machine & chat UI
**Branch:** `feat/pr-26-standup-session`
**Depends on:** PR 25
**What this gives the app:** An actual standup conversation you can type. The app asks the questions, in order, and keeps control of the conversation.

**Creates:** `src-tauri/src/session/mod.rs`, `session/state_machine.rs`, `src/features/standup/StandupWindow.tsx`, `standup/components/{MessageList,Composer,StageIndicator,ContextPanel}.tsx`.

Stages are driven by **Rust**, not the model: Context → Previous progress → Current priorities → Blockers → Capacity → Proposed commitments → Approval → Save & close. The model generates the language for each stage; it cannot skip, reorder, or invent stages.

UI shows streaming responses, a stage indicator, the retrieved context with the board each item came from, and the §7.5 loading sequence ("Starting local assistant… Loading language model… Ready."). Showing the context is not decoration: a user who can see the six tasks the model was handed can tell the difference between a bad suggestion and a bad retrieval. Typed input only in this PR — voice arrives in Wave 5.

**Interfaces produced:** `start_session(kind) -> SessionId`, `send_message(session, text)`, `advance_stage(session)`, `end_session(session)` (which shuts down inference).

**DoD:** A full typed standup runs end to end and the model process terminates when the window closes.
**Test:** Rust tests on stage transitions incl. illegal transitions rejected; Vitest on the chat UI with a mocked stream.

---

### - [ ] PR 27 — Structured output, validation & approval
**Branch:** `feat/pr-27-structured-approval`
**Depends on:** PR 26
**What this gives the app:** Everything the AI suggests gets checked by real code and shown to you for approval before a single task is saved.

**Creates:** `src-tauri/src/session/proposal.rs`, `session/validator.rs`, `src/features/standup/components/ApprovalPanel.tsx`.

The model returns the §11.5 JSON shape. Rust deserializes it with strict serde types and validates: titles non-empty and under a length cap, `horizon` a known variant, `parentTaskId` referencing an existing task, no duplicate proposals, and no path traversal in any string that will be treated as a path.

**`sourceFile` is optional and unvalidated until Wave 7.** The check the original plan specified — inside the vault, not in an excluded folder — has nothing to check against while there is no vault configured, and a validator that cannot answer its own question must not pretend to. So the field is accepted when present, carried through unread, and never used to resolve a file. PR 19's `is_indexable` is what makes it meaningful, and the inside-the-vault and exclusion checks are added to this validator in Wave 7 alongside it.

**Every other adversarial check stands in full** — titles, horizon, `parentTaskId`, duplicates, path traversal. Those are the checks that hold regardless of where the model's context came from, and none of them is weakened by the vault's absence.

Invalid output triggers **one** repair round-trip with the validation errors appended to the prompt; a second failure surfaces an error rather than guessing.

Approval UI: every proposed task can be individually approved, edited, or rejected. Nothing is written to SQLite until approval. Approved tasks become real tasks and the boards refresh.

**DoD:** A standup produces a plan you approve, and those tasks appear on the boards. **Deliberately malformed model JSON never reaches the database.**
**Test:** Rust tests feeding adversarial payloads — missing fields, wrong types, path traversal (`../../etc/passwd`), a parent ID that doesn't exist, a duplicate proposal, 10,000-character titles. Each must be rejected. Plus one test asserting a proposal carrying a `sourceFile` is accepted and that the value is never resolved to a filesystem path — the Wave 7 PR that adds the vault checks inherits this test file and turns that case into a rejection.

---

### - [ ] PR 28 — Bounded context expansion → **tag `v0.2.0`**
**Branch:** `feat/pr-28-context-expansion`
**Depends on:** PR 27
**What this gives the app:** Lets the AI ask to see the detail behind a specific commitment, or ask you a question, when what it has is not enough to answer well.

Lets the model say "I need to see what's under that" or "I don't know where you track this" — **without tool-calling.** Small quantized models are unreliable at tool-use protocols, and every tool call is another 5–10s round trip.

**Creates:** `src-tauri/src/session/expansion.rs`. **Modifies:** `session/proposal.rs`, `session/validator.rs`, `src/features/standup/components/MessageList.tsx`.

Extends PR 27's validated envelope with two optional fields:

```json
{
  "needs_context": ["task:9d3f1a7c-..."],
  "question_for_user": "I don't see anything about travel — where do you track that?"
}
```

`needs_context` asks for a **task subtree**, by the task ID the model saw in Tier 1's map. Rust validates each requested ID (well-formed, exists, belongs to this user's data), fetches it via PR 25's `fetch_task_subtree`, appends it, and re-prompts. An ID is a much easier thing for a small model to echo back correctly than a filesystem path, and a much easier thing for Rust to check — the id either resolves to a row or it does not. **Capped at 2 expansion rounds** so a request loop cannot spiral into a minute of latency. Parent context comes free from `parent_task_id` — fetching a weekly milestone also pulls the monthly commitment it sits under.

The prefixed form (`task:<id>`) is deliberate. Wave 7 adds `note:<path>` as a second requestable kind without changing the envelope or reteaching the model a new field.

`question_for_user` costs nothing extra: it just renders in the chat and waits for a reply.

This works with a weak model because it is only JSON output — not a protocol the model has to execute correctly.

**Interfaces produced:** `expand_context(session, requests: Vec<String>) -> Result<SessionContext>`, `ExpansionBudget { max_rounds: 2, max_items_per_round: 3 }`.

**DoD:** Asking about something outside the retrieved set causes the model to request the right subtree and answer correctly on the second pass. Expansion never exceeds 2 rounds. An unresolvable request is refused without erroring the session.
**Test:** Rust tests — a malformed ID is refused; a well-formed ID for a task that does not exist is refused without erroring the session; a request carrying a path instead of an ID is refused; the round cap holds when the model requests context every turn; parent expansion pulls the monthly commitment above a requested weekly milestone.

**After merge:** tag `v0.2.0`.

---

# Wave 5 — Voice (PR 29–32)

### - [ ] PR 29 — Audio capture, push-to-talk & VAD
**Branch:** `feat/pr-29-audio-capture`
**Depends on:** PR 28
**What this gives the app:** Your microphone, on a push-to-talk key, with nothing written to disk.

**Creates:** `src-tauri/src/audio/mod.rs`, `audio/capture.rs`, `audio/vad.rs`, `src/features/settings/sections/Voice.tsx`.

`cpal` for microphone capture, device enumeration, and level metering. Push-to-talk via `tauri-plugin-global-shortcut`. Simple energy-based VAD to trim silence.

**Audio stays in memory as `Vec<f32>` and is zeroed after transcription.** No temp files unless the user explicitly enables retention. The mic is only opened during an active session — never in the background (§16.1).

**Interfaces produced:** `list_input_devices()`, `start_capture(device) -> CaptureHandle`, `CaptureHandle::stop() -> AudioBuffer`, `trim_silence(&AudioBuffer) -> AudioBuffer`.

**DoD:** Hold the shortcut, speak, release → a buffer of the right duration and sample rate. Mic indicator is off outside a session.
**Test:** Rust tests on VAD with synthetic signals (silence, speech-like noise, clipping) and on buffer zeroing after use.

---

### - [ ] PR 30 — whisper.cpp transcription
**Branch:** `feat/pr-30-whisper`
**Depends on:** PR 29
**What this gives the app:** What you say becomes text, transcribed on your own machine and nowhere else.

**Creates:** `src-tauri/src/audio/whisper.rs`, extends `inference/registry.rs` with Whisper models.

Runs whisper.cpp on the captured buffer, resampling to 16kHz mono. Whisper model selection (tiny/base/small) follows the hardware profile from PR 23. Loaded on session start, unloaded on session end alongside the LLM.

**On cards with ≤8 GB VRAM, run Whisper on CPU.** The LLM plus KV cache already takes ~6 GB of 8 GB at the High Quality profile, and Whisper base transcribes 10s of audio on CPU in 1–2s. Make this the automatic default when detected VRAM headroom is under 2 GB.

**Fallback is mandatory:** if transcription fails or no mic exists, the composer stays fully usable for typing. §17.2 — voice enhances the product, it must never be required.

**Interfaces produced:** `transcribe(&AudioBuffer, &WhisperModel) -> Result<String>`.

**DoD:** Speaking a sentence produces accurate text in the composer. Unplugging the mic mid-session degrades to typing with a clear message, not a crash.
**Test:** Rust test transcribing a short committed WAV fixture and asserting keyword presence; failure-path test asserting the typed fallback stays enabled.

---

### - [ ] PR 31 — Sherpa-ONNX text-to-speech
**Branch:** `feat/pr-31-tts`
**Depends on:** PR 30
**What this gives the app:** The assistant talks back out loud, with a voice generated locally.

**Creates:** `src-tauri/src/audio/tts.rs`, `audio/playback.rs`, `audio/segmentation.rs`.

Sentence segmentation so speech starts before the full response finishes generating. Playback via `rodio`. Voice selection, speed control, auto-play toggle, and a **Stop Speaking** button (§12.4 — full interruption is deliberately out of scope for the MVP).

Per §12.3 the assistant summarizes rather than reading long task lists aloud — enforce this in the prompt, and cap spoken length.

**Interfaces produced:** `synthesize(text, voice) -> AudioBuffer`, `speak(stream)`, `stop_speaking()`.

**DoD:** The assistant's response is spoken locally with no network access. Stop Speaking halts playback immediately. TTS unloads with the session.
**Test:** Rust tests on sentence segmentation (abbreviations, decimals, ellipses shouldn't split wrongly) and on the stop signal cancelling the queue.

---

### - [ ] PR 32 — Onboarding wizard → **tag `v0.3.0` (MVP)**
**Branch:** `feat/pr-32-onboarding`
**Depends on:** PR 31
**What this gives the app:** A first-run walkthrough that takes a new user from install to their first standup without ever opening Settings.

**Creates:** `src/features/onboarding/OnboardingWizard.tsx` and one step component per §5.1 stage.

The steps that exist by now: explain local processing → detect hardware → recommend models → download after confirmation → test mic and voice → place boards → choose daily-only or all horizons → add a first commitment.

**§5.1's five vault steps are not in this wizard** — pick vault, read-only first, scan, show discovered structure, exclude folders. Obsidian does not exist until Wave 7, and a wizard step for a subsystem that is not installed is a dead end with a "coming soon" on it. Wave 7 inserts those five steps into the same wizard, which is why the step list is a data structure rather than a hard-coded sequence of components: adding a step there must not mean rewriting navigation here. A user who onboarded before Wave 7 gets the vault steps offered once on the update, not silently skipped.

The last step, **add a first commitment**, replaces what the vault scan used to provide. The original wizard ended with a vault full of structure the app had just discovered; without one, a new user reaches their first standup with nothing on the boards and the model has nothing to work from. Asking for one monthly commitment costs the user thirty seconds and makes the first session real.

The hardware step surfaces PR 23's benchmark result plainly — measured tokens/sec, the selected profile, and the expected turn latency — so the §7.5 startup tradeoff is set as an expectation before first use rather than discovered as a surprise. Offer the CUDA backend download here when an NVIDIA GPU is present.

**DoD:** A fresh install walks a new user from zero to a working first standup without touching Settings. Re-runnable from Settings. A step can be added to the sequence without editing navigation.
**Test:** Vitest on step navigation, back/forward state retention, the skip paths, and that the step list drives the wizard rather than being enumerated inside it.

**After merge:** tag `v0.3.0` and cut a release. **This is the MVP** — check it against every line of §23's acceptance criteria that does not depend on Obsidian, and note the remainder as Wave 7's acceptance gate.

---

# Wave 6 — Reviews (PR 33–35)

The review cycle closes the loop entirely inside the app. Each session reads the same SQLite numbers the boards show and writes its outcome back to them; nothing here waits on a file being written anywhere else, which is the reason the reviews can ship four PRs before the writer that eventually records them in a vault.

### - [ ] PR 33 — Evening check-in
**Branch:** `feat/pr-33-evening-checkin`
**Depends on:** PR 32
**What this gives the app:** An end-of-day check-in that asks what happened, and what to do with whatever did not.

Shorter session per §5.3.

**Queries implausible durations.** A task logged at eighteen hours is more likely a typo than a marathon, so it is raised here for confirmation. The user may be right, so this asks rather than refusing — but an unchallenged typo distorts every total built on it.

Distinguishes the five outcomes the spec names: still important / blocked externally / too large / no longer wanted / recurring avoidance. Reschedule, backlog, delegate, or drop each unfinished task. Language stays non-judgmental (§10.3).

**Creates:** `src-tauri/src/session/evening.rs`, `src/features/evening/EveningWindow.tsx`.
**DoD:** Unfinished tasks are triaged and the outcome persists with correct rollover accounting.
**Test:** Rust tests — each outcome produces the right status and rollover delta.

---

### - [ ] PR 34 — Weekly planning & retrospective
**Branch:** `feat/pr-34-weekly-review`
**Depends on:** PR 33
**What this gives the app:** A weekly review built on real numbers — planned versus completed — and a realistic plan for the week ahead.

Per §5.4, using PR 7's `period_stats` for all numbers — **the model never calculates completion rates or hour totals.**

**Includes the hours recap:** total time tracked for the week, broken down by area and project, from PR 11's `minutes_in_period`. Rust sums it; the model only narrates it. A week where the numbers and the narrative disagree is worse than no narrative.

Also surfaces the mismatches worth reflecting on: a task deferred five times with zero minutes recorded says something different from one deferred five times with six hours on it.

**Creates:** `src-tauri/src/session/weekly.rs`, `src/features/weekly-review/WeeklyReviewWindow.tsx`.
**DoD:** A weekly session reviews real stats including hours by area, sets milestones, and refreshes the weekly boards.
**Test:** Rust tests on stat computation across a week boundary; hours attributed to the correct week when a task spans the boundary; snapshot test on the generated review structure.

---

### - [ ] PR 35 — Monthly planning & retrospective → **tag `v0.4.0`**
**Branch:** `feat/pr-35-monthly-review`
**Depends on:** PR 34
**What this gives the app:** A monthly review that compares what you actually did against the commitments you set, and sets the next month's.

Per §5.5 — compares completed work to standing commitments, surfaces neglected areas, sets a limited number of measurable monthly commitments, seeds initial weekly milestones.

"Long-term goals" here means the monthly commitments on the Monthly board and the sections the user has named, not vault notes. Wave 7 widens the comparison to vault-side goals; the retrospective structure does not change when it does.

Uses tracked hours to make "neglected" concrete: an area with commitments but almost no recorded time is a clearer signal than one inferred from task counts alone.

**Creates:** `src-tauri/src/session/monthly.rs`, `src/features/monthly-review/MonthlyReviewWindow.tsx`.
**DoD:** A monthly session produces commitments with measurable targets that the Monthly board renders. **The full daily → evening → weekly → monthly cycle runs end to end on the app's own data.**
**Test:** Rust tests on neglected-area detection and commitment→milestone seeding.

**After merge:** tag `v0.4.0` and cut a release.

---

# Wave 7 — Obsidian read integration (PR 19–22)

By the time this wave starts the app is already a complete product: boards, a typed standup, voice, and the full review cycle, all running on its own SQLite data. Obsidian arrives as an **additional source**, not as the foundation it was originally sequenced to be. Nothing in Waves 4–6 gets rewritten to accommodate it; the context builder gains a tier and the ranker gains inputs.

### - [ ] PR 19 — Vault selection & folder scoping
**Branch:** `feat/pr-19-vault-config`
**Depends on:** PR 35
**What this gives the app:** Lets the app point at your Obsidian vault — and lets you decide which folders it must never look at.

**Creates:** `src-tauri/src/obsidian/mod.rs`, `obsidian/config.rs`, `migrations/009_vault.sql`, `src/features/settings/sections/Obsidian.tsx`.

Folder picker for the vault root. Include/exclude lists with the spec's suggested defaults pre-filled as *suggestions the user confirms*: `Private/`, `Journal/`, `Medical/`, `Financial/`. Read-only mode toggle, defaulting to **on**. Configurable target folders for daily/weekly/monthly notes.

Exclusion is enforced by a single `is_indexable(path) -> bool` function that every later Obsidian code path must call. Centralizing it is the whole point — a second exclusion check somewhere else is how a private note eventually leaks into a prompt.

**Interfaces produced:** `VaultConfig { root, include, exclude, read_only, daily_folder, weekly_folder, monthly_folder }`, `is_indexable(&VaultConfig, &Path) -> bool`.

**DoD:** Selecting a vault persists it. Excluded paths return false, including nested children and case variations.
**Test:** Rust tests over a temp fixture vault — exclusion of nested paths, glob edge cases, symlink refusal.

---

### - [ ] PR 20 — Markdown parser
**Branch:** `feat/pr-20-markdown-parser`
**Depends on:** PR 19
**What this gives the app:** Teaches the app to read your notes: the checkboxes, the tags, the priorities, the links between them.

Pure parsing library, zero I/O, so it can be tested exhaustively against fixtures.

**Creates:** `src-tauri/src/obsidian/parser.rs`, `obsidian/frontmatter.rs`, `tests/fixtures/vault/**` (realistic sample notes).

Parses per §9.1: YAML frontmatter (`serde_yaml`), note title, headings, markdown checkboxes with line numbers, wikilinks, `parent` relationships, tags, `status`, `priority`, due dates, `last_updated`, callouts.

Also extracts a **`summary_line`** — the one-line description each note contributes to PR 25's map once the vault tier is switched on. Resolution order, first hit wins: the `desired_outcome` frontmatter field (§9.6) → the first non-empty prose line after the H1 → the title alone. Truncate to 100 characters at a word boundary. Fully deterministic; no model involved.

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

### - [ ] PR 21 — Vault indexer & file watcher
**Branch:** `feat/pr-21-vault-indexer`
**Depends on:** PR 20
**What this gives the app:** Builds a fast index of your vault and keeps it current as you edit in Obsidian, so nothing has to be re-read from scratch.

**Creates:** `src-tauri/src/obsidian/indexer.rs`, `obsidian/watcher.rs`, `migrations/010_vault_index.sql`.

Walks the vault (honoring `is_indexable`), parses each note, writes a lightweight index across three tables:

- `notes` — path, title, **`summary_line`**, frontmatter fields (status, priority, parent, due), mtime, hash
- `note_tasks` — note_path, line, text, checked
- **`note_links`** — an edge list (`from_path`, `to_path`, `kind`) built from wikilinks and `parent:` frontmatter

The edge table is what makes the vault a real graph rather than a flat list. It costs almost nothing to populate and buys three things: a vault tier for PR 25's map, rendered as a hierarchy the same way the task tree is, **neighbor expansion** (pulling in a note's parent summary alongside the note itself), and §5.5's "identify neglected areas" as a graph query. Ancestor and descendant traversal is a recursive CTE — roughly eight lines of SQL.

Incremental — reindex a file only when mtime or hash changed.

File watcher via `notify`, debounced 1s, triggering targeted reindex. Full scan runs off the UI thread with progress events.

**Interfaces produced:** `index_vault(&VaultConfig) -> Result<IndexStats>`, `reindex_file(path)`, `start_watcher(app)`, `stop_watcher()`.

**DoD:** Indexing a real vault completes and reports counts. Editing a note in Obsidian updates the index within ~2s. Excluded folders produce zero rows — verify by querying the DB directly. A recursive CTE returns the correct ancestor chain for a nested note.
**Test:** Rust tests over a temp vault — initial index, incremental no-op, file added/modified/deleted, ancestor/descendant traversal, a link to a non-existent note (must not error), a link cycle (must terminate), and an explicit test asserting excluded files never appear in *any* of the three tables.

---

### - [ ] PR 22 — Relevance ranking & promote-to-board
**Branch:** `feat/pr-22-ranking-promote`
**Depends on:** PR 21
**What this gives the app:** The app can now tell you which vault tasks matter most today, and you can pull one onto a board with a link back to the note it came from.

**Creates:** `src-tauri/src/obsidian/ranking.rs`, `src/features/vault/VaultBrowser.tsx`, `src/features/vault/components/SourceBadge.tsx`.

Deterministic scoring per §9.3 — project priority + due-date urgency + active-status weight + weekly/monthly connection + rollover count + recent mentions. Weights are named constants in one place, documented, and tunable. **No embeddings.**

UI: browse ranked candidate tasks, promote one onto a board. Promotion copies text and sets `source_type: 'obsidian'`, `source_file`, `source_line`. Every promoted task shows a source badge; clicking it opens the note via the `obsidian://open?path=` URI.

**This ranker no longer feeds PR 25 — it enriches what PR 25 already ranks.** In the original sequence the standup had no context until this scorer existed. It ships four waves later now, and PR 25's Tier 2 has been ranking real tasks out of SQLite since Wave 4. What this adds is a second population of candidates and three signals the task table cannot supply on its own: recent mentions, note-graph connection, and vault-side project priority. The scoring weights stay in the same named-constants block, so a vault candidate and a board task are ordered against each other rather than in separate lists.

**This is also where the two deferred safety checks land**, because this is the first PR in which a model-authored `sourceFile` can point at a real file. **Modifies:** `session/validator.rs` — `sourceFile` becomes required-to-be-valid when present: inside the vault root and passing `is_indexable`, per PR 27. **Modifies:** `session/expansion.rs` — `needs_context` accepts `note:<path>` alongside `task:<id>`, and a request for an excluded path is refused with the refusal invisible to the model; it is not told the file exists. Both were written as deferrals in Wave 4 with the tests already in place; this PR turns those tests from accept to reject.

**DoD:** Ranking is stable and explainable — each candidate shows its score breakdown, matching §9.4's transparency example. Promoted tasks keep working source links. A vault candidate and a SQLite task with the same signals score the same. A proposal whose `sourceFile` points at an excluded folder is rejected, and a `needs_context` request for one is refused.
**Test:** Rust table-driven tests on scoring (a priority-9 overdue task outranks a priority-3 one; ordering is deterministic for equal scores); a test asserting the vault-sourced signals change the ranking of an existing task rather than producing a separate ordering; the decoy-content test — an excluded folder filled with distinctive strings, asserted absent from the rendered prompt; `sourceFile` pointing at `Private/` rejected; `note:` path traversal refused; Vitest on the badge and promote flow.

---

# Wave 8 — Obsidian writeback (PR 36–38)

Reading came first for a reason that survives the resequencing: the app must be able to describe a vault accurately before it is allowed to change one. Wave 7 built the index, the exclusion rule, and the source links; this wave is what earns the right to write.

### - [ ] PR 36 — Obsidian writer (append-only review notes)
**Branch:** `feat/pr-36-obsidian-writer`
**Depends on:** PR 22
**What this gives the app:** Your standups and reviews get written back into Obsidian — but only after you approve the exact change.

The reviews themselves shipped in Wave 6 and have been running against SQLite since. What this adds is the record: the daily, weekly, and monthly notes those sessions produce, written into the vault in §13's formats.

**Creates:** `src-tauri/src/obsidian/writer.rs`, `obsidian/diff.rs`, `src/features/approval/DiffApproval.tsx`.

Writes daily/weekly/monthly notes into the **dedicated folders only** (§9.5), in the exact formats from §13.1–13.3. Every write shows a diff preview with the file, the change, and the reason before it happens. Original project notes are untouched by this PR.

**Interfaces produced:** `propose_write(target, content) -> WriteProposal`, `apply_write(proposal) -> Result<()>` — and `apply_write` must be unreachable without an approved proposal.

**DoD:** A completed standup writes a correctly formatted daily note after you approve the diff. Rejecting writes nothing. Read-only mode blocks it entirely.
**Test:** Rust tests over a temp vault — write lands in the right folder with the right frontmatter; rejection is a no-op; a proposal targeting a path outside the approved folders is refused; read-only mode refuses all writes.

---

### - [ ] PR 37 — Conflict detection, backups & source-task updates
**Branch:** `feat/pr-37-write-conflicts`
**Depends on:** PR 36
**What this gives the app:** Protects your notes. Detects if you edited a file first, backs up before touching anything, and refuses to overwrite newer work.

**Creates:** `src-tauri/src/obsidian/conflict.rs`, `obsidian/backup.rs`.

Per §17.3: record mtime and hash when a proposal is created; re-check immediately before writing. If the file changed, show a conflict and preserve both versions rather than overwriting. Back up any existing file before modifying it.

Adds the one case where original notes may change: ticking a source checkbox (`- [ ]` → `- [x]`) at a recorded line, gated behind the "may update original tasks" setting, defaulting to **off**, and verified by matching the line's text before editing — never by line number alone, since the file may have shifted.

**DoD:** Editing a note in Obsidian between proposal and approval produces a conflict warning, not a lost edit. Checkbox writeback ticks the right line and refuses when the line text no longer matches.
**Test:** Rust tests — mtime change detected, hash change detected, backup created before modify, checkbox writeback with a shifted line refuses rather than corrupting.

---

### - [ ] PR 38 — Contextual AI helpers → **tag `v1.0.0-rc`**
**Branch:** `feat/pr-38-ai-helpers`
**Depends on:** PR 37
**What this gives the app:** Small AI helpers on a single task — break this down, help me unblock this — without starting a whole session.

The remaining §14 actions that may start AI after confirmation, invoked from a task's context menu rather than from a full session: **Break This Task Down**, **Help Me Resolve This Blocker**, **Summarize Progress**, **Suggest Priorities**.

**Creates:** `src-tauri/src/session/helpers.rs`, `src/features/boards/components/AiHelperMenu.tsx`, `prompts/helpers/*.md`.

Each is a single-shot request reusing PR 24's lifecycle and PR 27's validation and approval path — no new inference machinery. Every one shows a confirmation first ("This will start the local model, ~8s") because a board interaction must never silently spawn a process.

**DoD:** Each helper produces an approvable proposal and shuts the model down afterward per the idle-timeout setting. Cancelling at the confirmation dialog starts nothing.
**Test:** Rust tests that each helper routes through the same validator as PR 27; Vitest asserting the confirmation dialog gates the spawn call.

**After merge:** tag `v1.0.0-rc` and validate every §23 acceptance criterion, including the vault-dependent ones deferred at `v0.3.0`.

---

# Wave 9 — Cross-platform & community (post-1.0, not yet planned)

Deliberately unplanned until v1.0-rc ships and real usage reveals what actually matters. Expected content per §22 Phase 6: macOS support, Linux support, signed installers, automated releases, contributor documentation, a model adapter interface, theme and prompt customization, and an accessibility review. This wave was Wave 8 before Obsidian moved to the end; only its number changed.

Write this wave's PR sequence after the RC, not before.

---

## Risks to watch while executing

| Risk | Where it bites | Guard |
|---|---|---|
| Sidecar binaries bloat the repo | PR 23, 30, 31 | Never commit binaries. Vendor at build time via a script; document in `binaries/README.md`. |
| Windows Smart App Control blocks unsigned sidecars | PR 23, 30, 31 | **Confirmed real on 2026-08-20:** Smart App Control blocked `rustdoc.exe`, `rustfmt.exe`, and cargo build scripts on the dev machine, failing release builds outright. It judges on *reputation*, not signatures, so freshly-built zero-reputation binaries are exactly what it rejects — the same profile as a bundled `llama.cpp`, `whisper.cpp`, or Sherpa-ONNX sidecar. It ships enabled by default on many Windows 11 installs and has **no allowlist**; disabling it is irreversible without a system reset, so "turn off your security feature" is not an acceptable install step. Treat code-signing the sidecars as a shipping requirement, not a nice-to-have, and detect-and-explain the failure rather than letting a session hang. |
| Model process leaks memory between sessions | PR 24 | Task Manager **and `nvidia-smi`** check is part of PR 24's DoD, repeated at every later voice/session PR. |
| Excluded folders leak into a prompt | PR 19, 20, 21, 25 | Exclusion is one function (`is_indexable`) called by every Obsidian path, and the map's vault tier is built from indexed notes only, so an excluded folder is absent by construction rather than filtered after the fact. Wave 7 adds the decoy-content test — an excluded folder filled with distinctive strings, asserted absent from the rendered prompt — and restores the excluded-path refusal in `needs_context`. **This risk does not exist before Wave 7**, which is the one genuine safety benefit of the resequencing: the AI subsystem is fully exercised before it is ever pointed at private files. |
| VRAM misdetected, wrong profile chosen | PR 23 | Use DXGI/`nvidia-smi`, never WMI `AdapterRAM`. Regression test pinned to the 8 GB-reports-as-4,095 MB case, plus an empirical benchmark that overrides the heuristic. |
| Context expansion spirals into latency | PR 28 | Hard cap of 2 rounds and 3 items per round, enforced in Rust, not requested of the model. |
| Context map grows past its budget | PR 25 | Capped by token count *and* node count; overflow drops the lowest-ranked sections first, with a test. The cap has to hold for a large task tree in Wave 4 and again for a large vault in Wave 7 — same budget, two populations. |
| Window management fights the OS | PR 9, 16, 17a | Keep behaviors in `behaviors.rs` behind a trait so platform quirks stay isolated in Wave 9. **Already bitten once:** PR 17a found board creation off the main thread producing blank windows, and a close path that handled three of four dismissal routes. Route every open and close through one dispatcher. |
| Nobody logs durations, so the recap is empty | PR 11, 12, 33 | Logging must be one gesture with presets, offered at the natural moment (completion), and never mandatory. A recap built on a third of the week is still useful; a prompt users learn to dismiss is not. Implausible values are queried rather than rejected. |
| The standup has nothing to reason about | PR 25, 26, 32 | Moving Obsidian to the end means the model's context is whatever the user has put on the boards, and a new user has put nothing there. PR 32's wizard ends by asking for one monthly commitment; PR 26's first stage must handle an empty board set by asking rather than by proposing tasks out of nothing. A model inventing a plausible-looking week from an empty database is the worst first impression available. |
| Scope creep inside a PR | Everywhere | The DoD line "touches only its stated scope." Spin extras into new issues. |
| CI build times balloon | PR 3 onward | Rust cache from day one; gate model-dependent tests behind an env var. |

---

## Executing this plan

Before each PR: I write a task-level TDD plan for that PR, you glance at it, then I implement, open the PR, and hand it to you for approval. After merge I pull `main` and cut the next branch. One PR in flight at a time.
