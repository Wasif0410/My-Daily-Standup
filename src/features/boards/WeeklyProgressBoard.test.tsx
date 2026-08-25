import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { WeeklyProgressBoard } from "@/features/boards/WeeklyProgressBoard";
import { onTaskChanged } from "@/lib/taskEvents";
import { useTaskStore } from "@/stores/taskStore";
import type { Task } from "@/types/task";

vi.mock("@/lib/taskEvents", () => ({
  emitTaskChanged: vi.fn(),
  onTaskChanged: vi.fn().mockResolvedValue(() => {}),
  TASK_CHANGED: "task-changed",
}));

const mockInvoke = vi.mocked(invoke);
const mockOnTaskChanged = vi.mocked(onTaskChanged);

function week(today = "2026-08-18") {
  return { ...WEEK, today };
}

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
    title: "complete onboarding task",
    description: null,
    horizon: "daily",
    status: "planned",
    parentTaskId: null,
    sourceType: "manual",
    sourceFile: null,
    sourceLine: null,
    area: null,
    project: null,
    priority: 8,
    scheduledDate: "2026-08-18",
    periodStart: null,
    periodEnd: null,
    dueDate: null,
    completedAt: null,
    progressCurrent: null,
    progressTarget: null,
    progressUnit: null,
    blocker: null,
    notes: null,
    timeSpentMinutes: 35,
    rolloverCount: 0,
    createdAt: "2026-08-17T00:00:00.000000Z",
    updatedAt: "2026-08-17T00:00:00.000000Z",
    ...overrides,
  };
}

/** Answers each command by name; `saved` is the stored expansion, if any. */
function respond(
  tasks: Task[] = [],
  saved: string | null = null,
  today = "2026-08-18",
) {
  mockInvoke.mockImplementation((command: string) => {
    if (command === "week_current") return Promise.resolve(week(today));
    if (command === "task_list_scheduled_between") return Promise.resolve(tasks);
    if (command === "ui_state_get") return Promise.resolve(saved);
    if (command === "ui_state_set") return Promise.resolve(undefined);
    return Promise.resolve(tasks[0] ?? null);
  });
}

beforeEach(() => {
  mockInvoke.mockReset();
  mockOnTaskChanged.mockClear();
  mockOnTaskChanged.mockResolvedValue(() => {});
  useTaskStore.setState({ tasks: {}, filter: null, loading: false, error: null });
});

describe("WeeklyProgressBoard", () => {
  it("renders all seven days on an empty week", async () => {
    // The DoD's first line. A week with seven empty days is a fact about the
    // week, not an absence of information.
    respond();

    render(<WeeklyProgressBoard />);

    for (const name of [
      "Monday",
      "Tuesday",
      "Wednesday",
      "Thursday",
      "Friday",
      "Saturday",
      "Sunday",
    ]) {
      expect(
        await screen.findByRole("button", { name: new RegExp(name) }),
      ).toBeInTheDocument();
    }
  });

  it("loads tasks scheduled inside the week", async () => {
    respond();

    render(<WeeklyProgressBoard />);

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("task_list_scheduled_between", {
        start: "2026-08-17",
        end: "2026-08-23",
      });
    });
  });

  it("summarises each day from its own tasks", async () => {
    respond(
      [
        task({
          id: "a",
          scheduledDate: "2026-08-18",
          status: "completed",
          timeSpentMinutes: 35,
        }),
        task({ id: "b", scheduledDate: "2026-08-18", timeSpentMinutes: null }),
        task({ id: "c", scheduledDate: "2026-08-19", timeSpentMinutes: null }),
      ],
      null,
      "2026-08-17",
    );

    render(<WeeklyProgressBoard />);

    expect(
      await screen.findByLabelText("1 of 2 done, 35m, 50 percent"),
    ).toBeInTheDocument();
    expect(
      screen.getByLabelText("0 of 1 done, no time recorded, 0 percent"),
    ).toBeInTheDocument();
  });

  it("expands today on a board that has never been touched", async () => {
    respond([task({ id: "a", scheduledDate: "2026-08-18" })]);

    render(<WeeklyProgressBoard />);

    const tuesday = await screen.findByRole("button", { name: /Tuesday/ });
    expect(tuesday).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByRole("button", { name: /Monday/ })).toHaveAttribute(
      "aria-expanded",
      "false",
    );
  });

  it("marks today so it reads differently from the rest", async () => {
    respond([], null, "2026-08-20");

    const { container } = render(<WeeklyProgressBoard />);
    await screen.findByRole("button", { name: /Thursday/ });

    const today = container.querySelectorAll('[data-today="true"]');
    expect(today).toHaveLength(1);
  });

  it("restores the saved expansion instead of the default", async () => {
    respond([], JSON.stringify(["2026-08-21"]));

    render(<WeeklyProgressBoard />);

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /Friday/ })).toHaveAttribute(
        "aria-expanded",
        "true",
      );
    });
  });

  it("honours a saved state that collapses today", async () => {
    // Once someone has collapsed today deliberately, re-expanding it every
    // launch is the app arguing with them.
    respond([], JSON.stringify([]));

    render(<WeeklyProgressBoard />);

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /Tuesday/ })).toHaveAttribute(
        "aria-expanded",
        "false",
      );
    });
  });

  it("persists a toggle", async () => {
    const user = userEvent.setup();
    respond([], JSON.stringify([]));
    render(<WeeklyProgressBoard />);
    await screen.findByRole("button", { name: /Monday/ });

    await user.click(screen.getByRole("button", { name: /Monday/ }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("ui_state_set", {
        key: "weekly-progress.expanded",
        value: JSON.stringify(["2026-08-17"]),
      });
    });
  });

  it("reloads when another window announces a change", async () => {
    // The DoD's live-update line. Boards are separate windows with separate
    // stores; a broadcast is the only way one can learn about the other.
    let announce = () => {};
    mockOnTaskChanged.mockImplementation((handler: () => void) => {
      announce = handler;
      return Promise.resolve(() => {});
    });
    respond();
    render(<WeeklyProgressBoard />);
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith(
        "task_list_scheduled_between",
        expect.anything(),
      );
    });

    const before = mockInvoke.mock.calls.filter(
      (c) => c[0] === "task_list_scheduled_between",
    ).length;
    announce();

    await waitFor(() => {
      const after = mockInvoke.mock.calls.filter(
        (c) => c[0] === "task_list_scheduled_between",
      ).length;
      expect(after).toBeGreaterThan(before);
    });
  });

  it("stops listening when it unmounts", async () => {
    // A closed board that kept reloading would keep the database busy for a
    // window nobody can see.
    const stop = vi.fn();
    mockOnTaskChanged.mockResolvedValue(stop);
    respond();
    const { unmount } = render(<WeeklyProgressBoard />);
    await screen.findByRole("button", { name: /Monday/ });

    unmount();

    await waitFor(() => {
      expect(stop).toHaveBeenCalled();
    });
  });

  it("keeps completed work visible", async () => {
    // The board is a record of the week, not a list that empties as work is
    // done (spec §6.4).
    respond([
      task({
        id: "a",
        title: "finished thing",
        scheduledDate: "2026-08-18",
        status: "completed",
      }),
    ]);

    render(<WeeklyProgressBoard />);

    expect(await screen.findByText("finished thing")).toBeInTheDocument();
  });

  it("shows the ISO week", async () => {
    respond();

    render(<WeeklyProgressBoard />);

    expect(await screen.findByText("2026-W34")).toBeInTheDocument();
  });

  it("surfaces a failure rather than failing silently", async () => {
    mockInvoke.mockImplementation((command: string) => {
      if (command === "week_current") return Promise.resolve(WEEK);
      if (command === "ui_state_get") return Promise.resolve(null);
      return Promise.reject(new Error("disk is full"));
    });

    render(<WeeklyProgressBoard />);

    expect(await screen.findByRole("alert")).toHaveTextContent("disk is full");
  });
});
