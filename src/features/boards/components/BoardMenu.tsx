import { useEffect, useRef } from "react";
import type { ReactNode } from "react";
import type { BoardBehavior, BoardWindow } from "@/types/board";

interface BoardMenuProps {
  board: BoardWindow;
  onChange: (behavior: BoardBehavior) => void;
  onClose: () => void;
}

/** Matches the schema's CHECK constraints, so a control can stop at the edge
 *  rather than letting Rust silently absorb the click. */
const MIN_FONT = 10;
const MAX_FONT = 24;
const MIN_OPACITY = 0.2;
const OPACITY_STEP = 0.1;

/** Rounds away the float drift that repeated tenth-steps accumulate. */
function tenths(value: number): number {
  return Math.round(value * 10) / 10;
}

/**
 * Everything a board can be told to do (spec §6.7).
 *
 * Toggles report their current state through `aria-checked` rather than just
 * their label: a switch that always reads the same is a switch nobody can
 * predict.
 *
 * Stepped values — text size, opacity — use paired buttons rather than a
 * number field. A spinner inside a 340px window is a mis-click waiting to
 * happen, and the buttons can disable themselves at the limits, which tells the
 * user where the edge is instead of quietly ignoring them there.
 */
export function BoardMenu({ board, onChange, onClose }: BoardMenuProps) {
  const menu = useRef<HTMLDivElement>(null);

  useEffect(() => {
    menu.current?.querySelector<HTMLElement>('[role="menuitem"]')?.focus();
  }, []);

  useEffect(() => {
    function onPointerDown(event: MouseEvent) {
      if (!menu.current?.contains(event.target as Node)) onClose();
    }

    document.addEventListener("mousedown", onPointerDown);
    return () => document.removeEventListener("mousedown", onPointerDown);
  }, [onClose]);

  /** A toggle: fires, then dismisses. */
  function toggle(behavior: BoardBehavior) {
    onChange(behavior);
    onClose();
  }

  const nextTheme = board.theme === "dark" ? "light" : "dark";

  return (
    <div
      ref={menu}
      className="board-menu"
      role="menu"
      aria-label="Board settings"
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          event.preventDefault();
          onClose();
        }
      }}
    >
      <Toggle
        checked={board.alwaysOnTop}
        onSelect={() => toggle({ kind: "alwaysOnTop", value: !board.alwaysOnTop })}
      >
        Always on top
      </Toggle>

      <Toggle
        checked={board.desktopLevel}
        onSelect={() => toggle({ kind: "desktopLevel", value: !board.desktopLevel })}
      >
        Desktop level
      </Toggle>

      <hr className="board-menu-divider" />

      {/* Stepped in place: the menu stays open, because setting a size takes
          several clicks and closing after each one would make it unusable. */}
      <Stepper
        label="Text size"
        value={`${board.fontSize}px`}
        decreaseLabel="Smaller text"
        increaseLabel="Larger text"
        canDecrease={board.fontSize > MIN_FONT}
        canIncrease={board.fontSize < MAX_FONT}
        onDecrease={() => onChange({ kind: "fontSize", value: board.fontSize - 1 })}
        onIncrease={() => onChange({ kind: "fontSize", value: board.fontSize + 1 })}
      />

      <Stepper
        label="Opacity"
        value={`${Math.round(board.opacity * 100)}%`}
        decreaseLabel="More transparent"
        increaseLabel="More opaque"
        canDecrease={board.opacity > MIN_OPACITY}
        canIncrease={board.opacity < 1}
        onDecrease={() =>
          onChange({ kind: "opacity", value: tenths(board.opacity - OPACITY_STEP) })
        }
        onIncrease={() =>
          onChange({ kind: "opacity", value: tenths(board.opacity + OPACITY_STEP) })
        }
      />

      <hr className="board-menu-divider" />

      <Toggle
        checked={board.theme === "light"}
        onSelect={() => toggle({ kind: "theme", value: nextTheme })}
      >
        {`Switch to ${nextTheme}`}
      </Toggle>

      <Toggle
        checked={board.compact}
        onSelect={() => toggle({ kind: "compact", value: !board.compact })}
      >
        Compact
      </Toggle>

      <hr className="board-menu-divider" />

      <Toggle
        checked={board.locked}
        onSelect={() => toggle({ kind: "locked", value: !board.locked })}
      >
        {board.locked ? "Unlock position" : "Lock position"}
      </Toggle>

      {/* Stated before the fact, not discovered after losing a board. */}
      <p className="board-menu-note">
        While locked, clicks pass through to whatever is behind. Press{" "}
        <kbd>Ctrl+Alt+Shift+U</kbd> to unlock every board.
      </p>

      <hr className="board-menu-divider" />

      <Toggle
        checked={false}
        onSelect={() => toggle({ kind: "visible", value: false })}
      >
        Hide this board
      </Toggle>
    </div>
  );
}

/** A menu row that reports its own on/off state. */
function Toggle({
  children,
  checked,
  onSelect,
}: {
  children: ReactNode;
  checked: boolean;
  onSelect: () => void;
}) {
  return (
    <button
      type="button"
      role="menuitem"
      aria-checked={checked}
      className="board-menu-item"
      onClick={onSelect}
    >
      <span className="board-menu-mark" aria-hidden="true">
        {checked ? "✓" : ""}
      </span>
      {children}
    </button>
  );
}

/** A label, the current value, and a pair of bounded steps. */
function Stepper({
  label,
  value,
  decreaseLabel,
  increaseLabel,
  canDecrease,
  canIncrease,
  onDecrease,
  onIncrease,
}: {
  label: string;
  value: string;
  decreaseLabel: string;
  increaseLabel: string;
  canDecrease: boolean;
  canIncrease: boolean;
  onDecrease: () => void;
  onIncrease: () => void;
}) {
  return (
    <div className="board-menu-stepper" role="menuitem">
      <span className="board-menu-stepper-label">{label}</span>
      <button
        type="button"
        className="board-menu-step"
        aria-label={decreaseLabel}
        disabled={!canDecrease}
        onClick={onDecrease}
      >
        −
      </button>
      <span className="board-menu-stepper-value">{value}</span>
      <button
        type="button"
        className="board-menu-step"
        aria-label={increaseLabel}
        disabled={!canIncrease}
        onClick={onIncrease}
      >
        +
      </button>
    </div>
  );
}
