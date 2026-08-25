/**
 * Board section types, mirroring `src-tauri/src/storage/section.rs`.
 *
 * Rust serialises with `rename_all = "camelCase"`, so these names must match
 * the Rust field names exactly. A mismatch does not fail to compile — the
 * frontend just silently reads `undefined`.
 */

import type { BoardKind } from "@/types/board";

/** One line of free text inside a section. */
export interface SectionItem {
  id: string;
  sectionId: string;
  text: string;
  /** Display order within the section. Owned by Rust; never invented here. */
  position: number;
}

/** A named group of items belonging to exactly one board. */
export interface BoardSection {
  id: string;
  boardKind: BoardKind;
  title: string;
  /** Display order within the board. Owned by Rust. */
  position: number;
  items: SectionItem[];
}
