import { formatRange, relativeWeek, weekNumber } from "@/features/boards/weekLabel";
import type { Week } from "@/types/task";

interface WeekNavProps {
  week: Week;
  /** Weeks away from the current one: 0 is this week, -1 last week. */
  offset: number;
  onOffset: (offset: number) => void;
}

/**
 * The week a board is showing, and the way to page through weeks.
 *
 * Says which week it is in words first ("This week", "2 weeks ago"), because
 * that is the question a glance is asking, with the dates beneath it and the
 * ISO week number as a quiet tag. It replaced a single line of
 * `Oct 12–18 · Week 42` that was all detail and no answer.
 *
 * "Today" stays in place but is disabled on the current week, so the arrows
 * never move as it appears and disappears.
 */
export function WeekNav({ week, offset, onOffset }: WeekNavProps) {
  const number = weekNumber(week);

  return (
    <nav className="week-nav" aria-label="Week">
      <div className="week-nav-label">
        <span className="week-nav-relative" data-current={offset === 0}>
          {relativeWeek(offset)}
        </span>
        <span className="week-nav-dates">
          {formatRange(week)}
          {number !== null && <span className="week-nav-number">Week {number}</span>}
        </span>
      </div>

      <div className="week-nav-controls">
        <button
          type="button"
          className="week-nav-step"
          aria-label="Previous week"
          onClick={() => onOffset(offset - 1)}
        >
          <span aria-hidden="true">‹</span>
        </button>
        <button
          type="button"
          className="week-nav-today"
          aria-label="Back to this week"
          disabled={offset === 0}
          onClick={() => onOffset(0)}
        >
          Today
        </button>
        <button
          type="button"
          className="week-nav-step"
          aria-label="Next week"
          onClick={() => onOffset(offset + 1)}
        >
          <span aria-hidden="true">›</span>
        </button>
      </div>
    </nav>
  );
}
