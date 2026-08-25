import { describe, expect, it } from "vitest";
import {
  bucketByDay,
  completionPercent,
  summarise,
} from "@/features/boards/daySummary";
import type { Task, WeekDay } from "@/types/task";

const WEEK: WeekDay[] = [
  { date: "2026-08-17", name: "Monday" },
  { date: "2026-08-18", name: "Tuesday" },
  { date: "2026-08-19", name: "Wednesday" },
  { date: "2026-08-20", name: "Thursday" },
  { date: "2026-08-21", name: "Friday" },
  { date: "2026-08-22", name: "Saturday" },
  { date: "2026-08-23", name: "Sunday" },
];

function task(overrides: Partial<Task> = {}): Task {
  return {
    id: "task-1",
    title: "a task",
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
    timeSpentMinutes: null,
    rolloverCount: 0,
    createdAt: "2026-08-17T00:00:00.000000Z",
    updatedAt: "2026-08-17T00:00:00.000000Z",
    ...overrides,
  };
}

describe("bucketByDay", () => {
  it("returns every day even when the week is empty", () => {
    // A week with seven empty days *is* the information. Returning only the
    // busy days would turn the board into a list and lose the shape.
    const days = bucketByDay(WEEK, []);

    expect(days).toHaveLength(7);
    expect(days.map((d) => d.name)).toEqual([
      "Monday",
      "Tuesday",
      "Wednesday",
      "Thursday",
      "Friday",
      "Saturday",
      "Sunday",
    ]);
  });

  it("puts a task on its scheduled day", () => {
    const days = bucketByDay(WEEK, [task({ id: "a", scheduledDate: "2026-08-19" })]);

    expect(days[2]?.tasks.map((t) => t.id)).toEqual(["a"]);
    expect(days[1]?.tasks).toEqual([]);
  });

  it("ignores a task scheduled outside the week", () => {
    // Defensive: the query already excludes them. But a stray task silently
    // attached to the wrong day would be worse than one that never appears.
    const days = bucketByDay(WEEK, [task({ id: "a", scheduledDate: "2026-09-01" })]);

    expect(days.every((d) => d.tasks.length === 0)).toBe(true);
  });

  it("ignores an unscheduled task", () => {
    const days = bucketByDay(WEEK, [task({ id: "a", scheduledDate: null })]);

    expect(days.every((d) => d.tasks.length === 0)).toBe(true);
  });

  it("orders a day's tasks by priority", () => {
    const days = bucketByDay(WEEK, [
      task({ id: "low", scheduledDate: "2026-08-18", priority: 2 }),
      task({ id: "high", scheduledDate: "2026-08-18", priority: 9 }),
    ]);

    expect(days[1]?.tasks.map((t) => t.id)).toEqual(["high", "low"]);
  });

  it("carries each day's totals", () => {
    const days = bucketByDay(WEEK, [
      task({
        id: "a",
        scheduledDate: "2026-08-18",
        status: "completed",
        timeSpentMinutes: 35,
      }),
      task({ id: "b", scheduledDate: "2026-08-18" }),
    ]);

    expect(days[1]?.totals).toEqual({ completed: 1, total: 2, minutes: 35 });
  });
});

describe("summarise", () => {
  it("reports nothing for an empty day", () => {
    expect(summarise([])).toEqual({ completed: 0, total: 0, minutes: null });
  });

  it("counts completed against total", () => {
    const totals = summarise([
      task({ id: "a", status: "completed" }),
      task({ id: "b" }),
      task({ id: "c" }),
    ]);

    expect(totals.completed).toBe(1);
    expect(totals.total).toBe(3);
  });

  it("excludes cancelled work from both counts", () => {
    // Archived work is not a failure to finish. Counting it against the day
    // would make tidying up look like falling behind.
    const totals = summarise([
      task({ id: "a", status: "completed" }),
      task({ id: "b", status: "cancelled" }),
    ]);

    expect(totals).toMatchObject({ completed: 1, total: 1 });
  });

  it("sums recorded minutes", () => {
    const totals = summarise([
      task({ id: "a", timeSpentMinutes: 45 }),
      task({ id: "b", timeSpentMinutes: 60 }),
    ]);

    expect(totals.minutes).toBe(105);
  });

  it("reports null when nothing was recorded, never zero", () => {
    // Zero would claim the day's work took no time. Nobody measured it.
    const totals = summarise([task({ id: "a" }), task({ id: "b" })]);

    expect(totals.minutes).toBeNull();
  });

  it("sums only what was recorded when some durations are missing", () => {
    // A half-logged day reads as partial, not as fast.
    const totals = summarise([
      task({ id: "a", timeSpentMinutes: 35 }),
      task({ id: "b", timeSpentMinutes: null }),
    ]);

    expect(totals.minutes).toBe(35);
  });

  it("counts time from cancelled work as unrecorded too", () => {
    // Excluded from the counts, so excluded from the total it would inflate.
    const totals = summarise([
      task({ id: "a", status: "cancelled", timeSpentMinutes: 120 }),
      task({ id: "b", timeSpentMinutes: 30 }),
    ]);

    expect(totals.minutes).toBe(30);
  });
});

describe("completionPercent", () => {
  it("rounds to the nearest whole percent", () => {
    expect(completionPercent({ completed: 2, total: 3, minutes: null })).toBe(67);
  });

  it("reports half a day as 50", () => {
    expect(completionPercent({ completed: 1, total: 2, minutes: null })).toBe(50);
  });

  it("reports a finished day as 100", () => {
    expect(completionPercent({ completed: 1, total: 1, minutes: null })).toBe(100);
  });

  it("reports an empty day as 0, never 100 and never NaN", () => {
    // Nothing planned is not everything done, and 0/0 must not divide into NaN.
    const percent = completionPercent({ completed: 0, total: 0, minutes: null });

    expect(percent).toBe(0);
    expect(Number.isNaN(percent)).toBe(false);
  });

  it("reports an untouched day as 0", () => {
    expect(completionPercent({ completed: 0, total: 4, minutes: null })).toBe(0);
  });

  it("reports 99 for a day that is all but finished rather than rounding up to 100", () => {
    // 199 of 200 is 99.5, which rounds to 100. Showing 100% while a task is
    // still open is a lie the user would act on — they would close the board
    // believing the day is done. 100 is reserved for actually done.
    expect(completionPercent({ completed: 199, total: 200, minutes: null })).toBe(99);
  });

  it("never goes above 100 or below 0", () => {
    expect(completionPercent({ completed: 5, total: 5, minutes: null })).toBe(100);
    expect(completionPercent({ completed: 0, total: 1, minutes: null })).toBe(0);
  });
});
