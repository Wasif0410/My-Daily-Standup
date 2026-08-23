# PR 14 — Weekly Progress Board (expandable Monday–Sunday)

> **For agentic workers:** Execute task-by-task with `superpowers:executing-plans` or
> `superpowers:subagent-driven-development`. Strict TDD: write the named tests, confirm
> they fail, implement, confirm they pass, commit.

**Goal:** The whole week at a glance — seven collapsible days, each summarising how much
got done and how long it took, opening to reveal the tasks inside.

**Architecture:** Rust gains the week's seven days and its ISO label, a query for tasks
scheduled inside a date range, and a small key-value store for presentation state. The
frontend adds a `scheduled` store filter, a pure day-summary module, `DaySection` /
`DaySummary`, and — for the DoD's "updates live" requirement — a Tauri event the store
emits after every successful mutation, which every board listens for. Boards are separate
windows with separate stores, so nothing else can keep them in step.

**Tech Stack:** Rust · chrono · rusqlite · Tauri 2 events · React 19 · TypeScript 5.9.3 ·
zustand · Vitest + Testing Library

## Global Constraints

Inherited verbatim from `docs/superpowers/plans/2026-08-20-pr-sequence.md`.

- **Windows-first.** No platform-specific dependency without a `cfg` guard.
- **Local-first.** No feature may hard-depend on a network call.
- **No LLM.** Normal task operations must not start an inference process.
- **Rust owns state and dates.** The frontend never computes a week boundary or a "today".
- **TypeScript pinned at 5.9.3.**
- **Tokens, never literals.** No hard-coded colour or size in a component or `theme.css`.
- **Hover-revealed controls stay keyboard-reachable.** `opacity`, never `display: none`.
- **`null` duration means unmeasured, never zero.**
- **The full local gate before pushing:** `npm run lint`, `npm run format:check`,
  `npx tsc --noEmit`, `npx vitest run`, `cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`, `cargo test`.

---

## Design decisions settled here

| Decision | Why |
|---|---|
| **`Week` gains `label` and `days`; one command still answers "what week is it"** | The board needs the seven dates, their names, and "2026-W34" — all derived from the same boundary. Three round trips for one question, or a second type that could disagree with the first, are both worse. **Ride-along:** the Weekly Tasks board's subtitle switches from `start → end` to the label, matching the spec's mockups for both boards. |
| **Days bucket by `scheduled_date`** | The board answers "how did the week go, day by day". That axis is the day a task was scheduled for, not the period it belongs to. |
| **Bucketing is string equality on ISO dates, done in the frontend** | No arithmetic and no rule — Rust already supplied both the seven dates and the tasks. Sending them back to Rust to be compared would be ceremony. |
| **Day summaries are a pure frontend module** | A fold over data Rust already returned: count completed, count total, sum minutes. No date arithmetic, no domain rule, nothing the model could ever touch. Rust still owns the dates and the durations themselves. |
| **A new `ui_state` key-value table, not a column on `board_windows`** | Which days are open is presentation state, not window geometry, and there are seven of them per board. Named `ui_state` rather than `settings` so PR 16's real settings do not end up sharing a bucket with scroll positions. |
| **Today is expanded by default only until the user chooses** | Once someone has collapsed today deliberately, re-expanding it every launch is the app arguing with them. Saved state wins outright; the default applies to a board that has never been touched. |
| **Live cross-window updates via a frontend-emitted Tauri event** | Each board is its own window with its own store, so completing a task in one cannot reach another by any in-process means. The store emits `task-changed` after a *successful* mutation; every board reloads. Emitting from the frontend rather than threading an `AppHandle` through every Rust command keeps the backend untouched — the tradeoff is that a future Rust-initiated write would need to emit for itself, which is noted at the emit site. |
| **`core:event:allow-emit` and `allow-listen` added explicitly to capabilities** | They may already come with `core:default`; the capability file's own rule is that every permission is justified by a feature, and a test cannot catch a missing one. This is the PR 10 failure mode exactly, so it is verified in the dev log rather than assumed. |
| **Empty days render at full height, not collapsed away** | A week with three empty days *is* the information. Hiding them turns the board into a list of busy days and loses the shape. |

---

## File structure

| File | Responsibility |
|---|---|
| `src-tauri/src/domain/week.rs` (+`_tests`) | Gains `days` and the ISO `label` |
| `src-tauri/src/storage/task_repo.rs` (+`_tests`) | `list_scheduled_between` |
| `src-tauri/migrations/004_ui_state.sql` | The key-value table |
| `src-tauri/src/storage/ui_state.rs` (+`_tests`) | Reading and writing it |
| `src-tauri/src/commands/{mod,tasks,ui}.rs`, `lib.rs` | `task_list_scheduled_between`, `ui_state_get`, `ui_state_set` |
| `src-tauri/capabilities/default.json` | Event permissions |
| `src/lib/ipc.ts`, `src/types/task.ts` | Wrappers, `WeekDay` |
| `src/lib/taskEvents.ts` | Emit and subscribe to `task-changed` |
| `src/stores/taskStore.ts` | `scheduled` filter; emits after each mutation |
| `src/features/boards/daySummary.ts` (+test) | Bucketing and summarising |
| `src/features/boards/components/DaySummary.tsx` (+test) | `2/3   1h 45m` |
| `src/features/boards/components/DaySection.tsx` (+test) | One collapsible day |
| `src/features/boards/WeeklyProgressBoard.tsx` (+test) | The board |
| `src/features/boards/WeeklyBoard.tsx` | Subtitle switches to the label |
| `src/features/boards/BoardRoot.tsx` | Routes `weekly-progress` |

---

### Task 1: Rust — the week's seven days and its ISO label

**Files:** `src-tauri/src/domain/week.rs`, `week_tests.rs`.

**Interfaces:**
- `pub struct WeekDay { pub date: String, pub name: String }` — `"2026-08-17"`, `"Monday"`.
- `Week` gains `pub label: String` (ISO, e.g. `"2026-W34"`) and `pub days: Vec<WeekDay>`.

- [ ] **Step 1: Write the failing tests:**
  - `a_week_lists_seven_days_monday_first`
  - `a_week_lists_seven_days_sunday_first_when_it_starts_on_sunday` — the day *names*
    reorder too, not just the dates
  - `week_days_are_consecutive_dates`
  - `week_days_cross_a_month_boundary` — the week of 2026-09-01 runs 08-31 → 09-06
  - `a_week_is_labelled_with_its_iso_week`
  - `the_iso_label_uses_the_iso_week_not_the_configured_start` — a Sunday-start week is
    still labelled by the ISO week its Thursday falls in, because "2026-W34" means one
    fixed thing and inventing a private numbering would make it a lie
- [ ] **Step 2: Run to verify they fail.**
- [ ] **Step 3: Implement**, using `iso_week()` for the label and `succ_opt()` for the days.
- [ ] **Step 4: Run to verify they pass.**
- [ ] **Step 5: Full Rust gate.**
- [ ] **Step 6: Commit** — `feat: expand a week into its seven days`

---

### Task 2: Rust — tasks scheduled inside a date range

**Files:** `src-tauri/src/storage/task_repo.rs`, `task_repo_tests.rs`, `commands/*`, `lib.rs`.

**Interfaces:**
- `TaskRepo::list_scheduled_between(conn, start, end) -> Result<Vec<Task>, StorageError>`
- `AppState::list_scheduled_between`, command `task_list_scheduled_between { start, end }`

```sql
SELECT {COLUMNS} FROM tasks
WHERE scheduled_date IS NOT NULL AND scheduled_date BETWEEN ?1 AND ?2
ORDER BY scheduled_date, priority DESC NULLS LAST, created_at
```

- [ ] **Step 1: Write the failing tests:**
  - `list_scheduled_between_returns_tasks_inside_the_range`
  - `list_scheduled_between_includes_both_boundary_days` — inclusive; excluding Sunday
    would silently drop a seventh of every week
  - `list_scheduled_between_excludes_unscheduled_tasks` — no date means no day to sit in
  - `list_scheduled_between_ignores_the_period_columns` — a task whose *period* overlaps
    but whose `scheduled_date` falls outside must not appear
  - `list_scheduled_between_orders_by_date_then_priority`
- [ ] **Step 2: Run to verify they fail.**
- [ ] **Step 3: Implement** the query, the `AppState` method, the command, the handler.
- [ ] **Step 4: Run to verify they pass.**
- [ ] **Step 5: Full Rust gate.**
- [ ] **Step 6: Commit** — `feat: query tasks scheduled in a date range`

---

### Task 3: Rust — a key-value store for presentation state

**Files:** create `src-tauri/migrations/004_ui_state.sql`, `src-tauri/src/storage/ui_state.rs`
and `ui_state_tests.rs`; register in `storage/mod.rs` and `migrations.rs`; add commands.

```sql
CREATE TABLE ui_state (
    key   TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL
);
```

**Interfaces:**
- `UiStateRepo::get(conn, key) -> Result<Option<String>, StorageError>`
- `UiStateRepo::set(conn, key, value) -> Result<(), StorageError>`
- Commands `ui_state_get { key } -> Option<String>`, `ui_state_set { key, value }`

- [ ] **Step 1: Write the failing tests:**
  - `a_missing_key_reads_as_none` — absence is not an error; a board opening for the
    first time is the normal case
  - `a_value_round_trips`
  - `setting_an_existing_key_replaces_it` — upsert, not a second row
  - `an_empty_value_is_stored_rather_than_treated_as_absent` — "nothing expanded" is a
    real choice and must survive a restart, distinct from "never chosen"
  - `migration_004_brings_the_schema_to_version_four`
- [ ] **Step 2: Run to verify they fail.**
- [ ] **Step 3: Implement** the migration, the repo, `LATEST_VERSION = 4`, the commands.
- [ ] **Step 4: Run to verify they pass.**
- [ ] **Step 5: Full Rust gate.**
- [ ] **Step 6: Commit** — `feat: persist presentation state`

---

### Task 4: Frontend — the scheduled filter, ui state, and cross-window events

**Files:** `src/lib/ipc.ts`, `src/types/task.ts`, create `src/lib/taskEvents.ts` (+test),
`src/stores/taskStore.ts` (+test), `src-tauri/capabilities/default.json`.

**Interfaces:**

```typescript
// types/task.ts
export interface WeekDay { date: string; name: string }
export interface Week { start: string; end: string; label: string; days: WeekDay[] }

// ipc.ts
export function listTasksScheduledBetween(start: string, end: string): Promise<Task[]>;
export function getUiState(key: string): Promise<string | null>;
export function setUiState(key: string, value: string): Promise<void>;

// taskEvents.ts
export const TASK_CHANGED = "task-changed";
export function emitTaskChanged(): void;
export function onTaskChanged(handler: () => void): Promise<() => void>;

// taskStore.ts
type TaskFilter = … | { kind: "scheduled"; start: string; end: string };
```

Every store mutation that succeeds calls `emitTaskChanged()`. Failures do not: a rolled
back optimistic update has changed nothing, and telling other windows otherwise would make
them reload for no reason.

- [ ] **Step 1: Write the failing tests:**
  - `taskEvents`: `emitting is fire-and-forget so a listener failure cannot break a
    mutation`; `subscribing returns an unsubscribe`
  - store: `loads through the scheduled command`;
    `announces a change after a successful mutation`;
    `stays silent when a mutation fails` — nothing changed, so nothing to announce;
    `announces once per mutation, not once per optimistic step`
- [ ] **Step 2: Run to verify they fail.**
- [ ] **Step 3: Implement**, and add `core:event:allow-emit` and `core:event:allow-listen`
      to `capabilities/default.json`.
- [ ] **Step 4: Run to verify they pass.**
- [ ] **Step 5: Commit** — `feat: keep board windows in step`

---

### Task 5: Bucketing and summarising a day

**Files:** create `src/features/boards/daySummary.ts` (+test).

**Interfaces:**

```typescript
export interface DayTotals { completed: number; total: number; minutes: number | null }
export interface Day { date: string; name: string; tasks: Task[]; totals: DayTotals }
export function bucketByDay(days: WeekDay[], tasks: Task[]): Day[];
export function summarise(tasks: Task[]): DayTotals;
```

- [ ] **Step 1: Write the failing tests:**
  - `every_day_is_returned_even_when_empty` — seven days from an empty task list. A week
    with three empty days *is* the information.
  - `buckets a task onto its scheduled day`
  - `ignores a task scheduled outside the week` — defensive; the query already excludes
    them, but a board must not silently attach a stray task to the wrong day
  - `orders tasks within a day by priority`
  - `counts completed against total`
  - `excludes cancelled work from both counts` — archived work is not a failure to finish
  - `sums recorded minutes`
  - `reports null minutes when nothing was recorded` — never `0m`
  - `sums only the recorded durations when some are missing` — a half-logged day reads as
    partial, not as fast
- [ ] **Step 2: Run to verify they fail.**
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run to verify they pass.**
- [ ] **Step 5: Commit** — `feat: bucket a week's tasks into days`

---

### Task 6: `DaySummary` and `DaySection`

**Files:** create `DaySummary.tsx` (+test) and `DaySection.tsx` (+test); append to
`theme.css`.

**Interfaces:**
- `<DaySummary totals={DayTotals} />` — renders `2/3` and `1h 45m`, or `—`.
- `<DaySection day={Day} expanded today onToggle renderTask />`

- [ ] **Step 1: Write the failing `DaySummary` tests:**
  - `shows completed against total`
  - `shows the recorded time`
  - `shows a dash when nothing was recorded`
  - `shows 0/0 for an empty day rather than nothing` — a blank row would read as broken
  - `describes itself for screen readers` — "2 of 3 done, 1h 45m"
- [ ] **Step 2: Run to verify they fail. Step 3: Implement. Step 4: Verify.**
- [ ] **Step 5: Write the failing `DaySection` tests:**
  - `names the day`
  - `summarises without being opened` — the collapsed board still answers "how did the
    week go"
  - `hides its tasks when collapsed`
  - `reveals its tasks when expanded`
  - `reports a toggle to its owner` — the board persists it, so the section cannot own it
  - `marks today so it can be distinguished`
  - `is a real disclosure` — `aria-expanded` on a button, not a clickable div
  - `renders an empty day at full height with no tasks`
- [ ] **Step 6: Run to verify they fail. Step 7: Implement. Step 8: Verify.**
- [ ] **Step 9: Style both.**
- [ ] **Step 10: Commit** — `feat: add the day section and its summary`

---

### Task 7: `WeeklyProgressBoard`

**Files:** create `WeeklyProgressBoard.tsx` (+test).

Loads the week, loads `{ kind: "scheduled", start, end }`, buckets into seven days,
restores expansion from `ui_state` (falling back to today), persists every toggle, and
subscribes to `task-changed`.

- [ ] **Step 1: Write the failing tests:**
  - `renders all seven days on an empty week` — the DoD's first line
  - `loads tasks scheduled inside the week`
  - `summarises each day from its own tasks`
  - `expands today on a board that has never been touched`
  - `restores the saved expansion instead of the default` — including a saved state that
    collapses today
  - `persists a toggle`
  - `reloads when another window announces a change` — the DoD's live-update line
  - `keeps completed work visible` — the board is a record of the week
  - `shows the ISO week`
  - `surfaces a failure rather than failing silently`
- [ ] **Step 2: Run to verify they fail. Step 3: Implement. Step 4: Verify.**
- [ ] **Step 5: Commit** — `feat: add the weekly progress board`

---

### Task 8: Wire it up and verify against the real app

**Files:** `BoardRoot.tsx` (+test), `WeeklyBoard.tsx` (subtitle → label).

- [ ] **Step 1: Write the failing tests** — `BoardRoot puts the progress board in the
      weekly-progress window`; `WeeklyBoard shows the ISO week`.
- [ ] **Step 2: Run to verify they fail. Step 3: Implement. Step 4: Verify.**
- [ ] **Step 5: The full local gate** — all seven commands.
- [ ] **Step 6: Launch and read the dev log.** The event permissions are the risk: no test
      can catch a missing one, which is precisely how PR 10 shipped broken. Verify:
  - All seven days render, empty ones included.
  - Today is expanded and visually distinct on first open.
  - Collapse a day, restart, it is still collapsed.
  - **Complete a task on the Weekly Tasks board and watch this board update without a
    reload** — and confirm the dev log shows no event-permission error.
  - No sidecar process.
- [ ] **Step 7: Commit, push, open the PR, tick PR 14 in the sequence document.**

---

## Definition of Done

From the sequence document, verbatim:

- All seven days render on an empty week.
- A day's summary matches its contents when collapsed.
- Today is expanded on first open.
- Collapse state survives a restart.
- Completing a task on the Weekly Tasks board updates this board live.

## Deliberately out of scope

- **Week navigation.** Current week only; PR 16.
- **A settings-driven week start.** The parameter exists and defaults to Monday.
- **Per-day quick add.** Not in the spec for this board — it is a record, not an inbox.
