import { useEffect, useMemo, useState } from "react";
import { QuickAdd } from "@/features/boards/components/QuickAdd";
import { TaskContextMenu } from "@/features/boards/components/TaskContextMenu";
import { TaskGroup } from "@/features/boards/components/TaskGroup";
import { TaskRow } from "@/features/boards/components/TaskRow";
import { groupByProject } from "@/features/boards/grouping";
import { currentWeek, toCommandError } from "@/lib/ipc";
import { sortTasks, useTaskStore } from "@/stores/taskStore";
import type { CommandError, Task, Week } from "@/types/task";

/** Which row's menu is open, and where it was summoned. */
interface OpenMenu {
  taskId: string;
  x: number;
  y: number;
}

/** The seven days after a week ends — the "next week" a task moves into. */
function nextWeek(week: Week): Week {
  return { start: shift(week.start, 7), end: shift(week.end, 7) };
}

/**
 * Adds whole days to an ISO date.
 *
 * The one date calculation the frontend does, and it is deliberately the most
 * trivial kind: a fixed seven-day offset from a boundary Rust already computed.
 * Anything involving a week start, a locale, or "today" is asked of Rust.
 */
function shift(date: string, days: number): string {
  const shifted = new Date(`${date}T00:00:00Z`);
  shifted.setUTCDate(shifted.getUTCDate() + days);
  return shifted.toISOString().slice(0, 10);
}

/**
 * This week's commitments, grouped by project (spec §6.3).
 *
 * Owns three things the components below deliberately do not: which week is
 * being shown, which row's context menu is open, and every store action the
 * menu dispatches. `TaskRow`, `TaskGroup`, and `TaskContextMenu` stay free of
 * the store so PRs 14 and 15 can reuse them against different queries.
 */
export function WeeklyBoard() {
  const [week, setWeek] = useState<Week | null>(null);
  const [weekError, setWeekError] = useState<CommandError | null>(null);
  const [menu, setMenu] = useState<OpenMenu | null>(null);
  const [editing, setEditing] = useState<string | null>(null);

  const tasks = useTaskStore((state) => state.tasks);
  const storeError = useTaskStore((state) => state.error);
  const load = useTaskStore((state) => state.load);
  const add = useTaskStore((state) => state.add);
  const complete = useTaskStore((state) => state.complete);
  const uncomplete = useTaskStore((state) => state.uncomplete);
  const editTitle = useTaskStore((state) => state.editTitle);
  const recordTimeSpent = useTaskStore((state) => state.recordTimeSpent);
  const setPriority = useTaskStore((state) => state.setPriority);
  const moveToDate = useTaskStore((state) => state.moveToDate);
  const moveToWeek = useTaskStore((state) => state.moveToWeek);
  const promote = useTaskStore((state) => state.promote);
  const setBlocker = useTaskStore((state) => state.setBlocker);
  const addComment = useTaskStore((state) => state.addComment);
  const archive = useTaskStore((state) => state.archive);
  const remove = useTaskStore((state) => state.remove);

  useEffect(() => {
    let ignore = false;

    void (async () => {
      try {
        const found = await currentWeek();
        if (!ignore) setWeek(found);
      } catch (caught) {
        if (!ignore) setWeekError(toCommandError(caught));
      }
    })();

    return () => {
      ignore = true;
    };
  }, []);

  useEffect(() => {
    if (!week) return;
    void load({ kind: "period", start: week.start, end: week.end });
  }, [load, week]);

  const groups = useMemo(() => {
    // Archived work is cancelled, not deleted: the row survives in the
    // database and the board simply stops showing it.
    const live = Object.values(tasks).filter((task) => task.status !== "cancelled");
    return groupByProject(sortTasks(live));
  }, [tasks]);

  const menuTask = menu ? tasks[menu.taskId] : undefined;
  const error = weekError ?? storeError;

  function rowActions(task: Task) {
    return {
      onComplete: (completed: boolean) => {
        void (completed ? complete(task.id) : uncomplete(task.id));
      },
      onEdit: (title: string) => void editTitle(task.id, title),
      onSetTimeSpent: (minutes: number | null) =>
        void recordTimeSpent(task.id, minutes),
      onDelete: () => void remove(task.id),
      onOpenMenu: (at: { x: number; y: number }) =>
        setMenu({ taskId: task.id, x: at.x, y: at.y }),
    };
  }

  return (
    <>
      {error && (
        <p className="board-error" role="alert">
          {error.message}
        </p>
      )}

      {week && <p className="board-week">{`${week.start} → ${week.end}`}</p>}

      <QuickAdd
        placeholder="Add to this week…"
        onAdd={(title) => {
          if (!week) return;
          void add({
            title,
            horizon: "weekly",
            status: "planned",
            sourceType: "manual",
            periodStart: week.start,
            periodEnd: week.end,
          });
        }}
      />

      {groups.length === 0 ? (
        <p className="board-empty">Nothing planned this week.</p>
      ) : (
        groups.map((group) => (
          <TaskGroup
            key={group.label}
            label={group.label}
            tasks={group.tasks}
            renderTask={(task) => (
              <TaskRow
                key={task.id}
                task={task}
                editingRequested={editing === task.id}
                onEditingHandled={() => setEditing(null)}
                {...rowActions(task)}
              />
            )}
          />
        ))
      )}

      {menu && menuTask && (
        <TaskContextMenu
          task={menuTask}
          x={menu.x}
          y={menu.y}
          onClose={() => setMenu(null)}
          onComplete={(completed) => {
            void (completed ? complete(menuTask.id) : uncomplete(menuTask.id));
          }}
          // The menu cannot type into the row, so it asks the row to open its
          // own editor rather than duplicating one.
          onEdit={() => setEditing(menuTask.id)}
          onSetPriority={(priority) => void setPriority(menuTask.id, priority)}
          onMoveToDate={(date) => void moveToDate(menuTask.id, date)}
          onPromote={() => void promote(menuTask.id, "weekly")}
          onMoveToNextWeek={() => {
            if (!week) return;
            const next = nextWeek(week);
            void moveToWeek(menuTask.id, next.start, next.end);
          }}
          onSetBlocker={(blocker) => void setBlocker(menuTask.id, blocker)}
          onAddComment={(comment) => void addComment(menuTask.id, comment)}
          onArchive={() => void archive(menuTask.id)}
          onDelete={() => void remove(menuTask.id)}
        />
      )}
    </>
  );
}
