import type { ReactNode } from "react";
import { DaySummary } from "@/features/boards/components/DaySummary";
import type { Day } from "@/features/boards/daySummary";
import type { Task } from "@/types/task";

interface DaySectionProps {
  day: Day;
  expanded: boolean;
  /** Distinguished visually, and expanded by default on an untouched board. */
  today: boolean;
  onToggle: (expanded: boolean) => void;
  renderTask: (task: Task) => ReactNode;
}

/**
 * One collapsible day of the week (spec §6.4).
 *
 * Owns no state. Expansion persists to the database, so the board holds it and
 * passes it back down — the same reason `BoardShell` does not own its own
 * collapse. A section that kept its own copy would drift after a failed write.
 *
 * Empty days render exactly like busy ones. A week with three empty days *is*
 * the information; hiding them would turn the board into a list of busy days
 * and lose the shape of the week.
 */
export function DaySection({
  day,
  expanded,
  today,
  onToggle,
  renderTask,
}: DaySectionProps) {
  const listId = `day-${day.date}`;

  return (
    <section className="day-section" data-today={today} data-expanded={expanded}>
      <button
        type="button"
        className="day-header"
        aria-expanded={expanded}
        aria-controls={listId}
        onClick={() => onToggle(!expanded)}
      >
        <span className="day-chevron" aria-hidden="true">
          {expanded ? "▾" : "▸"}
        </span>
        <span className="day-name">{day.name}</span>
        <DaySummary totals={day.totals} />
      </button>

      {expanded &&
        (day.tasks.length === 0 ? (
          <p className="day-empty" id={listId}>
            Nothing scheduled.
          </p>
        ) : (
          <ul className="task-list-plain" id={listId} aria-label={day.name}>
            {day.tasks.map(renderTask)}
          </ul>
        ))}
    </section>
  );
}
