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

/**
 * The key two names are compared on: a heading is the same heading in any case.
 *
 * Exported because the boards match a group back to the section row that
 * declared it, and a second normaliser there could disagree with this one —
 * which would show a delete button on the wrong heading.
 */
export function headingKey(label: string): string {
  return label.trim().toLowerCase();
}

/**
 * Folds a board's declared groups into the ones its tasks produced.
 *
 * A group has two possible origins and they have to end up as one list. Most
 * headings are derived — they exist because some task carries that `area` or
 * `project`. A declared one was typed by the user before anything was filed
 * under it, and it has to survive being empty: a derived group vanishes the
 * moment its last task leaves, so without this, naming a group and then
 * looking for it would find nothing.
 *
 * A declared name that matches a derived one is the *same* group, not a second
 * one. The match is trimmed and case-insensitive because "JOB SEARCH" typed
 * into a heading field and "Job search" sitting on a task are one heading, and
 * rendering both would show the board's own structure as duplicated. The
 * derived label wins: the tasks are what the group is, and their spelling is
 * the one the rest of the app round-trips through `tasks.area`.
 *
 * Empty declared groups go on top. Derived groups keep the top-priority order
 * they already have, but a group with no tasks has no priority to be ordered
 * by, and putting it last would drop a heading the user made *seconds ago*
 * below a screenful of work — which reads as the button having done nothing.
 * Among themselves they hold the order they arrived in, which is the position
 * Rust assigned; re-sorting here would be a second opinion about that order.
 */
export function withDeclaredGroups(
  groups: TaskGrouping[],
  declared: string[],
): TaskGrouping[] {
  const taken = new Set(groups.map((group) => headingKey(group.label)));
  const empty: TaskGrouping[] = [];

  for (const name of declared) {
    const label = name.trim();
    // A heading with no text is not a heading — the same rule `groupBy`
    // applies to a blank `area`.
    if (!label) continue;

    const key = headingKey(label);
    // `taken` grows as it goes, so a name declared twice renders once.
    if (taken.has(key)) continue;

    taken.add(key);
    empty.push({ label, tasks: [] });
  }

  return [...empty, ...groups];
}
