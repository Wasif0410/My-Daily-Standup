import type { ReactNode } from "react";

interface BoardHeaderProps {
  title: string;
  subtitle?: string | undefined;
  collapsed: boolean;
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
  onToggleCollapsed,
  onClose,
  actions,
}: BoardHeaderProps) {
  return (
    <header className="board-header" data-tauri-drag-region>
      {/* The heading carries the drag region too: without it, the largest
          grabbable area of the header would be dead space. */}
      <h1 className="board-title" data-tauri-drag-region>
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
