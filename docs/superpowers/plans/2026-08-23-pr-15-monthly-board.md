# PR 15 — Monthly Progress Board

> **For agentic workers:** Execute task-by-task with `superpowers:executing-plans` or
> `superpowers:subagent-driven-development`. Strict TDD: write the named tests, confirm
> they fail, implement, confirm they pass, commit.

**Goal:** The month as outcomes rather than tasks — each commitment with its numeric
progress and a bar, rolled up from the work underneath it.

**Architecture:** Rust computes every number. A new domain function rolls a commitment's
progress up through one level of grandchildren, so a completed *daily* task moves the
*monthly* figure; the command returns the task, its progress, and a pre-clamped fraction,
leaving the component with nothing to divide. The board is read-only — spec §6.5 is a
report, not an inbox — so it fetches straight through IPC into local state and reloads on
the `task-changed` broadcast PR 14 added.

**Tech Stack:** Rust · rusqlite · Tauri 2 · React 19 · TypeScript 5.9.3 · Vitest +
Testing Library

## Global Constraints

Inherited verbatim from `docs/superpowers/plans/2026-08-20-pr-sequence.md`.

- **Windows-first.** No platform-specific dependency without a `cfg` guard.
- **Local-first.** No feature may hard-depend on a network call.
- **No LLM.** Normal task operations must not start an inference process.
- **Rust owns state, dates, and every displayed number.** The model contributes language,
  never arithmetic (§3.6, §24).
- **TypeScript pinned at 5.9.3.**
- **Tokens, never literals.**
- **`null` duration means unmeasured, never zero.**
- **The full local gate before pushing:** `npm run lint`, `npm run format:check`,
  `npx tsc --noEmit`, `npx vitest run`, `cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`, `cargo test`.

---

## Design decisions settled here

| Decision | Why |
|---|---|
| **The command returns a pre-clamped `fraction`, not just the numbers** | "Progress values come from `compute_progress`, never computed in the component" is the PR's own instruction. Handing the component `current` and `target` and expecting it not to divide would be an invitation. |
| **Roll-up counts a child as done when its *rule* is satisfied, not only when its status says so** | The DoD requires a completed daily task to move the monthly number. `is_complete_by_rule` already reports whether a weekly's dailies are all finished, and nothing auto-completes the weekly (§10.1 — computing progress must never write). Reading the rule is the only way the monthly figure can tell the truth without the app silently ticking boxes on the user's behalf. |
| **Roll-up goes exactly one level deeper than `compute_progress`** | Monthly → weekly → daily is the hierarchy the spec defines (§4). Unbounded recursion would buy nothing real and would make a cyclic `parent_task_id` hang the board. |
| **A commitment with a numeric target ignores its children** | `compute_progress` already prefers an explicit target, and for the same reason: "12 of 20 applications" says more than "1 of 3 subtasks". The roll-up must not quietly override the number the user set. |
| **The board is read-only** | Spec §6.5 shows outcomes and bars, no checkboxes. A month view that could be edited would duplicate the Weekly board's job at the wrong altitude. |
| **The bar is a styled div, not block characters** | Block characters are pinned to the font's metrics and go ragged at the small sizes and low opacities §6.1 calls for. A div scales cleanly and can carry `role="progressbar"`, which no run of `█` can. |
| **Over-target renders as a full bar and keeps the true number** | `22 / 20` is a real and good outcome. Clamping the bar stops it overflowing the track; clamping the *text* would erase the achievement. |
| **No monthly commitments is a distinct message from no progress** | An empty board must say the month has no commitments, not imply everything is at 0%. |

---

## File structure

| File | Responsibility |
|---|---|
| `src-tauri/src/domain/month.rs` (+`_tests`) | The current month's boundaries and label |
| `src-tauri/src/domain/progress.rs` | `Progress` gains `Serialize` |
| `src-tauri/src/domain/commitment.rs` (+`_tests`) | The roll-up |
| `src-tauri/src/storage/task_repo.rs` (+`_tests`) | `list_by_horizon_in_period` |
| `src-tauri/src/commands/*`, `lib.rs` | `month_current`, `task_monthly_progress` |
| `src/types/task.ts` | `Month`, `TaskProgress`, `Commitment` |
| `src/lib/ipc.ts` | `currentMonth`, `monthlyProgress` |
| `src/features/boards/components/ProgressBar.tsx` (+test) | The bar |
| `src/features/boards/MonthlyBoard.tsx` (+test) | The board |
| `src/features/boards/BoardRoot.tsx` | Routes `monthly-progress` |

---

### Task 1: Rust — the current month

**Files:** create `src-tauri/src/domain/month.rs` and `month_tests.rs`; register in
`domain/mod.rs`; add the `month_current` command.

**Interfaces:**
- `pub struct Month { pub start: String, pub end: String, pub label: String, pub today: String }`
  — `"2026-08-01"`, `"2026-08-31"`, `"August 2026"`.
- `pub fn month_containing(date: NaiveDate) -> Month`
- `pub fn current_month() -> Month` — `Local::now()`, matching `current_week`.
- Command `month_current -> Month`.

- [x] **Step 1: Write the failing tests:**
  - `a_month_runs_from_the_first_to_the_last_day`
  - `february_in_a_leap_year_ends_on_the_twenty_ninth` — 2028; the arithmetic must come
    from the calendar, not from a table of lengths
  - `february_in_a_common_year_ends_on_the_twenty_eighth` — 2026
  - `december_ends_on_the_thirty_first_and_does_not_roll_the_year`
  - `a_month_is_labelled_in_words`
  - `a_month_remembers_the_date_it_was_derived_from`
- [x] **Step 2: Run to verify they fail.**
- [x] **Step 3: Implement**, finding the last day as "first of next month, minus one day"
      rather than a length table.
- [x] **Step 4: Run to verify they pass. Step 5: Full Rust gate.**
- [x] **Step 6: Commit** — `feat: compute the current month in Rust`

---

### Task 2: Rust — rolling a commitment's progress up

**Files:** `src-tauri/src/domain/progress.rs` (add `Serialize`); create
`src-tauri/src/domain/commitment.rs` and `commitment_tests.rs`;
`src-tauri/src/storage/task_repo.rs` (+`_tests`).

**Interfaces:**

```rust
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Commitment {
    pub task: Task,
    pub progress: Progress,   // serialised { kind, .. }
    pub fraction: f64,        // 0.0..=1.0, already clamped
    pub complete: bool,
}

pub fn commitment_progress(repo, conn, task: &Task) -> Result<Commitment, StorageError>;
```

`Progress` serialises as an internally-tagged enum: `{ "kind": "numeric", "current": 12,
"target": 20 }`.

The roll-up: a commitment with a numeric target uses it directly. Otherwise its children
are counted, and a child counts as complete when `is_complete_by_rule(child, grandchildren)`
holds — which is how a finished daily reaches the monthly figure without anything writing
a status the user did not set.

New repo query:

```sql
SELECT {COLUMNS} FROM tasks
WHERE horizon = ?1
  AND period_start IS NOT NULL AND period_end IS NOT NULL
  AND period_start <= ?3 AND period_end >= ?2
ORDER BY priority DESC NULLS LAST, created_at
```

- [x] **Step 1: Write the failing tests:**
  - repo: `list_by_horizon_in_period_returns_only_that_horizon`;
    `list_by_horizon_in_period_matches_on_overlap_not_containment`
  - `a_commitment_with_a_numeric_target_reports_it`
  - `a_numeric_target_ignores_children` — the user's own number is not overridden
  - `a_commitment_with_no_target_counts_its_children`
  - `a_completed_daily_task_moves_the_monthly_number` — **the DoD.** Monthly → weekly →
    daily; completing the daily makes the weekly satisfied-by-rule and the monthly reads
    1 of 1.
  - `a_partly_finished_daily_set_does_not_count_its_weekly_as_done` — two dailies, one
    finished, monthly reads 0 of 1
  - `rolling_up_never_writes_a_status` — the weekly is still `planned` afterwards
    (§10.1); the whole design rests on this
  - `a_cancelled_child_leaves_the_denominator`
  - `a_commitment_with_nothing_under_it_is_binary`
  - `the_fraction_is_clamped_over_target` — `22 / 20` reports `1.0`
  - `the_fraction_of_an_empty_target_is_zero_not_a_division_by_zero`
- [x] **Step 2: Run to verify they fail. Step 3: Implement. Step 4: Verify.**
- [x] **Step 5: Add `task_monthly_progress { start, end } -> Vec<Commitment>`** and
      register it. **Step 6: Full Rust gate.**
- [x] **Step 7: Commit** — `feat: roll monthly commitments up from their children`

---

### Task 3: Frontend — types and wrappers

**Files:** `src/types/task.ts`, `src/lib/ipc.ts`.

```typescript
export interface Month { start: string; end: string; label: string; today: string }
export type TaskProgress =
  | { kind: "binary"; completed: boolean }
  | { kind: "numeric"; current: number; target: number }
  | { kind: "subtasks"; completed: number; total: number };
export interface Commitment {
  task: Task;
  progress: TaskProgress;
  /** Already clamped to 0..1 by Rust. Never recomputed here. */
  fraction: number;
  complete: boolean;
}
export function currentMonth(): Promise<Month>;
export function monthlyProgress(start: string, end: string): Promise<Commitment[]>;
```

- [x] **Step 1: Add them** (types and thin wrappers; covered by the board's tests rather
      than duplicated ones). **Step 2: Typecheck.**
- [x] **Step 3: Commit** — `feat: type monthly commitments`

---

### Task 4: `ProgressBar`

**Files:** create `src/features/boards/components/ProgressBar.tsx` (+test); append to
`theme.css`.

**Interfaces:** `<ProgressBar fraction={number} label={string} />`

- [x] **Step 1: Write the failing tests:**
  - `renders an empty bar at zero`
  - `renders a full bar at one`
  - `renders a partial bar` — 0.6 → 60%
  - `never overflows its track` — a fraction above 1 still renders 100%; Rust clamps, and
    this asserts the component does not undo it
  - `never renders a negative width`
  - `rounds the percentage for display` — 0.575 → 58%
  - `is a real progressbar` — `role="progressbar"` with `aria-valuenow`, `aria-valuemin`,
    `aria-valuemax`. No run of block characters can carry that.
  - `names what it is measuring` — an unlabelled bar on a board of four says nothing
- [x] **Step 2: Run to verify they fail. Step 3: Implement. Step 4: Verify.**
- [x] **Step 5: Style** — a filled div over a track, sized in percent so it stays legible
      at small font sizes and low opacity where block characters go ragged.
- [x] **Step 6: Commit** — `feat: add the progress bar`

---

### Task 5: `MonthlyBoard`

**Files:** create `src/features/boards/MonthlyBoard.tsx` (+test).

Fetches the month, fetches its commitments, renders each as a heading, a figure, and a
bar. Read-only. Subscribes to `task-changed` and refetches.

```text
AUGUST 2026

JOB SEARCH
12 / 20 applications
████████████░░░░░░░░ 60%
```

- [x] **Step 1: Write the failing tests:**
  - `asks Rust for the current month`
  - `loads the commitments for that month`
  - `shows the month in words`
  - `renders a numeric commitment with its unit` — `12 / 20 applications`
  - `renders a numeric commitment with no unit` — just `12 / 20`
  - `renders a subtask commitment as a count of children` — `2 / 3 done`
  - `renders a binary commitment without inventing a denominator` — "Done" / "Not done",
    not `1 / 1`
  - `renders the bar from the fraction Rust supplied` — asserts `aria-valuenow`, proving
    the component did not divide anything itself
  - `says the month has no commitments rather than showing nothing`
  - `refetches when another window announces a change`
  - `stops listening when it unmounts`
  - `surfaces a failure rather than failing silently`
- [x] **Step 2: Run to verify they fail. Step 3: Implement. Step 4: Verify.**
- [x] **Step 5: Commit** — `feat: add the monthly progress board`

---

### Task 6: Wire it up and verify against the real app

**Files:** `src/features/boards/BoardRoot.tsx` (+test).

- [x] **Step 1: Write the failing test** — `puts the monthly board in the
      monthly-progress window`. Every board now has content, so the placeholder branch
      disappears and the switch becomes exhaustive over real components.
- [x] **Step 2: Run to verify it fails. Step 3: Implement. Step 4: Verify.**
- [x] **Step 5: The full local gate** — all seven commands.
- [ ] **Step 6: Launch and read the dev log.**

      **Done so far:** the app launched and built clean, the dev log carries no error,
      permission denial, or panic, and no sidecar process appeared. Three `[seed]` monthly
      commitments were written — one numeric, one with a real monthly → weekly → daily
      chain, one binary with nothing under it.

      **The DoD was demonstrated against the real database.** Completing one outstanding
      daily task moved `DAILY STANDUP MVP` from `1 / 2 done` (50%) to `2 / 2 done` (100%),
      and the weekly parent's status stayed `planned` — the §10.1 guarantee that computing
      progress never writes. Note the verification script *mirrors* the Rust roll-up
      rather than calling it; the implementation itself is covered by
      `a_completed_daily_task_moves_the_monthly_number` and
      `rolling_up_never_writes_a_status`.

      **Still needs a human at the keyboard:** confirming the board renders the bars
      legibly at a small font size and low opacity, which is the one claim only eyes can
      settle.

      Remove the seeded rows afterwards with
      `DELETE FROM tasks WHERE title LIKE '[seed]%'`.
- [x] **Step 7: Commit, push, open the PR, tick PR 15 in the sequence document.**

---

## Definition of Done

From the sequence document, verbatim:

- Monthly commitments show correct roll-up from completed weekly/daily children.

Test coverage it asks for: the bar's percentage rendering including 0%, 100%, and
over-target; an integration test that a completed daily task moves the monthly number
(`a_completed_daily_task_moves_the_monthly_number`, Task 2).

## Deliberately out of scope

- **Editing from the monthly board.** §6.5 is a report. The Weekly board owns interaction.
- **Month navigation.** Current month only; PR 16.
- **Setting a commitment's target from the board.** No UI in the spec for it; the context
  menu on the Weekly board is where a task's fields get edited.
