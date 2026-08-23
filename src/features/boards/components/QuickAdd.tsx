import { useState } from "react";

interface QuickAddProps {
  onAdd: (title: string) => void;
  placeholder?: string | undefined;
}

/**
 * One-line task entry.
 *
 * No submit button and no form: a sticky note has no room for either, and
 * Enter is the gesture everyone already reaches for. The field clears itself
 * afterwards because planning happens in bursts — having to clear between
 * entries turns five tasks into ten gestures.
 *
 * Blank input is refused rather than creating an untitled row, which would be
 * invisible on the board and impossible to name afterwards.
 */
export function QuickAdd({ onAdd, placeholder = "Add a task…" }: QuickAddProps) {
  const [draft, setDraft] = useState("");

  return (
    <input
      className="quick-add"
      aria-label="Add a task"
      placeholder={placeholder}
      value={draft}
      onChange={(event) => setDraft(event.target.value)}
      onKeyDown={(event) => {
        if (event.key === "Enter") {
          event.preventDefault();
          const title = draft.trim();
          if (!title) return;

          onAdd(title);
          setDraft("");
        } else if (event.key === "Escape") {
          event.preventDefault();
          setDraft("");
        }
      }}
    />
  );
}
