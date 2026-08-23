import { render, screen } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { BoardRoot } from "@/features/boards/BoardRoot";
import { useTaskStore } from "@/stores/taskStore";

const mockInvoke = vi.mocked(invoke);

// BoardRoot drives the real window; jsdom has none.
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    onMoved: () => Promise.resolve(() => {}),
    onResized: () => Promise.resolve(() => {}),
    outerPosition: () => Promise.resolve({ x: 0, y: 0 }),
    innerSize: () =>
      Promise.resolve({ toLogical: () => ({ width: 340, height: 460 }) }),
    scaleFactor: () => Promise.resolve(1),
    setSize: () => Promise.resolve(),
    close: () => Promise.resolve(),
  }),
  LogicalSize: class {},
}));

beforeEach(() => {
  useTaskStore.setState({ tasks: {}, filter: null, loading: false, error: null });
  mockInvoke.mockResolvedValue([]);
});

describe("BoardRoot", () => {
  it("puts the priority board inside the priority window", async () => {
    render(<BoardRoot kind="priority" />);

    expect(
      await screen.findByText(/nothing at priority 5 or above/i),
    ).toBeInTheDocument();
  });

  it("still shows a placeholder for boards that have no content yet", async () => {
    // PRs 13-15 fill these in. Until then the window must say something rather
    // than render blank, which reads as broken.
    render(<BoardRoot kind="monthly-progress" />);

    expect(await screen.findByText("No tasks yet.")).toBeInTheDocument();
  });
});

describe("BoardRoot board content", () => {
  it("puts the weekly board in the weekly-tasks window", async () => {
    mockInvoke.mockImplementation((command: string) => {
      if (command === "week_current")
        return Promise.resolve({ start: "2026-08-17", end: "2026-08-23" });
      return Promise.resolve([]);
    });

    render(<BoardRoot kind="weekly-tasks" />);

    expect(await screen.findByText(/nothing planned this week/i)).toBeInTheDocument();
  });
});

describe("BoardRoot progress window", () => {
  it("puts the progress board in the weekly-progress window", async () => {
    mockInvoke.mockImplementation((command: string) => {
      if (command === "week_current")
        return Promise.resolve({
          start: "2026-08-17",
          end: "2026-08-23",
          label: "2026-W34",
          today: "2026-08-18",
          days: [
            { date: "2026-08-17", name: "Monday" },
            { date: "2026-08-18", name: "Tuesday" },
            { date: "2026-08-19", name: "Wednesday" },
            { date: "2026-08-20", name: "Thursday" },
            { date: "2026-08-21", name: "Friday" },
            { date: "2026-08-22", name: "Saturday" },
            { date: "2026-08-23", name: "Sunday" },
          ],
        });
      if (command === "ui_state_get") return Promise.resolve(null);
      return Promise.resolve([]);
    });

    render(<BoardRoot kind="weekly-progress" />);

    expect(await screen.findByRole("button", { name: /Monday/ })).toBeInTheDocument();
  });
});
