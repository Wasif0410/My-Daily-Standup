import { useState } from "react";
import { DurationField } from "@/features/boards/components/DurationField";
import { PriorityBadge } from "@/features/boards/components/PriorityBadge";
import type { Task } from "@/types/task";

interface TaskRowProps {
  task: Task;
  onComplete: (completed: boolean) => void;
  onEdit: (title: string) => void;
  onSetTimeSpent: (minutes: number | null) => void;
  /**
   * Makes the priority badge a control rather than a label.
   *
   * Optional because the boards that only report progress must not let anyone
   * edit from them.
   */
  onSetPriority?: ((priority: number | null) => void) | undefined;
  onMove?: (() => void) | undefined;
  onDelete?: (() => void) | undefined;
  /** Asks the board to open a context menu at the given client coordinates. */
  onOpenMenu?: ((at: { x: number; y: number }) => void) | undefined;
  /**
   * Set by the board to open this row's title editor from outside — the
   * context menu's "Edit title". The menu cannot type into the row, and a
   * second editor living in the menu would be a second place to get wrong.
   */
  editingRequested?: boolean | undefined;
  onEditingHandled?: (() => void) | undefined;
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
  onSetPriority,
  onMove,
  onDelete,
  onOpenMenu,
  editingRequested = false,
  onEditingHandled,
}: TaskRowProps) {
  const [draft, setDraft] = useState<string | null>(null);
  const completed = task.status === "completed";

  // Derived, never copied into state by an effect. The board asking for an
  // edit and the row opening one are the same fact; mirroring it into `draft`
  // would mean two sources of truth and a cascading render to sync them.
  const editing = draft !== null || editingRequested;
  const text = draft ?? task.title;

  function closeEditor() {
    setDraft(null);
    // Clears the board's request too, or the editor would reopen on the next
    // render.
    onEditingHandled?.();
  }

  function commitEdit() {
    const next = text.trim();
    closeEditor();

    // An empty title leaves nothing to double-click, so the row could never be
    // renamed again. Reject rather than accept and strand it.
    if (!next || next === task.title) return;

    onEdit(next);
  }

  /**
   * Row-level shortcuts.
   *
   * Fires only when the row itself is the event target. Without that guard `e`
   * would re-open the editor instead of typing an `e`, and Delete would delete
   * the task instead of a character — the row's own children handle their own
   * keys.
   */
  function onKeyDown(event: React.KeyboardEvent<HTMLLIElement>) {
    if (event.target !== event.currentTarget) return;

    if (event.shiftKey && event.key === "F10") {
      // The standard keyboard route to a context menu. Without it the whole
      // interaction set below is mouse-only.
      event.preventDefault();
      openMenuAt(event.currentTarget);
      return;
    }

    switch (event.key) {
      case "Enter":
      case " ":
        event.preventDefault();
        onComplete(!completed);
        break;
      case "e":
        event.preventDefault();
        setDraft(task.title);
        break;
      case "Delete":
        event.preventDefault();
        onDelete?.();
        break;
      case "ContextMenu":
        event.preventDefault();
        openMenuAt(event.currentTarget);
        break;
    }
  }

  /** Anchors a keyboard-opened menu to the row, since there is no pointer. */
  function openMenuAt(element: HTMLElement) {
    const box = element.getBoundingClientRect();
    onOpenMenu?.({ x: box.left, y: box.bottom });
  }

  return (
    <li
      className="task-row"
      data-completed={completed}
      tabIndex={0}
      onKeyDown={onKeyDown}
      onContextMenu={(event) => {
        if (!onOpenMenu) return;
        event.preventDefault();
        onOpenMenu({ x: event.clientX, y: event.clientY });
      }}
    >
      <PriorityBadge priority={task.priority} onChange={onSetPriority} />

      <input
        type="checkbox"
        className="task-checkbox"
        aria-label={task.title}
        checked={completed}
        onChange={(event) => onComplete(event.target.checked)}
      />

      {editing ? (
        <input
          className="task-title-input"
          aria-label="Edit title"
          autoFocus
          value={text}
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
              closeEditor();
            }
          }}
        />
      ) : (
        <span
          className="task-title"
          title={task.title}
          onDoubleClick={() => setDraft(task.title)}
        >
          {task.title}
        </span>
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
