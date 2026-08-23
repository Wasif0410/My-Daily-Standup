import type { ReactNode } from "react";
import type { Task } from "@/types/task";

interface AreaGroupProps {
  area: string;
  tasks: Task[];
  /** Rendering is the caller's, so a group is reusable by any board. */
  renderTask: (task: Task) => ReactNode;
}

/**
 * One area's worth of tasks, under a heading.
 *
 * Takes a render callback rather than wiring `TaskRow` itself: PR 13 groups by
 * project and PR 14 by day, and each needs its own handlers on the row. A group
 * that hard-coded the row would be rewritten three times.
 */
export function AreaGroup({ area, tasks, renderTask }: AreaGroupProps) {
  const headingId = `area-${area.replace(/\s+/g, "-").toLowerCase()}`;

  return (
    <section className="board-section">
      <h2 className="board-section-title" id={headingId}>
        {area}
      </h2>
      <ul className="task-list-plain" aria-labelledby={headingId}>
        {tasks.map(renderTask)}
      </ul>
    </section>
  );
}
