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
    innerSize: () => Promise.resolve({ toLogical: () => ({ width: 340, height: 460 }) }),
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
