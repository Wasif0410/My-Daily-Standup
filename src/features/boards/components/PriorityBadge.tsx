/** Where a priority sits on the 0-10 scale, for colour grading. */
type Tier = "high" | "medium" | "low" | "none";

function tierOf(priority: number | null): Tier {
  if (priority === null) return "none";
  if (priority >= 8) return "high";
  if (priority >= 4) return "medium";
  return "low";
}

/**
 * A task's priority, shown as `P8`.
 *
 * The number is always rendered. Colour grades it but never carries it alone:
 * hue is lost to colour-blind users, and lost again to a board sitting at 40%
 * opacity over a bright desktop.
 */
export function PriorityBadge({ priority }: { priority: number | null }) {
  return (
    <span
      className="priority-badge"
      data-tier={tierOf(priority)}
      aria-label={priority === null ? "No priority" : `Priority ${priority}`}
    >
      {priority === null ? "—" : `P${priority}`}
    </span>
  );
}
