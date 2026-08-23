import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { TaskRow } from "@/features/boards/components/TaskRow";
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
    project: null,
    priority: 8,
    scheduledDate: null,
    periodStart: null,
    periodEnd: null,
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
    onComplete: vi.fn(),
    onEdit: vi.fn(),
    onSetTimeSpent: vi.fn(),
    ...overrides,
  };
}

describe("TaskRow", () => {
  it("shows priority, title, and duration together", () => {
    render(<TaskRow {...props({ task: task({ timeSpentMinutes: 35 }) })} />);

    expect(screen.getByText("P8")).toBeInTheDocument();
    expect(screen.getByText("submit applications")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /35m/ })).toBeInTheDocument();
  });

  it("shows a dash for an unmeasured task", () => {
    render(<TaskRow {...props()} />);

    expect(screen.getByRole("button", { name: /not recorded/i })).toHaveTextContent(
      "—",
    );
  });

  describe("completion", () => {
    it("reports a completion to its owner", async () => {
      const onComplete = vi.fn();
      const user = userEvent.setup();
      render(<TaskRow {...props({ onComplete })} />);

      await user.click(screen.getByRole("checkbox", { name: "submit applications" }));

      expect(onComplete).toHaveBeenCalledWith(true);
    });

    it("reports an uncompletion", async () => {
      const onComplete = vi.fn();
      const user = userEvent.setup();
      render(
        <TaskRow {...props({ task: task({ status: "completed" }), onComplete })} />,
      );

      await user.click(screen.getByRole("checkbox", { name: "submit applications" }));

      expect(onComplete).toHaveBeenCalledWith(false);
    });

    it("checks the box for a completed task", () => {
      render(<TaskRow {...props({ task: task({ status: "completed" }) })} />);

      expect(screen.getByRole("checkbox")).toBeChecked();
    });

    it("keeps a completed task visible but marks it done", () => {
      // The board is a record of the week, not a list that empties as work
      // gets finished (spec §6.4).
      const { container } = render(
        <TaskRow {...props({ task: task({ status: "completed" }) })} />,
      );

      expect(screen.getByText("submit applications")).toBeVisible();
      expect(container.querySelector(".task-row")).toHaveAttribute(
        "data-completed",
        "true",
      );
    });
  });

  describe("inline edit", () => {
    it("opens an editor on double-click", async () => {
      const user = userEvent.setup();
      render(<TaskRow {...props()} />);

      await user.dblClick(screen.getByText("submit applications"));

      expect(screen.getByLabelText("Edit title")).toHaveValue("submit applications");
    });

    it("does not open on a single click", async () => {
      // A single click is how a row gets selected and how a checkbox gets
      // missed; opening an editor for it would make the board hostile.
      const user = userEvent.setup();
      render(<TaskRow {...props()} />);

      await user.click(screen.getByText("submit applications"));

      expect(screen.queryByLabelText("Edit title")).toBeNull();
    });

    it("commits on Enter", async () => {
      const onEdit = vi.fn();
      const user = userEvent.setup();
      render(<TaskRow {...props({ onEdit })} />);

      await user.dblClick(screen.getByText("submit applications"));
      await user.clear(screen.getByLabelText("Edit title"));
      await user.type(
        screen.getByLabelText("Edit title"),
        "submit six applications{Enter}",
      );

      expect(onEdit).toHaveBeenCalledWith("submit six applications");
      expect(screen.queryByLabelText("Edit title")).toBeNull();
    });

    it("abandons the edit on Escape", async () => {
      const onEdit = vi.fn();
      const user = userEvent.setup();
      render(<TaskRow {...props({ onEdit })} />);

      await user.dblClick(screen.getByText("submit applications"));
      await user.clear(screen.getByLabelText("Edit title"));
      await user.type(screen.getByLabelText("Edit title"), "nonsense{Escape}");

      expect(onEdit).not.toHaveBeenCalled();
      expect(screen.getByText("submit applications")).toBeInTheDocument();
    });

    it("commits on blur, so clicking away does not lose the edit", async () => {
      const onEdit = vi.fn();
      const user = userEvent.setup();
      render(<TaskRow {...props({ onEdit })} />);

      await user.dblClick(screen.getByText("submit applications"));
      await user.clear(screen.getByLabelText("Edit title"));
      await user.type(screen.getByLabelText("Edit title"), "renamed");
      await user.tab();

      expect(onEdit).toHaveBeenCalledWith("renamed");
    });

    it("refuses to commit an empty title", async () => {
      // An untitled row is unreachable: there is nothing left to double-click.
      const onEdit = vi.fn();
      const user = userEvent.setup();
      render(<TaskRow {...props({ onEdit })} />);

      await user.dblClick(screen.getByText("submit applications"));
      await user.clear(screen.getByLabelText("Edit title"));
      await user.type(screen.getByLabelText("Edit title"), "   {Enter}");

      expect(onEdit).not.toHaveBeenCalled();
      expect(screen.getByText("submit applications")).toBeInTheDocument();
    });

    it("does not dispatch when the title is unchanged", async () => {
      const onEdit = vi.fn();
      const user = userEvent.setup();
      render(<TaskRow {...props({ onEdit })} />);

      await user.dblClick(screen.getByText("submit applications"));
      await user.type(screen.getByLabelText("Edit title"), "{Enter}");

      expect(onEdit).not.toHaveBeenCalled();
    });
  });

  describe("duration", () => {
    it("reports a chosen preset to its owner", async () => {
      const onSetTimeSpent = vi.fn();
      const user = userEvent.setup();
      render(<TaskRow {...props({ onSetTimeSpent })} />);

      await user.click(screen.getByRole("button", { name: /not recorded/i }));
      await user.click(screen.getByRole("button", { name: "30m" }));

      expect(onSetTimeSpent).toHaveBeenCalledWith(30);
    });

    it("reports free text to its owner", async () => {
      const onSetTimeSpent = vi.fn();
      const user = userEvent.setup();
      render(<TaskRow {...props({ onSetTimeSpent })} />);

      await user.click(screen.getByRole("button", { name: /not recorded/i }));
      await user.type(screen.getByLabelText("Duration"), "2h{Enter}");

      expect(onSetTimeSpent).toHaveBeenCalledWith(120);
    });
  });

  describe("hover actions", () => {
    it("reports a delete to its owner", async () => {
      const onDelete = vi.fn();
      const user = userEvent.setup();
      render(<TaskRow {...props({ onDelete })} />);

      await user.click(screen.getByRole("button", { name: /delete/i }));

      expect(onDelete).toHaveBeenCalled();
    });

    it("reports a move to its owner", async () => {
      const onMove = vi.fn();
      const user = userEvent.setup();
      render(<TaskRow {...props({ onMove })} />);

      await user.click(screen.getByRole("button", { name: /move/i }));

      expect(onMove).toHaveBeenCalled();
    });

    it("omits an action with no handler rather than rendering a dead button", () => {
      render(<TaskRow {...props()} />);

      expect(screen.queryByRole("button", { name: /delete/i })).toBeNull();
      expect(screen.queryByRole("button", { name: /move/i })).toBeNull();
    });

    it("keeps hover-revealed actions reachable by keyboard", async () => {
      // They are hidden with opacity, not display, precisely so this works.
      const onDelete = vi.fn();
      const user = userEvent.setup();
      render(<TaskRow {...props({ onDelete })} />);

      const remove = screen.getByRole("button", { name: /delete/i });
      remove.focus();
      expect(remove).toHaveFocus();

      await user.keyboard("{Enter}");
      expect(onDelete).toHaveBeenCalled();
    });
  });
});
