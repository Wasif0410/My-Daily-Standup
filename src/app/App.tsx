import { useEffect, useMemo, useState } from "react";
import { sortTasks, useTaskStore } from "@/stores/taskStore";

/**
 * Development shell.
 *
 * Exercises the store end to end — React through zustand, typed IPC, Rust, and
 * SQLite. The real boards arrive in Wave 2 (PRs 9-14); this exists so the
 * plumbing is demonstrable rather than only unit-tested.
 */
export function App() {
  // Select the raw record and derive the ordering here. Calling a store method
  // inside the selector returns a new array each render and loops forever.
  const taskMap = useTaskStore((s) => s.tasks);
  const tasks = useMemo(() => sortTasks(Object.values(taskMap)), [taskMap]);
  const loading = useTaskStore((s) => s.loading);
  const error = useTaskStore((s) => s.error);
  const load = useTaskStore((s) => s.load);
  const add = useTaskStore((s) => s.add);
  const complete = useTaskStore((s) => s.complete);
  const uncomplete = useTaskStore((s) => s.uncomplete);
  const remove = useTaskStore((s) => s.remove);

  const [title, setTitle] = useState("");

  useEffect(() => {
    void load({ kind: "horizon", horizon: "daily" });
  }, [load]);

  async function addTask(event: React.FormEvent) {
    event.preventDefault();
    const trimmed = title.trim();
    if (!trimmed) return;

    setTitle("");
    await add({
      title: trimmed,
      horizon: "daily",
      status: "planned",
      sourceType: "manual",
    });
  }

  return (
    <main className="shell">
      <h1>My Daily Standup</h1>
      <p className="tagline">
        Long-term goals in, realistic daily actions out — entirely on your own machine.
      </p>

      <form className="add-task" onSubmit={(e) => void addTask(e)}>
        <input
          aria-label="New task"
          placeholder="What needs doing today?"
          value={title}
          onChange={(e) => setTitle(e.target.value)}
        />
        <button type="submit">Add</button>
      </form>

      {error && (
        <p className="error" role="alert" data-testid="error">
          {error.kind}: {error.message}
        </p>
      )}

      {loading ? (
        <p className="muted">Loading…</p>
      ) : tasks.length === 0 ? (
        <p className="muted">No tasks yet.</p>
      ) : (
        <ul className="task-list" data-testid="task-list">
          {tasks.map((task) => (
            <li key={task.id} data-completed={task.status === "completed"}>
              <label>
                <input
                  type="checkbox"
                  checked={task.status === "completed"}
                  onChange={() =>
                    void (task.status === "completed"
                      ? uncomplete(task.id)
                      : complete(task.id))
                  }
                />
                <span>{task.title}</span>
              </label>
              <button
                type="button"
                aria-label={`Delete ${task.title}`}
                className="delete"
                onClick={() => void remove(task.id)}
              >
                ×
              </button>
            </li>
          ))}
        </ul>
      )}
    </main>
  );
}
