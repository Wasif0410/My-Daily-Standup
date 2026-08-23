import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { BoardMenu } from "@/features/boards/components/BoardMenu";
import type { BoardWindow } from "@/types/board";

function board(overrides: Partial<BoardWindow> = {}): BoardWindow {
  return {
    kind: "priority",
    x: 0,
    y: 0,
    width: 340,
    height: 460,
    monitor: null,
    visible: true,
    collapsed: false,
    opacity: 1,
    alwaysOnTop: false,
    locked: false,
    fontSize: 13,
    theme: "dark",
    compact: false,
    desktopLevel: false,
    ...overrides,
  };
}

function props(overrides: Record<string, unknown> = {}) {
  return {
    board: board(),
    onChange: vi.fn(),
    onClose: vi.fn(),
    ...overrides,
  };
}

describe("BoardMenu", () => {
  it("offers every behaviour the spec lists", () => {
    render(<BoardMenu {...props()} />);

    for (const label of [
      /always on top/i,
      /desktop level/i,
      /lock/i,
      /transparency|opacity/i,
      /text size/i,
      /theme|light|dark/i,
      /compact/i,
      /hide/i,
    ]) {
      expect(screen.getByRole("menuitem", { name: label })).toBeInTheDocument();
    }
  });

  it("is a real menu", () => {
    render(<BoardMenu {...props()} />);

    expect(screen.getByRole("menu")).toBeInTheDocument();
  });

  describe("reflecting the current state", () => {
    it("shows a board that is already on top as on", () => {
      // A toggle that always reads "off" is a dead switch: the user cannot
      // tell what turning it does.
      render(<BoardMenu {...props({ board: board({ alwaysOnTop: true }) })} />);

      expect(screen.getByRole("menuitem", { name: /always on top/i })).toHaveAttribute(
        "aria-checked",
        "true",
      );
    });

    it("shows a board that is not on top as off", () => {
      render(<BoardMenu {...props()} />);

      expect(screen.getByRole("menuitem", { name: /always on top/i })).toHaveAttribute(
        "aria-checked",
        "false",
      );
    });

    it("shows the current text size", () => {
      render(<BoardMenu {...props({ board: board({ fontSize: 17 }) })} />);

      expect(screen.getByText("17px")).toBeInTheDocument();
    });

    it("shows the current opacity as a percentage", () => {
      render(<BoardMenu {...props({ board: board({ opacity: 0.7 }) })} />);

      expect(screen.getByText("70%")).toBeInTheDocument();
    });

    it("names the theme it would switch to", () => {
      render(<BoardMenu {...props({ board: board({ theme: "dark" }) })} />);

      expect(
        screen.getByRole("menuitem", { name: /switch to light/i }),
      ).toBeInTheDocument();
    });
  });

  describe("dispatching", () => {
    it("toggles always on top", async () => {
      const onChange = vi.fn();
      const user = userEvent.setup();
      render(<BoardMenu {...props({ onChange })} />);

      await user.click(screen.getByRole("menuitem", { name: /always on top/i }));

      expect(onChange).toHaveBeenCalledWith({ kind: "alwaysOnTop", value: true });
    });

    it("toggles desktop level", async () => {
      const onChange = vi.fn();
      const user = userEvent.setup();
      render(<BoardMenu {...props({ onChange })} />);

      await user.click(screen.getByRole("menuitem", { name: /desktop level/i }));

      expect(onChange).toHaveBeenCalledWith({ kind: "desktopLevel", value: true });
    });

    it("locks the board", async () => {
      const onChange = vi.fn();
      const user = userEvent.setup();
      render(<BoardMenu {...props({ onChange })} />);

      await user.click(screen.getByRole("menuitem", { name: /lock/i }));

      expect(onChange).toHaveBeenCalledWith({ kind: "locked", value: true });
    });

    it("switches the theme", async () => {
      const onChange = vi.fn();
      const user = userEvent.setup();
      render(<BoardMenu {...props({ onChange })} />);

      await user.click(screen.getByRole("menuitem", { name: /switch to light/i }));

      expect(onChange).toHaveBeenCalledWith({ kind: "theme", value: "light" });
    });

    it("toggles compact density", async () => {
      const onChange = vi.fn();
      const user = userEvent.setup();
      render(<BoardMenu {...props({ onChange })} />);

      await user.click(screen.getByRole("menuitem", { name: /compact/i }));

      expect(onChange).toHaveBeenCalledWith({ kind: "compact", value: true });
    });

    it("hides the board", async () => {
      const onChange = vi.fn();
      const user = userEvent.setup();
      render(<BoardMenu {...props({ onChange })} />);

      await user.click(screen.getByRole("menuitem", { name: /hide/i }));

      expect(onChange).toHaveBeenCalledWith({ kind: "visible", value: false });
    });
  });

  describe("stepped values", () => {
    it("steps the text size up and down rather than offering a free number", async () => {
      // A number spinner in a 340px board is a mis-click waiting to happen.
      const onChange = vi.fn();
      const user = userEvent.setup();
      render(<BoardMenu {...props({ onChange })} />);

      await user.click(screen.getByRole("button", { name: /larger text/i }));
      expect(onChange).toHaveBeenCalledWith({ kind: "fontSize", value: 14 });

      await user.click(screen.getByRole("button", { name: /smaller text/i }));
      expect(onChange).toHaveBeenCalledWith({ kind: "fontSize", value: 12 });
    });

    it("will not step the text size past its limits", async () => {
      const user = userEvent.setup();
      const { rerender } = render(
        <BoardMenu {...props({ board: board({ fontSize: 24 }) })} />,
      );
      expect(screen.getByRole("button", { name: /larger text/i })).toBeDisabled();

      rerender(<BoardMenu {...props({ board: board({ fontSize: 10 }) })} />);
      expect(screen.getByRole("button", { name: /smaller text/i })).toBeDisabled();
      await user.click(screen.getByRole("button", { name: /larger text/i }));
    });

    it("steps opacity in tenths", async () => {
      const onChange = vi.fn();
      const user = userEvent.setup();
      render(<BoardMenu {...props({ onChange })} />);

      await user.click(screen.getByRole("button", { name: /more transparent/i }));

      expect(onChange).toHaveBeenCalledWith({ kind: "opacity", value: 0.9 });
    });

    it("will not step opacity below the floor", () => {
      // Past 0.2 a board is invisible and unclickable. The control stops
      // rather than letting the clamp silently absorb the click.
      render(<BoardMenu {...props({ board: board({ opacity: 0.2 }) })} />);

      expect(screen.getByRole("button", { name: /more transparent/i })).toBeDisabled();
    });
  });

  it("warns that locking makes the board click-through, and names the way out", () => {
    // The user must not discover this by losing a board.
    render(<BoardMenu {...props()} />);

    expect(screen.getByText(/clicks pass through/i)).toBeInTheDocument();
    expect(screen.getByText(/ctrl\+alt\+shift\+u/i)).toBeInTheDocument();
  });

  it("closes on Escape", async () => {
    const onClose = vi.fn();
    const user = userEvent.setup();
    render(<BoardMenu {...props({ onClose })} />);

    await user.keyboard("{Escape}");

    expect(onClose).toHaveBeenCalled();
  });

  it("closes on an outside click", async () => {
    const onClose = vi.fn();
    const user = userEvent.setup();
    render(
      <div>
        <button type="button">elsewhere</button>
        <BoardMenu {...props({ onClose })} />
      </div>,
    );

    await user.click(screen.getByRole("button", { name: "elsewhere" }));

    expect(onClose).toHaveBeenCalled();
  });

  it("stays open while stepping a value", async () => {
    // Adjusting text size takes several clicks. Closing after each one would
    // make the control unusable.
    const onClose = vi.fn();
    const user = userEvent.setup();
    render(<BoardMenu {...props({ onClose })} />);

    await user.click(screen.getByRole("button", { name: /larger text/i }));

    expect(onClose).not.toHaveBeenCalled();
  });
});
