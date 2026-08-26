import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import userEvent from "@testing-library/user-event";
import { TaskGroup } from "@/features/boards/components/TaskGroup";
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

describe("TaskGroup", () => {
  it("heads the group with its area", () => {
    render(
      <TaskGroup
        label="Job search"
        tasks={[task()]}
        renderTask={(t) => <li key={t.id}>{t.title}</li>}
      />,
    );

    expect(screen.getByRole("heading", { name: "Job search" })).toBeInTheDocument();
  });

  it("renders each task through the callback it is given", () => {
    render(
      <TaskGroup
        label="Health"
        tasks={[
          task({ id: "a", title: "eye exam" }),
          task({ id: "b", title: "dentist" }),
        ]}
        renderTask={(t) => <li key={t.id}>{t.title}</li>}
      />,
    );

    expect(screen.getAllByRole("listitem")).toHaveLength(2);
    expect(screen.getByText("eye exam")).toBeInTheDocument();
    expect(screen.getByText("dentist")).toBeInTheDocument();
  });

  it("labels the list with its area for screen readers", () => {
    // Four unlabelled lists on one board are indistinguishable to anyone
    // navigating by landmark.
    render(
      <TaskGroup
        label="Health"
        tasks={[task()]}
        renderTask={(t) => <li key={t.id}>{t.title}</li>}
      />,
    );

    expect(screen.getByRole("list", { name: "Health" })).toBeInTheDocument();
  });
});

describe("TaskGroup rename", () => {
  it("leaves the heading as a heading when no rename is offered", () => {
    render(<TaskGroup label="Health" tasks={[]} renderTask={() => null} />);

    // A derived group has no section row behind it, so there is nothing to
    // rename and the heading must not pretend otherwise.
    expect(screen.getByRole("heading", { name: "Health" })).toBeInTheDocument();
    expect(screen.queryByLabelText("Rename Health")).not.toBeInTheDocument();
  });

  it("renames a declared group from its heading", async () => {
    const user = userEvent.setup();
    const onRename = vi.fn();
    render(
      <TaskGroup
        label="Health"
        tasks={[]}
        onRename={onRename}
        renderTask={() => null}
      />,
    );

    await user.click(screen.getByRole("heading", { name: "Health" }));
    const editor = screen.getByLabelText("Rename Health");
    await user.clear(editor);
    await user.type(editor, "Wellbeing");
    await user.tab();

    expect(onRename).toHaveBeenCalledWith("Wellbeing");
  });

  it("abandons a rename on Escape", async () => {
    const user = userEvent.setup();
    const onRename = vi.fn();
    render(
      <TaskGroup
        label="Health"
        tasks={[]}
        onRename={onRename}
        renderTask={() => null}
      />,
    );

    await user.click(screen.getByRole("heading", { name: "Health" }));
    await user.type(screen.getByLabelText("Rename Health"), " stuff{Escape}");

    expect(onRename).not.toHaveBeenCalled();
    expect(screen.getByRole("heading", { name: "Health" })).toBeInTheDocument();
  });

  it("refuses to rename a group to nothing", async () => {
    const user = userEvent.setup();
    const onRename = vi.fn();
    render(
      <TaskGroup
        label="Health"
        tasks={[]}
        onRename={onRename}
        renderTask={() => null}
      />,
    );

    await user.click(screen.getByRole("heading", { name: "Health" }));
    await user.clear(screen.getByLabelText("Rename Health"));
    await user.tab();

    // An unnamed group cannot be found again, and the rename would blank the
    // area on every task filed under it.
    expect(onRename).not.toHaveBeenCalled();
  });
});
