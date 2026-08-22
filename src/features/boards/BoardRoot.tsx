import { useCallback, useEffect, useRef } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { saveBoardGeometry } from "@/lib/ipc";
import type { BoardKind } from "@/types/board";

/** How long the window must sit still before its position is written. */
const GEOMETRY_DEBOUNCE_MS = 500;

const TITLES: Record<BoardKind, string> = {
  priority: "Priority Tasks",
  "weekly-tasks": "Weekly Tasks",
  "weekly-progress": "Weekly Progress",
  "monthly-progress": "Monthly Progress",
};

/**
 * The root of a sticky-note window.
 *
 * Renders the chrome shared by every board and persists the window's geometry
 * as the user drags or resizes it. Board content arrives in PRs 11-14; this is
 * the frame those slot into.
 */
export function BoardRoot({ kind }: { kind: BoardKind }) {
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const persist = useCallback(() => {
    // Debounced: a drag fires dozens of move events, and writing each one
    // would hammer the database for a single final position.
    if (timer.current) clearTimeout(timer.current);

    timer.current = setTimeout(() => {
      void (async () => {
        const window = getCurrentWindow();
        const [position, size] = await Promise.all([
          window.outerPosition(),
          window.innerSize(),
        ]);

        await saveBoardGeometry(kind, position.x, position.y, size.width, size.height);
      })();
    }, GEOMETRY_DEBOUNCE_MS);
  }, [kind]);

  useEffect(() => {
    const window = getCurrentWindow();
    const unlisten = window.onMoved(persist);
    const unlistenResize = window.onResized(persist);

    return () => {
      if (timer.current) clearTimeout(timer.current);
      void unlisten.then((off) => {
        off();
      });
      void unlistenResize.then((off) => {
        off();
      });
    };
  }, [persist]);

  return (
    <div className="board" data-board={kind}>
      {/* The whole header is the drag handle: a frameless window has no
          title bar for the OS to move. */}
      <header className="board-header" data-tauri-drag-region>
        <h1>{TITLES[kind]}</h1>
        <button
          type="button"
          className="board-close"
          aria-label="Close board"
          onClick={() => void getCurrentWindow().close()}
        >
          ×
        </button>
      </header>

      <div className="board-body">
        <p className="muted">No tasks yet.</p>
      </div>
    </div>
  );
}
