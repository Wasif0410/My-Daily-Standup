# PR 12 — Priority Tasks Board & the Shared Task Row

> **For agentic workers:** Execute task-by-task with `superpowers:executing-plans` or
> `superpowers:subagent-driven-development`. Steps use checkbox (`- [ ]`) syntax.
> Every task is strict TDD: write the named tests, confirm they fail, implement,
> confirm they pass, commit.

**Goal:** Ship the first working board — Priority Tasks, grouped by area — and with it
`TaskRow`, the row primitive that PRs 13, 14, and 15 all reuse.

**Architecture:** A new Rust query returns non-daily tasks at or above a priority
threshold, so the filter runs in SQL rather than being reassembled in the browser from
three horizon fetches. The frontend gains a `priority` filter on the existing
`taskStore`, three presentational components (`PriorityBadge`, `DurationField`,
`AreaGroup`), and `TaskRow`, which composes them and takes every interaction as a
callback. `PriorityBoard` is the only component that touches the store; everything below
it is pure props, which is what makes the row reusable by boards with entirely different
data sources.

**Tech Stack:** Rust · rusqlite · Tauri 2 commands · React 19 · TypeScript 5.9.3 ·
zustand · Vitest + Testing Library

## Global Constraints

Inherited verbatim from `docs/superpowers/plans/2026-08-20-pr-sequence.md`. A task that
violates any line here is rejected regardless of whether its tests pass.

- **Windows-first.** No platform-specific dependency without a `cfg` guard.
- **Local-first.** No feature may hard-depend on a network call.
- **No LLM.** Normal task operations must not start an inference process. This PR adds
  none, and the DoD requires confirming none exists.
- **Rust owns state.** Dates, filtering, and persistence happen in Rust. The frontend
  renders what it is given.
- **TypeScript pinned at 5.9.3.** Do not bump it; no `typescript-eslint` release
  supports TS 7.
- **Tokens, never literals.** Components reference `tokens.css` custom properties. No
  hard-coded colour or size in a component or in `theme.css`.
- **Colour is never the only signal.** Priority renders its number alongside its hue —
  colour-blind users and low-opacity boards both defeat hue.
- **Hover-revealed controls stay keyboard-reachable.** Hide with `opacity`, never
  `display: none`.
- **`null` duration means unmeasured, never zero.**
- **CI green before requesting a merge:** `npm run lint`, `npm run test`,
  `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`.

---

## Design decisions settled here

| Decision | Why |
|---|---|
| **A dedicated `task_list_priority` SQL query** | The board needs `horizon != 'daily' AND priority >= threshold`. Assembling that from three `list_by_horizon` calls would be three round-trips into a store whose `load` replaces its whole map. |
| **`priority IS NULL` is excluded** | Unprioritised is not "priority 0". A task nobody ranked has not been judged unimportant, and treating it as 0 would hide it from the board whose job is showing what matters. |
| **Default threshold 5** | Priority runs 0–10, so 5 is the midpoint: "above average importance". A prop, not a hard-code, so PR 16 can drive it from settings. |
| **Groups ordered by their highest-priority task** | This is the *Priority* board. Alphabetical ordering would file the urgent area under whatever letter it starts with. |
| **Null `area` renders as "Unsorted", ordered by the same rule** | Pinning it to the bottom for tidiness would bury an unfiled P10. |
| **A blank `area` (`""` or whitespace) counts as no area** | Obsidian frontmatter round-trips an unset value as an empty string, and a heading with no text is not a heading. |
| **Completed tasks stay visible, dimmed and struck through** | Spec §6.4: the board is a record, not a list that empties. |
| **`onMove` and `onDelete` are optional on `TaskRow`** | PR 12 has no move UI — that is PR 13's context menu. A button that opened nothing would be worse than no button. Same pattern as `BoardShell`'s `onClose`. |
| **Inline edit commits on blur, cancels only on Escape** | Clicking away from a rename you just typed and watching it vanish is the more damaging default. |
| **An empty title is refused, not committed** | An untitled row has nothing left to double-click, so it could never be renamed again. |
| **Only `PriorityBoard` touches the store** | `TaskRow` is reused by three later boards with different data sources. A row that reached into `useTaskStore` could serve only one of them. |

---

## File structure

| File | Responsibility |
|---|---|
| `src-tauri/src/storage/task_repo.rs` | `list_by_priority` — the SQL |
| `src-tauri/src/storage/task_repo_tests.rs` | Its tests |
| `src-tauri/src/commands/{mod,tasks}.rs`, `lib.rs` | `AppState::list_by_priority`, the `task_list_priority` command, handler registration |
| `src/lib/ipc.ts` | `listPriorityTasks(threshold)` |
| `src/stores/taskStore.ts` | The `{ kind: "priority"; threshold }` filter |
| `src/features/boards/components/PriorityBadge.tsx` | The `P8` badge, colour-graded |
| `src/features/boards/components/DurationField.tsx` | Read and write a recorded duration |
| `src/features/boards/components/TaskRow.tsx` | The shared row — composes the above, owns only the in-progress title edit |
| `src/features/boards/grouping.ts` | `groupByArea` — pure, so it can be memoised |
| `src/features/boards/components/AreaGroup.tsx` | One heading plus a list, rendered through a callback |
| `src/features/boards/PriorityBoard.tsx` | The only store-aware component |
| `src/features/boards/BoardRoot.tsx` | Routes a board kind to its content |
| `src/styles/theme.css` | Row, badge, and duration chrome — tokens only |

---

### Task 1: Rust — query tasks above a priority threshold

**Files:**
- Modify: `src-tauri/src/storage/task_repo.rs` (add `list_by_priority` after `list_for_period`)
- Modify: `src-tauri/src/storage/task_repo_tests.rs`
- Modify: `src-tauri/src/commands/mod.rs`, `src-tauri/src/commands/tasks.rs`, `src-tauri/src/lib.rs`

**Interfaces:**
- Consumes: `TaskRepo::query`, `COLUMNS`, `StorageError` — existing in `task_repo.rs`.
- Produces:
  - `TaskRepo::list_by_priority(&self, conn: &Connection, threshold: i64) -> Result<Vec<Task>, StorageError>`
  - `AppState::list_by_priority(&self, threshold: i64) -> Result<Vec<Task>, CommandError>`
  - Tauri command `task_list_priority { threshold: i64 } -> Vec<Task>`

The SQL is exactly:

```sql
SELECT {COLUMNS} FROM tasks
WHERE horizon != 'daily'
  AND priority IS NOT NULL AND priority >= ?1
ORDER BY priority DESC, created_at
```

- [x] **Step 1: Write the failing tests** in `task_repo_tests.rs`, with a
      `priority_task(title, area, priority)` helper building a `Weekly`/`Manual` task:
  - `list_by_priority_returns_tasks_at_or_above_the_threshold`
  - `list_by_priority_includes_a_task_exactly_at_the_threshold` — `>=`, not `>`. Off by
    one here silently hides a whole band from the only board that shows it.
  - `list_by_priority_excludes_daily_tasks` — daily work belongs to Weekly Progress.
  - `list_by_priority_excludes_unprioritised_tasks` — pins the NULL behaviour so a later
    `COALESCE` cannot break it silently.
  - `list_by_priority_orders_by_priority_then_age`
  - `list_by_priority_keeps_weekly_monthly_and_long_term`
- [x] **Step 2: Run to verify they fail** — `cargo test --manifest-path src-tauri/Cargo.toml list_by_priority`.
      Expected: `no method named list_by_priority`.
- [x] **Step 3: Implement** the query, the `AppState` method, the command, and the
      `generate_handler!` entry.
- [x] **Step 4: Run to verify they pass** — 6 passed.
- [x] **Step 5: Full Rust gate** — `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`.
- [x] **Step 6: Commit** — `feat: query tasks above a priority threshold`

---

### Task 2: IPC wrapper and a priority filter on the store

**Files:** `src/lib/ipc.ts`, `src/stores/taskStore.ts`, `src/stores/taskStore.test.ts`

**Interfaces:**
- Produces: `listPriorityTasks(threshold: number): Promise<Task[]>` calling
  `task_list_priority`; the filter variant `{ kind: "priority"; threshold: number }`.

`fetchFor`'s switch has no `default`, so TypeScript's exhaustiveness check is what forces
the new branch to exist — omitting it is a compile error, not a silent `undefined`.

- [x] **Step 1: Write the failing tests** in `taskStore.test.ts`:
  - `loads through the priority command` — asserts `invoke("task_list_priority", { threshold: 5 })`
  - `passes the threshold through rather than hard-coding one`
- [x] **Step 2: Run to verify they fail** — TS rejects `kind: "priority"`.
- [x] **Step 3: Implement** the wrapper and the filter branch.
- [x] **Step 4: Run to verify they pass.**
- [x] **Step 5: Commit** — `feat: load tasks by priority threshold`

---

### Task 3: `PriorityBadge`

**Files:** create `src/features/boards/components/PriorityBadge.tsx` and its test;
append to `src/styles/theme.css`.

**Interfaces:** `<PriorityBadge priority={number | null} />`. Renders `P8`, or `—` when
null. Sets `data-tier` to `high` (≥8), `medium` (≥4), `low` (<4), `none` (null), which
is what the CSS grades colour from.

- [x] **Step 1: Write the failing tests:**
  - `renders the priority as a number, not only a colour`
  - `renders a dash when nothing is prioritised`
  - `names the priority for screen readers` — `aria-label="Priority 8"`
  - `names an absent priority as unset rather than as zero` — `aria-label="No priority"`
  - `grades the tier so colour can reinforce the number` — all four tiers
- [x] **Step 2: Run to verify they fail** — module not found.
- [x] **Step 3: Implement** the component.
- [x] **Step 4: Run to verify they pass** — 5 passed.
- [x] **Step 5: Style** `.priority-badge` and its `[data-tier]` variants from
      `--priority-high` / `--priority-medium` / `--priority-low` / `--text-dim`,
      monospaced and `font-variant-numeric: tabular-nums` so the column does not jitter.
- [x] **Step 6: Commit** — `feat: add the priority badge`

---

### Task 4: `DurationField`

**Files:** create `src/features/boards/components/DurationField.tsx` and its test;
append to `src/styles/theme.css`.

**Interfaces:** `<DurationField minutes={number | null} onChange={(minutes: number | null) => void} />`.
Consumes `formatMinutes`, `parseDuration`, `DURATION_PRESETS` from `@/lib/duration`
(shipped in PR 11).

A closed button showing the formatted value; clicking opens a popover with the four
presets, a free-text input, and — only when a value is set — a Clear button. Unparseable
input is rejected with a `role="alert"` and the popover stays open with the text intact:
the user is one character from a valid entry. Recording a number they never typed is
worse than recording nothing, because nothing is a state the app represents honestly.

- [x] **Step 1: Write the failing tests:**
  - `shows a recorded duration` (95 → `1h 35m`)
  - `shows a dash when nothing was recorded` — never `0m`
  - `stays closed until asked`
  - `records a preset in one click` — logging must be one gesture or it stops happening
  - `closes after a preset is chosen`
  - `records free text on Enter` (`1h30m` → 90)
  - `reads a bare number as minutes` (`45` → 45; reading it as hours would inflate every
    total sixtyfold)
  - `refuses input it cannot parse instead of guessing` — no dispatch, alert shown,
    input still present
  - `clears a recorded duration back to unrecorded` — `onChange(null)`, not 0
  - `offers no clear button when there is nothing to clear`
  - `abandons the edit on Escape`
- [x] **Step 2: Run to verify they fail** — module not found.
- [x] **Step 3: Implement** the component.
- [x] **Step 4: Run to verify they pass** — 11 passed.
- [x] **Step 5: Style** the popover. Anchored above the row in the stack and offset from
      the button; a board is short, and a popover opening into `.board-body`'s overflow
      would be clipped.
- [x] **Step 6: Commit** — `feat: set a task's duration from the board`

---

### Task 5: `TaskRow`

**Files:** create `src/features/boards/components/TaskRow.tsx` and its test; append to
`src/styles/theme.css`.

**Interfaces:**

```typescript
interface TaskRowProps {
  task: Task;
  onComplete: (completed: boolean) => void;
  onEdit: (title: string) => void;
  onSetTimeSpent: (minutes: number | null) => void;
  /** Omitted until PR 13 builds the move UI. */
  onMove?: (() => void) | undefined;
  onDelete?: (() => void) | undefined;
}
```

Anatomy, left to right: `PriorityBadge` · checkbox (`aria-label` is the task title, so
tests and screen readers both address it by name) · title (or, while editing, an input
labelled `Edit title`) · `DurationField` · hover actions.

The only state it owns is the uncommitted title draft, which belongs here: it is scratch
text with no meaning outside this row.

- [x] **Step 1: Write the failing tests:**
  - `shows priority, title, and duration together`
  - `shows a dash for an unmeasured task`
  - completion: `reports a completion to its owner`, `reports an uncompletion`,
    `checks the box for a completed task`, `keeps a completed task visible but marks it
    done` (`data-completed="true"`)
  - inline edit: `opens an editor on double-click`, `does not open on a single click`,
    `commits on Enter`, `abandons the edit on Escape`, `commits on blur, so clicking away
    does not lose the edit`, `refuses to commit an empty title`, `does not dispatch when
    the title is unchanged`
  - duration: `reports a chosen preset to its owner`, `reports free text to its owner`
  - hover actions: `reports a delete to its owner`, `reports a move to its owner`,
    `omits an action with no handler rather than rendering a dead button`,
    `keeps hover-revealed actions reachable by keyboard`
- [x] **Step 2: Run to verify they fail** — module not found.
- [x] **Step 3: Implement** the component.
- [x] **Step 4: Run to verify they pass** — 18 passed.
- [x] **Step 5: Style** the row. Two things are load-bearing:
  - `.task-title { flex: 1; min-width: 0; }` — without `min-width: 0` a flex item refuses
    to shrink below its content and the ellipsis never appears.
  - A board is 340px by default, so degradation order is fixed: at container width
    ≤260px the duration hides, and only below that does the title truncate. "How long did
    it take" is the question you ask of a board you have widened; the title is the one you
    cannot lose. Requires `container-type: inline-size; container-name: board;` on
    `.board-body`.
- [x] **Step 6: Commit** — `feat: add the shared task row`

---

### Task 6: Grouping by area, and `AreaGroup`

**Files:** create `src/features/boards/grouping.ts` + test and
`src/features/boards/components/AreaGroup.tsx` + test.

**Interfaces:**
- `UNSORTED_AREA = "Unsorted"`
- `interface AreaGrouping { area: string; tasks: Task[] }`
- `groupByArea(tasks: Task[]): AreaGrouping[]` — groups ordered by their highest-priority
  task descending, ties alphabetical; tasks within a group ordered by the store's existing
  `sortTasks`. Pure and standalone so components can memoise it: called inline in a
  zustand selector it would return a fresh array every render and loop.
- `<AreaGroup area={string} tasks={Task[]} renderTask={(task: Task) => ReactNode} />` —
  takes a render callback rather than wiring `TaskRow` itself, because PR 13 groups by
  project and PR 14 by day, each needing different handlers on the row.

- [x] **Step 1: Write the failing `groupByArea` tests:**
  - `returns nothing for no tasks`
  - `collects tasks under their area`
  - `orders groups by their most important task`
  - `breaks a tie between areas alphabetically`
  - `sorts within a group by priority, then oldest first`
  - `gathers tasks with no area under one heading`
  - `treats a blank area as no area rather than as its own group`
- [x] **Step 2: Run to verify they fail.**
- [x] **Step 3: Implement** `grouping.ts`.
- [x] **Step 4: Run to verify they pass** — 7 passed.
- [x] **Step 5: Write the failing `AreaGroup` tests:**
  - `heads the group with its area`
  - `renders each task through the callback it is given`
  - `labels the list with its area for screen readers` — four unlabelled lists on one
    board are indistinguishable to anyone navigating by landmark
- [x] **Step 6: Run to verify they fail.**
- [x] **Step 7: Implement** `AreaGroup` (reuses the existing `.board-section` and
      `.board-section-title` chrome; adds `.task-list-plain`, distinct from the dev
      shell's `.task-list` that `global.css` still owns).
- [x] **Step 8: Run to verify they pass** — 3 passed.
- [x] **Step 9: Commit** — `feat: group board tasks by area`

---

### Task 7: `PriorityBoard`

**Files:** create `src/features/boards/PriorityBoard.tsx` and its test.

**Interfaces:** `DEFAULT_PRIORITY_THRESHOLD = 5`; `<PriorityBoard threshold={number | undefined} />`.

Loads `{ kind: "priority", threshold }` on mount, memoises `groupByArea(sortTasks(...))`,
and hands each row the store actions. `onMove` is deliberately not passed.

- [x] **Step 1: Write the failing tests:**
  - `loads through the priority filter at the default threshold`
  - `honours a threshold it is given`
  - `groups tasks under their areas`
  - `says so when there is nothing above the threshold` — the empty state names the
    threshold, so an empty board explains itself rather than looking broken
  - `completes a task through the store`
  - `records a duration through the store`
  - `commits an inline edit through the store`
  - `deletes a task through the store`
  - `surfaces a failure rather than failing silently`
- [x] **Step 2: Run to verify they fail** — module not found.
- [x] **Step 3: Implement** the component.
- [x] **Step 4: Run to verify they pass** — 9 passed.
- [x] **Step 5: Commit** — `feat: add the priority tasks board`

---

### Task 8: Mount the board in its window, and verify against the real app

**Files:** modify `src/features/boards/BoardRoot.tsx`; create its test.

`BoardRoot` gains a `boardContent(kind)` switch — a switch rather than a lookup table, so
TypeScript flags a board that PRs 13–15 forget to fill in. The remaining three keep the
`No tasks yet.` placeholder; a blank window reads as broken.

- [x] **Step 1: Write the failing tests** (mocking `@tauri-apps/api/window`):
  - `puts the priority board inside the priority window`
  - `still shows a placeholder for boards that have no content yet`
- [x] **Step 2: Run to verify they fail.**
- [x] **Step 3: Implement** the switch.
- [x] **Step 4: Run to verify they pass** — 2 passed.
- [x] **Step 5: Full gate.** `npm run lint && npm run test -- --run`, then
      `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`.
      Do not proceed on a failure.
- [ ] **Step 6: Launch the real app and read the dev log.** Non-negotiable, and the
      reason is in the checkpoint: PR 10 shipped with collapse silently broken because
      `src/test/setup.ts` mocks Tauri, so no test can catch a missing capability. All 55
      tests passed and CI was green.

      **Done so far:** the app was launched twice, built clean, and the dev log carries no
      permission error. No `llama`, `whisper`, or other sidecar process appeared. Seven
      `[seed]`-prefixed tasks were written to
      `%APPDATA%\com.wasif.dailystandup\standup.db`, and the command's exact SQL was run
      against that real database — it returns the five expected rows and correctly
      excludes both negative controls (a P2 task and a P10 *daily* task).

      **Still needs a human at the keyboard:**
  - The Priority window shows those tasks, grouped by area, ordered Job search → Unsorted
    → Health.
  - Ticking a checkbox persists across a restart.
  - A duration set from a preset and from free text both persist across a restart.
  - Double-click edits a title; Enter commits, Escape reverts.
  - Hover reveals the delete button; it is hidden otherwise.
  - Narrowing the window collapses the duration before the title truncates.

      Remove the seeded rows afterwards with
      `DELETE FROM tasks WHERE title LIKE '[seed]%'`.
- [x] **Step 7: Commit** — `feat: show the priority board in its window`
- [x] **Step 8: Push, open the PR, tick PR 12 in the sequence document, wait for green CI.**

---

## Definition of Done

From the sequence document, verbatim:

- Board shows real tasks from SQLite.
- Completing one persists and survives restart.
- A duration can be set from the row and is visible afterwards.
- **Confirm no inference process exists** — Task Manager shows no sidecar.

And the coverage it asks for — grouping, empty state, inline edit commit/cancel, checkbox
dispatch, priority rendered as a number not only a colour, duration formatted for none /
minutes / hours, presets and free text both dispatch — each has a named test above.

## Deliberately out of scope

- **The move UI.** PR 13's context menu. `TaskRow` takes `onMove` and renders the button
  only when handed one.
- **Setting a priority from the row.** Spec §6.6 lists it; PR 13 implements it alongside
  the rest of the interaction set.
- **Blockers and comments.** PR 13.
- **A settings-driven threshold.** PR 16. The prop exists so that PR is a one-line change
  here.
