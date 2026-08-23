import type { ReactNode } from "react";
import type { Task } from "@/types/task";

interface TaskGroupProps {
  /** The heading: an area on the Priority board, a project on the Weekly one. */
  label: string;
  tasks: Task[];
  /** Rendering is the caller's, so a group is reusable by any board. */
  renderTask: (task: Task) => ReactNode;
}

/**
 * One group of tasks, under a heading.
 *
 * Deliberately knows nothing about *what* it is grouping. The Priority board
 * groups by area and the Weekly board by project; a component that named either
 * one would have to be written twice.
 *
 * Takes a render callback rather than wiring `TaskRow` itself, for the same
 * reason: each board needs its own handlers on the row.
 */
export function TaskGroup({ label, tasks, renderTask }: TaskGroupProps) {
  const headingId = `group-${label.replace(/\s+/g, "-").toLowerCase()}`;

  return (
    <section className="board-section">
      <h2 className="board-section-title" id={headingId}>
        {label}
      </h2>
      <ul className="task-list-plain" aria-labelledby={headingId}>
        {tasks.map(renderTask)}
      </ul>
    </section>
  );
}
