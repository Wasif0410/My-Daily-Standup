import { completionPercent, type DayTotals } from "@/features/boards/daySummary";
import { formatMinutes } from "@/lib/duration";

/**
 * What a day reports without being opened: `1h 45m   2/3   ▰▰▱`.
 *
 * The whole point of the collapsed board (spec §6.4) — it has to answer "how
 * did the week go" before anyone expands anything.
 *
 * An empty day says "No tasks" in words. It used to show `0/0 — 0%`, and seven
 * of those read as a board that had failed to load rather than a quiet week.
 *
 * Completion is drawn as a short bar rather than printed as a percentage: the
 * bar says where to look first, and `2/3` beside it is what the user acts on.
 * The bar is decorative to assistive technology, because the summary sits
 * inside the day's toggle button and a progressbar is not allowed there; the
 * spoken label carries the same figures in words. Time appears only when some
 * was recorded, since a dash in every row was noise.
 *
 * `data-complete` is emitted for the stylesheet, which colours a finished day —
 * the flag follows `completionPercent`, so it is set only when the day is
 * genuinely done.
 */
export function DaySummary({ totals }: { totals: DayTotals }) {
  if (totals.total <= 0) {
    return (
      <span className="day-summary" data-empty="true" aria-label="No tasks">
        <span className="day-none" aria-hidden="true">
          No tasks
        </span>
      </span>
    );
  }

  const time = formatMinutes(totals.minutes);
  const percent = completionPercent(totals);

  const spoken =
    `${totals.completed} of ${totals.total} done, ` +
    (totals.minutes === null ? "no time recorded" : time) +
    `, ${percent} percent`;

  return (
    <span className="day-summary" data-complete={percent === 100} aria-label={spoken}>
      {totals.minutes !== null && (
        <span className="day-time" aria-hidden="true">
          {time}
        </span>
      )}
      <span className="day-count" aria-hidden="true">
        {totals.completed}/{totals.total}
      </span>
      <span className="day-bar" aria-hidden="true">
        <span className="day-bar-fill" style={{ width: `${percent}%` }} />
      </span>
    </span>
  );
}
