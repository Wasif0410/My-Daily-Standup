# PR 13 — Weekly Tasks Board & Task Interactions

> **For agentic workers:** Execute task-by-task with `superpowers:executing-plans` or
> `superpowers:subagent-driven-development`. Steps use checkbox (`- [ ]`) syntax.
> Strict TDD: write the named tests, confirm they fail, implement, confirm they pass,
> commit.

**Goal:** Put the current week's commitments on screen, grouped by project, and implement
every interaction in spec §6.6 — complete, uncomplete, edit, set priority, record
duration, move to another day, promote daily→weekly, move to another week, add a blocker,
add a comment, delete, archive.

**Architecture:** Three new Rust rules — week boundaries, blocker/status coupling, and
period moves through the rollover engine — because Rust owns every date and every
load-bearing state transition. The frontend adds five store actions, a right-click
context menu, a quick-add field, and keyboard shortcuts on the row. `TaskRow` from PR 12
grows a keyboard layer and a menu hook; it is still the same primitive, still store-free.

**Tech Stack:** Rust · chrono · rusqlite · Tauri 2 · React 19 · TypeScript 5.9.3 ·
zustand · Vitest + Testing Library

## Global Constraints

Inherited verbatim from `docs/superpowers/plans/2026-08-20-pr-sequence.md`.

- **Windows-first.** No platform-specific dependency without a `cfg` guard.
- **Local-first.** No feature may hard-depend on a network call.
- **No LLM.** Normal task operations must not start an inference process.
- **Rust owns state and dates.** The frontend never computes a week boundary or a
  "today". It asks.
- **TypeScript pinned at 5.9.3.**
- **Tokens, never literals.** No hard-coded colour or size in a component or `theme.css`.
- **Hover-revealed controls stay keyboard-reachable.** `opacity`, never `display: none`.
- **`null` duration means unmeasured, never zero.**
- **The full local gate before pushing:** `npm run lint`, `npm run format:check`,
  `npx tsc --noEmit`, `npx vitest run`, `cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`, `cargo test`. PR 12 skipped
  `format:check` and failed CI on it.

---

## Design decisions settled here

| Decision | Why |
|---|---|
| **Archive is `status: 'cancelled'`, not a new column** | The row survives and leaves the board, which is what archive means as distinct from delete. The status already exists and is already `CHECK`-constrained, so no migration. |
| **Adding a blocker sets `status: 'blocked'`; clearing it returns to `'planned'`** | A `blocked` status that does not track the blocker text is decoration. Coupling them in one Rust rule means the two can never disagree. |
| **Clearing a blocker on a completed task leaves it completed** | Resolving a blocker must not un-finish work. |
| **A comment appends to `notes` with a date prefix** | A field that overwrites the previous comment is not a comment. Prefixed with the date because an undated note is useless a week later. |
| **Moving a task to a *later* week increments `rollover_count`** | Same rule as a later day, same reflection prompt (§10.3). Pushing a commitment to next week is the canonical deferral. **Flagged for Wasif** — it extends a rule he has an open question about. |
| **Week starts Monday, computed in Rust from `Local::now()`** | Spec §6.4 defaults to Monday. `Local`, not `Utc`: a planner that rolls over at 8pm because the user is in UTC-4 is wrong. PR 16 wires the start day to settings; the parameter exists now. |
| **The context menu is a real `role="menu"`** | Right-click is not the only way in. Shift+F10 and the Menu key must reach it, and a `div` full of `div`s reaches nobody. |
| **Row-level shortcuts fire only when the row itself has focus** | `e` must type an `e` inside the title editor, not re-open it. |
| **`AreaGroup` becomes `TaskGroup` with a `label` prop** | This board groups by project, PR 15 by commitment. The component was never area-specific; only its name was. One rename now beats three boards importing something called `AreaGroup` to render projects. |
| **Quick-add creates into the *current* week, not the viewed one** | The board only ever shows the current period in this PR. When PR 16 adds week navigation, quick-add takes the viewed period — noted so that change is a one-liner. |

---

## File structure

| File | Responsibility |
|---|---|
| `src-tauri/src/domain/week.rs` (+`_tests`) | `current_week`, `week_containing` — the only place a week boundary is computed |
| `src-tauri/src/domain/blocker.rs` (+`_tests`) | Blocker text and `blocked` status, kept in step |
| `src-tauri/src/domain/rollover.rs` | Gains `move_to_period` beside `reschedule` |
| `src-tauri/src/commands/{mod,tasks}.rs`, `lib.rs` | `week_current`, `task_set_blocker`, `task_add_comment`, `task_move_to_period`, `task_archive` |
| `src/lib/ipc.ts` | Typed wrappers for the five new commands |
| `src/stores/taskStore.ts` | `setBlocker`, `addComment`, `setPriority`, `moveToWeek`, `archive` |
| `src/features/boards/grouping.ts` | `groupByProject` beside `groupByArea`, sharing one `groupBy` |
| `src/features/boards/components/TaskGroup.tsx` | Renamed from `AreaGroup`, `label` prop |
| `src/features/boards/components/TaskContextMenu.tsx` | The §6.6 action list |
| `src/features/boards/components/QuickAdd.tsx` | One-line task entry |
| `src/features/boards/components/TaskRow.tsx` | Gains keyboard shortcuts and a menu trigger |
| `src/features/boards/WeeklyBoard.tsx` | The board; the only store-aware component |
| `src/features/boards/BoardRoot.tsx` | Routes `weekly-tasks` to it |

---

### Task 1: Rust — week boundaries

**Files:** create `src-tauri/src/domain/week.rs` and `week_tests.rs`; register both in
`domain/mod.rs`; add the `week_current` command.

**Interfaces:**
- `pub struct Week { pub start: String, pub end: String }` — ISO-8601 dates, serialised
  camelCase.
- `pub fn week_containing(date: NaiveDate, starts_on: Weekday) -> Week`
- `pub fn current_week(starts_on: Weekday) -> Week` — uses `chrono::Local::now()`.
- Command `week_current { starts_on: Option<String> } -> Week`, defaulting to Monday.

- [x] **Step 1: Write the failing tests:**
  - `week_containing_a_midweek_date_spans_monday_to_sunday`
  - `week_containing_monday_returns_that_monday` — a boundary date belongs to the week it
    starts, not the one before
  - `week_containing_sunday_returns_the_preceding_monday`
  - `week_containing_respects_a_sunday_start` — the setting PR 16 will drive
  - `week_containing_crosses_a_month_boundary` — 2026-09-01 is a Tuesday; its week starts
    in August. Weeks do not stop at months.
  - `week_containing_crosses_a_year_boundary`
  - `current_week_is_seven_days_long`
- [x] **Step 2: Run to verify they fail.**
- [x] **Step 3: Implement** `week.rs` using `NaiveDate::week(starts_on)`.
- [x] **Step 4: Run to verify they pass.**
- [x] **Step 5: Add the command** and register it in `generate_handler!`.
- [x] **Step 6: Full Rust gate.**
- [x] **Step 7: Commit** — `feat: compute the current week in Rust`

---

### Task 2: Rust — a blocker and the blocked status move together

**Files:** create `src-tauri/src/domain/blocker.rs` and `blocker_tests.rs`; add
`task_set_blocker` and `task_add_comment`.

**Interfaces:**
- `pub fn set_blocker(repo, conn, id, blocker: Option<&str>) -> Result<Task, StorageError>`
- `pub fn add_comment(repo, conn, id, comment: &str, today: NaiveDate) -> Result<Task, StorageError>`
- Commands `task_set_blocker { id, blocker: Option<String> }`,
  `task_add_comment { id, comment: String }`.

- [x] **Step 1: Write the failing tests:**
  - `setting_a_blocker_marks_the_task_blocked`
  - `clearing_a_blocker_returns_the_task_to_planned`
  - `clearing_a_blocker_leaves_a_completed_task_completed` — resolving a blocker must not
    un-finish work
  - `setting_a_blocker_does_not_touch_the_rollover_count` — only `reschedule` may
  - `a_blank_blocker_counts_as_clearing_it`
  - `a_comment_appends_rather_than_replacing` — two comments, both present
  - `a_comment_carries_its_date` — an undated note is useless a week later
  - `a_comment_on_an_empty_notes_field_does_not_lead_with_a_blank_line`
- [x] **Step 2: Run to verify they fail.**
- [x] **Step 3: Implement** `blocker.rs`.
- [x] **Step 4: Run to verify they pass.**
- [x] **Step 5: Add both commands** and register them.
- [x] **Step 6: Full Rust gate.**
- [x] **Step 7: Commit** — `feat: couple a blocker to the blocked status`

---

### Task 3: Rust — move a task to another week

**Files:** extend `src-tauri/src/domain/rollover.rs`; append to `rollover_tests.rs`; add
`task_move_to_period` and `task_archive`.

**Interfaces:**
- `pub fn move_to_period(repo, conn, id, start: &str, end: &str) -> Result<Task, StorageError>`
  — sets `period_start`/`period_end`, incrementing `rollover_count` only when `start`
  moves later, mirroring `reschedule` exactly.
- Commands `task_move_to_period { id, start, end }`, `task_archive { id }`.

- [x] **Step 1: Write the failing tests:**
  - `moving_to_a_later_week_counts_as_a_deferral`
  - `pulling_a_task_into_an_earlier_week_does_not_count`
  - `moving_within_the_same_week_does_not_count`
  - `giving_an_unscheduled_task_its_first_period_does_not_count`
  - `move_to_period_rejects_a_malformed_date` — before touching the database, so a bad
    value cannot half-update a task
  - `archiving_a_task_cancels_it_without_deleting_the_row`
- [x] **Step 2: Run to verify they fail.**
- [x] **Step 3: Implement** `move_to_period`, factoring the shared "is this later?" check
      out of `reschedule` rather than duplicating it.
- [x] **Step 4: Run to verify they pass.**
- [x] **Step 5: Add both commands** and register them.
- [x] **Step 6: Full Rust gate.**
- [x] **Step 7: Commit** — `feat: move a task to another week`

---

### Task 4: Frontend — IPC wrappers and store actions

**Files:** `src/lib/ipc.ts`, `src/types/task.ts` (add `Week`), `src/stores/taskStore.ts`,
`src/stores/taskStore.test.ts`.

**Interfaces produced:**

```typescript
// ipc.ts
export function currentWeek(startsOn?: string): Promise<Week>;
export function setBlocker(id: string, blocker: string | null): Promise<Task>;
export function addComment(id: string, comment: string): Promise<Task>;
export function moveTaskToPeriod(id: string, start: string, end: string): Promise<Task>;
export function archiveTask(id: string): Promise<Task>;

// taskStore.ts — all optimistic, all rolling back to the exact prior value
setBlocker: (id: string, blocker: string | null) => Promise<void>;
addComment: (id: string, comment: string) => Promise<void>;
setPriority: (id: string, priority: number | null) => Promise<void>;
moveToWeek: (id: string, start: string, end: string) => Promise<void>;
archive: (id: string) => Promise<void>;
```

- [x] **Step 1: Write the failing store tests:**
  - `setBlocker dispatches through the blocker command, not a plain update` — a plain
    update would leave the status disagreeing with the text
  - `setBlocker marks the task blocked optimistically`
  - `clearing a blocker restores the planned status optimistically`
  - `addComment dispatches through the comment command`
  - `setPriority patches the priority`
  - `setPriority clears a priority with an explicit null`
  - `moveToWeek goes through the period command, never a plain update` — only that path
    counts the deferral
  - `archive cancels the task rather than removing it from the map`
  - `a failed blocker write rolls back to the exact prior value`
- [x] **Step 2: Run to verify they fail.**
- [x] **Step 3: Implement** the wrappers and the five actions, reusing the existing
      `optimistic` helper.
- [x] **Step 4: Run to verify they pass.**
- [x] **Step 5: Commit** — `feat: add the remaining task interactions to the store`

---

### Task 5: Group by project

**Files:** `src/features/boards/grouping.ts` + test; rename
`components/AreaGroup.tsx` → `components/TaskGroup.tsx` (and its test); update
`PriorityBoard.tsx`.

**Interfaces:**
- `groupByArea(tasks)` and `groupByProject(tasks)`, both returning `TaskGrouping[]`
  (`{ label: string; tasks: Task[] }`), sharing one private `groupBy(tasks, keyOf, fallback)`.
- `UNSORTED_AREA = "Unsorted"`, `UNSORTED_PROJECT = "No project"`.
- `<TaskGroup label={string} tasks={Task[]} renderTask={...} />`.

Renaming `area` → `label` on the grouping type touches `PriorityBoard`; TypeScript will
point at every site.

- [x] **Step 1: Write the failing tests** — mirror the `groupByArea` set for
      `groupByProject`, plus `groups by project independently of area` and
      `an unassigned project gets its own heading`.
- [x] **Step 2: Run to verify they fail.**
- [x] **Step 3: Implement**, then `git mv` the component and update its import in
      `PriorityBoard.tsx`.
- [x] **Step 4: Run the whole suite** — PR 12's tests must still pass unchanged in
      behaviour.
- [x] **Step 5: Commit** — `refactor: generalise task grouping beyond areas`

---

### Task 6: `TaskContextMenu`

**Files:** create `src/features/boards/components/TaskContextMenu.tsx` + test; append to
`theme.css`.

**Interfaces:**

```typescript
interface TaskContextMenuProps {
  task: Task;
  x: number;
  y: number;
  onClose: () => void;
  onComplete: (completed: boolean) => void;
  onEdit: () => void;
  onSetPriority: (priority: number | null) => void;
  onMoveToDate: (date: string) => void;
  onPromote: () => void;
  onMoveToNextWeek: () => void;
  onSetBlocker: (blocker: string | null) => void;
  onAddComment: (comment: string) => void;
  onArchive: () => void;
  onDelete: () => void;
}
```

A `role="menu"` with `role="menuitem"` children, positioned at the click point, closing on
Escape, on outside click, and after any action fires.

- [x] **Step 1: Write the failing tests:**
  - `renders every interaction from the spec` — one assertion per §6.6 item
  - `shows "Uncomplete" for a completed task` — a menu offering "Complete" on finished
    work is lying about state
  - `closes after an action fires`
  - `closes on Escape`
  - `closes on an outside click`
  - `is reachable by keyboard` — arrow keys move between items
  - `dispatches each action to its handler` — one test per handler
  - `offers "Promote to weekly" only for a daily task`
- [x] **Step 2: Run to verify they fail.**
- [x] **Step 3: Implement.**
- [x] **Step 4: Run to verify they pass.**
- [x] **Step 5: Style** the menu — `--surface-raised`, `--border-strong`, tokens only.
- [x] **Step 6: Commit** — `feat: add the task context menu`

---

### Task 7: Keyboard shortcuts on the row, and the menu trigger

**Files:** `src/features/boards/components/TaskRow.tsx` + test.

`TaskRow` gains `tabIndex={0}`, an `onContextMenu` handler, and a key handler that fires
**only when the row itself is the event target** — otherwise `e` would re-open the editor
instead of typing an `e`.

| Key | Action |
|---|---|
| `Enter` / `Space` | Toggle complete |
| `e` | Edit title |
| `Delete` | Delete |
| `Shift+F10` or `ContextMenu` | Open the menu |

- [x] **Step 1: Write the failing tests:**
  - `toggles completion on Enter when the row is focused`
  - `toggles completion on Space`
  - `opens the title editor on e`
  - `deletes on Delete`
  - `opens the menu on Shift+F10`
  - `opens the menu on right-click`
  - `ignores shortcuts typed inside the title editor` — the one that matters: type `e`
    while editing and assert the text contains it and no second editor opened
  - `does not delete when Delete is pressed inside the editor`
- [x] **Step 2: Run to verify they fail.**
- [x] **Step 3: Implement.**
- [x] **Step 4: Run to verify they pass** — PR 12's 19 row tests must still pass.
- [x] **Step 5: Commit** — `feat: drive a task row from the keyboard`

---

### Task 8: `QuickAdd`

**Files:** create `src/features/boards/components/QuickAdd.tsx` + test; append to
`theme.css`.

**Interfaces:** `<QuickAdd onAdd={(title: string) => void} placeholder={string | undefined} />`.
One input; Enter submits and clears; Escape clears and blurs; whitespace-only is refused.

- [x] **Step 1: Write the failing tests:**
  - `adds a task on Enter`
  - `clears the field after adding, ready for the next one`
  - `refuses a whitespace-only title`
  - `trims surrounding whitespace`
  - `clears without adding on Escape`
  - `does not submit an empty field`
- [x] **Step 2: Run to verify they fail.**
- [x] **Step 3: Implement.**
- [x] **Step 4: Run to verify they pass.**
- [x] **Step 5: Style.**
- [x] **Step 6: Commit** — `feat: add tasks from the board`

---

### Task 9: `WeeklyBoard`, wiring, and verification

**Files:** create `src/features/boards/WeeklyBoard.tsx` + test; modify `BoardRoot.tsx` and
its test.

Asks Rust for the current week, loads `{ kind: "period", start, end }`, groups by project,
renders `QuickAdd` plus a `TaskGroup` per project, and owns the context-menu state (which
row, at which point). The week appears as the board subtitle.

- [x] **Step 1: Write the failing tests:**
  - `asks Rust for the current week rather than computing one`
  - `loads the period the week covers`
  - `groups tasks by project`
  - `says so when the week is empty`
  - `adds a task into the current week` — asserts `periodStart`/`periodEnd` and
    `horizon: "weekly"` on the created task
  - `opens the context menu on right-click and dispatches an action through the store`
  - `moving to next week goes through the period command`
  - `adding a blocker goes through the blocker command`
  - `surfaces a failure rather than failing silently`
  - `BoardRoot puts the weekly board in the weekly-tasks window`
- [x] **Step 2: Run to verify they fail.**
- [x] **Step 3: Implement.**
- [x] **Step 4: Run to verify they pass.**
- [x] **Step 5: The full local gate** — all seven commands from Global Constraints.
- [ ] **Step 6: Launch the app and read the dev log.** Non-negotiable; PR 10's lesson.

      **Done so far:** the app was launched, built clean, and the dev log carries no
      permission error. No sidecar process appeared. Six `[seed]`-prefixed weekly tasks
      were written into the current week (2026-08-17 → 2026-08-23) across three projects
      plus one unfiled, and the board's query was run against the real database — six
      rows, all at `rollover_count = 0`, which is the baseline for the check below.

      **Still needs a human at the keyboard:**
  - Every §6.6 interaction works and persists across a restart.
  - **Moving a task to another day increments `rollover_count` exactly once.** Baseline is
    0 for every seeded row. Right-click → *Move to another day* → a date **later** than
    2026-08-21, then re-run the verification script: the moved row should read 1, every
    other row still 0. Moving it *earlier* must leave it at 1, not 2.
  - Right-click and Shift+F10 both open the menu; arrow keys move through it.
  - Archive removes a task from the board but leaves the row in the database
    (`SELECT status FROM tasks WHERE title LIKE '%Choose hotel%'` → `cancelled`).
  - Adding a blocker flips the status to `blocked` and shows the text; resolving it
    returns the task to `planned`.

      Remove the seeded rows afterwards with
      `DELETE FROM tasks WHERE title LIKE '[seed]%'`.
- [x] **Step 7: Commit, push, open the PR, tick PR 13 in the sequence document.**

---

## Definition of Done

From the sequence document, verbatim:

- Every interaction in §6.6 works and persists.
- Moving a task to another day increments `rollover_count` exactly once (verify in DB).

Test coverage it asks for: Vitest for the menu and each action's dispatch; a Rust test
asserting the rollover increment via the real reschedule path (already exists from PR 7 —
Task 3 extends it to period moves).

## Deliberately out of scope

- **Linking to an Obsidian note and opening it.** §6.6 lists both; the vault reader is
  Wave 4 (PR 19-22). There is nothing to link to yet.
- **Week navigation.** The board shows the current week only. PR 16.
- **A settings-driven week start.** The parameter exists and defaults to Monday; PR 16
  supplies the value.
