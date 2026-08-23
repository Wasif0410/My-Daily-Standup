import { sortTasks } from "@/stores/taskStore";
import type { Task } from "@/types/task";

/** Where tasks with no area land. */
export const UNSORTED_AREA = "Unsorted";

export interface AreaGrouping {
  area: string;
  /** Already sorted for display. */
  tasks: Task[];
}

/** The highest priority in a group, for ordering the groups themselves. */
function topPriority(tasks: Task[]): number {
  return tasks.reduce((best, task) => Math.max(best, task.priority ?? -1), -1);
}

/**
 * Groups tasks by area, most important area first.
 *
 * Ordered by each group's highest-priority task rather than alphabetically:
 * this is the Priority board, and an alphabetical sort would file the urgent
 * area under whatever letter it happens to start with.
 *
 * The unsorted group is ordered by the same rule and given no special place. A
 * task can be both unfiled and the most important thing on the board, and
 * pinning the group to the bottom for tidiness would bury it.
 *
 * Pure and standalone so components can memoise it — called inside a zustand
 * selector it would return a fresh array on every render and loop.
 */
export function groupByArea(tasks: Task[]): AreaGrouping[] {
  const byArea = new Map<string, Task[]>();

  for (const task of tasks) {
    // A blank area is an absent one. Obsidian frontmatter round-trips an unset
    // value as an empty string, and a heading with no text is not a heading.
    const trimmed = task.area?.trim();
    const area = trimmed ? trimmed : UNSORTED_AREA;

    const existing = byArea.get(area);
    if (existing) {
      existing.push(task);
    } else {
      byArea.set(area, [task]);
    }
  }

  return [...byArea.entries()]
    .map(([area, grouped]) => ({ area, tasks: sortTasks(grouped) }))
    .sort((a, b) => {
      const difference = topPriority(b.tasks) - topPriority(a.tasks);
      return difference !== 0 ? difference : a.area.localeCompare(b.area);
    });
}
