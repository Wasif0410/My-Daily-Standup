/**
 * Task types, mirroring `src-tauri/src/storage/task.rs`.
 *
 * Rust serialises with `rename_all = "camelCase"`, so these names must match
 * the Rust field names exactly. A mismatch does not fail to compile — the
 * frontend just silently reads `undefined` — so `commands/tests.rs` asserts the
 * casing on the Rust side.
 */

export type TaskHorizon = "daily" | "weekly" | "monthly" | "long-term";

export type TaskStatus =
  | "backlog"
  | "planned"
  | "in-progress"
  | "blocked"
  | "completed"
  | "cancelled"
  | "deferred";

export type TaskSource = "obsidian" | "standup" | "manual";

/** A stored task. */
export interface Task {
  id: string;
  title: string;
  description: string | null;

  horizon: TaskHorizon;
  status: TaskStatus;

  parentTaskId: string | null;

  sourceType: TaskSource;
  sourceFile: string | null;
  sourceLine: number | null;

  area: string | null;
  project: string | null;
  priority: number | null;

  /** ISO-8601 date, e.g. "2026-08-21". */
  scheduledDate: string | null;
  periodStart: string | null;
  periodEnd: string | null;
  dueDate: string | null;
  completedAt: string | null;

  progressCurrent: number | null;
  progressTarget: number | null;
  progressUnit: string | null;

  blocker: string | null;
  notes: string | null;

  /** Incremented only when a task is rescheduled (spec §10.3). */
  rolloverCount: number;

  createdAt: string;
  updatedAt: string;
}

/**
 * Input for creating a task.
 *
 * `id`, timestamps, and `rolloverCount` are owned by Rust and cannot be set
 * here — ids in particular are generated server-side so a proposal cannot
 * overwrite an existing task by choosing one.
 */
export interface NewTask {
  title: string;
  horizon: TaskHorizon;
  status: TaskStatus;
  sourceType: TaskSource;

  description?: string | null;
  parentTaskId?: string | null;
  sourceFile?: string | null;
  sourceLine?: number | null;
  area?: string | null;
  project?: string | null;
  priority?: number | null;
  scheduledDate?: string | null;
  periodStart?: string | null;
  periodEnd?: string | null;
  dueDate?: string | null;
  progressCurrent?: number | null;
  progressTarget?: number | null;
  progressUnit?: string | null;
  notes?: string | null;
}

/**
 * A partial update.
 *
 * Three states per field, and the distinction matters:
 *
 * - **omit the key** — leave the field unchanged
 * - **`null`** — clear the field
 * - **a value** — set the field
 *
 * Without the middle case there would be no way to resolve a blocker or
 * un-complete a task. Rust decodes this with a custom deserialiser, since
 * serde's default collapses an absent key and an explicit null onto the same
 * value.
 *
 * Unknown keys are rejected by Rust rather than ignored, so a typo fails
 * loudly.
 */
export interface TaskPatch {
  title?: string;
  horizon?: TaskHorizon;
  status?: TaskStatus;

  description?: string | null;
  parentTaskId?: string | null;
  area?: string | null;
  project?: string | null;
  priority?: number | null;
  scheduledDate?: string | null;
  periodStart?: string | null;
  periodEnd?: string | null;
  dueDate?: string | null;
  completedAt?: string | null;
  progressCurrent?: number | null;
  progressTarget?: number | null;
  progressUnit?: string | null;
  blocker?: string | null;
  notes?: string | null;
}

/** What went wrong, in a form the UI can branch on. */
export type ErrorKind = "not-found" | "invalid-input" | "storage" | "internal";

export interface CommandError {
  kind: ErrorKind;
  message: string;
}
