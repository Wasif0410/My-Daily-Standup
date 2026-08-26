import { useEffect, useMemo, useState } from "react";
import { AddTaskHere } from "@/features/boards/components/AddTaskHere";
import { TaskContextMenu } from "@/features/boards/components/TaskContextMenu";
import { TaskGroup } from "@/features/boards/components/TaskGroup";
import { TaskRow } from "@/features/boards/components/TaskRow";
import {
  groupByProject,
  UNSORTED_PROJECT,
  headingKey,
  withDeclaredGroups,
} from "@/features/boards/grouping";
import { currentWeek, toCommandError } from "@/lib/ipc";
import { formatWeek } from "@/features/boards/weekLabel";
import { useCollapsedGroups } from "@/features/boards/useCollapsedGroups";
import { useSectionStore } from "@/stores/sectionStore";
import { sortTasks, useTaskStore } from "@/stores/taskStore";
import type { CommandError, Task, Week } from "@/types/task";

/** Which row's menu is open, and where it was summoned. */
interface OpenMenu {
  taskId: string;
  x: number;
  y: number;
}

/**
 * The seven days after a week ends — the "next week" a task moves into.
 *
 * Only the boundaries, not a whole `Week`: a label and day names would be
 * invented here rather than computed by Rust, and nothing needs them.
 */
function nextWeek(week: Week): { start: string; end: string } {
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
 *
 * Its headings come from two places at once: the projects the tasks carry, and
 * the groups the user declared through the board header. A declared group has
 * to show while still empty — that is the whole point of having declared it —
 * so {@link withDeclaredGroups} folds the two lists into one.
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

  const sections = useSectionStore((state) => state.sections);
  const sectionError = useSectionStore((state) => state.error);
  const loadSections = useSectionStore((state) => state.load);
  const { isCollapsed, toggle: toggleGroup } = useCollapsedGroups("weekly-tasks");
  const renameSection = useSectionStore((state) => state.renameSection);
  const removeSection = useSectionStore((state) => state.removeSection);

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

  useEffect(() => {
    void loadSections("weekly-tasks");
  }, [loadSections]);

  const groups = useMemo(() => {
    // Archived work is cancelled, not deleted: the row survives in the
    // database and the board simply stops showing it.
    const live = Object.values(tasks).filter((task) => task.status !== "cancelled");
    return withDeclaredGroups(
      groupByProject(sortTasks(live)),
      sections.map((section) => section.title),
    );
  }, [tasks, sections]);

  /**
   * The section row behind a heading, when the user declared it.
   *
   * A heading that came only from the tasks has no row, so it offers no rename
   * and no delete: "Unsorted" is not a name anyone chose, and deleting it would
   * mean unfiling every task that merely lacks one.
   */
  function declaredFor(label: string) {
    return sections.find((section) => headingKey(section.title) === headingKey(label));
  }

  const menuTask = menu ? tasks[menu.taskId] : undefined;
  const error = weekError ?? storeError ?? sectionError;

  /**
   * Creates a task filed under the heading it was typed into, in this week.
   *
   * The heading is the filing. The board-wide quick-add this replaced could
   * not say which project a task belonged to, so everything it made landed
   * under "No project" and had to be moved afterwards.
   */
  function addToGroup(label: string, title: string) {
    if (!week) return;

    void add({
      title,
      // "No project" is the board's word for no project at all, not a project
      // by that name.
      project: label === UNSORTED_PROJECT ? null : label,
      horizon: "weekly",
      status: "planned",
      sourceType: "manual",
      periodStart: week.start,
      periodEnd: week.end,
    });
  }

  function rowActions(task: Task) {
    return {
      onComplete: (completed: boolean) => {
        void (completed ? complete(task.id) : uncomplete(task.id));
      },
      onEdit: (title: string) => void editTitle(task.id, title),
      onSetTimeSpent: (minutes: number | null) =>
        void recordTimeSpent(task.id, minutes),
      // Priority is the field the board sorts by, so changing it moves the
      // row and often the group with it. That is exactly why it is on the
      // badge rather than only in the menu.
      onSetPriority: (priority: number | null) => void setPriority(task.id, priority),
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

      {week && <p className="board-week">{formatWeek(week)}</p>}

      {groups.length === 0 ? (
        <p className="board-empty">Nothing planned this week.</p>
      ) : (
        groups.map((group) => (
          <TaskGroup
            key={group.label}
            label={group.label}
            tasks={group.tasks}
            collapsed={isCollapsed(group.label)}
            onToggle={(shut) => toggleGroup(group.label, shut)}
            onRename={
              declaredFor(group.label)
                ? (title) => {
                    const declared = declaredFor(group.label);
                    if (declared) void renameSection(declared.id, title);
                  }
                : undefined
            }
            action={
              <>
                <AddTaskHere
                  label={group.label}
                  onAdd={(title) => addToGroup(group.label, title)}
                />
                {declaredFor(group.label) && (
                  <button
                    type="button"
                    className="board-section-action"
                    aria-label={`Delete ${group.label}`}
                    onClick={() => {
                      const declared = declaredFor(group.label);
                      if (declared) void removeSection(declared.id);
                    }}
                  >
                    ×
                  </button>
                )}
              </>
            }
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
