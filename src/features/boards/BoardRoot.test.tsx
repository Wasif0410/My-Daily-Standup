import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
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

  it("puts the monthly board in the monthly-progress window", async () => {
    // The last placeholder is gone: every board kind now maps to a real
    // component, and the switch will not compile if a future one does not.
    mockInvoke.mockImplementation((command: string) => {
      if (command === "month_current")
        return Promise.resolve({
          start: "2026-08-01",
          end: "2026-08-31",
          label: "August 2026",
          today: "2026-08-23",
        });
      return Promise.resolve([]);
    });

    render(<BoardRoot kind="monthly-progress" />);

    expect(await screen.findByText("August 2026")).toBeInTheDocument();
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

describe("BoardRoot window behaviours", () => {
  function withBoard(overrides: Record<string, unknown> = {}) {
    mockInvoke.mockImplementation((command: string) => {
      if (command === "board_list")
        return Promise.resolve([
          {
            kind: "priority",
            x: 0,
            y: 0,
            width: 340,
            height: 460,
            monitor: null,
            visible: true,
            collapsed: false,
            opacity: 1,
            alwaysOnTop: false,
            locked: false,
            fontSize: 13,
            theme: "dark",
            compact: false,
            desktopLevel: false,
            ...overrides,
          },
        ]);
      if (command === "board_set_behavior") return Promise.resolve({});
      return Promise.resolve([]);
    });
  }

  it("opens the board menu from the header", async () => {
    const user = userEvent.setup();
    withBoard();
    render(<BoardRoot kind="priority" />);

    await user.click(await screen.findByRole("button", { name: /board settings/i }));

    expect(screen.getByRole("menu", { name: /board settings/i })).toBeInTheDocument();
  });

  it("sends a behaviour change to Rust", async () => {
    const user = userEvent.setup();
    withBoard();
    render(<BoardRoot kind="priority" />);

    await user.click(await screen.findByRole("button", { name: /board settings/i }));
    await user.click(screen.getByRole("menuitem", { name: /always on top/i }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("board_set_behavior", {
        kind: "priority",
        behavior: { kind: "alwaysOnTop", value: true },
      });
    });
  });

  it("applies a saved appearance to the shell", async () => {
    withBoard({ theme: "light", fontSize: 17, compact: true });
    const { container } = render(<BoardRoot kind="priority" />);

    await waitFor(() => {
      const shell = container.querySelector(".board");
      expect(shell).toHaveAttribute("data-theme", "light");
      expect(shell).toHaveAttribute("data-compact", "true");
      expect(shell?.getAttribute("style")).toContain("--board-font-size: 17px");
    });
  });
});

describe("BoardRoot closing and failure", () => {
  function board(overrides: Record<string, unknown> = {}) {
    return {
      kind: "priority",
      x: 0,
      y: 0,
      width: 340,
      height: 460,
      monitor: null,
      visible: true,
      collapsed: false,
      opacity: 1,
      alwaysOnTop: false,
      locked: false,
      fontSize: 13,
      theme: "dark",
      compact: false,
      desktopLevel: false,
      ...overrides,
    };
  }

  it("closes through the command, so the board stays shut after a restart", async () => {
    // getCurrentWindow().close() destroys the window but never records it, so
    // `visible` stays 1 and restore_boards brings the board straight back on
    // the next launch. Only board_close persists the choice.
    const user = userEvent.setup();
    mockInvoke.mockImplementation((command: string) => {
      if (command === "board_list") return Promise.resolve([board()]);
      return Promise.resolve(null);
    });
    render(<BoardRoot kind="priority" />);

    await user.click(await screen.findByRole("button", { name: /close board/i }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("board_close", { kind: "priority" });
    });
  });

  it("shows the failure instead of a blank window when loading throws", async () => {
    // The white-window bug: an unguarded throw in this component unmounted the
    // whole tree, leaving a frameless transparent window rendering as an
    // opaque white rectangle.
    mockInvoke.mockImplementation((command: string) => {
      if (command === "board_list") return Promise.reject(new Error("no IPC here"));
      return Promise.resolve([]);
    });

    render(<BoardRoot kind="priority" />);

    expect(await screen.findByText("no IPC here")).toBeInTheDocument();
  });

  it("still renders its chrome when loading fails", async () => {
    // A board that loses its title bar has no close button and no menu, so
    // there is no way to get rid of it.
    mockInvoke.mockImplementation((command: string) => {
      if (command === "board_list") return Promise.reject(new Error("no IPC here"));
      return Promise.resolve([]);
    });

    render(<BoardRoot kind="priority" />);

    await screen.findByText("no IPC here");
    expect(screen.getByRole("heading", { name: "Priority Tasks" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /close board/i })).toBeInTheDocument();
  });
});
