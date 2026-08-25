import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/react";
import { afterEach, vi } from "vitest";

// Tauri's IPC does not exist in jsdom. Tests that care about a specific
// command mock it explicitly; this default keeps unrelated renders from
// throwing on the bridge call.
//
// The section commands answer with their own shapes rather than the blanket
// "": a component that merely renders while subscribing to `section_list`
// would be handed a string where it expects an array, which passes nothing and
// only shows up as a crash in CI. Every section command is answered here, not
// in the individual test files, for the same reason.
vi.mock("@tauri-apps/api/core", () => {
  const section = {
    id: "test-section",
    boardKind: "priority",
    title: "Section",
    position: 0,
    items: [],
  };
  const item = { id: "test-item", sectionId: "test-section", text: "", position: 0 };

  const defaults: Record<string, unknown> = {
    section_list: [],
    section_create: section,
    section_rename: section,
    section_delete: undefined,
    section_item_add: item,
    section_item_update: item,
    section_item_delete: undefined,
  };

  return {
    invoke: vi.fn((command: string) =>
      Promise.resolve(command in defaults ? defaults[command] : ""),
    ),
  };
});

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
