import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { GeneralBoard } from "@/features/boards/GeneralBoard";
import { useSectionStore } from "@/stores/sectionStore";
import { useTaskStore } from "@/stores/taskStore";
import type { BoardSection } from "@/types/section";
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

function section(title: string, position = 0): BoardSection {
  return { id: `s-${position}`, boardKind: "priority", title, position };
}

/**
 * Answers each command by name, so ordering between them does not matter.
 *
 * The board makes two independent round-trips on mount — its tasks and its
 * declared groups — and a queue of one-shot answers would hand whichever
 * arrived first the wrong reply.
 */
function respond(
  tasks: Task[] = [],
  sections: BoardSection[] = [],
  overrides: Record<string, unknown> = {},
) {
  mockInvoke.mockImplementation((command: string) => {
    if (command in overrides) return Promise.resolve(overrides[command]);
    if (command === "task_list_general") return Promise.resolve(tasks);
    if (command === "section_list") return Promise.resolve(sections);
    return Promise.resolve(tasks[0] ?? null);
  });
}

beforeEach(() => {
  mockInvoke.mockReset();
  useTaskStore.setState({ tasks: {}, filter: null, loading: false, error: null });
  useSectionStore.setState({ sections: [], loading: false, error: null });
});

describe("GeneralBoard", () => {
  it("loads the tasks tied to no week or day", async () => {
    respond();

    render(<GeneralBoard />);

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("task_list_general", undefined);
    });
  });

  it("shows tasks of every priority, unranked ones included", async () => {
    // The catch-all for things with no slot yet: a P2 errand and a task nobody
    // ranked belong here as much as an urgent one.
    respond([
      task({ id: "a", title: "urgent thing", priority: 9 }),
      task({ id: "b", title: "low errand", priority: 2 }),
      task({ id: "c", title: "unranked idea", priority: null }),
    ]);

    render(<GeneralBoard />);

    expect(await screen.findByText("urgent thing")).toBeInTheDocument();
    expect(screen.getByText("low errand")).toBeInTheDocument();
    expect(screen.getByText("unranked idea")).toBeInTheDocument();
  });

  it("groups tasks under their areas", async () => {
    respond([
      task({ id: "a", title: "submit applications", area: "Job search", priority: 9 }),
      task({ id: "b", title: "book eye exam", area: "Health", priority: 6 }),
    ]);

    render(<GeneralBoard />);

    expect(
      await screen.findByRole("heading", { name: /job search/i }),
    ).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: /health/i })).toBeInTheDocument();
    expect(screen.getByText("submit applications")).toBeInTheDocument();
    expect(screen.getByText("book eye exam")).toBeInTheDocument();
  });

  it("explains what belongs here when it is empty", async () => {
    // An empty board must not look broken, and a new user has to learn what
    // this board is for from the board itself.
    respond();

    render(<GeneralBoard />);

    expect(
      await screen.findByText(/not tied to a particular week/i),
    ).toBeInTheDocument();
  });

  it("completes a task through the store", async () => {
    const user = userEvent.setup();
    respond([task()], [], { task_update: task({ status: "completed" }) });
    render(<GeneralBoard />);
    await screen.findByText("submit applications");

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
    respond([task()], [], { task_set_time_spent: task({ timeSpentMinutes: 30 }) });
    render(<GeneralBoard />);
    await screen.findByText("submit applications");

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
    respond([task()], [], { task_update: task({ title: "submit six applications" }) });
    render(<GeneralBoard />);
    await screen.findByText("submit applications");

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
    respond([task()], [], { task_delete: undefined });
    render(<GeneralBoard />);
    await screen.findByText("submit applications");

    await user.click(screen.getByRole("button", { name: "Delete task" }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("task_delete", { id: "task-1" });
    });
  });

  it("surfaces a failure rather than failing silently", async () => {
    mockInvoke.mockImplementation((command: string) => {
      if (command === "section_list") return Promise.resolve([]);
      return Promise.reject(new Error("disk is full"));
    });

    render(<GeneralBoard />);

    expect(await screen.findByRole("alert")).toHaveTextContent("disk is full");
  });

  describe("declared groups", () => {
    it("loads the groups declared for this board", async () => {
      respond();

      render(<GeneralBoard />);

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith("section_list", {
          boardKind: "priority",
        });
      });
    });

    it("shows a declared group that has no tasks yet", async () => {
      // A group derived from tasks disappears the moment it has none. One the
      // user made on purpose has to stand there waiting to be filled.
      respond([], [section("Job Search")]);

      render(<GeneralBoard />);

      expect(
        await screen.findByRole("heading", { name: /job search/i }),
      ).toBeInTheDocument();
    });

    it("does not render a declared group twice when tasks already made it", async () => {
      respond([task({ id: "a", area: "Health", priority: 7 })], [section("health")]);

      render(<GeneralBoard />);
      await screen.findByRole("heading", { name: /health/i });

      expect(screen.getAllByRole("heading", { name: /health/i })).toHaveLength(1);
    });
  });

  describe("adding a task to a group", () => {
    it("files the task under that group's area, with no priority set", async () => {
      // Every priority shows here, so nothing is forced on a new task: it is
      // ranked when the user decides, not by the board that happened to make it.
      const user = userEvent.setup();
      respond([task({ id: "a", area: "Job search", priority: 9 })], [], {
        task_create: task({ id: "new", title: "email two contacts" }),
      });
      render(<GeneralBoard />);
      await screen.findByRole("heading", { name: /job search/i });

      await user.click(
        screen.getByRole("button", { name: "Add a task to Job search" }),
      );
      await user.type(
        screen.getByLabelText("Add to Job search"),
        "email two contacts{Enter}",
      );

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith("task_create", {
          input: {
            title: "email two contacts",
            area: "Job search",
            horizon: "weekly",
            status: "planned",
            sourceType: "manual",
          },
        });
      });
    });

    it("adds into a declared group that has no tasks yet", async () => {
      const user = userEvent.setup();
      respond([], [section("Job Search")], {
        task_create: task({ id: "new", title: "rewrite the resume" }),
      });
      render(<GeneralBoard />);
      await screen.findByRole("heading", { name: /job search/i });

      await user.click(
        screen.getByRole("button", { name: "Add a task to Job Search" }),
      );
      await user.type(
        screen.getByLabelText("Add to Job Search"),
        "rewrite the resume{Enter}",
      );

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith("task_create", {
          input: {
            title: "rewrite the resume",
            area: "Job Search",
            horizon: "weekly",
            status: "planned",
            sourceType: "manual",
          },
        });
      });
    });

    it("leaves the area unset when adding under the unsorted heading", async () => {
      // "Unsorted" is the board's word for no area at all, not an area named
      // "Unsorted" — filing one there would make the fallback real.
      const user = userEvent.setup();
      respond([task({ id: "a", area: null, priority: 9 })], [], {
        task_create: task({ id: "new", title: "pay the invoice", area: null }),
      });
      render(<GeneralBoard />);
      await screen.findByRole("heading", { name: /unsorted/i });

      await user.click(screen.getByRole("button", { name: "Add a task to Unsorted" }));
      await user.type(
        screen.getByLabelText("Add to Unsorted"),
        "pay the invoice{Enter}",
      );

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith("task_create", {
          input: {
            title: "pay the invoice",
            area: null,
            horizon: "weekly",
            status: "planned",
            sourceType: "manual",
          },
        });
      });
    });
  });

  it("sets a priority on a row through the context menu", async () => {
    // Priority is what decides whether a task is on this board at all, so the
    // board that is defined by it must be able to change it.
    const user = userEvent.setup();
    respond([task()], [], { task_update: task({ priority: 10 }) });
    render(<GeneralBoard />);
    await screen.findByText("submit applications");

    await user.pointer({
      keys: "[MouseRight]",
      target: screen.getByText("submit applications"),
    });
    await user.click(screen.getByRole("menuitem", { name: /set priority/i }));
    await user.click(screen.getByRole("menuitem", { name: "P10" }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("task_update", {
        id: "task-1",
        patch: { priority: 10 },
      });
    });
  });
});
