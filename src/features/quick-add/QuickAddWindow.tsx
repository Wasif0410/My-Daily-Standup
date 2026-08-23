import { useEffect, useState } from "react";
import { QuickAdd } from "@/features/boards/components/QuickAdd";
import { closeQuickAdd, createTask, currentWeek, toCommandError } from "@/lib/ipc";
import { emitTaskChanged } from "@/lib/taskEvents";
import type { CommandError } from "@/types/task";

/**
 * The tray's capture box (spec §6.8).
 *
 * One line, no chrome, gone as soon as it has taken the task. Deliberately not
 * a planning surface — asking for a horizon, a project, or a priority at
 * capture time is the friction that stops capture happening at all. Everything
 * it creates is a daily task for today, movable from the Weekly board
 * afterwards.
 *
 * Starts nothing. §6.8 is explicit that adding a normal task must not load a
 * model, and there is a test asserting no other command is called.
 */
export function QuickAddWindow() {
  const [today, setToday] = useState<string | null>(null);
  const [failure, setFailure] = useState<CommandError | null>(null);

  useEffect(() => {
    let ignore = false;

    // Asked of Rust, like every other date in the app. A browser clock would
    // file an 11pm capture on the wrong day.
    void (async () => {
      try {
        const week = await currentWeek();
        if (!ignore) setToday(week.today);
      } catch (caught) {
        if (!ignore) setFailure(toCommandError(caught));
      }
    })();

    return () => {
      ignore = true;
    };
  }, []);

  function capture(title: string) {
    void (async () => {
      try {
        await createTask({
          title,
          horizon: "daily",
          status: "planned",
          sourceType: "manual",
          scheduledDate: today,
        });

        // Every open board reloads. Without this the task is filed but
        // invisible until something else forces a refresh.
        emitTaskChanged();
        await closeQuickAdd();
      } catch (caught) {
        // Stay open. Closing on failure would throw away what was just typed,
        // with no record of it anywhere.
        setFailure(toCommandError(caught));
      }
    })();
  }

  return (
    <div
      className="quick-add-window"
      onKeyDown={(event) => {
        if (event.key === "Escape") void closeQuickAdd();
      }}
    >
      <QuickAdd placeholder="Quick add a task…" onAdd={capture} />

      {failure && (
        <p className="board-error" role="alert">
          {failure.message}
        </p>
      )}
    </div>
  );
}
