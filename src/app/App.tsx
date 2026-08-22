import { useCallback, useEffect, useState } from "react";
import { createTask, listTasksByHorizon, toCommandError, updateTask } from "@/lib/ipc";
import type { CommandError, Task } from "@/types/task";

/**
 * Development shell.
 *
 * Proves the whole pipeline end to end — React through typed IPC, into Rust,
 * into SQLite, and back. The real boards arrive in Wave 2 (PRs 8-14); this
 * exists so PR 6 has something demonstrable rather than only unit-tested.
 */
export function App() {
  const [tasks, setTasks] = useState<Task[]>([]);
  const [title, setTitle] = useState("");
  const [error, setError] = useState<CommandError | null>(null);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    try {
      setTasks(await listTasksByHorizon("daily"));
      setError(null);
    } catch (caught) {
      setError(toCommandError(caught));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    // The `ignore` flag is the canonical guard against a response arriving
    // after unmount, which would otherwise set state on a dead component.
    let ignore = false;

    async function load() {
      try {
        const loaded = await listTasksByHorizon("daily");
        if (!ignore) {
          setTasks(loaded);
          setError(null);
        }
      } catch (caught) {
        if (!ignore) setError(toCommandError(caught));
      } finally {
        if (!ignore) setLoading(false);
      }
    }

    void load();

    return () => {
      ignore = true;
    };
  }, []);

  async function addTask(event: React.FormEvent) {
    event.preventDefault();
    const trimmed = title.trim();
    if (!trimmed) return;

    try {
      await createTask({
        title: trimmed,
        horizon: "daily",
        status: "planned",
        sourceType: "manual",
      });
      setTitle("");
      await refresh();
    } catch (caught) {
      setError(toCommandError(caught));
    }
  }

  async function toggle(task: Task) {
    try {
      await updateTask(task.id, {
        status: task.status === "completed" ? "planned" : "completed",
        // Clearing on un-complete is exactly the case the doubled option in
        // TaskPatch exists for.
        completedAt: task.status === "completed" ? null : new Date().toISOString(),
      });
      await refresh();
    } catch (caught) {
      setError(toCommandError(caught));
    }
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
                  onChange={() => void toggle(task)}
                />
                <span>{task.title}</span>
              </label>
            </li>
          ))}
        </ul>
      )}
    </main>
  );
}
