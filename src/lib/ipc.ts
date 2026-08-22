/**
 * Typed wrappers around Tauri's IPC.
 *
 * No component calls `invoke` with a raw command string. Every call goes
 * through a function here, so a renamed or removed command breaks the build
 * instead of failing at runtime.
 */

import { invoke } from "@tauri-apps/api/core";
import type {
  CommandError,
  ErrorKind,
  NewTask,
  Task,
  TaskHorizon,
  TaskPatch,
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
