import { useEffect, useMemo } from "react";
import { AreaGroup } from "@/features/boards/components/AreaGroup";
import { TaskRow } from "@/features/boards/components/TaskRow";
import { groupByArea } from "@/features/boards/grouping";
import { sortTasks, useTaskStore } from "@/stores/taskStore";

/**
 * The lowest priority the board shows.
 *
 * Priority runs 0-10, so this is the midpoint: "more important than average".
 * A prop rather than a constant at the call site, so PR 16 can drive it from
 * settings without this component changing.
 */
export const DEFAULT_PRIORITY_THRESHOLD = 5;

/**
 * Important work that outlives a single day, grouped by area (spec §6.2).
 *
 * The only component in this feature that touches the store. Everything below
 * it takes props, which is what lets PRs 13-15 reuse the same row and group
 * against entirely different queries.
 */
export function PriorityBoard({
  threshold = DEFAULT_PRIORITY_THRESHOLD,
}: {
  threshold?: number;
}) {
  const tasks = useTaskStore((state) => state.tasks);
  const error = useTaskStore((state) => state.error);
  const load = useTaskStore((state) => state.load);
  const complete = useTaskStore((state) => state.complete);
  const uncomplete = useTaskStore((state) => state.uncomplete);
  const editTitle = useTaskStore((state) => state.editTitle);
  const recordTimeSpent = useTaskStore((state) => state.recordTimeSpent);
  const remove = useTaskStore((state) => state.remove);

  useEffect(() => {
    void load({ kind: "priority", threshold });
  }, [load, threshold]);

  // Memoised because grouping builds fresh arrays: called inline in a selector
  // it would hand React a new value every render and loop.
  const groups = useMemo(() => groupByArea(sortTasks(Object.values(tasks))), [tasks]);

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
          <AreaGroup
            key={group.area}
            area={group.area}
            tasks={group.tasks}
            renderTask={(task) => (
              <TaskRow
                key={task.id}
                task={task}
                onComplete={(completed) => {
                  void (completed ? complete(task.id) : uncomplete(task.id));
                }}
                onEdit={(title) => void editTitle(task.id, title)}
                onSetTimeSpent={(minutes) => void recordTimeSpent(task.id, minutes)}
                // No onMove: PR 13 owns the move UI, and a button that opened
                // nothing would be worse than no button.
                onDelete={() => void remove(task.id)}
              />
            )}
          />
        ))
      )}
    </>
  );
}
