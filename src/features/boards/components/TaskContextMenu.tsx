import { useEffect, useRef, useState } from "react";
import type { CSSProperties } from "react";
import type { Task } from "@/types/task";

interface TaskContextMenuProps {
  task: Task;
  /** Where the pointer was, in client coordinates. */
  x: number;
  y: number;
  onClose: () => void;
  onComplete: (completed: boolean) => void;
  onEdit: () => void;
  onSetPriority: (priority: number | null) => void;
  onMoveToDate: (date: string) => void;
  onPromote: () => void;
  /**
   * Omitted by a board that does not know which week it is showing.
   *
   * "Next week" is only meaningful relative to a current one, and the Priority
   * board is a standing list rather than a period. The item is left out rather
   * than wired to nothing — a menu entry that does nothing is worse than an
   * absent one.
   */
  onMoveToNextWeek?: (() => void) | undefined;
  onSetBlocker: (blocker: string | null) => void;
  onAddComment: (comment: string) => void;
  onArchive: () => void;
  onDelete: () => void;
}

/** Which follow-up input the menu is showing, if any. */
type Prompt = "priority" | "date" | "blocker" | "comment" | null;

/** The priorities offered directly, plus a way back to unprioritised. */
const PRIORITY_CHOICES = [10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0];

/**
 * The full §6.6 interaction set for one task.
 *
 * A real `role="menu"`: right-click is not the only way in, and Shift+F10, the
 * Menu key, and arrow-key navigation all have to reach it. A `div` full of
 * `div`s reaches nobody.
 *
 * Actions that need a value — a priority, a date, a blocker, a comment — open a
 * prompt inside the menu rather than a separate dialog. A sticky note has no
 * room for a modal, and a browser `prompt()` would freeze the window.
 */
export function TaskContextMenu({
  task,
  x,
  y,
  onClose,
  onComplete,
  onEdit,
  onSetPriority,
  onMoveToDate,
  onPromote,
  onMoveToNextWeek,
  onSetBlocker,
  onAddComment,
  onArchive,
  onDelete,
}: TaskContextMenuProps) {
  const menu = useRef<HTMLDivElement>(null);
  const [prompt, setPrompt] = useState<Prompt>(null);
  const [draft, setDraft] = useState("");

  const completed = task.status === "completed";
  const blocked = task.blocker !== null && task.blocker !== "";

  useEffect(() => {
    // Focus the first item so the keyboard works the moment the menu opens.
    menu.current?.querySelector<HTMLElement>('[role="menuitem"]')?.focus();
  }, []);

  useEffect(() => {
    function onPointerDown(event: MouseEvent) {
      if (!menu.current?.contains(event.target as Node)) onClose();
    }

    // Captured on the document rather than a backdrop element: a backdrop
    // covering the board would swallow the very right-click that opened this.
    document.addEventListener("mousedown", onPointerDown);
    return () => document.removeEventListener("mousedown", onPointerDown);
  }, [onClose]);

  /** Runs an action and dismisses the menu. */
  function act(action: () => void) {
    action();
    onClose();
  }

  function openPrompt(next: Prompt, initial = "") {
    setPrompt(next);
    setDraft(initial);
  }

  function onKeyDown(event: React.KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      onClose();
      return;
    }

    if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;

    const items = [
      ...(menu.current?.querySelectorAll<HTMLElement>('[role="menuitem"]') ?? []),
    ];
    if (items.length === 0) return;

    event.preventDefault();
    const current = items.indexOf(document.activeElement as HTMLElement);
    const step = event.key === "ArrowDown" ? 1 : -1;
    // Wraps in both directions: a menu that dead-ends at the last item makes
    // the bottom action the hardest one to reach.
    const next = (current + step + items.length) % items.length;
    items[next]?.focus();
  }

  const style = { left: `${x}px`, top: `${y}px` } as CSSProperties;

  return (
    <div
      ref={menu}
      className="context-menu"
      role="menu"
      aria-label={`Actions for ${task.title}`}
      style={style}
      onKeyDown={onKeyDown}
    >
      {prompt === null && (
        <>
          <MenuItem onSelect={() => act(() => onComplete(!completed))}>
            {completed ? "Uncomplete" : "Complete"}
          </MenuItem>
          <MenuItem onSelect={() => act(onEdit)}>Edit title</MenuItem>
          <MenuItem onSelect={() => openPrompt("priority")}>Set priority…</MenuItem>

          <hr className="context-menu-divider" />

          <MenuItem onSelect={() => openPrompt("date", task.scheduledDate ?? "")}>
            Move to another day…
          </MenuItem>
          {onMoveToNextWeek && (
            <MenuItem onSelect={() => act(onMoveToNextWeek)}>
              Move to next week
            </MenuItem>
          )}
          {task.horizon === "daily" && (
            <MenuItem onSelect={() => act(onPromote)}>Promote to weekly</MenuItem>
          )}

          <hr className="context-menu-divider" />

          {blocked ? (
            <MenuItem onSelect={() => act(() => onSetBlocker(null))}>
              Resolve blocker
            </MenuItem>
          ) : (
            <MenuItem onSelect={() => openPrompt("blocker")}>Add blocker…</MenuItem>
          )}
          <MenuItem onSelect={() => openPrompt("comment")}>Add comment…</MenuItem>

          <hr className="context-menu-divider" />

          <MenuItem onSelect={() => act(onArchive)}>Archive</MenuItem>
          <MenuItem danger onSelect={() => act(onDelete)}>
            Delete
          </MenuItem>
        </>
      )}

      {prompt === "priority" && (
        <div className="context-menu-priorities">
          {PRIORITY_CHOICES.map((value) => (
            <MenuItem key={value} onSelect={() => act(() => onSetPriority(value))}>
              {`P${value}`}
            </MenuItem>
          ))}
          <MenuItem onSelect={() => act(() => onSetPriority(null))}>None</MenuItem>
        </div>
      )}

      {prompt === "date" && (
        <PromptField
          label="New date"
          type="date"
          value={draft}
          placeholder="2026-08-25"
          onChange={setDraft}
          onCommit={(value) => act(() => onMoveToDate(value))}
          onCancel={onClose}
        />
      )}

      {prompt === "blocker" && (
        <PromptField
          label="Blocker"
          value={draft}
          placeholder="What is holding this up?"
          onChange={setDraft}
          onCommit={(value) => act(() => onSetBlocker(value))}
          onCancel={onClose}
        />
      )}

      {prompt === "comment" && (
        <PromptField
          label="Comment"
          value={draft}
          placeholder="A short note"
          onChange={setDraft}
          onCommit={(value) => act(() => onAddComment(value))}
          onCancel={onClose}
        />
      )}
    </div>
  );
}

/**
 * One menu row.
 *
 * A `button` rather than a styled `div`: it is focusable, Enter and Space
 * already activate it, and the `menuitem` role sits on something that behaves
 * like one.
 */
function MenuItem({
  children,
  danger = false,
  onSelect,
}: {
  children: React.ReactNode;
  danger?: boolean;
  onSelect: () => void;
}) {
  return (
    <button
      type="button"
      role="menuitem"
      className="context-menu-item"
      data-danger={danger}
      onClick={onSelect}
    >
      {children}
    </button>
  );
}

/**
 * A follow-up input inside the menu.
 *
 * Enter commits, Escape cancels. Blank input is refused rather than committed —
 * an empty blocker or comment says nothing and would still overwrite state.
 */
function PromptField({
  label,
  type = "text",
  value,
  placeholder,
  onChange,
  onCommit,
  onCancel,
}: {
  label: string;
  type?: string;
  value: string;
  placeholder: string;
  onChange: (value: string) => void;
  onCommit: (value: string) => void;
  onCancel: () => void;
}) {
  return (
    <label className="context-menu-prompt">
      <span className="context-menu-prompt-label">{label}</span>
      <input
        className="context-menu-input"
        type={type}
        aria-label={label}
        placeholder={placeholder}
        autoFocus
        value={value}
        onChange={(event) => onChange(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Enter") {
            event.preventDefault();
            const trimmed = value.trim();
            if (trimmed) onCommit(trimmed);
          } else if (event.key === "Escape") {
            event.preventDefault();
            onCancel();
          }
        }}
      />
    </label>
  );
}
