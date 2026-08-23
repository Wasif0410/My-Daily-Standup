import { useCallback, useEffect, useMemo, useState } from "react";
import { DaySection } from "@/features/boards/components/DaySection";
import { TaskRow } from "@/features/boards/components/TaskRow";
import { bucketByDay } from "@/features/boards/daySummary";
import { currentWeek, getUiState, setUiState, toCommandError } from "@/lib/ipc";
import { onTaskChanged } from "@/lib/taskEvents";
import { useTaskStore } from "@/stores/taskStore";
import type { CommandError, Week } from "@/types/task";

/** Where this board's expanded days are stored. */
const EXPANDED_KEY = "weekly-progress.expanded";

/**
 * The whole week, Monday to Sunday, as seven collapsible days (spec §6.4).
 *
 * A record rather than a worklist: every day is listed even when empty,
 * completed work stays visible, and the collapsed header still answers "how did
 * the week go".
 *
 * Reloads whenever any window announces a task change. Boards are separate
 * Tauri windows with separate stores, so completing something on the Weekly
 * Tasks board cannot reach this one any other way.
 */
export function WeeklyProgressBoard() {
  const [week, setWeek] = useState<Week | null>(null);
  const [failure, setFailure] = useState<CommandError | null>(null);
  /** Null until the saved state has been read, so the default is not applied
   *  over the top of a choice that is still loading. */
  const [expanded, setExpanded] = useState<Set<string> | null>(null);

  const tasks = useTaskStore((state) => state.tasks);
  const storeError = useTaskStore((state) => state.error);
  const load = useTaskStore((state) => state.load);
  const complete = useTaskStore((state) => state.complete);
  const uncomplete = useTaskStore((state) => state.uncomplete);
  const editTitle = useTaskStore((state) => state.editTitle);
  const recordTimeSpent = useTaskStore((state) => state.recordTimeSpent);

  useEffect(() => {
    let ignore = false;

    void (async () => {
      try {
        const found = await currentWeek();
        if (ignore) return;
        setWeek(found);

        const saved = await getUiState(EXPANDED_KEY);
        if (ignore) return;

        // A saved choice wins outright, including one that expands nothing.
        // Re-opening today every launch after someone deliberately closed it
        // is the app arguing with the user.
        setExpanded(
          saved === null
            ? new Set([found.today])
            : new Set(JSON.parse(saved) as string[]),
        );
      } catch (caught) {
        if (!ignore) setFailure(toCommandError(caught));
      }
    })();

    return () => {
      ignore = true;
    };
  }, []);

  const reload = useCallback(() => {
    if (!week) return;
    void load({ kind: "scheduled", start: week.start, end: week.end });
  }, [load, week]);

  useEffect(reload, [reload]);

  useEffect(() => {
    let stop: (() => void) | null = null;
    let cancelled = false;

    void onTaskChanged(reload).then((off) => {
      // Unmounting before the subscription resolves would otherwise leave a
      // listener nothing can reach to remove.
      if (cancelled) off();
      else stop = off;
    });

    return () => {
      cancelled = true;
      stop?.();
    };
  }, [reload]);

  const days = useMemo(
    // Memoised: bucketing builds fresh arrays, and an unmemoised call in a
    // selector would hand React a new value every render and loop.
    () => (week ? bucketByDay(week.days, Object.values(tasks)) : []),
    [week, tasks],
  );

  function toggle(date: string, open: boolean) {
    const next = new Set(expanded ?? []);
    if (open) next.add(date);
    else next.delete(date);

    // Optimistic, like every other board interaction: the day opens now and
    // the write follows.
    setExpanded(next);
    void setUiState(EXPANDED_KEY, JSON.stringify([...next])).catch(
      (caught: unknown) => {
        setFailure(toCommandError(caught));
      },
    );
  }

  const error = failure ?? storeError;

  return (
    <>
      {error && (
        <p className="board-error" role="alert">
          {error.message}
        </p>
      )}

      {week && <p className="board-week">{week.label}</p>}

      {days.map((day) => (
        <DaySection
          key={day.date}
          day={day}
          expanded={expanded?.has(day.date) ?? false}
          today={day.date === week?.today}
          onToggle={(open) => toggle(day.date, open)}
          renderTask={(task) => (
            <TaskRow
              key={task.id}
              task={task}
              onComplete={(completed) => {
                void (completed ? complete(task.id) : uncomplete(task.id));
              }}
              onEdit={(title) => void editTitle(task.id, title)}
              onSetTimeSpent={(minutes) => void recordTimeSpent(task.id, minutes)}
            />
          )}
        />
      ))}
    </>
  );
}
