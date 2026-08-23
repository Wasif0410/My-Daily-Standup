import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useTaskStore } from "@/stores/taskStore";
import type { Task } from "@/types/task";

const mockInvoke = vi.mocked(invoke);

function task(overrides: Partial<Task> = {}): Task {
  return {
    id: "task-1",
    title: "call the clinic",
    description: null,
    horizon: "daily",
    status: "planned",
    parentTaskId: null,
    sourceType: "manual",
    sourceFile: null,
    sourceLine: null,
    area: null,
    project: null,
    priority: null,
    scheduledDate: "2026-08-21",
    periodStart: null,
    periodEnd: null,
    dueDate: null,
    completedAt: null,
    progressCurrent: null,
    progressTarget: null,
    progressUnit: null,
    blocker: null,
    notes: null,
    timeSpentMinutes: null,
    rolloverCount: 0,
    createdAt: "2026-08-21T00:00:00.000000Z",
    updatedAt: "2026-08-21T00:00:00.000000Z",
    ...overrides,
  };
}

/** Resets the store between tests — zustand state is module-level. */
function reset() {
  useTaskStore.setState({
    tasks: {},
    filter: null,
    loading: false,
    error: null,
  });
}

/** A promise plus the handles to settle it, for observing intermediate state. */
function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

beforeEach(() => {
  mockInvoke.mockReset();
  reset();
});

describe("loading", () => {
  it("populates tasks from the backend", async () => {
    mockInvoke.mockResolvedValue([task()]);

    await useTaskStore.getState().load({ kind: "horizon", horizon: "daily" });

    expect(Object.values(useTaskStore.getState().tasks)).toHaveLength(1);
    expect(useTaskStore.getState().loading).toBe(false);
  });

  it("records an error instead of throwing", async () => {
    mockInvoke.mockRejectedValue({ kind: "storage", message: "disk full" });

    await useTaskStore.getState().load({ kind: "horizon", horizon: "daily" });

    expect(useTaskStore.getState().error).toMatchObject({ kind: "storage" });
    expect(useTaskStore.getState().loading).toBe(false);
  });

  it("routes each filter to the matching command", async () => {
    mockInvoke.mockResolvedValue([]);
    const store = useTaskStore.getState();

    await store.load({ kind: "horizon", horizon: "weekly" });
    expect(mockInvoke).toHaveBeenLastCalledWith("task_list_by_horizon", {
      horizon: "weekly",
    });

    await store.load({ kind: "date", date: "2026-08-21" });
    expect(mockInvoke).toHaveBeenLastCalledWith("task_list_for_date", {
      date: "2026-08-21",
    });

    await store.load({ kind: "period", start: "2026-08-17", end: "2026-08-23" });
    expect(mockInvoke).toHaveBeenLastCalledWith("task_list_for_period", {
      start: "2026-08-17",
      end: "2026-08-23",
    });
  });
});

describe("optimistic updates", () => {
  it("applies the change locally before the backend responds", () => {
    useTaskStore.setState({ tasks: { "task-1": task() } });
    const pending = deferred<Task>();
    mockInvoke.mockReturnValue(pending.promise);

    // Deliberately not awaited: the point is the state *during* the request.
    void useTaskStore.getState().complete("task-1");

    expect(useTaskStore.getState().tasks["task-1"]?.status).toBe("completed");

    pending.resolve(task({ status: "completed" }));
  });

  it("replaces the optimistic value with the server's response", async () => {
    useTaskStore.setState({ tasks: { "task-1": task() } });
    // The server sets completedAt, which the optimistic guess cannot know.
    mockInvoke.mockResolvedValue(
      task({ status: "completed", completedAt: "2026-08-21T10:00:00.000000Z" }),
    );

    await useTaskStore.getState().complete("task-1");

    expect(useTaskStore.getState().tasks["task-1"]?.completedAt).toBe(
      "2026-08-21T10:00:00.000000Z",
    );
  });

  it("rolls back to the previous value when the backend rejects", async () => {
    useTaskStore.setState({ tasks: { "task-1": task({ title: "original" }) } });
    mockInvoke.mockRejectedValue({ kind: "storage", message: "constraint" });

    await useTaskStore.getState().editTitle("task-1", "renamed");

    expect(useTaskStore.getState().tasks["task-1"]?.title).toBe("original");
    expect(useTaskStore.getState().error).toMatchObject({ kind: "storage" });
  });

  it("restores a deleted task when the backend rejects", async () => {
    useTaskStore.setState({ tasks: { "task-1": task() } });
    mockInvoke.mockRejectedValue({ kind: "storage", message: "locked" });

    await useTaskStore.getState().remove("task-1");

    expect(useTaskStore.getState().tasks["task-1"]).toBeDefined();
  });

  it("removes the task on a successful delete", async () => {
    useTaskStore.setState({ tasks: { "task-1": task() } });
    mockInvoke.mockResolvedValue(null);

    await useTaskStore.getState().remove("task-1");

    expect(useTaskStore.getState().tasks["task-1"]).toBeUndefined();
  });

  it("hides a deleted row immediately rather than waiting on the backend", () => {
    useTaskStore.setState({ tasks: { "task-1": task() } });
    const pending = deferred<null>();
    mockInvoke.mockReturnValue(pending.promise);

    void useTaskStore.getState().remove("task-1");

    expect(useTaskStore.getState().tasks["task-1"]).toBeUndefined();

    pending.resolve(null);
  });

  it("ignores an action against an unknown id", async () => {
    await useTaskStore.getState().complete("no-such-task");

    expect(mockInvoke).not.toHaveBeenCalled();
  });
});

describe("actions", () => {
  beforeEach(() => {
    useTaskStore.setState({ tasks: { "task-1": task() } });
    mockInvoke.mockResolvedValue(task());
  });

  it("uncomplete clears completedAt with an explicit null", async () => {
    useTaskStore.setState({
      tasks: { "task-1": task({ status: "completed", completedAt: "x" }) },
    });

    await useTaskStore.getState().uncomplete("task-1");

    expect(mockInvoke).toHaveBeenCalledWith("task_update", {
      id: "task-1",
      patch: { status: "planned", completedAt: null },
    });
  });

  it("moveToDate goes through the reschedule command, not a plain update", async () => {
    // A plain update would bypass rollover counting and silently break the
    // reflection prompt in spec §10.3.
    await useTaskStore.getState().moveToDate("task-1", "2026-08-25");

    expect(mockInvoke).toHaveBeenCalledWith("task_reschedule", {
      id: "task-1",
      to: "2026-08-25",
    });
  });

  it("recordTimeSpent sends the duration", async () => {
    await useTaskStore.getState().recordTimeSpent("task-1", 45);

    expect(mockInvoke).toHaveBeenCalledWith("task_set_time_spent", {
      id: "task-1",
      minutes: 45,
    });
  });

  it("recordTimeSpent sends null to clear a duration", async () => {
    // Null means unrecorded, which is distinct from zero: a task nobody timed
    // must contribute nothing to a total rather than counting as instant.
    await useTaskStore.getState().recordTimeSpent("task-1", null);

    expect(mockInvoke).toHaveBeenCalledWith("task_set_time_spent", {
      id: "task-1",
      minutes: null,
    });
  });

  it("promote changes the horizon", async () => {
    await useTaskStore.getState().promote("task-1", "weekly");

    expect(mockInvoke).toHaveBeenCalledWith("task_update", {
      id: "task-1",
      patch: { horizon: "weekly" },
    });
  });

  it("add creates a task and stores the server's version", async () => {
    mockInvoke.mockResolvedValue(task({ id: "task-2", title: "new one" }));

    await useTaskStore.getState().add({
      title: "new one",
      horizon: "daily",
      status: "planned",
      sourceType: "manual",
    });

    expect(useTaskStore.getState().tasks["task-2"]?.title).toBe("new one");
  });
});

describe("selectors", () => {
  it("returns tasks sorted by priority then creation order", () => {
    useTaskStore.setState({
      tasks: {
        a: task({ id: "a", priority: 3, createdAt: "2026-08-01T00:00:00Z" }),
        b: task({ id: "b", priority: 9, createdAt: "2026-08-02T00:00:00Z" }),
        c: task({ id: "c", priority: null, createdAt: "2026-08-03T00:00:00Z" }),
      },
    });

    const ordered = useTaskStore.getState().orderedTasks();

    // Highest priority first; unprioritised work sinks rather than
    // interleaving unpredictably.
    expect(ordered.map((t) => t.id)).toEqual(["b", "a", "c"]);
  });
});

describe("the priority filter", () => {
  it("loads through the priority command", async () => {
    // Not three list_by_horizon calls stitched together: load() replaces the
    // whole task map, so a merged fetch would keep only the last horizon.
    mockInvoke.mockResolvedValue([task({ id: "a", priority: 8 })]);

    await useTaskStore.getState().load({ kind: "priority", threshold: 5 });

    expect(mockInvoke).toHaveBeenCalledWith("task_list_priority", { threshold: 5 });
    expect(useTaskStore.getState().tasks["a"]?.priority).toBe(8);
  });

  it("passes the threshold through rather than hard-coding one", async () => {
    mockInvoke.mockResolvedValue([]);

    await useTaskStore.getState().load({ kind: "priority", threshold: 8 });

    expect(mockInvoke).toHaveBeenCalledWith("task_list_priority", { threshold: 8 });
  });
});

describe("the remaining §6.6 interactions", () => {
  /** Seeds one task into the store, as a board would after loading. */
  function seed(overrides: Partial<Task> = {}) {
    const seeded = task({ id: "task-1", ...overrides });
    useTaskStore.setState({ tasks: { [seeded.id]: seeded } });
    return seeded;
  }

  describe("blockers", () => {
    it("dispatches through the blocker command, not a plain update", async () => {
      // A plain update would set the text and leave the status disagreeing
      // with it. Only the blocker command keeps the two in step.
      seed();
      mockInvoke.mockResolvedValue(task({ blocker: "waiting", status: "blocked" }));

      await useTaskStore.getState().setBlocker("task-1", "waiting");

      expect(mockInvoke).toHaveBeenCalledWith("task_set_blocker", {
        id: "task-1",
        blocker: "waiting",
      });
    });

    it("marks the task blocked optimistically", async () => {
      seed();
      const pending = deferred<Task>();
      mockInvoke.mockReturnValue(pending.promise);

      const settled = useTaskStore.getState().setBlocker("task-1", "waiting");

      expect(useTaskStore.getState().tasks["task-1"]?.status).toBe("blocked");
      pending.resolve(task({ blocker: "waiting", status: "blocked" }));
      await settled;
    });

    it("restores the planned status when a blocker is cleared", async () => {
      seed({ blocker: "waiting", status: "blocked" });
      const pending = deferred<Task>();
      mockInvoke.mockReturnValue(pending.promise);

      const settled = useTaskStore.getState().setBlocker("task-1", null);

      expect(useTaskStore.getState().tasks["task-1"]?.status).toBe("planned");
      expect(useTaskStore.getState().tasks["task-1"]?.blocker).toBeNull();
      pending.resolve(task());
      await settled;
    });

    it("leaves a completed task completed when its blocker is cleared", async () => {
      // Resolving a blocker must not un-finish work.
      seed({ status: "completed", blocker: "waiting" });
      const pending = deferred<Task>();
      mockInvoke.mockReturnValue(pending.promise);

      const settled = useTaskStore.getState().setBlocker("task-1", null);

      expect(useTaskStore.getState().tasks["task-1"]?.status).toBe("completed");
      pending.resolve(task({ status: "completed" }));
      await settled;
    });

    it("rolls back to the exact prior value when the write fails", async () => {
      seed({ blocker: null, status: "planned" });
      mockInvoke.mockRejectedValue({ kind: "storage", message: "disk full" });

      await useTaskStore.getState().setBlocker("task-1", "waiting");

      const restored = useTaskStore.getState().tasks["task-1"];
      expect(restored?.blocker).toBeNull();
      expect(restored?.status).toBe("planned");
      expect(useTaskStore.getState().error?.message).toBe("disk full");
    });
  });

  describe("comments", () => {
    it("dispatches through the comment command", async () => {
      seed();
      mockInvoke.mockResolvedValue(task({ notes: "2026-08-23: portal was down" }));

      await useTaskStore.getState().addComment("task-1", "portal was down");

      expect(mockInvoke).toHaveBeenCalledWith("task_add_comment", {
        id: "task-1",
        comment: "portal was down",
      });
    });

    it("takes the stored notes rather than guessing the format", async () => {
      // Rust owns the date stamp and the joining, so there is nothing sensible
      // to guess optimistically here.
      seed();
      mockInvoke.mockResolvedValue(task({ notes: "2026-08-23: portal was down" }));

      await useTaskStore.getState().addComment("task-1", "portal was down");

      expect(useTaskStore.getState().tasks["task-1"]?.notes).toBe(
        "2026-08-23: portal was down",
      );
    });
  });

  describe("priority", () => {
    it("patches the priority", async () => {
      seed();
      mockInvoke.mockResolvedValue(task({ priority: 8 }));

      await useTaskStore.getState().setPriority("task-1", 8);

      expect(mockInvoke).toHaveBeenCalledWith("task_update", {
        id: "task-1",
        patch: { priority: 8 },
      });
    });

    it("clears a priority with an explicit null", async () => {
      // Omitting the key would mean "leave alone", so there would be no way
      // back to unprioritised.
      seed({ priority: 8 });
      mockInvoke.mockResolvedValue(task({ priority: null }));

      await useTaskStore.getState().setPriority("task-1", null);

      expect(mockInvoke).toHaveBeenCalledWith("task_update", {
        id: "task-1",
        patch: { priority: null },
      });
    });
  });

  describe("moving to another week", () => {
    it("goes through the period command, never a plain update", async () => {
      // Only that path counts the move as a deferral (spec §10.3).
      seed();
      mockInvoke.mockResolvedValue(
        task({ periodStart: "2026-08-24", periodEnd: "2026-08-30" }),
      );

      await useTaskStore.getState().moveToWeek("task-1", "2026-08-24", "2026-08-30");

      expect(mockInvoke).toHaveBeenCalledWith("task_move_to_period", {
        id: "task-1",
        start: "2026-08-24",
        end: "2026-08-30",
      });
    });
  });

  describe("archiving", () => {
    it("cancels the task rather than deleting the row", async () => {
      seed();
      mockInvoke.mockResolvedValue(task({ status: "cancelled" }));

      await useTaskStore.getState().archive("task-1");

      expect(mockInvoke).toHaveBeenCalledWith("task_archive", { id: "task-1" });
      expect(useTaskStore.getState().tasks["task-1"]?.status).toBe("cancelled");
    });
  });
});
