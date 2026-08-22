import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { App } from "@/app/App";
import { useTaskStore } from "@/stores/taskStore";
import type { Task } from "@/types/task";

const mockInvoke = vi.mocked(invoke);

function task(overrides: Partial<Task> = {}): Task {
  return {
    id: "task-1",
    title: "call the clinic",
    description: null,
    horizon: "daily",
    status: "planned",
    parentTaskId: null,
    sourceType: "manual",
    sourceFile: null,
    sourceLine: null,
    area: null,
    project: null,
    priority: null,
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
    rolloverCount: 0,
    createdAt: "2026-08-21T00:00:00.000000Z",
    updatedAt: "2026-08-21T00:00:00.000000Z",
    ...overrides,
  };
}

/** Routes each command to a canned response. */
function respond(handlers: Record<string, unknown>) {
  mockInvoke.mockImplementation((command: string) =>
    Promise.resolve(handlers[command] ?? null),
  );
}

beforeEach(() => {
  mockInvoke.mockReset();
  useTaskStore.setState({ tasks: {}, filter: null, loading: false, error: null });
});

describe("App", () => {
  it("renders the application title", () => {
    respond({ task_list_by_horizon: [] });

    render(<App />);

    expect(
      screen.getByRole("heading", { name: "My Daily Standup" }),
    ).toBeInTheDocument();
  });

  it("lists tasks loaded from the backend", async () => {
    respond({ task_list_by_horizon: [task({ title: "call the clinic" })] });

    render(<App />);

    expect(await screen.findByText("call the clinic")).toBeInTheDocument();
  });

  it("shows an empty state when there are no tasks", async () => {
    respond({ task_list_by_horizon: [] });

    render(<App />);

    expect(await screen.findByText("No tasks yet.")).toBeInTheDocument();
  });

  it("creates a task and refreshes the list", async () => {
    respond({ task_list_by_horizon: [], task_create: task() });
    const user = userEvent.setup();

    render(<App />);
    await screen.findByText("No tasks yet.");

    await user.type(screen.getByLabelText("New task"), "write the spec");
    await user.click(screen.getByRole("button", { name: "Add" }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("task_create", {
        input: {
          title: "write the spec",
          horizon: "daily",
          status: "planned",
          sourceType: "manual",
        },
      });
    });
  });

  it("does not submit an empty title", async () => {
    respond({ task_list_by_horizon: [] });
    const user = userEvent.setup();

    render(<App />);
    await screen.findByText("No tasks yet.");

    await user.click(screen.getByRole("button", { name: "Add" }));

    expect(mockInvoke).not.toHaveBeenCalledWith("task_create", expect.anything());
  });

  it("clears completedAt when un-completing a task", async () => {
    // The doubled option exists for exactly this: null means clear, not
    // "leave alone".
    respond({
      task_list_by_horizon: [task({ status: "completed" })],
      task_update: task(),
    });
    const user = userEvent.setup();

    render(<App />);
    await user.click(await screen.findByRole("checkbox"));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("task_update", {
        id: "task-1",
        patch: { status: "planned", completedAt: null },
      });
    });
  });

  it("surfaces a backend failure instead of failing silently", async () => {
    mockInvoke.mockRejectedValue({ kind: "storage", message: "disk full" });

    render(<App />);

    expect(await screen.findByTestId("error")).toHaveTextContent("storage: disk full");
  });
});
