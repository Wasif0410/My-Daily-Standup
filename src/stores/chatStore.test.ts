import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useChatStore } from "@/stores/chatStore";
import type { ChatReply, ChatStatus } from "@/types/chat";

const mockInvoke = vi.mocked(invoke);

function status(overrides: Partial<ChatStatus> = {}): ChatStatus {
  return { running: false, model: null, port: null, ...overrides };
}

function running(): ChatStatus {
  return status({ running: true, model: "qwen2.5-3b-instruct", port: 8080 });
}

function reply(overrides: Partial<ChatReply> = {}): ChatReply {
  return {
    content: "Morning.",
    promptTokens: 12,
    completionTokens: 34,
    elapsedMs: 980,
    ...overrides,
  };
}

/** Resets the store between tests — zustand state is module-level. */
function reset() {
  useChatStore.setState({
    status: null,
    starting: false,
    sending: false,
    lastReply: null,
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

describe("refreshing", () => {
  it("starts knowing nothing about the model", () => {
    // `null` is a third answer — "not asked yet" — distinct from "not running".
    // A panel that rendered "stopped" before asking would be guessing.
    expect(useChatStore.getState().status).toBeNull();
  });

  it("records the status the backend reports", async () => {
    mockInvoke.mockResolvedValue(running());

    await useChatStore.getState().refresh();

    expect(mockInvoke).toHaveBeenCalledWith("chat_status", {});
    expect(useChatStore.getState().status).toEqual(running());
  });

  it("leaves the status untouched when the query fails", async () => {
    mockInvoke.mockRejectedValue({ kind: "internal", message: "no bridge" });

    await useChatStore.getState().refresh();

    expect(useChatStore.getState().status).toBeNull();
    expect(useChatStore.getState().error).toMatchObject({ kind: "internal" });
  });
});

describe("starting", () => {
  it("marks itself starting for as long as the load takes", () => {
    // The measured load is a few seconds for 2.4 GB plus a health check, so
    // this flag is the only thing standing between the user and a frozen
    // window.
    const pending = deferred<ChatStatus>();
    mockInvoke.mockReturnValue(pending.promise);

    void useChatStore.getState().start();

    expect(useChatStore.getState().starting).toBe(true);

    pending.resolve(running());
  });

  it("keeps `sending` false while the model loads", () => {
    // Two different waits of very different lengths. The panel says different
    // things about each, so one flag could not serve both.
    const pending = deferred<ChatStatus>();
    mockInvoke.mockReturnValue(pending.promise);

    void useChatStore.getState().start();

    expect(useChatStore.getState().sending).toBe(false);

    pending.resolve(running());
  });

  it("takes the running status from the reply, not from having asked", async () => {
    mockInvoke.mockResolvedValue(running());

    await useChatStore.getState().start();

    expect(mockInvoke).toHaveBeenCalledWith("chat_start", {});
    expect(useChatStore.getState().status).toEqual(running());
    expect(useChatStore.getState().starting).toBe(false);
  });

  it("shows nothing as running until the backend answers", () => {
    // Deliberately not optimistic. Whether a model actually loaded is only
    // knowable from the reply — the process can die on the health check.
    const pending = deferred<ChatStatus>();
    mockInvoke.mockReturnValue(pending.promise);

    void useChatStore.getState().start();

    expect(useChatStore.getState().status).toBeNull();

    pending.resolve(running());
  });

  it("leaves the previous status alone when the start is rejected", async () => {
    const before = status();
    useChatStore.setState({ status: before });
    mockInvoke.mockRejectedValue({ kind: "internal", message: "model missing" });

    await useChatStore.getState().start();

    expect(useChatStore.getState().status).toEqual(before);
    expect(useChatStore.getState().error).toMatchObject({ kind: "internal" });
    expect(useChatStore.getState().starting).toBe(false);
  });

  it("clears a previous error once a start succeeds", async () => {
    useChatStore.setState({ error: { kind: "internal", message: "model missing" } });
    mockInvoke.mockResolvedValue(running());

    await useChatStore.getState().start();

    expect(useChatStore.getState().error).toBeNull();
  });
});

describe("sending", () => {
  it("refuses to send while no model is running", async () => {
    useChatStore.setState({ status: status() });

    await useChatStore.getState().send("hello");

    // Not even attempted. The command would fail anyway, and a backend error
    // about a missing port explains nothing the user can act on.
    expect(mockInvoke).not.toHaveBeenCalled();
    expect(useChatStore.getState().error).toMatchObject({ kind: "invalid-input" });
    expect(useChatStore.getState().error?.message).toMatch(/start/i);
  });

  it("refuses to send before the status has ever been queried", async () => {
    await useChatStore.getState().send("hello");

    expect(mockInvoke).not.toHaveBeenCalled();
    expect(useChatStore.getState().error).toMatchObject({ kind: "invalid-input" });
  });

  it("sends the message and keeps the reply", async () => {
    useChatStore.setState({ status: running() });
    mockInvoke.mockResolvedValue(reply({ content: "Two things today." }));

    await useChatStore.getState().send("what is up");

    expect(mockInvoke).toHaveBeenCalledWith("chat_send", { message: "what is up" });
    expect(useChatStore.getState().lastReply).toMatchObject({
      content: "Two things today.",
    });
    expect(useChatStore.getState().sending).toBe(false);
  });

  it("marks itself sending while the model thinks", () => {
    useChatStore.setState({ status: running() });
    const pending = deferred<ChatReply>();
    mockInvoke.mockReturnValue(pending.promise);

    void useChatStore.getState().send("what is up");

    expect(useChatStore.getState().sending).toBe(true);
    // Separate flags: this wait is about a second, not several, and the panel
    // must not claim a model is loading when one already has.
    expect(useChatStore.getState().starting).toBe(false);

    pending.resolve(reply());
  });

  it("keeps the previous reply when a send fails", async () => {
    // Blanking the transcript on failure would destroy the answer the user was
    // reading, and the failure is already reported separately.
    const previous = reply({ content: "Morning." });
    useChatStore.setState({ status: running(), lastReply: previous });
    mockInvoke.mockRejectedValue({ kind: "internal", message: "connection reset" });

    await useChatStore.getState().send("again");

    expect(useChatStore.getState().lastReply).toEqual(previous);
    expect(useChatStore.getState().error).toMatchObject({ kind: "internal" });
    expect(useChatStore.getState().sending).toBe(false);
  });

  it("clears a previous error once a send succeeds", async () => {
    useChatStore.setState({
      status: running(),
      error: { kind: "internal", message: "connection reset" },
    });
    mockInvoke.mockResolvedValue(reply());

    await useChatStore.getState().send("hello");

    expect(useChatStore.getState().error).toBeNull();
  });
});

describe("stopping", () => {
  it("takes the stopped status from the reply", async () => {
    useChatStore.setState({ status: running() });
    mockInvoke.mockResolvedValue(status());

    await useChatStore.getState().stop();

    expect(mockInvoke).toHaveBeenCalledWith("chat_stop", {});
    expect(useChatStore.getState().status).toEqual(status());
  });

  it("clears the last reply", async () => {
    // There is no conversation history yet, so a reply outliving the model it
    // came from would be an answer with nothing behind it.
    useChatStore.setState({ status: running(), lastReply: reply() });
    mockInvoke.mockResolvedValue(status());

    await useChatStore.getState().stop();

    expect(useChatStore.getState().lastReply).toBeNull();
  });

  it("leaves the status and the reply alone when the stop is rejected", async () => {
    const before = running();
    const previous = reply();
    useChatStore.setState({ status: before, lastReply: previous });
    mockInvoke.mockRejectedValue({ kind: "internal", message: "refused to die" });

    await useChatStore.getState().stop();

    expect(useChatStore.getState().status).toEqual(before);
    expect(useChatStore.getState().lastReply).toEqual(previous);
    expect(useChatStore.getState().error).toMatchObject({ kind: "internal" });
  });
});
