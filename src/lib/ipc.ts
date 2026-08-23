/**
 * Typed wrappers around Tauri's IPC.
 *
 * No component calls `invoke` with a raw command string. Every call goes
 * through a function here, so a renamed or removed command breaks the build
 * instead of failing at runtime.
 */

import { invoke } from "@tauri-apps/api/core";
import type { BoardBehavior, BoardKind, BoardWindow } from "@/types/board";
import type {
  CommandError,
  ErrorKind,
  NewTask,
  Task,
  TaskHorizon,
  TaskPatch,
  Commitment,
  Month,
  Week,
} from "@/types/task";

/**
 * A failed command.
 *
 * A real `Error` subclass rather than the bare `{ kind, message }` object Rust
 * sends, so rejections carry a stack trace and behave like every other error in
 * the app. `kind` is preserved so callers can still branch on the cause.
 */
export class IpcError extends Error implements CommandError {
  readonly kind: ErrorKind;

  constructor(kind: ErrorKind, message: string) {
    super(message);
    this.name = "IpcError";
    this.kind = kind;
  }
}

/**
 * Narrows an unknown rejection to the wire shape Rust sends.
 *
 * A failure inside the IPC transport itself surfaces as something else
 * entirely, so this cannot assume.
 */
export function isCommandError(error: unknown): error is CommandError {
  return (
    typeof error === "object" && error !== null && "kind" in error && "message" in error
  );
}

/**
 * Converts any rejection into an `IpcError`.
 *
 * Callers should never have to handle two shapes of failure.
 */
export function toCommandError(error: unknown): IpcError {
  if (error instanceof IpcError) {
    return error;
  }

  if (isCommandError(error)) {
    return new IpcError(error.kind, error.message);
  }

  return new IpcError(
    "internal",
    error instanceof Error ? error.message : String(error),
  );
}

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    throw toCommandError(error);
  }
}

export function createTask(input: NewTask): Promise<Task> {
  return call<Task>("task_create", { input });
}

/** Resolves to `null` when no task has that id — a miss is not an error. */
export function getTask(id: string): Promise<Task | null> {
  return call<Task | null>("task_get", { id });
}

export function updateTask(id: string, patch: TaskPatch): Promise<Task> {
  return call<Task>("task_update", { id, patch });
}

export function deleteTask(id: string): Promise<void> {
  return call<void>("task_delete", { id });
}

export function listTasksByHorizon(horizon: TaskHorizon): Promise<Task[]> {
  return call<Task[]>("task_list_by_horizon", { horizon });
}

/** Tasks scheduled for one day. `date` is ISO-8601, e.g. "2026-08-21". */
export function listTasksForDate(date: string): Promise<Task[]> {
  return call<Task[]>("task_list_for_date", { date });
}

/** Tasks whose period overlaps `[start, end]`. */
export function listTasksForPeriod(start: string, end: string): Promise<Task[]> {
  return call<Task[]>("task_list_for_period", { start, end });
}

/**
 * Tasks scheduled on any day in `[start, end]`, inclusive.
 *
 * Keyed on the scheduled day, not the period: the Weekly Progress board asks
 * "how did the week go, day by day", and a commitment that merely overlaps the
 * week has no day to sit on.
 */
export function listTasksScheduledBetween(start: string, end: string): Promise<Task[]> {
  return call<Task[]>("task_list_scheduled_between", { start, end });
}

/**
 * Tasks for the Priority board: non-daily work at or above `threshold`.
 *
 * Filtered in SQL rather than here. The alternative is fetching three horizons
 * and merging them, which is three round-trips for one list.
 */
export function listPriorityTasks(threshold: number): Promise<Task[]> {
  return call<Task[]>("task_list_priority", { threshold });
}

/**
 * Records how long a task took, in minutes.
 *
 * `null` clears it back to unrecorded, which is distinct from zero: an
 * unmeasured task contributes nothing to a total.
 */
export function setTimeSpent(id: string, minutes: number | null): Promise<Task> {
  return call<Task>("task_set_time_spent", { id, minutes });
}

/**
 * Sets or clears a task's blocker. `null` clears it.
 *
 * Never {@link updateTask} with a `blocker`: only this path keeps the text and
 * the `blocked` status in step, and once they can disagree every view has to
 * pick one to believe.
 */
export function setBlocker(id: string, blocker: string | null): Promise<Task> {
  return call<Task>("task_set_blocker", { id, blocker });
}

/**
 * Appends a dated comment to a task's notes.
 *
 * Rust owns the date stamp and the joining, so the caller sends only the text.
 */
export function addComment(id: string, comment: string): Promise<Task> {
  return call<Task>("task_add_comment", { id, comment });
}

/**
 * Moves a task to another week.
 *
 * Routed through the rollover engine like {@link rescheduleTask}, so pushing a
 * commitment into a later week is counted as the deferral it is.
 */
export function moveTaskToPeriod(
  id: string,
  start: string,
  end: string,
): Promise<Task> {
  return call<Task>("task_move_to_period", { id, start, end });
}

/**
 * Archives a task: cancelled, not deleted.
 *
 * The row survives; only the board loses it. Use {@link deleteTask} when the
 * task should genuinely cease to exist.
 */
export function archiveTask(id: string): Promise<Task> {
  return call<Task>("task_archive", { id });
}

/**
 * The week today falls in.
 *
 * Asked of Rust rather than derived here: the frontend computing its own
 * "today" from the browser clock is how a board and its database end up
 * disagreeing about which week it is.
 */
export function currentWeek(startsOn?: string): Promise<Week> {
  return call<Week>("week_current", { startsOn: startsOn ?? null });
}

/**
 * The month today falls in.
 *
 * Asked of Rust for the same reason as {@link currentWeek}: a frontend that
 * derived its own month from the browser clock would disagree with the
 * database on the evening of the 31st.
 */
export function currentMonth(): Promise<Month> {
  return call<Month>("month_current");
}

/**
 * Monthly commitments overlapping `[start, end]`, with progress rolled up from
 * the work beneath them.
 *
 * Every figure and the bar's fraction are computed in Rust. Nothing on the
 * Monthly board divides (spec §3.6).
 */
export function monthlyProgress(start: string, end: string): Promise<Commitment[]> {
  return call<Commitment[]>("task_monthly_progress", { start, end });
}

/**
 * Moves a task to a new date.
 *
 * Routed through the rollover engine, so a deferral is counted. Never set
 * `scheduledDate` via {@link updateTask} — that bypasses the counter and
 * silently breaks the reflection prompt.
 */
export function rescheduleTask(id: string, to: string): Promise<Task> {
  return call<Task>("task_reschedule", { id, to });
}

export function childrenOfTask(parentId: string): Promise<Task[]> {
  return call<Task[]>("task_children_of", { parentId });
}

// --- boards -----------------------------------------------------------------

export function openBoard(kind: BoardKind): Promise<void> {
  return call<void>("board_open", { kind });
}

export function closeBoard(kind: BoardKind): Promise<void> {
  return call<void>("board_close", { kind });
}

/** Records a board's position and size. Callers should debounce this. */
export function saveBoardGeometry(
  kind: BoardKind,
  x: number,
  y: number,
  width: number,
  height: number,
): Promise<void> {
  return call<void>("board_save_geometry", { kind, x, y, width, height });
}

/** Records whether a board is collapsed to its title bar. */
export function setBoardCollapsed(kind: BoardKind, collapsed: boolean): Promise<void> {
  return call<void>("board_set_collapsed", { kind, collapsed });
}

export function listBoards(): Promise<BoardWindow[]> {
  return call<BoardWindow[]>("board_list");
}

/**
 * Applies one behaviour change and persists it, returning the new saved state.
 *
 * One call rather than a write followed by an apply: two would be two chances
 * for the window and the database to disagree.
 */
export function setBoardBehavior(
  kind: BoardKind,
  behavior: BoardBehavior,
): Promise<BoardWindow> {
  return call<BoardWindow>("board_set_behavior", { kind, behavior });
}

/**
 * Unlocks every board.
 *
 * The escape hatch from a fully locked, click-through desktop (spec §6.7).
 * Also bound to Ctrl+Alt+Shift+U at the OS level, for when no window can be
 * clicked at all.
 */
export function unlockAllBoards(): Promise<void> {
  return call<void>("board_unlock_all");
}

// --- presentation state ------------------------------------------------------

/**
 * Reads one piece of presentation state — which days a board has expanded, and
 * the like.
 *
 * `null` means never set, which is how a board tells "the user chose nothing"
 * apart from "the user chose to expand nothing".
 */
export function getUiState(key: string): Promise<string | null> {
  return call<string | null>("ui_state_get", { key });
}

export function setUiState(key: string, value: string): Promise<void> {
  return call<void>("ui_state_set", { key, value });
}

/**
 * Closes the Quick Add window.
 *
 * A command rather than `getCurrentWindow().close()` so the capability file
 * does not have to grant a close permission to a window whose only job is one
 * input.
 */
export function closeQuickAdd(): Promise<void> {
  return call<void>("quick_add_close");
}
