import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { DaySection } from "@/features/boards/components/DaySection";
import type { Day } from "@/features/boards/daySummary";
import type { Task } from "@/types/task";

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

function day(overrides: Partial<Day> = {}): Day {
  const tasks = overrides.tasks ?? [task()];
  return {
    date: "2026-08-18",
    name: "Tuesday",
    tasks,
    totals: { completed: 1, total: 2, minutes: 35 },
    ...overrides,
  };
}

function props(overrides: Record<string, unknown> = {}) {
  return {
    day: day(),
    expanded: false,
    today: false,
    onToggle: vi.fn(),
    renderTask: (t: Task) => <li key={t.id}>{t.title}</li>,
    ...overrides,
  };
}

describe("DaySection", () => {
  it("names the day", () => {
    render(<DaySection {...props()} />);

    expect(screen.getByRole("button", { name: /Tuesday/ })).toBeInTheDocument();
  });

  it("summarises without being opened", () => {
    // A collapsed board still has to answer "how did the week go".
    render(<DaySection {...props()} />);

    expect(screen.getByText("1/2")).toBeInTheDocument();
    expect(screen.getByText("35m")).toBeInTheDocument();
  });

  it("hides its tasks when collapsed", () => {
    render(<DaySection {...props()} />);

    expect(screen.queryByText("complete onboarding task")).toBeNull();
  });

  it("reveals its tasks when expanded", () => {
    render(<DaySection {...props({ expanded: true })} />);

    expect(screen.getByText("complete onboarding task")).toBeInTheDocument();
  });

  it("is a real disclosure, not a clickable div", () => {
    // Keyboard users get the same affordance, and a screen reader announces
    // whether the day is open.
    const { rerender } = render(<DaySection {...props()} />);
    expect(screen.getByRole("button", { name: /Tuesday/ })).toHaveAttribute(
      "aria-expanded",
      "false",
    );

    rerender(<DaySection {...props({ expanded: true })} />);
    expect(screen.getByRole("button", { name: /Tuesday/ })).toHaveAttribute(
      "aria-expanded",
      "true",
    );
  });

  it("reports a toggle to its owner rather than keeping the state", async () => {
    // The board persists expansion to the database, so the section cannot own
    // it — the two would drift apart after a failed write.
    const onToggle = vi.fn();
    const user = userEvent.setup();
    render(<DaySection {...props({ onToggle })} />);

    await user.click(screen.getByRole("button", { name: /Tuesday/ }));

    expect(onToggle).toHaveBeenCalledWith(true);
  });

  it("reports a collapse when it is already open", async () => {
    const onToggle = vi.fn();
    const user = userEvent.setup();
    render(<DaySection {...props({ expanded: true, onToggle })} />);

    await user.click(screen.getByRole("button", { name: /Tuesday/ }));

    expect(onToggle).toHaveBeenCalledWith(false);
  });

  it("marks today so it can be distinguished", () => {
    const { container } = render(<DaySection {...props({ today: true })} />);

    expect(container.querySelector(".day-section")).toHaveAttribute(
      "data-today",
      "true",
    );
  });

  it("renders an empty day rather than hiding it", () => {
    // A week with three empty days *is* the information.
    render(
      <DaySection
        {...props({
          day: day({ tasks: [], totals: { completed: 0, total: 0, minutes: null } }),
          expanded: true,
        })}
      />,
    );

    expect(screen.getByRole("button", { name: /Tuesday/ })).toBeInTheDocument();
    expect(screen.getByText("0/0")).toBeInTheDocument();
  });

  it("says an expanded day is empty rather than showing a blank space", () => {
    render(
      <DaySection
        {...props({
          day: day({ tasks: [], totals: { completed: 0, total: 0, minutes: null } }),
          expanded: true,
        })}
      />,
    );

    expect(screen.getByText(/nothing scheduled/i)).toBeInTheDocument();
  });
});
