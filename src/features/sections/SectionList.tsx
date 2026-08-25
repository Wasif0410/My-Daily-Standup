import { useState } from "react";
import { SectionCard } from "@/features/sections/SectionCard";
import type { BoardSection } from "@/types/section";

interface SectionListProps {
  sections: BoardSection[];
  onAddSection: (title: string) => void;
  onRenameSection: (id: string, title: string) => void;
  onDeleteSection: (id: string) => void;
  onAddItem: (sectionId: string, text: string) => void;
  onUpdateItem: (id: string, text: string) => void;
  onDeleteItem: (id: string) => void;
}

/**
 * Every section on a board, and the way to add another.
 *
 * Renders the array exactly as given. Ordering is the store's job — sorting
 * here as well would mean two places could disagree about what order the user
 * put things in.
 *
 * A board with no sections renders nothing at all, not an empty state.
 * Sections are optional, and three of the four boards may never grow one; a
 * standing "no sections yet" message would compete for a narrow board's space
 * with the tasks that are the point of it.
 */
export function SectionList({
  sections,
  onAddSection,
  onRenameSection,
  onDeleteSection,
  onAddItem,
  onUpdateItem,
  onDeleteItem,
}: SectionListProps) {
  const [adding, setAdding] = useState(false);
  const [draft, setDraft] = useState("");

  function stopAdding() {
    setAdding(false);
    setDraft("");
  }

  return (
    <div className="section-list">
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

      {adding ? (
        <input
          className="section-new-input"
          aria-label="New section name"
          placeholder="Section name…"
          autoFocus
          value={draft}
          onChange={(event) => setDraft(event.target.value)}
          onBlur={stopAdding}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault();
              const title = draft.trim();
              if (!title) return;

              onAddSection(title);
              stopAdding();
            } else if (event.key === "Escape") {
              event.preventDefault();
              stopAdding();
            }
          }}
        />
      ) : (
        <button
          type="button"
          className="section-add"
          aria-label="Add a section"
          onClick={() => setAdding(true)}
        >
          <span aria-hidden="true">+</span> Section
        </button>
      )}
    </div>
  );
}
