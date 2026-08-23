import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { AreaGroup } from "@/features/boards/components/AreaGroup";
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

describe("AreaGroup", () => {
  it("heads the group with its area", () => {
    render(
      <AreaGroup
        area="Job search"
        tasks={[task()]}
        renderTask={(t) => <li key={t.id}>{t.title}</li>}
      />,
    );

    expect(screen.getByRole("heading", { name: "Job search" })).toBeInTheDocument();
  });

  it("renders each task through the callback it is given", () => {
    render(
      <AreaGroup
        area="Health"
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
      <AreaGroup
        area="Health"
        tasks={[task()]}
        renderTask={(t) => <li key={t.id}>{t.title}</li>}
      />,
    );

    expect(screen.getByRole("list", { name: "Health" })).toBeInTheDocument();
  });
});
