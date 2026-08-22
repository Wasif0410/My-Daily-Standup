/**
 * Board window types, mirroring `src-tauri/src/storage/board.rs`.
 */

export type BoardKind =
  "priority" | "weekly-tasks" | "weekly-progress" | "monthly-progress";

export const BOARD_KINDS: BoardKind[] = [
  "priority",
  "weekly-tasks",
  "weekly-progress",
  "monthly-progress",
];

/** A board window's saved state. */
export interface BoardWindow {
  kind: BoardKind;
  /** Null until the window has been placed once. */
  x: number | null;
  y: number | null;
  width: number;
  height: number;
  monitor: string | null;
  visible: boolean;
  collapsed: boolean;
  opacity: number;
  alwaysOnTop: boolean;
  locked: boolean;
}

/** Narrows an untrusted string — a URL parameter — to a known board. */
export function parseBoardKind(value: string | null): BoardKind | null {
  return BOARD_KINDS.find((kind) => kind === value) ?? null;
}
