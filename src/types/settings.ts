/**
 * Application settings, mirroring `src-tauri/src/storage/settings.rs`.
 *
 * Rust serialises with `rename_all = "camelCase"`, so these names must match
 * the Rust field names exactly. A mismatch does not fail to compile — the
 * frontend just silently reads `undefined`.
 *
 * One row, not a table: there is exactly one set of these, and everything that
 * reads them — the Priority board's cut-off, which day a week starts on,
 * whether the app launches with the OS — was already written to be
 * configurable and has simply had nowhere to store the answer.
 */

/**
 * Which day a week starts on.
 *
 * A union rather than `string` because the database has a CHECK constraint on
 * exactly these three values. With a plain string a typo — "Monday", "mon" —
 * compiles happily, travels all the way to SQLite, and fails there at runtime,
 * in a write the user has already made. As a union the same typo is a build
 * error, which is the only place a fixed set of three values should ever be
 * caught.
 */
export type WeekStart = "monday" | "sunday" | "saturday";

/** The stored settings. */
export interface Settings {
  /** The Priority board's cut-off: tasks at or above this are shown. 0-10,
   *  validated in Rust — a value outside that range is rejected, not clamped
   *  silently. */
  priorityThreshold: number;
  weekStartsOn: WeekStart;
  /** Whether the app registers itself with the OS autostart. Persisting this
   *  and applying it to the OS are one operation in Rust, so the two cannot
   *  drift apart. */
  launchAtLogin: boolean;
}

/**
 * A partial update.
 *
 * An omitted key means "leave alone", the same convention as `TaskPatch`.
 * There is no `null` case here: none of these three fields is nullable, so
 * there is nothing to clear — a caller either sets a value or says nothing.
 */
export interface SettingsPatch {
  priorityThreshold?: number;
  weekStartsOn?: WeekStart;
  launchAtLogin?: boolean;
}
