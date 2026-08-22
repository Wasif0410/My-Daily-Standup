import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  childrenOfTask,
  createTask,
  deleteTask,
  getTask,
  isCommandError,
  listTasksByHorizon,
  listTasksForDate,
  listTasksForPeriod,
  toCommandError,
  updateTask,
} from "@/lib/ipc";
import type { NewTask } from "@/types/task";

const mockInvoke = vi.mocked(invoke);

const newTask: NewTask = {
  title: "call the clinic",
  horizon: "daily",
  status: "planned",
  sourceType: "manual",
};

beforeEach(() => {
  mockInvoke.mockReset();
  mockInvoke.mockResolvedValue(null);
});

describe("command arguments", () => {
  // Argument names must match the Rust parameter names exactly. Tauri matches
  // by name, so a mismatch fails at runtime with no compile-time warning.
  it.each([
    ["task_create", () => createTask(newTask), { input: newTask }],
    ["task_get", () => getTask("abc"), { id: "abc" }],
    [
      "task_update",
      () => updateTask("abc", { title: "renamed" }),
      { id: "abc", patch: { title: "renamed" } },
    ],
    ["task_delete", () => deleteTask("abc"), { id: "abc" }],
    ["task_list_by_horizon", () => listTasksByHorizon("weekly"), { horizon: "weekly" }],
    [
      "task_list_for_date",
      () => listTasksForDate("2026-08-21"),
      { date: "2026-08-21" },
    ],
    [
      "task_list_for_period",
      () => listTasksForPeriod("2026-08-17", "2026-08-23"),
      { start: "2026-08-17", end: "2026-08-23" },
    ],
    ["task_children_of", () => childrenOfTask("parent"), { parentId: "parent" }],
  ])("%s passes the expected arguments", async (command, run, expected) => {
    await run();

    expect(mockInvoke).toHaveBeenCalledWith(command, expected);
  });
});

describe("patch semantics", () => {
  it("omits an unset field so Rust leaves it unchanged", async () => {
    await updateTask("abc", { title: "renamed" });

    const [, args] = mockInvoke.mock.calls[0] as [string, { patch: object }];
    expect(args.patch).not.toHaveProperty("blocker");
  });

  it("sends an explicit null to clear a field", async () => {
    // The distinction between omitted and null is what makes "resolve this
    // blocker" expressible at all.
    await updateTask("abc", { blocker: null });

    const [, args] = mockInvoke.mock.calls[0] as [string, { patch: { blocker: null } }];
    expect(args.patch.blocker).toBeNull();
    expect("blocker" in args.patch).toBe(true);
  });
});

describe("error handling", () => {
  it("passes a tagged CommandError through unchanged", async () => {
    mockInvoke.mockRejectedValueOnce({ kind: "not-found", message: "no task" });

    await expect(getTask("missing")).rejects.toMatchObject({
      kind: "not-found",
      message: "no task",
    });
  });

  it("wraps a transport failure as an internal error", async () => {
    // A failure inside IPC itself is not a CommandError, but callers should
    // only ever have to handle one shape.
    mockInvoke.mockRejectedValueOnce(new Error("bridge unavailable"));

    await expect(getTask("abc")).rejects.toMatchObject({
      kind: "internal",
      message: "bridge unavailable",
    });
  });

  it("wraps a non-Error rejection too", async () => {
    mockInvoke.mockRejectedValueOnce("something odd");

    await expect(getTask("abc")).rejects.toMatchObject({
      kind: "internal",
      message: "something odd",
    });
  });
});

describe("isCommandError", () => {
  it.each([
    [{ kind: "not-found", message: "x" }, true],
    [new Error("plain"), false],
    [null, false],
    ["string", false],
    [{ kind: "not-found" }, false],
  ])("classifies %o as %s", (value, expected) => {
    expect(isCommandError(value)).toBe(expected);
  });
});

describe("toCommandError", () => {
  it("produces a real Error so rejections carry a stack trace", () => {
    const error = toCommandError("plain string");

    expect(error).toBeInstanceOf(Error);
    expect(error.stack).toBeDefined();
    expect(error.kind).toBe("internal");
  });

  it("preserves the kind of an already-tagged error", () => {
    expect(toCommandError({ kind: "storage", message: "constraint" })).toMatchObject({
      kind: "storage",
      message: "constraint",
    });
  });
});
