/**
 * Keeping board windows in step.
 *
 * Each board is its own Tauri window running its own React app and its own
 * zustand store. Completing a task on the Weekly Tasks board cannot reach the
 * Weekly Progress board by any in-process route — there is no shared memory
 * between them. A broadcast is the only mechanism there is.
 */

import { emit, listen } from "@tauri-apps/api/event";

/** The one channel. Every board emits and listens on it. */
export const TASK_CHANGED = "task-changed";

/**
 * Tells every window a task changed.
 *
 * Fire-and-forget on purpose. By the time this runs the write has already
 * succeeded; a rejected promise here would turn a stale board into a broken
 * checkbox. Failures are swallowed rather than surfaced.
 *
 * Emitted from the frontend rather than from Rust, which keeps every command
 * free of an `AppHandle`. The tradeoff: a future Rust-initiated write — a
 * vault sync, say — would have to emit for itself.
 */
export function emitTaskChanged(): void {
  void emit(TASK_CHANGED).catch(() => {
    // Deliberately silent. See above.
  });
}

/**
 * Runs `handler` whenever any window announces a change.
 *
 * Resolves to an unsubscribe. Callers must invoke it on unmount, or a closed
 * board keeps reloading in the background.
 */
export async function onTaskChanged(handler: () => void): Promise<() => void> {
  return await listen(TASK_CHANGED, () => {
    handler();
  });
}
