import { useEffect, useMemo, useState } from "react";
import { AddTaskHere } from "@/features/boards/components/AddTaskHere";
import { TaskContextMenu } from "@/features/boards/components/TaskContextMenu";
import { TaskGroup } from "@/features/boards/components/TaskGroup";
import { TaskRow } from "@/features/boards/components/TaskRow";
import {
  groupByArea,
  UNSORTED_AREA,
  headingKey,
  withDeclaredGroups,
} from "@/features/boards/grouping";
import { useCollapsedGroups } from "@/features/boards/useCollapsedGroups";
import { useSectionStore } from "@/stores/sectionStore";
import { sortTasks, useTaskStore } from "@/stores/taskStore";
import type { Task } from "@/types/task";

/**
 * The lowest priority the board shows.
 *
 * Priority runs 0-10, so this is the midpoint: "more important than average".
 * A prop rather than a constant at the call site, so PR 16 can drive it from
 * settings without this component changing.
 */
export const DEFAULT_PRIORITY_THRESHOLD = 5;

/** Which row's menu is open, and where it was summoned. */
interface OpenMenu {
  taskId: string;
  x: number;
  y: number;
}

/**
 * Important work that outlives a single day, grouped by area (spec §6.2).
 *
 * Its headings come from two places at once. Most are derived from the areas
 * the tasks themselves carry; the rest were declared by the user through the
 * board header, and those have to show even while empty, because a heading you
 * just made and cannot see reads as a button that did nothing.
 * {@link withDeclaredGroups} folds the two into one list.
 *
 * The only component in this feature that touches a store. Everything below it
 * takes props, which is what lets PRs 13-15 reuse the same row and group
 * against entirely different queries.
 */
export function PriorityBoard({
  threshold = DEFAULT_PRIORITY_THRESHOLD,
}: {
  threshold?: number;
}) {
  const [menu, setMenu] = useState<OpenMenu | null>(null);
  const [editing, setEditing] = useState<string | null>(null);

  const tasks = useTaskStore((state) => state.tasks);
  const taskError = useTaskStore((state) => state.error);
  const load = useTaskStore((state) => state.load);
  const add = useTaskStore((state) => state.add);
  const complete = useTaskStore((state) => state.complete);
  const uncomplete = useTaskStore((state) => state.uncomplete);
  const editTitle = useTaskStore((state) => state.editTitle);
  const recordTimeSpent = useTaskStore((state) => state.recordTimeSpent);
  const setPriority = useTaskStore((state) => state.setPriority);
  const moveToDate = useTaskStore((state) => state.moveToDate);
  const promote = useTaskStore((state) => state.promote);
  const setBlocker = useTaskStore((state) => state.setBlocker);
  const addComment = useTaskStore((state) => state.addComment);
  const archive = useTaskStore((state) => state.archive);
  const remove = useTaskStore((state) => state.remove);

  const sections = useSectionStore((state) => state.sections);
  const sectionError = useSectionStore((state) => state.error);
  const loadSections = useSectionStore((state) => state.load);
  const { isCollapsed, toggle: toggleGroup } = useCollapsedGroups("priority");
  const renameSection = useSectionStore((state) => state.renameSection);
  const removeSection = useSectionStore((state) => state.removeSection);

  useEffect(() => {
    void load({ kind: "priority", threshold });
  }, [load, threshold]);

  useEffect(() => {
    void loadSections("priority");
  }, [loadSections]);

  // Memoised because grouping builds fresh arrays: called inline in a selector
  // it would hand React a new value every render and loop.
  const groups = useMemo(
    () =>
      withDeclaredGroups(
        groupByArea(sortTasks(Object.values(tasks))),
        sections.map((section) => section.title),
      ),
    [tasks, sections],
  );

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
  const error = taskError ?? sectionError;

  /**
   * Creates a task that will actually appear on the board that made it.
   *
   * The priority is the threshold, not a default and not nothing. The query
   * behind this board is `priority >= threshold`, and `priority IS NULL` never
   * satisfies it — a task created without one would be filed correctly and
   * then vanish from the only screen the user was looking at.
   */
  function addToGroup(label: string, title: string) {
    void add({
      title,
      // "Unsorted" is this board's word for no area at all. Filing a task
      // under a literal area of that name would turn the fallback into a real
      // group that then never empties.
      area: label === UNSORTED_AREA ? null : label,
      horizon: "weekly",
      status: "planned",
      sourceType: "manual",
      priority: threshold,
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

      {groups.length === 0 ? (
        <p className="board-empty">Nothing at priority {threshold} or above.</p>
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
                // No onMove: PR 13 owns the move UI, and a button that opened
                // nothing would be worse than no button.
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
          // The reason this board has a menu at all. Priority is what decides
          // whether a task belongs here, so the board defined by it has to be
          // able to change it — including down, to get something off the list.
          onSetPriority={(priority) => void setPriority(menuTask.id, priority)}
          onMoveToDate={(date) => void moveToDate(menuTask.id, date)}
          onPromote={() => void promote(menuTask.id, "weekly")}
          // No onMoveToNextWeek: this board is a standing list, not a period,
          // and it does not know which week is current.
          onSetBlocker={(blocker) => void setBlocker(menuTask.id, blocker)}
          onAddComment={(comment) => void addComment(menuTask.id, comment)}
          onArchive={() => void archive(menuTask.id)}
          onDelete={() => void remove(menuTask.id)}
        />
      )}
    </>
  );
}
