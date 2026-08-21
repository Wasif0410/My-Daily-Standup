import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/react";
import { afterEach, vi } from "vitest";

// Tauri's IPC does not exist in jsdom. Tests that care about a specific
// command mock it explicitly; this default keeps unrelated renders from
// throwing on the bridge call.
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue(""),
}));

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});
