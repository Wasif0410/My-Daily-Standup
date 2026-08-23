import { sortTasks } from "@/stores/taskStore";
import type { Task } from "@/types/task";

/** Where tasks with no area land. */
export const UNSORTED_AREA = "Unsorted";

/** Where tasks with no project land. */
export const UNSORTED_PROJECT = "No project";

export interface TaskGrouping {
  label: string;
  /** Already sorted for display. */
  tasks: Task[];
}

/** The highest priority in a group, for ordering the groups themselves. */
function topPriority(tasks: Task[]): number {
  return tasks.reduce((best, task) => Math.max(best, task.priority ?? -1), -1);
}

/**
 * Groups tasks by whatever `keyOf` returns, most important group first.
 *
 * Ordered by each group's highest-priority task rather than alphabetically. On
 * the Priority board an alphabetical sort would file the urgent area under
 * whatever letter it happens to start with; on the Weekly board it would do the
 * same to the project that actually needs attention this week.
 *
 * The fallback group is ordered by the same rule and given no special place. A
 * task can be both unfiled and the most important thing on the board, and
 * pinning the group to the bottom for tidiness would bury it.
 *
 * Pure and standalone so components can memoise it — called inside a zustand
 * selector it would return a fresh array on every render and loop.
 */
function groupBy(
  tasks: Task[],
  keyOf: (task: Task) => string | null,
  fallback: string,
): TaskGrouping[] {
  const grouped = new Map<string, Task[]>();

  for (const task of tasks) {
    // A blank key is an absent one. Obsidian frontmatter round-trips an unset
    // value as an empty string, and a heading with no text is not a heading.
    const trimmed = keyOf(task)?.trim();
    const label = trimmed ? trimmed : fallback;

    const existing = grouped.get(label);
    if (existing) {
      existing.push(task);
    } else {
      grouped.set(label, [task]);
    }
  }

  return [...grouped.entries()]
    .map(([label, members]) => ({ label, tasks: sortTasks(members) }))
    .sort((a, b) => {
      const difference = topPriority(b.tasks) - topPriority(a.tasks);
      return difference !== 0 ? difference : a.label.localeCompare(b.label);
    });
}

/** Groups by area, for the Priority board (spec §6.2). */
export function groupByArea(tasks: Task[]): TaskGrouping[] {
  return groupBy(tasks, (task) => task.area, UNSORTED_AREA);
}

/** Groups by project, for the Weekly Tasks board (spec §6.3). */
export function groupByProject(tasks: Task[]): TaskGrouping[] {
  return groupBy(tasks, (task) => task.project, UNSORTED_PROJECT);
}
