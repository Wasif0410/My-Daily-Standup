import { useCallback, useEffect, useState } from "react";
import { ProgressBar } from "@/features/boards/components/ProgressBar";
import { currentMonth, monthlyProgress, toCommandError } from "@/lib/ipc";
import { onTaskChanged } from "@/lib/taskEvents";
import type { Commitment, CommandError, Month, TaskProgress } from "@/types/task";

/**
 * The figure above the bar, e.g. `12 / 20 applications`.
 *
 * Three shapes because the three kinds of progress are three different
 * statements. A binary commitment gets words rather than `1 / 1`, which would
 * imply a measure the task does not have.
 */
function figure(progress: TaskProgress, unit: string | null): string {
  switch (progress.kind) {
    case "numeric":
      return unit
        ? `${progress.current} / ${progress.target} ${unit}`
        : `${progress.current} / ${progress.target}`;
    case "subtasks":
      return `${progress.completed} / ${progress.total} done`;
    case "binary":
      return progress.completed ? "Done" : "Not done";
  }
}

/**
 * The month as outcomes rather than tasks (spec §6.5).
 *
 * Read-only by design: a month view that could be edited would duplicate the
 * Weekly board's job at the wrong altitude. That is also why it holds its own
 * state instead of using the task store — it never mutates anything, and its
 * data is commitments-with-progress rather than plain tasks.
 *
 * Every number here was computed in Rust, including the bar's fraction. The
 * component formats; it does not calculate (spec §3.6).
 */
export function MonthlyBoard() {
  const [month, setMonth] = useState<Month | null>(null);
  const [commitments, setCommitments] = useState<Commitment[]>([]);
  const [failure, setFailure] = useState<CommandError | null>(null);

  useEffect(() => {
    let ignore = false;

    void (async () => {
      try {
        const found = await currentMonth();
        if (!ignore) setMonth(found);
      } catch (caught) {
        if (!ignore) setFailure(toCommandError(caught));
      }
    })();

    return () => {
      ignore = true;
    };
  }, []);

  const reload = useCallback(() => {
    if (!month) return;

    void (async () => {
      try {
        setCommitments(await monthlyProgress(month.start, month.end));
        setFailure(null);
      } catch (caught) {
        setFailure(toCommandError(caught));
      }
    })();
  }, [month]);

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

  return (
    <>
      {failure && (
        <p className="board-error" role="alert">
          {failure.message}
        </p>
      )}

      {month && <p className="board-week">{month.label}</p>}

      {commitments.length === 0 ? (
        // Distinct from "everything is at 0%": a month with nothing committed
        // is a different state from a month going badly.
        <p className="board-empty">No commitments this month.</p>
      ) : (
        commitments.map(({ task, progress, fraction }) => (
          <section className="commitment" key={task.id}>
            <h2 className="board-section-title">{task.title}</h2>
            <p className="commitment-figure">{figure(progress, task.progressUnit)}</p>
            <ProgressBar fraction={fraction} label={task.title} />
          </section>
        ))
      )}
    </>
  );
}
