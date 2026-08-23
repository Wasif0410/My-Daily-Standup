import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { TaskContextMenu } from "@/features/boards/components/TaskContextMenu";
import type { Task } from "@/types/task";

function task(overrides: Partial<Task> = {}): Task {
  return {
    id: "task-1",
    title: "submit applications",
    description: null,
    horizon: "weekly",
    status: "planned",
    parentTaskId: null,
    sourceType: "manual",
    sourceFile: null,
    sourceLine: null,
    area: "Job search",
    project: "Job Search",
    priority: 8,
    scheduledDate: null,
    periodStart: "2026-08-17",
    periodEnd: "2026-08-23",
    dueDate: null,
    completedAt: null,
    progressCurrent: null,
    progressTarget: null,
    progressUnit: null,
    blocker: null,
    notes: null,
    timeSpentMinutes: null,
    rolloverCount: 0,
    createdAt: "2026-08-21T00:00:00.000000Z",
    updatedAt: "2026-08-21T00:00:00.000000Z",
    ...overrides,
  };
}

function props(overrides: Record<string, unknown> = {}) {
  return {
    task: task(),
    x: 10,
    y: 20,
    onClose: vi.fn(),
    onComplete: vi.fn(),
    onEdit: vi.fn(),
    onSetPriority: vi.fn(),
    onMoveToDate: vi.fn(),
    onPromote: vi.fn(),
    onMoveToNextWeek: vi.fn(),
    onSetBlocker: vi.fn(),
    onAddComment: vi.fn(),
    onArchive: vi.fn(),
    onDelete: vi.fn(),
    ...overrides,
  };
}

describe("TaskContextMenu", () => {
  it("is a real menu, not a div full of divs", () => {
    // Shift+F10 and the Menu key have to reach it, and assistive technology
    // has to announce it as a menu.
    render(<TaskContextMenu {...props()} />);

    expect(screen.getByRole("menu")).toBeInTheDocument();
    expect(screen.getAllByRole("menuitem").length).toBeGreaterThan(5);
  });

  it("offers every interaction the spec lists", () => {
    render(<TaskContextMenu {...props()} />);

    for (const label of [
      /complete/i,
      /edit/i,
      /priority/i,
      /move to another day/i,
      /next week/i,
      /blocker/i,
      /comment/i,
      /archive/i,
      /delete/i,
    ]) {
      expect(screen.getByRole("menuitem", { name: label })).toBeInTheDocument();
    }
  });

  it("says Uncomplete for a completed task", () => {
    // A menu offering "Complete" on finished work is lying about state.
    render(<TaskContextMenu {...props({ task: task({ status: "completed" }) })} />);

    expect(screen.getByRole("menuitem", { name: /uncomplete/i })).toBeInTheDocument();
  });

  it("offers Promote to weekly only for a daily task", () => {
    const { rerender } = render(<TaskContextMenu {...props()} />);
    expect(screen.queryByRole("menuitem", { name: /promote/i })).toBeNull();

    rerender(<TaskContextMenu {...props({ task: task({ horizon: "daily" }) })} />);
    expect(screen.getByRole("menuitem", { name: /promote/i })).toBeInTheDocument();
  });

  it("says Resolve blocker when one is set", () => {
    render(<TaskContextMenu {...props({ task: task({ blocker: "waiting" }) })} />);

    expect(
      screen.getByRole("menuitem", { name: /resolve blocker/i }),
    ).toBeInTheDocument();
  });

  it("positions itself where the click happened", () => {
    const { container } = render(<TaskContextMenu {...props({ x: 120, y: 240 })} />);

    const style = container.querySelector(".context-menu")?.getAttribute("style");
    expect(style).toContain("120px");
    expect(style).toContain("240px");
  });

  describe("dispatching", () => {
    it("completes a task", async () => {
      const onComplete = vi.fn();
      const user = userEvent.setup();
      render(<TaskContextMenu {...props({ onComplete })} />);

      await user.click(screen.getByRole("menuitem", { name: /^complete$/i }));

      expect(onComplete).toHaveBeenCalledWith(true);
    });

    it("uncompletes a completed task", async () => {
      const onComplete = vi.fn();
      const user = userEvent.setup();
      render(
        <TaskContextMenu
          {...props({ task: task({ status: "completed" }), onComplete })}
        />,
      );

      await user.click(screen.getByRole("menuitem", { name: /uncomplete/i }));

      expect(onComplete).toHaveBeenCalledWith(false);
    });

    it("starts a title edit", async () => {
      const onEdit = vi.fn();
      const user = userEvent.setup();
      render(<TaskContextMenu {...props({ onEdit })} />);

      await user.click(screen.getByRole("menuitem", { name: /edit title/i }));

      expect(onEdit).toHaveBeenCalled();
    });

    it("moves to next week", async () => {
      const onMoveToNextWeek = vi.fn();
      const user = userEvent.setup();
      render(<TaskContextMenu {...props({ onMoveToNextWeek })} />);

      await user.click(screen.getByRole("menuitem", { name: /next week/i }));

      expect(onMoveToNextWeek).toHaveBeenCalled();
    });

    it("archives", async () => {
      const onArchive = vi.fn();
      const user = userEvent.setup();
      render(<TaskContextMenu {...props({ onArchive })} />);

      await user.click(screen.getByRole("menuitem", { name: /archive/i }));

      expect(onArchive).toHaveBeenCalled();
    });

    it("deletes", async () => {
      const onDelete = vi.fn();
      const user = userEvent.setup();
      render(<TaskContextMenu {...props({ onDelete })} />);

      await user.click(screen.getByRole("menuitem", { name: /delete/i }));

      expect(onDelete).toHaveBeenCalled();
    });

    it("sets a priority from the submenu", async () => {
      const onSetPriority = vi.fn();
      const user = userEvent.setup();
      render(<TaskContextMenu {...props({ onSetPriority })} />);

      await user.click(screen.getByRole("menuitem", { name: /priority/i }));
      await user.click(screen.getByRole("menuitem", { name: "P9" }));

      expect(onSetPriority).toHaveBeenCalledWith(9);
    });

    it("moves to a date the user picks", async () => {
      const onMoveToDate = vi.fn();
      const user = userEvent.setup();
      render(<TaskContextMenu {...props({ onMoveToDate })} />);

      await user.click(screen.getByRole("menuitem", { name: /move to another day/i }));
      await user.type(screen.getByLabelText("New date"), "2026-08-25");
      await user.keyboard("{Enter}");

      expect(onMoveToDate).toHaveBeenCalledWith("2026-08-25");
    });

    it("adds a blocker the user types", async () => {
      const onSetBlocker = vi.fn();
      const user = userEvent.setup();
      render(<TaskContextMenu {...props({ onSetBlocker })} />);

      await user.click(screen.getByRole("menuitem", { name: /add blocker/i }));
      await user.type(screen.getByLabelText("Blocker"), "waiting on the portal{Enter}");

      expect(onSetBlocker).toHaveBeenCalledWith("waiting on the portal");
    });

    it("resolves a blocker without asking for text", async () => {
      const onSetBlocker = vi.fn();
      const user = userEvent.setup();
      render(
        <TaskContextMenu
          {...props({ task: task({ blocker: "waiting" }), onSetBlocker })}
        />,
      );

      await user.click(screen.getByRole("menuitem", { name: /resolve blocker/i }));

      expect(onSetBlocker).toHaveBeenCalledWith(null);
    });

    it("adds a comment the user types", async () => {
      const onAddComment = vi.fn();
      const user = userEvent.setup();
      render(<TaskContextMenu {...props({ onAddComment })} />);

      await user.click(screen.getByRole("menuitem", { name: /comment/i }));
      await user.type(screen.getByLabelText("Comment"), "portal was down{Enter}");

      expect(onAddComment).toHaveBeenCalledWith("portal was down");
    });
  });

  describe("closing", () => {
    it("closes after an action fires", async () => {
      const onClose = vi.fn();
      const user = userEvent.setup();
      render(<TaskContextMenu {...props({ onClose })} />);

      await user.click(screen.getByRole("menuitem", { name: /^complete$/i }));

      expect(onClose).toHaveBeenCalled();
    });

    it("closes on Escape", async () => {
      const onClose = vi.fn();
      const user = userEvent.setup();
      render(<TaskContextMenu {...props({ onClose })} />);

      await user.keyboard("{Escape}");

      expect(onClose).toHaveBeenCalled();
    });

    it("closes on an outside click", async () => {
      const onClose = vi.fn();
      const user = userEvent.setup();
      render(
        <div>
          <button type="button">elsewhere</button>
          <TaskContextMenu {...props({ onClose })} />
        </div>,
      );

      await user.click(screen.getByRole("button", { name: "elsewhere" }));

      expect(onClose).toHaveBeenCalled();
    });

    it("does not close while a prompt is open", async () => {
      // Typing a blocker means clicking into a field. Closing on that click
      // would make the prompt impossible to use.
      const onClose = vi.fn();
      const user = userEvent.setup();
      render(<TaskContextMenu {...props({ onClose })} />);

      await user.click(screen.getByRole("menuitem", { name: /add blocker/i }));
      await user.click(screen.getByLabelText("Blocker"));

      expect(onClose).not.toHaveBeenCalled();
    });
  });

  it("moves between items with the arrow keys", async () => {
    // Hover is not the only way to reach a menu item.
    const user = userEvent.setup();
    render(<TaskContextMenu {...props()} />);

    const items = screen.getAllByRole("menuitem");
    expect(items[0]).toHaveFocus();

    await user.keyboard("{ArrowDown}");
    expect(items[1]).toHaveFocus();

    await user.keyboard("{ArrowUp}");
    expect(items[0]).toHaveFocus();
  });

  it("wraps from the last item back to the first", async () => {
    const user = userEvent.setup();
    render(<TaskContextMenu {...props()} />);

    const items = screen.getAllByRole("menuitem");
    await user.keyboard("{ArrowUp}");

    expect(items[items.length - 1]).toHaveFocus();
  });
});
