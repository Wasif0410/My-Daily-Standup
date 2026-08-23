import { useState } from "react";
import { DurationField } from "@/features/boards/components/DurationField";
import { PriorityBadge } from "@/features/boards/components/PriorityBadge";
import type { Task } from "@/types/task";

interface TaskRowProps {
  task: Task;
  onComplete: (completed: boolean) => void;
  onEdit: (title: string) => void;
  onSetTimeSpent: (minutes: number | null) => void;
  /** Omitted until PR 13 builds the move UI. */
  onMove?: (() => void) | undefined;
  onDelete?: (() => void) | undefined;
}

/**
 * One task, as every board renders it.
 *
 * The primitive PRs 13-15 build on, which is why it owns no data: it takes a
 * task and five callbacks and reaches for nothing else. A row that read from
 * the task store could only ever serve the one board whose filter matched.
 *
 * The single piece of state it does own is the in-progress title edit, which
 * belongs here — it is scratch text that has been committed nowhere and has no
 * meaning outside this row.
 */
export function TaskRow({
  task,
  onComplete,
  onEdit,
  onSetTimeSpent,
  onMove,
  onDelete,
}: TaskRowProps) {
  const [draft, setDraft] = useState<string | null>(null);
  const completed = task.status === "completed";

  function commitEdit() {
    const next = draft?.trim() ?? "";
    setDraft(null);

    // An empty title leaves nothing to double-click, so the row could never be
    // renamed again. Reject rather than accept and strand it.
    if (!next || next === task.title) return;

    onEdit(next);
  }

  return (
    <li className="task-row" data-completed={completed}>
      <PriorityBadge priority={task.priority} />

      <input
        type="checkbox"
        className="task-checkbox"
        aria-label={task.title}
        checked={completed}
        onChange={(event) => onComplete(event.target.checked)}
      />

      {draft === null ? (
        <span
          className="task-title"
          title={task.title}
          onDoubleClick={() => setDraft(task.title)}
        >
          {task.title}
        </span>
      ) : (
        <input
          className="task-title-input"
          aria-label="Edit title"
          autoFocus
          value={draft}
          onChange={(event) => setDraft(event.target.value)}
          // Blur commits rather than cancels: clicking away from a rename you
          // just typed and watching it vanish is the more damaging default.
          onBlur={commitEdit}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault();
              commitEdit();
            } else if (event.key === "Escape") {
              event.preventDefault();
              setDraft(null);
            }
          }}
        />
      )}

      <DurationField minutes={task.timeSpentMinutes} onChange={onSetTimeSpent} />

      <span className="task-actions">
        {onMove && (
          <button
            type="button"
            className="task-action"
            aria-label="Move task"
            onClick={onMove}
          >
            ⇄
          </button>
        )}
        {onDelete && (
          <button
            type="button"
            className="task-action"
            aria-label="Delete task"
            onClick={onDelete}
          >
            ×
          </button>
        )}
      </span>
    </li>
  );
}
