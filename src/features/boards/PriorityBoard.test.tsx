import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { PriorityBoard } from "@/features/boards/PriorityBoard";
import { useTaskStore } from "@/stores/taskStore";
import type { Task } from "@/types/task";

const mockInvoke = vi.mocked(invoke);

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

beforeEach(() => {
  useTaskStore.setState({ tasks: {}, filter: null, loading: false, error: null });
});

describe("PriorityBoard", () => {
  it("loads through the priority filter at the default threshold", async () => {
    mockInvoke.mockResolvedValue([]);

    render(<PriorityBoard />);

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("task_list_priority", { threshold: 5 });
    });
  });

  it("honours a threshold it is given", async () => {
    mockInvoke.mockResolvedValue([]);

    render(<PriorityBoard threshold={8} />);

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("task_list_priority", { threshold: 8 });
    });
  });

  it("groups tasks under their areas", async () => {
    mockInvoke.mockResolvedValue([
      task({ id: "a", title: "submit applications", area: "Job search", priority: 9 }),
      task({ id: "b", title: "book eye exam", area: "Health", priority: 6 }),
    ]);

    render(<PriorityBoard />);

    expect(
      await screen.findByRole("heading", { name: "Job search" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Health" })).toBeInTheDocument();
    expect(screen.getByText("submit applications")).toBeInTheDocument();
    expect(screen.getByText("book eye exam")).toBeInTheDocument();
  });

  it("says so when there is nothing above the threshold", async () => {
    // An empty board must not look broken. Naming the threshold explains why
    // it is empty to a user who knows they have tasks.
    mockInvoke.mockResolvedValue([]);

    render(<PriorityBoard />);

    expect(
      await screen.findByText(/nothing at priority 5 or above/i),
    ).toBeInTheDocument();
  });

  it("completes a task through the store", async () => {
    const user = userEvent.setup();
    mockInvoke.mockResolvedValueOnce([task()]);
    render(<PriorityBoard />);
    await screen.findByText("submit applications");

    mockInvoke.mockResolvedValueOnce(task({ status: "completed" }));
    await user.click(screen.getByRole("checkbox", { name: "submit applications" }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith(
        "task_update",
        expect.objectContaining({ id: "task-1" }),
      );
    });
    expect(useTaskStore.getState().tasks["task-1"]?.status).toBe("completed");
  });

  it("records a duration through the store", async () => {
    const user = userEvent.setup();
    mockInvoke.mockResolvedValueOnce([task()]);
    render(<PriorityBoard />);
    await screen.findByText("submit applications");

    mockInvoke.mockResolvedValueOnce(task({ timeSpentMinutes: 30 }));
    await user.click(screen.getByRole("button", { name: /not recorded/i }));
    await user.click(screen.getByRole("button", { name: "30m" }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("task_set_time_spent", {
        id: "task-1",
        minutes: 30,
      });
    });
  });

  it("commits an inline edit through the store", async () => {
    const user = userEvent.setup();
    mockInvoke.mockResolvedValueOnce([task()]);
    render(<PriorityBoard />);
    await screen.findByText("submit applications");

    mockInvoke.mockResolvedValueOnce(task({ title: "submit six applications" }));
    await user.dblClick(screen.getByText("submit applications"));
    await user.clear(screen.getByLabelText("Edit title"));
    await user.type(
      screen.getByLabelText("Edit title"),
      "submit six applications{Enter}",
    );

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("task_update", {
        id: "task-1",
        patch: { title: "submit six applications" },
      });
    });
  });

  it("deletes a task through the store", async () => {
    const user = userEvent.setup();
    mockInvoke.mockResolvedValueOnce([task()]);
    render(<PriorityBoard />);
    await screen.findByText("submit applications");

    mockInvoke.mockResolvedValueOnce(undefined);
    await user.click(screen.getByRole("button", { name: /delete/i }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("task_delete", { id: "task-1" });
    });
  });

  it("surfaces a failure rather than failing silently", async () => {
    mockInvoke.mockRejectedValueOnce({ kind: "storage", message: "disk is full" });

    render(<PriorityBoard />);

    expect(await screen.findByRole("alert")).toHaveTextContent("disk is full");
  });
});
