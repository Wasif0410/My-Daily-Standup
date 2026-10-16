import type { ReactNode } from "react";

interface BoardEmptyProps {
  /** What the board is for, in a few words. */
  title: string;
  /** What will appear here, and how it gets here. */
  hint: string;
  /** The board's first action, when it has one. */
  children?: ReactNode;
}

/**
 * What a board says before it has anything on it.
 *
 * An empty board is the first thing a new user sees, and a single grey line
 * ("Nothing planned this week.") left them with nowhere to go. This says what
 * the board is for and, where the board can create anything, offers the way
 * to start.
 */
export function BoardEmpty({ title, hint, children }: BoardEmptyProps) {
  return (
    <div className="board-empty">
      <p className="board-empty-title">{title}</p>
      <p className="board-empty-hint">{hint}</p>
      {children && <div className="board-empty-action">{children}</div>}
    </div>
  );
}
