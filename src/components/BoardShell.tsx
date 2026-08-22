import type { CSSProperties, ReactNode } from "react";
import { BoardHeader } from "@/components/BoardHeader";
import type { BoardKind } from "@/types/board";

/** One accent per board, so four scattered windows are distinguishable at a
 *  glance rather than by reading their titles. */
const ACCENTS: Record<BoardKind, string> = {
  priority: "var(--accent-priority)",
  "weekly-tasks": "var(--accent-weekly-tasks)",
  "weekly-progress": "var(--accent-weekly-progress)",
  "monthly-progress": "var(--accent-monthly-progress)",
};

interface BoardShellProps {
  kind: BoardKind;
  title: string;
  /** Period or context, e.g. "2026-W34". */
  subtitle?: string;
  collapsed?: boolean;
  locked?: boolean;
  /** 0.2–1.0. Floored in the schema, since a fully transparent board would be
   *  invisible and unclickable with no way to recover it. */
  opacity?: number;
  /** Required: every board collapses, and the owner persists the choice. An
   *  optional control that silently did nothing would be worse than none. */
  onToggleCollapsed: (collapsed: boolean) => void;
  onClose?: () => void;
  /** Board-specific header controls. */
  headerActions?: ReactNode;
  children: ReactNode;
}

/**
 * The visual chassis every board renders inside.
 *
 * Owns no state. Collapse persists to the database, so the parent holds it and
 * passes it back down — otherwise the shell and the stored value could drift
 * apart after a failed write.
 *
 * Runtime-adjustable values (accent, opacity, font size) are set as custom
 * properties here so PR 16 can drive them from settings without any component
 * needing to change.
 */
export function BoardShell({
  kind,
  title,
  subtitle,
  collapsed = false,
  locked = false,
  opacity = 1,
  onToggleCollapsed,
  onClose,
  headerActions,
  children,
}: BoardShellProps) {
  const style = {
    "--board-accent": ACCENTS[kind],
    "--board-opacity": opacity,
  } as CSSProperties;

  return (
    <div
      className="board"
      data-board={kind}
      data-collapsed={collapsed}
      data-locked={locked}
      style={style}
    >
      <BoardHeader
        title={title}
        subtitle={subtitle}
        collapsed={collapsed}
        onToggleCollapsed={() => onToggleCollapsed(!collapsed)}
        onClose={onClose}
        actions={headerActions}
      />

      <div className="board-body">{children}</div>
    </div>
  );
}
