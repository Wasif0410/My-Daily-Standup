import { useCallback, useEffect, useRef, useState } from "react";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import { BoardShell } from "@/components/BoardShell";
import { BoardMenu } from "@/features/boards/components/BoardMenu";
import { PriorityBoard } from "@/features/boards/PriorityBoard";
import { WeeklyBoard } from "@/features/boards/WeeklyBoard";
import { MonthlyBoard } from "@/features/boards/MonthlyBoard";
import { WeeklyProgressBoard } from "@/features/boards/WeeklyProgressBoard";
import {
  listBoards,
  saveBoardGeometry,
  setBoardBehavior,
  setBoardCollapsed,
  toCommandError,
} from "@/lib/ipc";
import type { BoardBehavior, BoardKind, BoardWindow } from "@/types/board";
import type { CommandError } from "@/types/task";

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
 * What goes inside a board window.
 *
 * A switch rather than a lookup table, so TypeScript flags a board that has
 * no content. Every kind now maps to a real component; adding a fifth board
 * will not compile until it is filled in.
 */
function boardContent(kind: BoardKind) {
  switch (kind) {
    case "priority":
      return <PriorityBoard />;
    case "weekly-tasks":
      return <WeeklyBoard />;
    case "weekly-progress":
      return <WeeklyProgressBoard />;
    case "monthly-progress":
      return <MonthlyBoard />;
  }
}

/**
 * The root of a sticky-note window.
 *
 * Owns the state the shell deliberately does not: saved geometry, collapse, and
 * the persistence of both. Board content is routed by {@link boardContent};
 * this is the frame each one slots into.
 */
export function BoardRoot({ kind }: { kind: BoardKind }) {
  const [board, setBoard] = useState<BoardWindow | null>(null);
  const [failure, setFailure] = useState<CommandError | null>(null);
  const [menuOpen, setMenuOpen] = useState(false);
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
        try {
          const window = getCurrentWindow();
          const width = board?.width ?? 340;
          const height = collapsed ? COLLAPSED_HEIGHT : (expandedHeight.current ?? 460);

          await window.setSize(new LogicalSize(width, height));
          await setBoardCollapsed(kind, collapsed);
        } catch (error) {
          // Roll back rather than leaving the chevron disagreeing with the
          // window. A silent failure here previously made collapse look like
          // it worked while the window stayed full height.
          setBoard((current) =>
            current ? { ...current, collapsed: !collapsed } : current,
          );
          setFailure(toCommandError(error));
        }
      })();
    },
    // Depend on the whole board: an optional chain still reads `board`, and
    // the React Compiler refuses to optimise when the stated deps are
    // narrower than the inferred ones.
    [kind, board],
  );

  /**
   * Sends a behaviour change and takes back whatever Rust actually stored.
   *
   * Not optimistic, unlike the task board interactions: a clamp or the
   * always-on-top/desktop-level exclusion can change the value on the way
   * through, and guessing would flash the wrong state before correcting it.
   */
  function changeBehavior(behavior: BoardBehavior) {
    void (async () => {
      try {
        setBoard(await setBoardBehavior(kind, behavior));
      } catch (error) {
        setFailure(toCommandError(error));
      }
    })();
  }

  return (
    <BoardShell
      kind={kind}
      title={TITLES[kind]}
      collapsed={board?.collapsed ?? false}
      locked={board?.locked ?? false}
      opacity={board?.opacity ?? 1}
      fontSize={board?.fontSize ?? 13}
      theme={board?.theme ?? "dark"}
      compact={board?.compact ?? false}
      onToggleCollapsed={toggleCollapsed}
      onClose={() => void getCurrentWindow().close()}
      headerActions={
        <button
          type="button"
          className="board-action"
          aria-label="Board settings"
          aria-expanded={menuOpen}
          onClick={() => setMenuOpen((open) => !open)}
        >
          ⚙
        </button>
      }
    >
      {failure && (
        <p className="board-error" role="alert">
          {failure.message}
        </p>
      )}
      {boardContent(kind)}

      {menuOpen && board && (
        <BoardMenu
          board={board}
          onChange={changeBehavior}
          onClose={() => setMenuOpen(false)}
        />
      )}
    </BoardShell>
  );
}
