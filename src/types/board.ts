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
  /** Logical pixels, clamped 10-24 by the schema. */
  fontSize: number;
  theme: BoardTheme;
  /** Tighter spacing for a board kept small. */
  compact: boolean;
  /** Sits below ordinary windows. Mutually exclusive with `alwaysOnTop`. */
  desktopLevel: boolean;
}

/** Which palette a board draws in (spec §6.7). */
export type BoardTheme = "dark" | "light";

/**
 * One behaviour change, mirroring `src-tauri/src/windows/behaviors.rs`.
 *
 * A tagged union rather than a partial `BoardWindow`: the mutual exclusion
 * between `alwaysOnTop` and `desktopLevel` is resolved in Rust, and sending a
 * whole object would invite a caller to set both.
 */
export type BoardBehavior =
  | { kind: "alwaysOnTop"; value: boolean }
  | { kind: "desktopLevel"; value: boolean }
  | { kind: "locked"; value: boolean }
  | { kind: "opacity"; value: number }
  | { kind: "fontSize"; value: number }
  | { kind: "theme"; value: BoardTheme }
  | { kind: "compact"; value: boolean }
  | { kind: "visible"; value: boolean };

/** Narrows an untrusted string — a URL parameter — to a known board. */
export function parseBoardKind(value: string | null): BoardKind | null {
  return BOARD_KINDS.find((kind) => kind === value) ?? null;
}
