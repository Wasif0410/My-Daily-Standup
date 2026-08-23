import { render, screen, waitFor } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { MonthlyBoard } from "@/features/boards/MonthlyBoard";
import { onTaskChanged } from "@/lib/taskEvents";
import type { Commitment, Task, TaskProgress } from "@/types/task";

vi.mock("@/lib/taskEvents", () => ({
  emitTaskChanged: vi.fn(),
  onTaskChanged: vi.fn().mockResolvedValue(() => {}),
  TASK_CHANGED: "task-changed",
}));

const mockInvoke = vi.mocked(invoke);
const mockOnTaskChanged = vi.mocked(onTaskChanged);

const MONTH = {
  start: "2026-08-01",
  end: "2026-08-31",
  label: "August 2026",
  today: "2026-08-23",
};

function task(overrides: Partial<Task> = {}): Task {
  return {
    id: "task-1",
    title: "Job search",
    description: null,
    horizon: "monthly",
    status: "planned",
    parentTaskId: null,
    sourceType: "manual",
    sourceFile: null,
    sourceLine: null,
    area: null,
    project: null,
    priority: 8,
    scheduledDate: null,
    periodStart: MONTH.start,
    periodEnd: MONTH.end,
    dueDate: null,
    completedAt: null,
    progressCurrent: 12,
    progressTarget: 20,
    progressUnit: "applications",
    blocker: null,
    notes: null,
    timeSpentMinutes: null,
    rolloverCount: 0,
    createdAt: "2026-08-01T00:00:00.000000Z",
    updatedAt: "2026-08-01T00:00:00.000000Z",
    ...overrides,
  };
}

function commitment(
  progress: TaskProgress,
  fraction: number,
  overrides: Partial<Task> = {},
): Commitment {
  return {
    task: task(overrides),
    progress,
    fraction,
    complete: fraction >= 1,
  };
}

function respond(commitments: Commitment[] = []) {
  mockInvoke.mockImplementation((command: string) => {
    if (command === "month_current") return Promise.resolve(MONTH);
    if (command === "task_monthly_progress") return Promise.resolve(commitments);
    return Promise.resolve(null);
  });
}

beforeEach(() => {
  mockInvoke.mockReset();
  mockOnTaskChanged.mockClear();
  mockOnTaskChanged.mockResolvedValue(() => {});
});

describe("MonthlyBoard", () => {
  it("asks Rust for the current month", async () => {
    respond();

    render(<MonthlyBoard />);

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("month_current", undefined);
    });
  });

  it("loads the commitments for that month", async () => {
    respond();

    render(<MonthlyBoard />);

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("task_monthly_progress", {
        start: "2026-08-01",
        end: "2026-08-31",
      });
    });
  });

  it("shows the month in words", async () => {
    respond();

    render(<MonthlyBoard />);

    expect(await screen.findByText("August 2026")).toBeInTheDocument();
  });

  it("names each commitment", async () => {
    respond([commitment({ kind: "numeric", current: 12, target: 20 }, 0.6)]);

    render(<MonthlyBoard />);

    expect(
      await screen.findByRole("heading", { name: "Job search" }),
    ).toBeInTheDocument();
  });

  it("renders a numeric commitment with its unit", async () => {
    respond([commitment({ kind: "numeric", current: 12, target: 20 }, 0.6)]);

    render(<MonthlyBoard />);

    expect(await screen.findByText("12 / 20 applications")).toBeInTheDocument();
  });

  it("renders a numeric commitment with no unit", async () => {
    respond([
      commitment({ kind: "numeric", current: 4, target: 7 }, 0.57, {
        progressUnit: null,
      }),
    ]);

    render(<MonthlyBoard />);

    expect(await screen.findByText("4 / 7")).toBeInTheDocument();
  });

  it("keeps the true figure when a commitment is over target", async () => {
    // 22 of 20 is a good outcome. The bar is clamped; the number is not.
    respond([commitment({ kind: "numeric", current: 22, target: 20 }, 1)]);

    render(<MonthlyBoard />);

    expect(await screen.findByText("22 / 20 applications")).toBeInTheDocument();
  });

  it("renders a subtask commitment as a count of children", async () => {
    respond([
      commitment({ kind: "subtasks", completed: 2, total: 3 }, 0.667, {
        title: "MVP",
        progressTarget: null,
        progressCurrent: null,
        progressUnit: null,
      }),
    ]);

    render(<MonthlyBoard />);

    expect(await screen.findByText("2 / 3 done")).toBeInTheDocument();
  });

  it("renders a binary commitment without inventing a denominator", async () => {
    // "1 / 1" would imply a measure the task does not have.
    respond([
      commitment({ kind: "binary", completed: false }, 0, {
        title: "Transfer email accounts",
        progressTarget: null,
        progressCurrent: null,
        progressUnit: null,
      }),
    ]);

    render(<MonthlyBoard />);

    expect(await screen.findByText("Not done")).toBeInTheDocument();
    expect(screen.queryByText("1 / 1")).toBeNull();
  });

  it("renders a completed binary commitment as done", async () => {
    respond([
      commitment({ kind: "binary", completed: true }, 1, {
        progressTarget: null,
        progressCurrent: null,
        progressUnit: null,
      }),
    ]);

    render(<MonthlyBoard />);

    expect(await screen.findByText("Done")).toBeInTheDocument();
  });

  it("renders the bar from the fraction Rust supplied", async () => {
    // Proves the component divided nothing itself: the fraction here does not
    // match 12/20, and the bar follows the fraction.
    respond([commitment({ kind: "numeric", current: 12, target: 20 }, 0.25)]);

    render(<MonthlyBoard />);

    const bar = await screen.findByRole("progressbar", { name: "Job search" });
    expect(bar).toHaveAttribute("aria-valuenow", "25");
  });

  it("says the month has no commitments rather than showing nothing", async () => {
    // An empty board must not imply everything is at 0%.
    respond();

    render(<MonthlyBoard />);

    expect(await screen.findByText(/no commitments this month/i)).toBeInTheDocument();
  });

  it("refetches when another window announces a change", async () => {
    let announce = () => {};
    mockOnTaskChanged.mockImplementation((handler: () => void) => {
      announce = handler;
      return Promise.resolve(() => {});
    });
    respond();
    render(<MonthlyBoard />);
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith(
        "task_monthly_progress",
        expect.anything(),
      );
    });

    const before = mockInvoke.mock.calls.filter(
      (c) => c[0] === "task_monthly_progress",
    ).length;
    announce();

    await waitFor(() => {
      const after = mockInvoke.mock.calls.filter(
        (c) => c[0] === "task_monthly_progress",
      ).length;
      expect(after).toBeGreaterThan(before);
    });
  });

  it("stops listening when it unmounts", async () => {
    const stop = vi.fn();
    mockOnTaskChanged.mockResolvedValue(stop);
    respond();
    const { unmount } = render(<MonthlyBoard />);
    await screen.findByText("August 2026");

    unmount();

    await waitFor(() => {
      expect(stop).toHaveBeenCalled();
    });
  });

  it("surfaces a failure rather than failing silently", async () => {
    mockInvoke.mockImplementation((command: string) => {
      if (command === "month_current") return Promise.resolve(MONTH);
      return Promise.reject(new Error("disk is full"));
    });

    render(<MonthlyBoard />);

    expect(await screen.findByRole("alert")).toHaveTextContent("disk is full");
  });
});
