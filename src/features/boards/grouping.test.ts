import { describe, expect, it } from "vitest";
import { groupByArea, UNSORTED_AREA } from "@/features/boards/grouping";
import type { Task } from "@/types/task";

function task(overrides: Partial<Task> = {}): Task {
  return {
    id: "task-1",
    title: "a task",
    description: null,
    horizon: "weekly",
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
    timeSpentMinutes: null,
    rolloverCount: 0,
    createdAt: "2026-08-21T00:00:00.000000Z",
    updatedAt: "2026-08-21T00:00:00.000000Z",
    ...overrides,
  };
}

describe("groupByArea", () => {
  it("returns nothing for no tasks", () => {
    expect(groupByArea([])).toEqual([]);
  });

  it("collects tasks under their area", () => {
    const groups = groupByArea([
      task({ id: "a", area: "Health", priority: 6 }),
      task({ id: "b", area: "Health", priority: 5 }),
      task({ id: "c", area: "Job search", priority: 9 }),
    ]);

    expect(groups.map((g) => g.area)).toEqual(["Job search", "Health"]);
    expect(groups[1]?.tasks.map((t) => t.id)).toEqual(["a", "b"]);
  });

  it("orders groups by their most important task", () => {
    // This is the Priority board. Sorting areas alphabetically would push the
    // urgent one below "Admin".
    const groups = groupByArea([
      task({ id: "a", area: "Admin", priority: 5 }),
      task({ id: "b", area: "Zebra", priority: 10 }),
    ]);

    expect(groups.map((g) => g.area)).toEqual(["Zebra", "Admin"]);
  });

  it("breaks a tie between areas alphabetically", () => {
    const groups = groupByArea([
      task({ id: "a", area: "Health", priority: 7 }),
      task({ id: "b", area: "Admin", priority: 7 }),
    ]);

    expect(groups.map((g) => g.area)).toEqual(["Admin", "Health"]);
  });

  it("sorts within a group by priority, then oldest first", () => {
    const groups = groupByArea([
      task({
        id: "old",
        area: "Health",
        priority: 7,
        createdAt: "2026-08-01T00:00:00.000000Z",
      }),
      task({
        id: "new",
        area: "Health",
        priority: 7,
        createdAt: "2026-08-20T00:00:00.000000Z",
      }),
      task({ id: "top", area: "Health", priority: 9 }),
    ]);

    expect(groups[0]?.tasks.map((t) => t.id)).toEqual(["top", "old", "new"]);
  });

  it("gathers tasks with no area under one heading", () => {
    const groups = groupByArea([task({ id: "a", area: null, priority: 8 })]);

    expect(groups[0]?.area).toBe(UNSORTED_AREA);
  });

  it("treats a blank area as no area rather than as its own group", () => {
    // Obsidian frontmatter round-trips an empty value as "", and a group
    // headed by nothing at all is not a group.
    const groups = groupByArea([
      task({ id: "a", area: "  ", priority: 8 }),
      task({ id: "b", area: null, priority: 7 }),
    ]);

    expect(groups).toHaveLength(1);
    expect(groups[0]?.area).toBe(UNSORTED_AREA);
  });
});
