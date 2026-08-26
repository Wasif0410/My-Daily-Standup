/**
 * Board section types, mirroring `src-tauri/src/storage/section.rs`.
 *
 * Rust serialises with `rename_all = "camelCase"`, so these names must match
 * the Rust field names exactly. A mismatch does not fail to compile — the
 * frontend just silently reads `undefined`.
 */

import type { BoardKind } from "@/types/board";

/**
 * A named task group on one board — the "JOB SEARCH" / "HEALTH" headings.
 *
 * Carries no contents of its own. What sits under the heading are real tasks,
 * matched on `tasks.area` / `tasks.project` and loaded by `taskStore`; a copy
 * nested here would be a second source of truth for the same rows.
 */
export interface BoardSection {
  id: string;
  boardKind: BoardKind;
  title: string;
  /** Display order within the board. Owned by Rust. */
  position: number;
}
