import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { WeeklyBoard } from "@/features/boards/WeeklyBoard";
import { useSectionStore } from "@/stores/sectionStore";
import { useTaskStore } from "@/stores/taskStore";
import type { BoardSection } from "@/types/section";
import type { Task } from "@/types/task";

vi.mock("@/lib/taskEvents", () => ({
  emitTaskChanged: vi.fn(),
  onTaskChanged: vi.fn().mockResolvedValue(() => {}),
  TASK_CHANGED: "task-changed",
}));

const mockInvoke = vi.mocked(invoke);

const WEEK = {
  start: "2026-08-17",
  end: "2026-08-23",
  label: "2026-W34",
  today: "2026-08-18",
  days: [
    { date: "2026-08-17", name: "Monday" },
    { date: "2026-08-18", name: "Tuesday" },
    { date: "2026-08-19", name: "Wednesday" },
    { date: "2026-08-20", name: "Thursday" },
    { date: "2026-08-21", name: "Friday" },
    { date: "2026-08-22", name: "Saturday" },
    { date: "2026-08-23", name: "Sunday" },
  ],
};

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
    periodStart: WEEK.start,
    periodEnd: WEEK.end,
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
  return { id: `s-${position}`, boardKind: "weekly-tasks", title, position };
}

/** Answers each command by name, so ordering between them does not matter. */
function respond(
  tasks: Task[] = [],
  sections: BoardSection[] = [],
  overrides: Record<string, unknown> = {},
) {
  mockInvoke.mockImplementation((command: string) => {
    if (command in overrides) return Promise.resolve(overrides[command]);
    if (command === "week_current") return Promise.resolve(WEEK);
    if (command === "task_list_for_period") return Promise.resolve(tasks);
    if (command === "section_list") return Promise.resolve(sections);
    return Promise.resolve(tasks[0] ?? null);
  });
}

beforeEach(() => {
  mockInvoke.mockReset();
  useTaskStore.setState({ tasks: {}, filter: null, loading: false, error: null });
  useSectionStore.setState({ sections: [], loading: false, error: null });
});

describe("WeeklyBoard", () => {
  it("asks Rust for the current week rather than computing one", async () => {
    // A frontend deriving its own week from the browser clock is how a board
    // and its database end up disagreeing about which week it is.
    respond();

    render(<WeeklyBoard />);

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("week_current", { startsOn: null });
    });
  });

  it("loads the period that week covers", async () => {
    respond();

    render(<WeeklyBoard />);

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("task_list_for_period", {
        start: "2026-08-17",
        end: "2026-08-23",
      });
    });
  });

  it("groups tasks by project", async () => {
    respond([
      task({
        id: "a",
        title: "submit applications",
        project: "Job Search",
        priority: 9,
      }),
      task({ id: "b", title: "choose a hotel", project: "Travel", priority: 5 }),
    ]);

    render(<WeeklyBoard />);

    expect(
      await screen.findByRole("heading", { name: "Job Search" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Travel" })).toBeInTheDocument();
  });

  it("says so when the week is empty", async () => {
    respond();

    render(<WeeklyBoard />);

    expect(await screen.findByText(/nothing planned this week/i)).toBeInTheDocument();
  });

  it("hides archived work without deleting it", async () => {
    // Cancelled is the archive. The row survives in the database; the board
    // simply stops showing it.
    respond([
      task({ id: "a", title: "still live" }),
      task({ id: "b", title: "archived one", status: "cancelled" }),
    ]);

    render(<WeeklyBoard />);

    await screen.findByText("still live");
    expect(screen.queryByText("archived one")).toBeNull();
  });

  it("carries no standing quick-add bar", async () => {
    // A board-wide field cannot say which project the task belongs to, so
    // everything it made landed under "No project" and had to be filed by
    // hand afterwards. The per-group + replaced it.
    respond([task()]);

    render(<WeeklyBoard />);
    await screen.findByText("submit applications");

    expect(screen.queryByLabelText("Add a task")).not.toBeInTheDocument();
  });

  it("loads the groups declared for this board", async () => {
    respond();

    render(<WeeklyBoard />);

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("section_list", {
        boardKind: "weekly-tasks",
      });
    });
  });

  it("shows a declared group that has no tasks yet", async () => {
    respond([], [section("Job Search")]);

    render(<WeeklyBoard />);

    expect(
      await screen.findByRole("heading", { name: "Job Search" }),
    ).toBeInTheDocument();
  });

  it("does not render a declared group the tasks already made", async () => {
    respond([task({ id: "a", project: "Travel", priority: 7 })], [section("travel")]);

    render(<WeeklyBoard />);
    await screen.findByRole("heading", { name: /travel/i });

    expect(screen.getAllByRole("heading", { name: /travel/i })).toHaveLength(1);
  });

  it("adds a task into its group and the current week", async () => {
    const user = userEvent.setup();
    respond([task({ id: "a", project: "Job Search", priority: 9 })], [], {
      task_create: task({ id: "new", title: "email two contacts" }),
    });
    render(<WeeklyBoard />);
    await screen.findByRole("heading", { name: "Job Search" });

    await user.click(screen.getByRole("button", { name: "Add a task to Job Search" }));
    await user.type(
      screen.getByLabelText("Add to Job Search"),
      "email two contacts{Enter}",
    );

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("task_create", {
        input: {
          title: "email two contacts",
          project: "Job Search",
          horizon: "weekly",
          status: "planned",
          sourceType: "manual",
          periodStart: "2026-08-17",
          periodEnd: "2026-08-23",
        },
      });
    });
  });

  it("leaves the project unset when adding under the no-project heading", async () => {
    // "No project" is the board's word for no project at all, not a project
    // called "No project".
    const user = userEvent.setup();
    respond([task({ id: "a", project: null, priority: 9 })], [], {
      task_create: task({ id: "new", title: "pay the invoice", project: null }),
    });
    render(<WeeklyBoard />);
    await screen.findByRole("heading", { name: "No project" });

    await user.click(screen.getByRole("button", { name: "Add a task to No project" }));
    await user.type(
      screen.getByLabelText("Add to No project"),
      "pay the invoice{Enter}",
    );

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("task_create", {
        input: {
          title: "pay the invoice",
          project: null,
          horizon: "weekly",
          status: "planned",
          sourceType: "manual",
          periodStart: "2026-08-17",
          periodEnd: "2026-08-23",
        },
      });
    });
  });

  it("adds into a declared group that has no tasks yet", async () => {
    const user = userEvent.setup();
    respond([], [section("Job Search")], {
      task_create: task({ id: "new", title: "rewrite the resume" }),
    });
    render(<WeeklyBoard />);
    await screen.findByRole("heading", { name: "Job Search" });

    await user.click(screen.getByRole("button", { name: "Add a task to Job Search" }));
    await user.type(
      screen.getByLabelText("Add to Job Search"),
      "rewrite the resume{Enter}",
    );

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("task_create", {
        input: {
          title: "rewrite the resume",
          project: "Job Search",
          horizon: "weekly",
          status: "planned",
          sourceType: "manual",
          periodStart: "2026-08-17",
          periodEnd: "2026-08-23",
        },
      });
    });
  });

  it("opens the context menu on right-click", async () => {
    const user = userEvent.setup();
    respond([task()]);
    render(<WeeklyBoard />);
    await screen.findByText("submit applications");

    await user.pointer({
      keys: "[MouseRight]",
      target: screen.getByText("submit applications"),
    });

    expect(screen.getByRole("menu")).toBeInTheDocument();
  });

  it("moves a task to next week through the period command", async () => {
    // Through that path and no other: only it counts the move as a deferral.
    const user = userEvent.setup();
    respond([task()]);
    render(<WeeklyBoard />);
    await screen.findByText("submit applications");

    await user.pointer({
      keys: "[MouseRight]",
      target: screen.getByText("submit applications"),
    });
    await user.click(screen.getByRole("menuitem", { name: /next week/i }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("task_move_to_period", {
        id: "task-1",
        start: "2026-08-24",
        end: "2026-08-30",
      });
    });
  });

  it("adds a blocker through the blocker command", async () => {
    const user = userEvent.setup();
    respond([task()]);
    render(<WeeklyBoard />);
    await screen.findByText("submit applications");

    await user.pointer({
      keys: "[MouseRight]",
      target: screen.getByText("submit applications"),
    });
    await user.click(screen.getByRole("menuitem", { name: /add blocker/i }));
    await user.type(screen.getByLabelText("Blocker"), "portal is down{Enter}");

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("task_set_blocker", {
        id: "task-1",
        blocker: "portal is down",
      });
    });
  });

  it("adds a comment through the comment command", async () => {
    const user = userEvent.setup();
    respond([task()]);
    render(<WeeklyBoard />);
    await screen.findByText("submit applications");

    await user.pointer({
      keys: "[MouseRight]",
      target: screen.getByText("submit applications"),
    });
    await user.click(screen.getByRole("menuitem", { name: /comment/i }));
    await user.type(screen.getByLabelText("Comment"), "half done{Enter}");

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("task_add_comment", {
        id: "task-1",
        comment: "half done",
      });
    });
  });

  it("archives rather than deleting when asked to archive", async () => {
    const user = userEvent.setup();
    respond([task()]);
    render(<WeeklyBoard />);
    await screen.findByText("submit applications");

    await user.pointer({
      keys: "[MouseRight]",
      target: screen.getByText("submit applications"),
    });
    await user.click(screen.getByRole("menuitem", { name: /archive/i }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("task_archive", { id: "task-1" });
    });
    expect(mockInvoke).not.toHaveBeenCalledWith("task_delete", expect.anything());
  });

  it("moves a task to another day through the reschedule command", async () => {
    const user = userEvent.setup();
    respond([task()]);
    render(<WeeklyBoard />);
    await screen.findByText("submit applications");

    await user.pointer({
      keys: "[MouseRight]",
      target: screen.getByText("submit applications"),
    });
    await user.click(screen.getByRole("menuitem", { name: /move to another day/i }));
    await user.type(screen.getByLabelText("New date"), "2026-08-25");
    await user.keyboard("{Enter}");

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("task_reschedule", {
        id: "task-1",
        to: "2026-08-25",
      });
    });
  });

  it("completes a task from the row", async () => {
    const user = userEvent.setup();
    // The update has to answer with the completed task: the store replaces its
    // optimistic guess with whatever the backend actually stored.
    respond([task()], [], { task_update: task({ status: "completed" }) });
    render(<WeeklyBoard />);
    await screen.findByText("submit applications");

    await user.click(screen.getByRole("checkbox", { name: "submit applications" }));

    await waitFor(() => {
      expect(useTaskStore.getState().tasks["task-1"]?.status).toBe("completed");
    });
  });

  it("shows the week it is displaying", async () => {
    // A week board without its week is ambiguous the moment you look away.
    // The ISO label leads, matching the spec's mockups for both week boards.
    respond();

    render(<WeeklyBoard />);

    expect(await screen.findByText("2026-W34")).toBeInTheDocument();
    expect(screen.getByText(/2026-08-17/)).toBeInTheDocument();
  });

  it("surfaces a failure rather than failing silently", async () => {
    mockInvoke.mockImplementation((command: string) => {
      if (command === "week_current") return Promise.resolve(WEEK);
      return Promise.reject(new Error("disk is full"));
    });

    render(<WeeklyBoard />);

    expect(await screen.findByRole("alert")).toHaveTextContent("disk is full");
  });
});
