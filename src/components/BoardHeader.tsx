import type { ReactNode } from "react";

interface BoardHeaderProps {
  title: string;
  subtitle?: string | undefined;
  collapsed: boolean;
  /** A locked board must not move, so the drag region comes off entirely. */
  locked?: boolean;
  onToggleCollapsed: () => void;
  onClose?: (() => void) | undefined;
  /** Board-specific controls, placed before the shared ones. */
  actions?: ReactNode;
}

/**
 * A board's title bar.
 *
 * Doubles as the window's drag region — a frameless window has no OS title bar,
 * so without `data-tauri-drag-region` the board cannot be moved at all.
 */
export function BoardHeader({
  title,
  subtitle,
  collapsed,
  locked = false,
  onToggleCollapsed,
  onClose,
  actions,
}: BoardHeaderProps) {
  // Dropped entirely when locked rather than merely ignored.
  // `set_ignore_cursor_events` stops clicks reaching the webview, but a board
  // locked while the pointer is already over its header would otherwise still
  // be draggable — and "locked" would be a half-truth.
  const drag = locked ? {} : { "data-tauri-drag-region": true };

  return (
    <header className="board-header" {...drag}>
      {/* The heading carries the drag region too: without it, the largest
          grabbable area of the header would be dead space. */}
      <h1 className="board-title" {...drag}>
        {title}
      </h1>

      {subtitle && <span className="board-subtitle">{subtitle}</span>}

      <div className="board-actions">
        {actions}

        <button
          type="button"
          className="board-action"
          aria-label={collapsed ? "Expand board" : "Collapse board"}
          aria-expanded={!collapsed}
          onClick={onToggleCollapsed}
        >
          {collapsed ? "▸" : "▾"}
        </button>

        {onClose && (
          <button
            type="button"
            className="board-action"
            aria-label="Close board"
            onClick={onClose}
          >
            ×
          </button>
        )}
      </div>
    </header>
  );
}
