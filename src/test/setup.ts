import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/react";
import { afterEach, vi } from "vitest";

// Tauri's IPC does not exist in jsdom. Tests that care about a specific
// command mock it explicitly; this default keeps unrelated renders from
// throwing on the bridge call.
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue(""),
}));

// Same reasoning for the event bridge, which boards use to stay in step across
// windows. Without this, any component that subscribes on mount reaches into
// Tauri's internals and throws — even in a test that has nothing to do with
// events. Tests that care mock `@/lib/taskEvents` explicitly instead.
vi.mock("@tauri-apps/api/event", () => ({
  emit: vi.fn().mockResolvedValue(undefined),
  listen: vi.fn().mockResolvedValue(() => {}),
}));

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});
