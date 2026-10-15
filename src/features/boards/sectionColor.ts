import {
  headingKey,
  UNSORTED_AREA,
  UNSORTED_PROJECT,
} from "@/features/boards/grouping";

/**
 * The colours a section can take, as tokens so each theme supplies its own
 * shade (tokens.css). Eight muted hues, chosen to sit apart from each other
 * and from the status colours on a dark surface.
 */
export const SECTION_COLORS = [
  "var(--section-1)",
  "var(--section-2)",
  "var(--section-3)",
  "var(--section-4)",
  "var(--section-5)",
  "var(--section-6)",
  "var(--section-7)",
  "var(--section-8)",
] as const;

const NEUTRAL = new Set([headingKey(UNSORTED_AREA), headingKey(UNSORTED_PROJECT)]);

/**
 * A section's colour, picked from its name.
 *
 * Every group used to be the same grey panel, so a board was a column of
 * identical boxes and nothing told "School" from "Job Search" except reading.
 * A colour per section makes the groups findable at a glance, the way
 * project colours work in Todoist or Notion. It is always paired with the
 * name, never instead of it.
 *
 * Picked by hashing the name rather than stored, so the same section is the
 * same colour on every board and after every restart with nothing to save.
 * The catch-all headings get no colour: they are the absence of a section.
 */
export function sectionColor(label: string): string | null {
  const key = headingKey(label);
  if (NEUTRAL.has(key)) return null;

  // FNV-1a: cheap, stable, and spreads short names well.
  let hash = 0x811c9dc5;
  for (let index = 0; index < key.length; index += 1) {
    hash ^= key.charCodeAt(index);
    hash = Math.imul(hash, 0x01000193);
  }

  return SECTION_COLORS[(hash >>> 0) % SECTION_COLORS.length] ?? null;
}
