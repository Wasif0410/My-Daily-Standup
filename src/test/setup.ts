import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/react";
import { afterEach, vi } from "vitest";

// Tauri's IPC does not exist in jsdom. Tests that care about a specific
// command mock it explicitly; this default keeps unrelated renders from
// throwing on the bridge call.
//
// The section and settings commands answer with their own shapes rather than
// the blanket "": a component that merely renders while subscribing to
// `section_list` or `settings_get` would be handed a string where it expects
// an array or an object, which passes nothing and only shows up as a crash in
// CI. Every such command is answered here, not in the individual test files,
// for the same reason.
vi.mock("@tauri-apps/api/core", () => {
  const section = {
    id: "test-section",
    boardKind: "priority",
    title: "Section",
    position: 0,
  };

  // A full `Settings`, matching the Rust defaults. A subscriber reads
  // `settings.priorityThreshold` on its first render, and `""` has no such
  // field.
  const settings = {
    priorityThreshold: 5,
    weekStartsOn: "monday",
    launchAtLogin: false,
  };

  // A stopped `ChatStatus` and a full `ChatReply`, for the same reason. A
  // component subscribing to `chat_status` reads `status.running` on its first
  // render, and `""` has no such field — the panel would then decide what to
  // show from `undefined`.
  const chatStatus = { running: false, model: null, port: null };

  const chatReply = {
    content: "",
    promptTokens: 0,
    completionTokens: 0,
    elapsedMs: 0,
  };

  const defaults: Record<string, unknown> = {
    section_list: [],
    section_create: section,
    section_rename: section,
    section_delete: undefined,
    settings_get: settings,
    settings_update: settings,
    chat_status: chatStatus,
    chat_start: chatStatus,
    chat_send: chatReply,
    chat_stop: chatStatus,
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
