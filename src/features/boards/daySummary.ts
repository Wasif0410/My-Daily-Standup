import { sortTasks } from "@/stores/taskStore";
import type { Task, WeekDay } from "@/types/task";

/** What a day header reports without being opened. */
export interface DayTotals {
  completed: number;
  total: number;
  /** Recorded minutes, or `null` when nothing was measured — never zero. */
  minutes: number | null;
}

/** One day of the week, with its tasks and its summary. */
export interface Day {
  date: string;
  name: string;
  tasks: Task[];
  totals: DayTotals;
}

/**
 * Summarises a day's tasks.
 *
 * Cancelled work is excluded from every figure. Archiving something is not a
 * failure to finish it, and counting it against the day would make tidying up
 * look like falling behind — including its recorded time, which would inflate
 * a total the user no longer considers part of the day.
 *
 * `minutes` is `null` when nothing was recorded, never `0`. Zero claims the
 * work took no time; nobody measured it. A day where only some tasks were
 * logged sums those and reads as partial rather than as fast.
 */
export function summarise(tasks: Task[]): DayTotals {
  const counted = tasks.filter((task) => task.status !== "cancelled");

  const recorded = counted
    .map((task) => task.timeSpentMinutes)
    .filter((minutes): minutes is number => minutes !== null);

  return {
    completed: counted.filter((task) => task.status === "completed").length,
    total: counted.length,
    minutes:
      recorded.length === 0
        ? null
        : recorded.reduce((sum, minutes) => sum + minutes, 0),
  };
}

/**
 * Sorts a week's tasks into its seven days.
 *
 * Every day is returned, empty ones included. A week with three empty days *is*
 * the information — showing only the busy ones turns the board into a list and
 * loses the shape of the week (spec §6.4).
 *
 * A task whose date falls outside the week is dropped rather than attached
 * somewhere. The query already excludes them; a stray one landing on the wrong
 * day would be worse than one that never appears.
 *
 * Pure, so components can memoise it — called inline in a selector it would
 * return a fresh array on every render and loop.
 */
export function bucketByDay(days: WeekDay[], tasks: Task[]): Day[] {
  const byDate = new Map<string, Task[]>(days.map((day) => [day.date, []]));

  for (const task of tasks) {
    if (task.scheduledDate === null) continue;
    byDate.get(task.scheduledDate)?.push(task);
  }

  return days.map((day) => {
    const members = sortTasks(byDate.get(day.date) ?? []);
    return {
      date: day.date,
      name: day.name,
      tasks: members,
      totals: summarise(members),
    };
  });
}
