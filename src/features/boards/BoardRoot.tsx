import { useCallback, useEffect, useRef, useState } from "react";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import { BoardShell } from "@/components/BoardShell";
import { listBoards, saveBoardGeometry, setBoardCollapsed } from "@/lib/ipc";
import type { BoardKind, BoardWindow } from "@/types/board";

/** How long the window must sit still before its position is written. */
const GEOMETRY_DEBOUNCE_MS = 500;

/** Height of the header alone, in logical pixels. A collapsed board shrinks
 *  to this so it really is a title bar rather than a mostly-empty window. */
const COLLAPSED_HEIGHT = 34;

const TITLES: Record<BoardKind, string> = {
  priority: "Priority Tasks",
  "weekly-tasks": "Weekly Tasks",
  "weekly-progress": "Weekly Progress",
  "monthly-progress": "Monthly Progress",
};

/**
 * The root of a sticky-note window.
 *
 * Owns the state the shell deliberately does not: saved geometry, collapse, and
 * the persistence of both. Board content arrives in PRs 12-15; this is the
 * frame those slot into.
 */
export function BoardRoot({ kind }: { kind: BoardKind }) {
  const [board, setBoard] = useState<BoardWindow | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  /** Height to restore when expanding. Kept in a ref so collapsing twice in a
   *  row cannot overwrite it with the collapsed height. */
  const expandedHeight = useRef<number | null>(null);

  useEffect(() => {
    let ignore = false;

    void (async () => {
      const boards = await listBoards();
      const mine = boards.find((b) => b.kind === kind);
      if (!ignore && mine) {
        setBoard(mine);
        expandedHeight.current = mine.height;
      }
    })();

    return () => {
      ignore = true;
    };
  }, [kind]);

  const persistGeometry = useCallback(() => {
    // Debounced: a drag fires dozens of move events, and writing each one
    // would hammer the database for a single final position.
    if (timer.current) clearTimeout(timer.current);

    timer.current = setTimeout(() => {
      void (async () => {
        const window = getCurrentWindow();
        const [position, size, factor] = await Promise.all([
          window.outerPosition(),
          window.innerSize(),
          window.scaleFactor(),
        ]);
        const logical = size.toLogical(factor);

        // Never record the collapsed height as the board's size, or expanding
        // after a restart would restore a title bar.
        if (logical.height > COLLAPSED_HEIGHT + 4) {
          expandedHeight.current = logical.height;
        }

        await saveBoardGeometry(
          kind,
          position.x,
          position.y,
          logical.width,
          expandedHeight.current ?? logical.height,
        );
      })();
    }, GEOMETRY_DEBOUNCE_MS);
  }, [kind]);

  useEffect(() => {
    const window = getCurrentWindow();
    const moved = window.onMoved(persistGeometry);
    const resized = window.onResized(persistGeometry);

    return () => {
      if (timer.current) clearTimeout(timer.current);
      void moved.then((off) => {
        off();
      });
      void resized.then((off) => {
        off();
      });
    };
  }, [persistGeometry]);

  const toggleCollapsed = useCallback(
    (collapsed: boolean) => {
      // Optimistic, like every other board interaction: the window resizes
      // immediately rather than after a database round-trip.
      setBoard((current) => (current ? { ...current, collapsed } : current));

      void (async () => {
        const window = getCurrentWindow();
        const width = board?.width ?? 340;
        const height = collapsed ? COLLAPSED_HEIGHT : (expandedHeight.current ?? 460);

        await window.setSize(new LogicalSize(width, height));
        await setBoardCollapsed(kind, collapsed);
      })();
    },
    [kind, board?.width],
  );

  return (
    <BoardShell
      kind={kind}
      title={TITLES[kind]}
      collapsed={board?.collapsed ?? false}
      locked={board?.locked ?? false}
      opacity={board?.opacity ?? 1}
      onToggleCollapsed={toggleCollapsed}
      onClose={() => void getCurrentWindow().close()}
    >
      <p className="board-empty">No tasks yet.</p>
    </BoardShell>
  );
}
