import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { BoardShell } from "@/components/BoardShell";

describe("BoardShell", () => {
  it("renders its title and children", () => {
    render(
      <BoardShell kind="priority" title="Priority Tasks" onToggleCollapsed={vi.fn()}>
        <p>a task</p>
      </BoardShell>,
    );

    expect(screen.getByRole("heading", { name: "Priority Tasks" })).toBeInTheDocument();
    expect(screen.getByText("a task")).toBeInTheDocument();
  });

  it("renders a subtitle when given one", () => {
    render(
      <BoardShell
        kind="weekly-progress"
        title="Weekly Progress"
        subtitle="2026-W34"
        onToggleCollapsed={vi.fn()}
      >
        <p>body</p>
      </BoardShell>,
    );

    expect(screen.getByText("2026-W34")).toBeInTheDocument();
  });

  it("gives each board its own accent colour", () => {
    // Four windows scattered across a desktop are far easier to tell apart by
    // colour than by reading their titles.
    const { container, rerender } = render(
      <BoardShell kind="priority" title="Priority Tasks" onToggleCollapsed={vi.fn()}>
        <p>body</p>
      </BoardShell>,
    );
    const priority = container.querySelector(".board")?.getAttribute("style");

    rerender(
      <BoardShell
        kind="monthly-progress"
        title="Monthly Progress"
        onToggleCollapsed={vi.fn()}
      >
        <p>body</p>
      </BoardShell>,
    );
    const monthly = container.querySelector(".board")?.getAttribute("style");

    expect(priority).toContain("--board-accent");
    expect(priority).not.toEqual(monthly);
  });

  it("marks the header as the window drag region", () => {
    // A frameless window has no OS title bar, so without this the board
    // cannot be moved at all.
    const { container } = render(
      <BoardShell kind="priority" title="Priority Tasks" onToggleCollapsed={vi.fn()}>
        <p>body</p>
      </BoardShell>,
    );

    expect(container.querySelector("[data-tauri-drag-region]")).not.toBeNull();
  });

  describe("collapse", () => {
    it("shows the body by default", () => {
      render(
        <BoardShell kind="priority" title="Priority Tasks" onToggleCollapsed={vi.fn()}>
          <p>body content</p>
        </BoardShell>,
      );

      expect(screen.getByText("body content")).toBeVisible();
      expect(screen.getByRole("button", { name: /collapse/i })).toBeInTheDocument();
    });

    it("starts collapsed when told to", () => {
      const { container } = render(
        <BoardShell
          kind="priority"
          title="Priority Tasks"
          collapsed
          onToggleCollapsed={vi.fn()}
        >
          <p>body content</p>
        </BoardShell>,
      );

      expect(container.querySelector(".board")).toHaveAttribute(
        "data-collapsed",
        "true",
      );
      expect(screen.getByRole("button", { name: /expand/i })).toBeInTheDocument();
    });

    it("reports a collapse toggle to its owner", async () => {
      // The shell does not own the state: collapse persists to the database,
      // so the parent decides and passes it back down.
      const onToggleCollapsed = vi.fn();
      const user = userEvent.setup();

      render(
        <BoardShell
          kind="priority"
          title="Priority Tasks"
          onToggleCollapsed={onToggleCollapsed}
        >
          <p>body</p>
        </BoardShell>,
      );

      await user.click(screen.getByRole("button", { name: /collapse/i }));

      expect(onToggleCollapsed).toHaveBeenCalledWith(true);
    });

    it("keeps the title visible when collapsed", () => {
      // Collapsing to a title bar is only useful if the title survives.
      render(
        <BoardShell
          kind="priority"
          title="Priority Tasks"
          collapsed
          onToggleCollapsed={vi.fn()}
        >
          <p>body</p>
        </BoardShell>,
      );

      expect(screen.getByRole("heading", { name: "Priority Tasks" })).toBeVisible();
    });
  });

  describe("controls", () => {
    it("renders a close button that reports to its owner", async () => {
      const onClose = vi.fn();
      const user = userEvent.setup();

      render(
        <BoardShell
          kind="priority"
          title="Priority Tasks"
          onClose={onClose}
          onToggleCollapsed={vi.fn()}
        >
          <p>body</p>
        </BoardShell>,
      );

      await user.click(screen.getByRole("button", { name: /close/i }));

      expect(onClose).toHaveBeenCalled();
    });

    it("omits the close button when no handler is given", () => {
      render(
        <BoardShell kind="priority" title="Priority Tasks" onToggleCollapsed={vi.fn()}>
          <p>body</p>
        </BoardShell>,
      );

      expect(screen.queryByRole("button", { name: /close/i })).toBeNull();
    });

    it("keeps controls reachable by keyboard despite being hover-revealed", async () => {
      // Hiding controls with opacity would strand keyboard users if the
      // elements were not still focusable.
      const user = userEvent.setup();

      render(
        <BoardShell
          kind="priority"
          title="Priority Tasks"
          onClose={vi.fn()}
          onToggleCollapsed={vi.fn()}
        >
          <p>body</p>
        </BoardShell>,
      );

      await user.tab();

      expect(screen.getByRole("button", { name: /collapse/i })).toHaveFocus();
    });
  });

  it("applies saved opacity as a custom property", () => {
    // PR 16 drives this from settings; the component only reads it.
    const { container } = render(
      <BoardShell
        kind="priority"
        title="Priority Tasks"
        opacity={0.7}
        onToggleCollapsed={vi.fn()}
      >
        <p>body</p>
      </BoardShell>,
    );

    expect(container.querySelector(".board")?.getAttribute("style")).toContain(
      "--board-opacity: 0.7",
    );
  });

  it("marks a locked board so it can be styled as inert", () => {
    const { container } = render(
      <BoardShell
        kind="priority"
        title="Priority Tasks"
        locked
        onToggleCollapsed={vi.fn()}
      >
        <p>body</p>
      </BoardShell>,
    );

    expect(container.querySelector(".board")).toHaveAttribute("data-locked", "true");
  });
});

describe("BoardShell appearance", () => {
  it("applies a saved font size as a custom property", () => {
    // PR 10 put --board-font-size in the token layer precisely so this needed
    // no component change.
    const { container } = render(
      <BoardShell
        kind="priority"
        title="Priority Tasks"
        fontSize={17}
        onToggleCollapsed={vi.fn()}
      >
        <p>body</p>
      </BoardShell>,
    );

    expect(container.querySelector(".board")?.getAttribute("style")).toContain(
      "--board-font-size: 17px",
    );
  });

  it("marks its theme so the tokens can switch", () => {
    const { container } = render(
      <BoardShell
        kind="priority"
        title="Priority Tasks"
        theme="light"
        onToggleCollapsed={vi.fn()}
      >
        <p>body</p>
      </BoardShell>,
    );

    expect(container.querySelector(".board")).toHaveAttribute("data-theme", "light");
  });

  it("marks compact density", () => {
    const { container } = render(
      <BoardShell
        kind="priority"
        title="Priority Tasks"
        compact
        onToggleCollapsed={vi.fn()}
      >
        <p>body</p>
      </BoardShell>,
    );

    expect(container.querySelector(".board")).toHaveAttribute("data-compact", "true");
  });

  it("defaults to dark and comfortable", () => {
    const { container } = render(
      <BoardShell kind="priority" title="Priority Tasks" onToggleCollapsed={vi.fn()}>
        <p>body</p>
      </BoardShell>,
    );

    const shell = container.querySelector(".board");
    expect(shell).toHaveAttribute("data-theme", "dark");
    expect(shell).toHaveAttribute("data-compact", "false");
  });

  it("renders board-specific header actions", () => {
    // Where the board menu's trigger lives.
    render(
      <BoardShell
        kind="priority"
        title="Priority Tasks"
        headerActions={<button type="button">Board settings</button>}
        onToggleCollapsed={vi.fn()}
      >
        <p>body</p>
      </BoardShell>,
    );

    expect(screen.getByRole("button", { name: "Board settings" })).toBeInTheDocument();
  });

  describe("locking", () => {
    it("is draggable when unlocked", () => {
      const { container } = render(
        <BoardShell kind="priority" title="Priority Tasks" onToggleCollapsed={vi.fn()}>
          <p>body</p>
        </BoardShell>,
      );

      expect(container.querySelector("[data-tauri-drag-region]")).not.toBeNull();
    });

    it("is not draggable when locked", () => {
      // The second half of "locked". set_ignore_cursor_events stops clicks
      // reaching the webview, but a board locked while the pointer is already
      // over it would otherwise still be draggable.
      const { container } = render(
        <BoardShell
          kind="priority"
          title="Priority Tasks"
          locked
          onToggleCollapsed={vi.fn()}
        >
          <p>body</p>
        </BoardShell>,
      );

      expect(container.querySelector("[data-tauri-drag-region]")).toBeNull();
    });
  });
});
