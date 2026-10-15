import { formatRange, relativeWeek, weekNumber } from "@/features/boards/weekLabel";
import type { Week, WeekDay } from "@/types/task";

/** What a day holds, for the dot under it: work left, or all of it done. */
export type DayMark = "open" | "done";

interface WeekNavProps {
  week: Week;
  /** Weeks away from the current one: 0 is this week, -1 last week. */
  offset: number;
  onOffset: (offset: number) => void;
  /** Days that have tasks, keyed by ISO date. Days not listed have none. */
  marks?: Record<string, DayMark> | undefined;
  /** Makes each day a button. Boards with nothing to open per day omit it. */
  onDay?: ((date: string) => void) | undefined;
}

const MARK_WORDS: Record<DayMark, string> = {
  open: "tasks left",
  done: "all done",
};

/** The day of the month, read straight off the ISO date: no Date, no zone. */
function dayOfMonth(date: string): number {
  return Number(date.slice(8, 10));
}

function isWeekend(day: WeekDay): boolean {
  return day.name === "Saturday" || day.name === "Sunday";
}

/**
 * The week a board is showing: a heading in words, the dates, and a strip of
 * its seven days, the way a calendar app shows "this week".
 *
 * The strip is what makes the header worth its space. Today is a filled
 * circle in the board's colour, days already gone are quieter, and on a
 * board that knows about days a dot under each says whether there is work
 * left on it. Paging is the segmented control on the right; "Today" stays in
 * place but is disabled on the current week, so the arrows never move.
 */
export function WeekNav({ week, offset, onOffset, marks, onDay }: WeekNavProps) {
  const number = weekNumber(week);

  return (
    <nav className="week-nav" aria-label="Week" data-current={offset === 0}>
      <div className="week-nav-top">
        <div className="week-nav-label">
          <span className="week-nav-relative">{relativeWeek(offset)}</span>
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
      </div>

      <ol className="week-strip">
        {week.days.map((day) => {
          const today = day.date === week.today;
          const mark = marks?.[day.date];
          const label =
            `${day.name} ${dayOfMonth(day.date)}` +
            (today ? ", today" : "") +
            (mark ? `, ${MARK_WORDS[mark]}` : "");

          const content = (
            <>
              <span className="week-strip-name" aria-hidden="true">
                {day.name.charAt(0)}
              </span>
              <span className="week-strip-date" aria-hidden="true">
                {dayOfMonth(day.date)}
              </span>
              <span className="week-strip-mark" data-mark={mark} aria-hidden="true" />
            </>
          );

          const shared = {
            className: "week-strip-day",
            "aria-label": label,
            "aria-current": today ? ("date" as const) : undefined,
            "data-today": today,
            "data-past": day.date < week.today,
            "data-weekend": isWeekend(day),
          };

          return (
            <li key={day.date}>
              {onDay ? (
                <button type="button" {...shared} onClick={() => onDay(day.date)}>
                  {content}
                </button>
              ) : (
                // An image role, so the label is read out: a bare span with
                // an aria-label is ignored by most screen readers.
                <span role="img" {...shared}>
                  {content}
                </span>
              )}
            </li>
          );
        })}
      </ol>
    </nav>
  );
}
