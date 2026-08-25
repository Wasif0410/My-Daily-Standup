/**
 * Board section state.
 *
 * Shaped like `taskStore`: a mutation is applied locally first where there is
 * something sensible to guess, and a rejection restores the exact list captured
 * before the guess. Typing into a section has to feel instant, but a failed
 * write must never leave half a change on screen.
 *
 * Unlike `taskStore`, nothing here emits the cross-window task-changed event.
 * A section belongs to one board and exactly one window shows a given board, so
 * there is no other store holding a copy to keep in step — the asymmetry is
 * deliberate, not an oversight.
 */

import { create } from "zustand";
import {
  addSectionItem,
  createSection,
  deleteSection,
  deleteSectionItem,
  listSections,
  renameSection,
  toCommandError,
  updateSectionItem,
} from "@/lib/ipc";
import type { BoardKind } from "@/types/board";
import type { BoardSection, SectionItem } from "@/types/section";
import type { CommandError } from "@/types/task";

/**
 * Orders sections, and the items within each, by `position`.
 *
 * Applied after every operation rather than only after `load`, because the UI
 * renders straight from this array: a created section or an item the backend
 * placed elsewhere would otherwise sit wherever the local splice happened to
 * put it.
 */
export function sortSections(sections: BoardSection[]): BoardSection[] {
  return [...sections]
    .sort((a, b) => a.position - b.position)
    .map((section) => ({
      ...section,
      items: [...section.items].sort((a, b) => a.position - b.position),
    }));
}

interface SectionState {
  /** An array, not a map: it is the render order, and Rust owns that order. */
  sections: BoardSection[];
  loading: boolean;
  error: CommandError | null;

  load: (kind: BoardKind) => Promise<void>;

  addSection: (kind: BoardKind, title: string) => Promise<void>;
  renameSection: (id: string, title: string) => Promise<void>;
  removeSection: (id: string) => Promise<void>;

  addItem: (sectionId: string, text: string) => Promise<void>;
  updateItem: (id: string, text: string) => Promise<void>;
  removeItem: (id: string) => Promise<void>;
}

/** Applies `change` to one section, leaving every other section untouched. */
function inSection(
  sections: BoardSection[],
  id: string,
  change: (section: BoardSection) => BoardSection,
): BoardSection[] {
  return sections.map((section) => (section.id === id ? change(section) : section));
}

/** Applies `change` to whichever section holds the item, wherever it lives. */
function inItems(
  sections: BoardSection[],
  change: (items: SectionItem[]) => SectionItem[],
): BoardSection[] {
  return sections.map((section) => ({ ...section, items: change(section.items) }));
}

export const useSectionStore = create<SectionState>((set, get) => {
  /**
   * Runs one mutation against the section list.
   *
   * `guess` is applied immediately when the change is one the frontend can
   * predict — a rename, a deletion. It is skipped when Rust owns the result,
   * as it does for anything with a generated id or position. `commit` is handed
   * the list as it stood before the guess and returns the list to keep.
   *
   * On failure that same pre-guess list is restored wholesale, so there is no
   * path on which part of a change survives a rejection.
   */
  async function mutate(
    guess: ((sections: BoardSection[]) => BoardSection[]) | null,
    commit: (previous: BoardSection[]) => Promise<BoardSection[]>,
  ) {
    const previous = get().sections;

    if (guess) {
      set({ sections: sortSections(guess(previous)), error: null });
    }

    try {
      set({ sections: sortSections(await commit(previous)), error: null });
    } catch (caught) {
      set({ sections: previous, error: toCommandError(caught) });
    }
  }

  return {
    sections: [],
    loading: false,
    error: null,

    async load(kind) {
      set({ loading: true });

      try {
        set({ sections: sortSections(await listSections(kind)), error: null });
      } catch (caught) {
        set({ error: toCommandError(caught) });
      } finally {
        set({ loading: false });
      }
    },

    async addSection(kind, title) {
      // No optimistic insert: the id and the position come from Rust, and
      // inventing either only to swap it out invites a duplicate on failure.
      await mutate(null, async (previous) => [
        ...previous,
        await createSection(kind, title),
      ]);
    },

    async renameSection(id, title) {
      await mutate(
        (sections) => inSection(sections, id, (section) => ({ ...section, title })),
        async (previous) => {
          const stored = await renameSection(id, title);
          return inSection(previous, id, () => stored);
        },
      );
    },

    async removeSection(id) {
      // The items go with the section: they are nested inside it, so dropping
      // the section drops them, and a restore brings the whole thing back.
      await mutate(
        (sections) => sections.filter((section) => section.id !== id),
        async (previous) => {
          await deleteSection(id);
          return previous.filter((section) => section.id !== id);
        },
      );
    },

    async addItem(sectionId, text) {
      // Same reasoning as `addSection`: Rust assigns the id and the position.
      await mutate(null, async (previous) => {
        const stored = await addSectionItem(sectionId, text);
        return inSection(previous, sectionId, (section) => ({
          ...section,
          items: [...section.items, stored],
        }));
      });
    },

    async updateItem(id, text) {
      await mutate(
        (sections) =>
          inItems(sections, (items) =>
            items.map((item) => (item.id === id ? { ...item, text } : item)),
          ),
        async (previous) => {
          const stored = await updateSectionItem(id, text);
          return inItems(previous, (items) =>
            items.map((item) => (item.id === id ? stored : item)),
          );
        },
      );
    },

    async removeItem(id) {
      await mutate(
        (sections) =>
          inItems(sections, (items) => items.filter((item) => item.id !== id)),
        async (previous) => {
          await deleteSectionItem(id);
          return inItems(previous, (items) => items.filter((item) => item.id !== id));
        },
      );
    },
  };
});
