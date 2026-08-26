import { useRef, useState, type ReactNode } from "react";
import type { Task } from "@/types/task";

interface TaskGroupProps {
  /** The heading: an area on the Priority board, a project on the Weekly one. */
  label: string;
  tasks: Task[];
  /**
   * Put beside the heading — in practice the board's `AddTaskHere`.
   *
   * A slot rather than the button itself, for the same reason as
   * `renderTask`: what "add a task here" means differs per board, and a group
   * that knew would have to be written twice.
   */
  action?: ReactNode | undefined;
  /**
   * Offered only for a group the user declared.
   *
   * A derived group is the shape of the data — "Unsorted" is not a name
   * anyone chose, and renaming it would mean rewriting the area of every task
   * that merely lacks one. Absent, the heading stays a plain heading.
   */
  onRename?: ((title: string) => void) | undefined;
  /** Closed groups keep their heading and their count, and drop their rows. */
  collapsed?: boolean | undefined;
  onToggle?: ((collapsed: boolean) => void) | undefined;
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
export function TaskGroup({
  label,
  tasks,
  action,
  onRename,
  collapsed = false,
  onToggle,
  renderTask,
}: TaskGroupProps) {
  const headingId = `group-${label.replace(/\s+/g, "-").toLowerCase()}`;
  const listId = `${headingId}-tasks`;

  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState("");
  /** Set by Escape so the teardown that follows does not commit the draft. */
  const discarded = useRef(false);

  function begin() {
    if (!onRename) return;
    setDraft(label);
    setEditing(true);
  }

  function commit() {
    setEditing(false);
    if (discarded.current) {
      discarded.current = false;
      return;
    }

    const next = draft.trim();
    // A group with no name cannot be found again, and renaming to nothing
    // would blank the area on every task filed under it.
    if (!next || next === label) return;
    onRename?.(next);
  }

  return (
    <section className="board-section">
      {/* The action sits beside the heading, not inside it: a button nested in
          the h2 becomes part of the heading's accessible name, so "Health"
          would be announced as "Health +". */}
      <div className="board-section-header">
        {/* Collapsing lives on the chevron rather than on the heading, because
            the heading is already the rename target and one gesture cannot
            mean two things. */}
        <button
          type="button"
          className="board-section-chevron"
          aria-expanded={!collapsed}
          aria-controls={listId}
          aria-label={`${collapsed ? "Expand" : "Collapse"} ${label}`}
          onClick={() => onToggle?.(!collapsed)}
        >
          <span aria-hidden="true">{collapsed ? "›" : "⌄"}</span>
        </button>

        {editing ? (
          <input
            className="board-section-rename"
            aria-label={`Rename ${label}`}
            autoFocus
            value={draft}
            onChange={(event) => setDraft(event.target.value)}
            onBlur={commit}
            onKeyDown={(event) => {
              if (event.key === "Enter") {
                event.preventDefault();
                commit();
              } else if (event.key === "Escape") {
                event.preventDefault();
                discarded.current = true;
                setEditing(false);
              }
            }}
          />
        ) : (
          <h2
            className="board-section-title"
            id={headingId}
            data-renameable={onRename ? true : undefined}
            {...(onRename
              ? {
                  tabIndex: 0,
                  onClick: begin,
                  onKeyDown: (event: React.KeyboardEvent) => {
                    if (event.key === "Enter") {
                      event.preventDefault();
                      begin();
                    }
                  },
                }
              : {})}
          >
            {label}
          </h2>
        )}
        {action}
        {/* Last, so it lands hard against the board's edge and the figures
            form a column. Placed before the action it would sit wherever the
            heading's words happened to end, which reads as floating in the
            middle of the row.

            A closed group still reports its size: the whole point of closing
            one is to stop reading it, which only works if the figure stays. */}
        <span className="board-section-count">{tasks.length}</span>
      </div>
      {!collapsed && (
        <ul className="task-list-plain" id={listId} aria-labelledby={headingId}>
          {tasks.map(renderTask)}
        </ul>
      )}
    </section>
  );
}
