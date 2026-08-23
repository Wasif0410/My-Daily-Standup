/**
 * Reading and writing recorded durations.
 *
 * Spec §10.4. Logging has to be one gesture — if it takes more than a few
 * seconds it stops happening, and the weekly recap becomes worthless.
 */

/** Common values, offered as one-tap buttons. */
export const DURATION_PRESETS = [
  { minutes: 15, label: "15m" },
  { minutes: 30, label: "30m" },
  { minutes: 60, label: "1h" },
  { minutes: 120, label: "2h" },
] as const;

/**
 * Formats minutes for display.
 *
 * `null` renders as a dash, not "0m". Zero would claim the work took no time,
 * when in fact it was never recorded, and the boards must not conflate the two.
 */
export function formatMinutes(minutes: number | null): string {
  if (minutes === null) return "—";

  const hours = Math.floor(minutes / 60);
  const remainder = minutes % 60;

  if (hours === 0) return `${remainder}m`;
  if (remainder === 0) return `${hours}h`;
  return `${hours}h ${remainder}m`;
}

/**
 * Parses a typed duration into minutes, or `null` if it is not one.
 *
 * Accepts `90`, `45m`, `2h`, `1h30m`, `1h 30m`, and `1.5h`.
 *
 * A bare number means minutes. That is the most common entry, so it must not
 * require a unit — and reading it as hours would silently inflate every total
 * sixtyfold.
 *
 * Invalid input returns `null` rather than a guess. A negative value is
 * rejected outright rather than clamped, since clamping would record something
 * the user never typed.
 */
export function parseDuration(input: string): number | null {
  const text = input.trim().toLowerCase();
  if (!text) return null;

  // A bare number is minutes.
  if (/^\d+(\.\d+)?$/.test(text)) {
    return Math.round(Number(text));
  }

  const match = /^(?:(\d+(?:\.\d+)?)\s*h)?\s*(?:(\d+(?:\.\d+)?)\s*m)?$/.exec(text);
  if (!match) return null;

  const [, hours, minutes] = match;
  // The pattern matches an empty string, so require at least one component.
  if (hours === undefined && minutes === undefined) return null;

  const total = Number(hours ?? 0) * 60 + Number(minutes ?? 0);
  return Math.round(total);
}
