import { useState } from "react";
import { SectionCard } from "@/features/sections/SectionCard";
import type { BoardSection } from "@/types/section";

interface SectionListProps {
  sections: BoardSection[];
  /** Driven by the board header's + button, which is where adding lives. */
  adding: boolean;
  onAddSection: (title: string) => void;
  /** Raised when the name field is finished with, committed or abandoned. */
  onDoneAdding: () => void;
  onRenameSection: (id: string, title: string) => void;
  onDeleteSection: (id: string) => void;
  onAddItem: (sectionId: string, text: string) => void;
  onUpdateItem: (id: string, text: string) => void;
  onDeleteItem: (id: string) => void;
}

/**
 * Every section on a board.
 *
 * Renders the array exactly as given. Ordering is the store's job — sorting
 * here as well would mean two places could disagree about what order the user
 * put things in.
 *
 * Carries no add button. Adding a section is a board-level action, so it sits
 * in the board header beside the settings gear with the board's other
 * controls; a second button down here would be two ways to do one thing, and
 * it would sit below the tasks where nobody would think to look. The header
 * owns the flag and this renders the field.
 *
 * A board with no sections and nothing being added renders nothing at all,
 * not an empty state. Sections are optional, and three of the four boards may
 * never grow one; a standing "no sections yet" would compete for a narrow
 * board's space with the tasks that are the point of it.
 */
export function SectionList({
  sections,
  adding,
  onAddSection,
  onDoneAdding,
  onRenameSection,
  onDeleteSection,
  onAddItem,
  onUpdateItem,
  onDeleteItem,
}: SectionListProps) {
  const [draft, setDraft] = useState("");

  function finish() {
    setDraft("");
    onDoneAdding();
  }

  return (
    <div className="section-list">
      {/* Above the sections, because the button that opens it is above them
          too — a field that appeared at the bottom of a scrolled list would
          leave no visible sign that the press did anything. */}
      {adding && (
        <input
          className="section-new-input"
          aria-label="New section name"
          placeholder="Section name…"
          autoFocus
          value={draft}
          onChange={(event) => setDraft(event.target.value)}
          onBlur={finish}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault();
              const title = draft.trim();
              if (!title) return;

              onAddSection(title);
              finish();
            } else if (event.key === "Escape") {
              event.preventDefault();
              finish();
            }
          }}
        />
      )}

      {sections.map((section) => (
        <SectionCard
          key={section.id}
          section={section}
          onRename={onRenameSection}
          onDelete={onDeleteSection}
          onAddItem={onAddItem}
          onUpdateItem={onUpdateItem}
          onDeleteItem={onDeleteItem}
        />
      ))}
    </div>
  );
}
