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
  /** The add field is opened from the header rather than standing open. A
   *  field under every section is a column of empty boxes down a 340px
   *  board. */
  const [adding, setAdding] = useState(false);

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

  function commitItem(id: string, original: string, thenAddAnother = false) {
    setEditingItem(null);
    if (discarded.current) {
      discarded.current = false;
      return;
    }

    // Enter on a bullet opens the next one, the way any outliner behaves. It
    // is the second route in — the header's + is the first — and it is the one
    // that matters while actually writing, because it never leaves the
    // keyboard.
    if (thenAddAnother) setAdding(true);

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

        {/* Beside the heading it fills, so the way in sits with the thing it
            acts on rather than at the far end of a list of notes. */}
        <button
          type="button"
          className="section-action"
          aria-label={`Add a note to ${section.title}`}
          onClick={() => setAdding(true)}
        >
          +
        </button>

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
                    commitItem(item.id, item.text, true);
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

      {/* Opened by the header's plus, and it stays open afterwards: notes
          arrive in bursts, so a run of them should be one Enter each rather
          than a trip back to the button between every line. */}
      {adding && (
        <input
          className="section-entry"
          aria-label={`Add to ${section.title}`}
          placeholder="Add a note…"
          autoFocus
          value={entry}
          onChange={(event) => setEntry(event.target.value)}
          onBlur={() => {
            setEntry("");
            setAdding(false);
          }}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault();
              const text = entry.trim();
              if (!text) return;

              onAddItem(section.id, text);
              setEntry("");
            } else if (event.key === "Escape") {
              event.preventDefault();
              setEntry("");
              setAdding(false);
            }
          }}
        />
      )}
    </section>
  );
}
