import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useSettingsStore } from "@/stores/settingsStore";
import type { Settings } from "@/types/settings";

const mockInvoke = vi.mocked(invoke);

function settings(overrides: Partial<Settings> = {}): Settings {
  return {
    priorityThreshold: 5,
    weekStartsOn: "monday",
    launchAtLogin: false,
    ...overrides,
  };
}

/** Resets the store between tests — zustand state is module-level. */
function reset() {
  useSettingsStore.setState({ settings: null, loading: false, error: null });
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
  it("starts with no settings at all", () => {
    // Not seeded with defaults on purpose. A board that renders a guessed
    // threshold and then jumps to the user's real one has shown a wrong board.
    expect(useSettingsStore.getState().settings).toBeNull();
  });

  it("populates settings from the backend", async () => {
    mockInvoke.mockResolvedValue(settings({ priorityThreshold: 7 }));

    await useSettingsStore.getState().load();

    expect(mockInvoke).toHaveBeenCalledWith("settings_get", {});
    expect(useSettingsStore.getState().settings).toMatchObject({
      priorityThreshold: 7,
    });
    expect(useSettingsStore.getState().loading).toBe(false);
  });

  it("stays null when the first load fails", async () => {
    // The caller is waiting on a real answer, and a fabricated one here would
    // be indistinguishable from a loaded one.
    mockInvoke.mockRejectedValue({ kind: "storage", message: "disk full" });

    await useSettingsStore.getState().load();

    expect(useSettingsStore.getState().settings).toBeNull();
    expect(useSettingsStore.getState().error).toMatchObject({ kind: "storage" });
    expect(useSettingsStore.getState().loading).toBe(false);
  });

  it("marks itself loading while the request is in flight", () => {
    const pending = deferred<Settings>();
    mockInvoke.mockReturnValue(pending.promise);

    // Deliberately not awaited: the point is the state *during* the request.
    void useSettingsStore.getState().load();

    expect(useSettingsStore.getState().loading).toBe(true);

    pending.resolve(settings());
  });

  it("replaces the previous value wholesale on a second load", async () => {
    useSettingsStore.setState({
      settings: settings({ priorityThreshold: 3, launchAtLogin: true }),
    });
    mockInvoke.mockResolvedValue(
      settings({ priorityThreshold: 9, weekStartsOn: "sunday" }),
    );

    await useSettingsStore.getState().load();

    expect(useSettingsStore.getState().settings).toEqual({
      priorityThreshold: 9,
      weekStartsOn: "sunday",
      launchAtLogin: false,
    });
  });

  it("clears a previous error once a load succeeds", async () => {
    useSettingsStore.setState({ error: { kind: "storage", message: "disk full" } });
    mockInvoke.mockResolvedValue(settings());

    await useSettingsStore.getState().load();

    expect(useSettingsStore.getState().error).toBeNull();
  });
});

describe("updating", () => {
  it("sends the patch under the `patch` argument", async () => {
    useSettingsStore.setState({ settings: settings() });
    mockInvoke.mockResolvedValue(settings({ priorityThreshold: 7 }));

    await useSettingsStore.getState().update({ priorityThreshold: 7 });

    expect(mockInvoke).toHaveBeenCalledWith("settings_update", {
      patch: { priorityThreshold: 7 },
    });
  });

  it("keeps the returned value, not the requested one", async () => {
    // Rust validates and clamps, and applying `launchAtLogin` can fail at the
    // OS level. Only what comes back is true.
    useSettingsStore.setState({ settings: settings() });
    mockInvoke.mockResolvedValue(settings({ priorityThreshold: 10 }));

    await useSettingsStore.getState().update({ priorityThreshold: 99 });

    expect(useSettingsStore.getState().settings?.priorityThreshold).toBe(10);
  });

  it("shows nothing until the backend answers", () => {
    // Deliberately not optimistic, unlike `taskStore`.
    useSettingsStore.setState({ settings: settings({ priorityThreshold: 5 }) });
    const pending = deferred<Settings>();
    mockInvoke.mockReturnValue(pending.promise);

    void useSettingsStore.getState().update({ priorityThreshold: 7 });

    expect(useSettingsStore.getState().settings?.priorityThreshold).toBe(5);

    pending.resolve(settings({ priorityThreshold: 7 }));
  });

  it("leaves settings exactly as they were when rejected", async () => {
    const before = settings({ priorityThreshold: 5, launchAtLogin: false });
    useSettingsStore.setState({ settings: before });
    mockInvoke.mockRejectedValue({
      kind: "invalid-input",
      message: "threshold out of range",
    });

    await useSettingsStore.getState().update({ priorityThreshold: 99 });

    expect(useSettingsStore.getState().settings).toEqual(before);
    expect(useSettingsStore.getState().error).toMatchObject({
      kind: "invalid-input",
    });
  });

  it("leaves settings null when an update is rejected before any load", async () => {
    mockInvoke.mockRejectedValue({ kind: "internal", message: "autostart refused" });

    await useSettingsStore.getState().update({ launchAtLogin: true });

    expect(useSettingsStore.getState().settings).toBeNull();
    expect(useSettingsStore.getState().error).toMatchObject({ kind: "internal" });
  });

  it("clears a previous error once an update succeeds", async () => {
    useSettingsStore.setState({
      settings: settings(),
      error: { kind: "invalid-input", message: "out of range" },
    });
    mockInvoke.mockResolvedValue(settings({ weekStartsOn: "saturday" }));

    await useSettingsStore.getState().update({ weekStartsOn: "saturday" });

    expect(useSettingsStore.getState().error).toBeNull();
    expect(useSettingsStore.getState().settings?.weekStartsOn).toBe("saturday");
  });

  it("sends only the fields it was given, so the rest are left alone", async () => {
    useSettingsStore.setState({ settings: settings() });
    mockInvoke.mockResolvedValue(settings({ launchAtLogin: true }));

    await useSettingsStore.getState().update({ launchAtLogin: true });

    expect(mockInvoke).toHaveBeenCalledWith("settings_update", {
      patch: { launchAtLogin: true },
    });
  });
});
