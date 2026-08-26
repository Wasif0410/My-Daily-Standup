import type { Week } from "@/types/task";

const MONTHS = [
  "Jan",
  "Feb",
  "Mar",
  "Apr",
  "May",
  "Jun",
  "Jul",
  "Aug",
  "Sep",
  "Oct",
  "Nov",
  "Dec",
];

/** Splits an ISO date without going through `Date`, which would apply a zone. */
function parts(iso: string): { month: number; day: number } | null {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(iso);
  if (!match) return null;

  const month = Number(match[2]);
  const day = Number(match[3]);
  if (month < 1 || month > 12) return null;

  return { month, day };
}

/**
 * A week as a person would say it: `Aug 24–30 · Week 35`.
 *
 * The board previously showed `2026-W35 2026-08-24 → 2026-08-30`, which is
 * precise and unreadable at a glance — three numbers that all look alike, on
 * the one line meant to answer "which week am I looking at".
 *
 * The month is not repeated when both ends fall inside it, because "Aug 24 –
 * Aug 30" spends a board's scarce width saying August twice. It is repeated
 * across a boundary, where dropping it would make `Aug 31–6` unreadable.
 *
 * Parsed with a regex rather than `Date`: `new Date("2026-08-24")` is UTC
 * midnight, which in any negative offset renders as the 23rd. The board would
 * name the wrong week for everyone west of Greenwich.
 *
 * Falls back to the raw label if anything fails to parse. A malformed date is
 * not worth blanking the header over.
 */
export function formatWeek(week: Week): string {
  const start = parts(week.start);
  const end = parts(week.end);
  const number = /W(\d{1,2})$/.exec(week.label)?.[1];

  if (!start || !end) return week.label;

  const from = `${MONTHS[start.month - 1]} ${start.day}`;
  const to =
    start.month === end.month ? `${end.day}` : `${MONTHS[end.month - 1]} ${end.day}`;

  const range = `${from}–${to}`;
  return number ? `${range} · Week ${number}` : range;
}
