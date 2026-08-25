import { useEffect } from "react";
import { SectionList } from "@/features/sections/SectionList";
import { useSectionStore } from "@/stores/sectionStore";
import type { BoardKind } from "@/types/board";

/**
 * A board's sections, wired to the store.
 *
 * The seam between the store and {@link SectionList}, which is deliberately
 * ignorant of both. Everything stateful lives here so the list and the cards
 * stay renderable in a test with nothing behind them.
 *
 * Loads on mount and on a change of board. A board window only ever shows one
 * kind, so in practice this runs once per window — but the dependency is real
 * and leaving it out would be a bug waiting for the first component that
 * reuses this.
 */
export function BoardSections({ kind }: { kind: BoardKind }) {
  const sections = useSectionStore((state) => state.sections);
  const load = useSectionStore((state) => state.load);
  const addSection = useSectionStore((state) => state.addSection);
  const renameSection = useSectionStore((state) => state.renameSection);
  const removeSection = useSectionStore((state) => state.removeSection);
  const addItem = useSectionStore((state) => state.addItem);
  const updateItem = useSectionStore((state) => state.updateItem);
  const removeItem = useSectionStore((state) => state.removeItem);

  useEffect(() => {
    void load(kind);
  }, [load, kind]);

  return (
    <SectionList
      sections={sections}
      onAddSection={(title) => void addSection(kind, title)}
      onRenameSection={(id, title) => void renameSection(id, title)}
      onDeleteSection={(id) => void removeSection(id)}
      onAddItem={(sectionId, text) => void addItem(sectionId, text)}
      onUpdateItem={(id, text) => void updateItem(id, text)}
      onDeleteItem={(id) => void removeItem(id)}
    />
  );
}
