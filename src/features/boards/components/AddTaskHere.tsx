import { useState } from "react";

interface AddTaskHereProps {
  /** The group's heading, used to name the control and nothing else. */
  label: string;
  onAdd: (title: string) => void;
}

/**
 * The way into one group: a + in its heading that opens a one-line field.
 *
 * Presentational, like {@link TaskRow} and {@link TaskGroup}. It knows the name
 * of the group only well enough to say it out loud; what an `area` or a
 * `project` is, and which of the two this heading stands for, is the board's
 * business. That is what lets both boards share it while creating quite
 * different tasks.
 *
 * It replaced the board-wide quick-add, which could not say where a task
 * belonged — everything it made landed under the unfiled heading and had to be
 * moved by hand. Attaching entry to a heading makes the filing a side effect of
 * where you typed.
 *
 * The field is opened rather than standing open. One per group down a 340px
 * board is a column of empty boxes competing with the tasks that are the point
 * of it.
 *
 * Enter commits and clears but leaves the field open: planning happens in
 * bursts, and closing between entries would turn five tasks into ten gestures.
 * Escape closes and discards. Blank input is refused rather than creating an
 * untitled row, which would be invisible on the board and impossible to name
 * afterwards.
 */
export function AddTaskHere({ label, onAdd }: AddTaskHereProps) {
  const [open, setOpen] = useState(false);
  const [draft, setDraft] = useState("");

  function close() {
    // The draft goes with the field. Reopening onto half a thought typed
    // minutes ago and abandoned is worse than an empty box.
    setDraft("");
    setOpen(false);
  }

  if (!open) {
    return (
      <button
        type="button"
        className="board-section-action"
        // Named after the group: every heading on the board carries one of
        // these, and "Add a task" alone would be a row of identical buttons to
        // anyone not looking at the screen.
        aria-label={`Add a task to ${label}`}
        onClick={() => setOpen(true)}
      >
        +
      </button>
    );
  }

  return (
    <input
      className="board-section-add"
      aria-label={`Add to ${label}`}
      placeholder="Add a task…"
      autoFocus
      value={draft}
      onChange={(event) => setDraft(event.target.value)}
      // Clicking away is finishing, not committing: an unsent line is scratch
      // text, unlike a rename of something that already exists.
      onBlur={close}
      onKeyDown={(event) => {
        if (event.key === "Enter") {
          event.preventDefault();
          const title = draft.trim();
          // Refused, not closed. Pressing Enter on an empty field is a slip,
          // and shutting the field would punish it.
          if (!title) return;

          onAdd(title);
          setDraft("");
        } else if (event.key === "Escape") {
          event.preventDefault();
          close();
        }
      }}
    />
  );
}
