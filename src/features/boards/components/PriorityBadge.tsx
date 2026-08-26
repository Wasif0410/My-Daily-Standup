import { useState } from "react";

/** Where a priority sits on the 0-10 scale, for colour grading. */
type Tier = "high" | "medium" | "low" | "none";

/**
 * The levels the picker offers.
 *
 * The column stores 0-10, but 0 is not offered: a task ranked lowest and a
 * task nobody ranked are different things, and the app already says the second
 * one with `null`. Two ways to mean "bottom" would make the boards' own rule —
 * `priority IS NULL` never reaches the Priority board — read as arbitrary.
 */
const LEVELS = [10, 9, 8, 7, 6, 5, 4, 3, 2, 1];

function tierOf(priority: number | null): Tier {
  if (priority === null) return "none";
  if (priority >= 8) return "high";
  if (priority >= 4) return "medium";
  return "low";
}

function labelFor(priority: number | null): string {
  return priority === null ? "No priority" : `Priority ${priority}`;
}

interface PriorityBadgeProps {
  priority: number | null;
  /**
   * Makes the badge a control. Absent, it is a read-only label.
   *
   * The Monthly board reports priorities it must not let anyone edit — it is
   * read-only by design (§6.5) — so being a control is opt-in rather than the
   * default.
   */
  onChange?: ((priority: number | null) => void) | undefined;
}

/**
 * A task's priority, shown as `P8`, and the gesture that changes it.
 *
 * The number is always rendered. Colour grades it but never carries it alone:
 * hue is lost to colour-blind users, and lost again to a board sitting at 40%
 * opacity over a bright desktop.
 *
 * Editing is on the badge rather than only in the context menu because
 * priority is the one field the boards sort by — changing it moves the row,
 * and often the whole group, so it is the field people reach for most. A
 * gesture that important should not need a right-click to find.
 */
export function PriorityBadge({ priority, onChange }: PriorityBadgeProps) {
  const [open, setOpen] = useState(false);
  const label = labelFor(priority);
  const text = priority === null ? "—" : `P${priority}`;

  if (!onChange) {
    return (
      <span className="priority-badge" data-tier={tierOf(priority)} aria-label={label}>
        {text}
      </span>
    );
  }

  function choose(value: number | null) {
    onChange?.(value);
    setOpen(false);
  }

  return (
    <span
      className="priority-field"
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          event.preventDefault();
          setOpen(false);
        }
      }}
    >
      <button
        type="button"
        className="priority-badge"
        data-tier={tierOf(priority)}
        aria-label={label}
        aria-expanded={open}
        onClick={() => setOpen((current) => !current)}
      >
        {text}
      </button>

      {open && (
        <div className="priority-popover">
          <div className="priority-levels">
            {LEVELS.map((level) => (
              <button
                key={level}
                type="button"
                className="priority-level"
                data-tier={tierOf(level)}
                aria-label={`Priority ${level}`}
                // Marks the level already set, so the picker says where you
                // are rather than only where you could go.
                aria-current={level === priority ? true : undefined}
                onClick={() => choose(level)}
              >
                {level}
              </button>
            ))}
          </div>

          {priority !== null && (
            <button
              type="button"
              className="priority-clear"
              aria-label="Clear priority"
              onClick={() => choose(null)}
            >
              Clear
            </button>
          )}
        </div>
      )}
    </span>
  );
}
