/**
 * Board section state — the named task groups a board is divided into.
 *
 * Shaped like `taskStore`: a mutation is applied locally first where there is
 * something sensible to guess, and a rejection restores the exact list captured
 * before the guess. Renaming a heading has to feel instant, but a failed write
 * must never leave half a change on screen.
 *
 * A section holds no contents. The rows under a heading are tasks, owned by
 * `taskStore` and matched on `tasks.area` / `tasks.project`; this store never
 * keeps a copy of them, because two stores holding the same rows is two answers
 * to the same question.
 *
 * That ownership is also why only two of these operations announce anything.
 * A section change is not itself news — one board owns its sections, and one
 * window shows that board, so no other store holds a copy to keep in step. But
 * a rename and a delete both rewrite `tasks.area` / `tasks.project` in Rust,
 * which leaves every loaded task list stale. Those two emit the cross-window
 * task-changed event so the boards reload their *tasks*. Adding a section
 * touches no task and stays silent.
 */

import { create } from "zustand";
import {
  createSection,
  deleteSection,
  listSections,
  renameSection,
  toCommandError,
} from "@/lib/ipc";
import { emitTaskChanged } from "@/lib/taskEvents";
import type { BoardKind } from "@/types/board";
import type { BoardSection } from "@/types/section";
import type { CommandError } from "@/types/task";

/**
 * Orders sections by `position`.
 *
 * Applied after every operation rather than only after `load`, because the UI
 * renders straight from this array: a created section, or one the backend
 * placed elsewhere, would otherwise sit wherever the local splice happened to
 * put it.
 */
export function sortSections(sections: BoardSection[]): BoardSection[] {
  return [...sections].sort((a, b) => a.position - b.position);
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
}

/** Applies `change` to one section, leaving every other section untouched. */
function inSection(
  sections: BoardSection[],
  id: string,
  change: (section: BoardSection) => BoardSection,
): BoardSection[] {
  return sections.map((section) => (section.id === id ? change(section) : section));
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
   *
   * `announce` says whether the mutation also moved tasks in Rust. It fires
   * only after a successful commit: a rolled-back mutation changed nothing, and
   * telling the other windows otherwise makes them all reload for no reason.
   */
  async function mutate(
    guess: ((sections: BoardSection[]) => BoardSection[]) | null,
    commit: (previous: BoardSection[]) => Promise<BoardSection[]>,
    announce = false,
  ) {
    const previous = get().sections;

    if (guess) {
      set({ sections: sortSections(guess(previous)), error: null });
    }

    try {
      set({ sections: sortSections(await commit(previous)), error: null });

      if (announce) {
        emitTaskChanged();
      }
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
      // No announcement either — an empty heading files no task under itself.
      await mutate(null, async (previous) => [
        ...previous,
        await createSection(kind, title),
      ]);
    },

    async renameSection(id, title) {
      // Announced: Rust rewrites the `tasks.area` / `tasks.project` naming this
      // group, so every board's task list is stale until it reloads. The store
      // does not patch those tasks itself — that would make it a second owner
      // of rows `taskStore` already owns.
      await mutate(
        (sections) => inSection(sections, id, (section) => ({ ...section, title })),
        async (previous) => {
          const stored = await renameSection(id, title);
          return inSection(previous, id, () => stored);
        },
        true,
      );
    },

    async removeSection(id) {
      // Only the heading goes. Its tasks were never held here, and Rust decides
      // what becomes of them — hence the announcement, for the same reason as
      // `renameSection`.
      await mutate(
        (sections) => sections.filter((section) => section.id !== id),
        async (previous) => {
          await deleteSection(id);
          return previous.filter((section) => section.id !== id);
        },
        true,
      );
    },
  };
});
