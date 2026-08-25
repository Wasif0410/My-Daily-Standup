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

/**
 * How much of a day is done, as a whole percentage.
 *
 * `100` is returned only when every counted task is actually finished. A day
 * that merely rounds up to 100 — 199 of 200 is 99.5 — reports 99 instead,
 * because a header reading 100% is the one signal that lets someone stop
 * looking at the day, and it must never say that while work remains.
 *
 * An empty day is 0, not 100 and not NaN. Dividing 0 by 0 would produce NaN
 * and render as "NaN%", and treating "nothing planned" as "everything done"
 * would light up an untouched day as finished.
 */
export function completionPercent(totals: DayTotals): number {
  if (totals.total <= 0) return 0;

  const exact = (totals.completed / totals.total) * 100;
  if (exact >= 100) return totals.completed >= totals.total ? 100 : 99;

  return Math.min(99, Math.max(0, Math.round(exact)));
}
