import { useRef, useState } from "react";
import type { BoardSection } from "@/types/section";

interface SectionCardProps {
  section: BoardSection;
  onRename: (id: string, title: string) => void;
  onDelete: (id: string) => void;
  onAddItem: (sectionId: string, text: string) => void;
  onUpdateItem: (id: string, text: string) => void;
  onDeleteItem: (id: string) => void;
}

/**
 * One named section and the bullets under it.
 *
 * Presentational on purpose: it holds the draft text of whatever is being
 * typed and nothing else. Every change leaves through a callback, so the card
 * can be rendered in a test without a store, a database, or a Tauri bridge —
 * the same reason {@link TaskGroup} takes a render callback rather than wiring
 * rows itself.
 *
 * Editing follows the rule the boards already use: a field commits when focus
 * leaves it, and only Escape discards. Losing a rename by clicking away is the
 * more damaging default of the two.
 */
export function SectionCard({
  section,
  onRename,
  onDelete,
  onAddItem,
  onUpdateItem,
  onDeleteItem,
}: SectionCardProps) {
  const [editingTitle, setEditingTitle] = useState(false);
  const [editingItem, setEditingItem] = useState<string | null>(null);
  const [draft, setDraft] = useState("");
  const [entry, setEntry] = useState("");

  /** Set by Escape so the teardown that follows does not commit the draft. */
  const discarded = useRef(false);

  const headingId = `section-${section.id}`;

  function beginTitleEdit() {
    setDraft(section.title);
    setEditingTitle(true);
  }

  function commitTitle() {
    setEditingTitle(false);
    if (discarded.current) {
      discarded.current = false;
      return;
    }

    const next = draft.trim();
    // A section with no name cannot be identified again afterwards, so the
    // old one stands rather than the board growing an unlabelled group.
    if (!next || next === section.title) return;
    onRename(section.id, next);
  }

  function beginItemEdit(id: string, text: string) {
    setDraft(text);
    setEditingItem(id);
  }

  function commitItem(id: string, original: string) {
    setEditingItem(null);
    if (discarded.current) {
      discarded.current = false;
      return;
    }

    const next = draft.trim();
    if (!next || next === original) return;
    onUpdateItem(id, next);
  }

  function discard() {
    discarded.current = true;
    setEditingTitle(false);
    setEditingItem(null);
  }

  return (
    <section className="section-card" aria-labelledby={headingId}>
      <div className="section-card-header">
        {editingTitle ? (
          <input
            className="section-heading-input"
            aria-label="Edit section name"
            autoFocus
            value={draft}
            onChange={(event) => setDraft(event.target.value)}
            onBlur={commitTitle}
            onKeyDown={(event) => {
              if (event.key === "Enter") {
                event.preventDefault();
                commitTitle();
              } else if (event.key === "Escape") {
                event.preventDefault();
                discard();
              }
            }}
          />
        ) : (
          <h3
            className="section-heading"
            id={headingId}
            tabIndex={0}
            onClick={beginTitleEdit}
            onKeyDown={(event) => {
              if (event.key === "Enter") {
                event.preventDefault();
                beginTitleEdit();
              }
            }}
          >
            {section.title}
          </h3>
        )}

        <button
          type="button"
          className="section-action"
          aria-label={`Delete ${section.title}`}
          onClick={() => onDelete(section.id)}
        >
          ×
        </button>
      </div>

      <ul className="section-items">
        {section.items.map((item) => (
          <li key={item.id} className="section-item">
            {editingItem === item.id ? (
              <input
                className="section-item-input"
                aria-label="Edit bullet"
                autoFocus
                value={draft}
                onChange={(event) => setDraft(event.target.value)}
                onBlur={() => commitItem(item.id, item.text)}
                onKeyDown={(event) => {
                  if (event.key === "Enter") {
                    event.preventDefault();
                    commitItem(item.id, item.text);
                  } else if (event.key === "Escape") {
                    event.preventDefault();
                    discard();
                  }
                }}
              />
            ) : (
              <>
                <span
                  className="section-item-text"
                  onClick={() => beginItemEdit(item.id, item.text)}
                >
                  {item.text}
                </span>
                <button
                  type="button"
                  className="section-action"
                  aria-label={`Delete ${item.text}`}
                  onClick={() => onDeleteItem(item.id)}
                >
                  ×
                </button>
              </>
            )}
          </li>
        ))}
      </ul>

      {/* Always present, including on a section with nothing in it yet. A new
          section is empty by definition; without the field it would be a
          heading with no way to fill it. */}
      <input
        className="section-entry"
        aria-label={`Add to ${section.title}`}
        placeholder="Add a note…"
        value={entry}
        onChange={(event) => setEntry(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Enter") {
            event.preventDefault();
            const text = entry.trim();
            if (!text) return;

            onAddItem(section.id, text);
            // Cleared rather than kept: notes arrive in bursts, and clearing
            // by hand between them doubles the gestures.
            setEntry("");
          } else if (event.key === "Escape") {
            event.preventDefault();
            setEntry("");
          }
        }}
      />
    </section>
  );
}
