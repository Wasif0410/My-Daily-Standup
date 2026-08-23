import type { DayTotals } from "@/features/boards/daySummary";
import { formatMinutes } from "@/lib/duration";

/**
 * What a day reports without being opened: `2/3   1h 45m`.
 *
 * The whole point of the collapsed board (spec §6.4) — it has to answer "how
 * did the week go" before anyone expands anything.
 *
 * An empty day still shows `0/0`. Leaving it blank would read as a rendering
 * failure rather than as a quiet day.
 */
export function DaySummary({ totals }: { totals: DayTotals }) {
  const time = formatMinutes(totals.minutes);

  // "2/3" and an em dash are shorthand a screen reader cannot make sense of.
  const spoken =
    `${totals.completed} of ${totals.total} done, ` +
    (totals.minutes === null ? "no time recorded" : time);

  return (
    <span className="day-summary" aria-label={spoken}>
      <span className="day-count" aria-hidden="true">
        {totals.completed}/{totals.total}
      </span>
      <span className="day-time" aria-hidden="true">
        {time}
      </span>
    </span>
  );
}
