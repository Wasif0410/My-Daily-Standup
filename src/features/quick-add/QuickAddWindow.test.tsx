import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { QuickAddWindow } from "@/features/quick-add/QuickAddWindow";
import { emitTaskChanged } from "@/lib/taskEvents";

vi.mock("@/lib/taskEvents", () => ({
  emitTaskChanged: vi.fn(),
  onTaskChanged: vi.fn().mockResolvedValue(() => {}),
  TASK_CHANGED: "task-changed",
}));

const mockInvoke = vi.mocked(invoke);
const mockAnnounce = vi.mocked(emitTaskChanged);

const TODAY = "2026-08-23";

function respond(overrides: Record<string, unknown> = {}) {
  mockInvoke.mockImplementation((command: string) => {
    if (command in overrides) return Promise.resolve(overrides[command]);
    if (command === "week_current")
      return Promise.resolve({
        start: "2026-08-17",
        end: "2026-08-23",
        label: "2026-W34",
        today: TODAY,
        days: [],
      });
    return Promise.resolve(null);
  });
}

beforeEach(() => {
  mockInvoke.mockReset();
  mockAnnounce.mockClear();
});

describe("QuickAddWindow", () => {
  it("captures a task on Enter", async () => {
    const user = userEvent.setup();
    respond();
    render(<QuickAddWindow />);

    await user.type(screen.getByLabelText("Add a task"), "call the clinic{Enter}");

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith(
        "task_create",
        expect.objectContaining({}),
      );
    });
  });

  it("creates it as a daily task scheduled for today", async () => {
    // Capture means "this, now". Asking for a horizon at capture time is the
    // friction that stops capture happening; it can be moved afterwards.
    const user = userEvent.setup();
    respond();
    render(<QuickAddWindow />);

    await user.type(screen.getByLabelText("Add a task"), "call the clinic{Enter}");

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("task_create", {
        input: {
          title: "call the clinic",
          horizon: "daily",
          status: "planned",
          sourceType: "manual",
          scheduledDate: TODAY,
        },
      });
    });
  });

  it("asks Rust for today rather than reading the browser clock", async () => {
    // Every other date in the app comes from Rust; this one must too, or a
    // task captured at 11pm lands on the wrong day.
    respond();

    render(<QuickAddWindow />);

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("week_current", { startsOn: null });
    });
  });

  it("announces the change so the boards update", async () => {
    const user = userEvent.setup();
    respond();
    render(<QuickAddWindow />);

    await user.type(screen.getByLabelText("Add a task"), "call the clinic{Enter}");

    await waitFor(() => {
      expect(mockAnnounce).toHaveBeenCalled();
    });
  });

  it("closes itself after capturing", async () => {
    // A capture box left open is a box floating over everything an hour later.
    const user = userEvent.setup();
    respond();
    render(<QuickAddWindow />);

    await user.type(screen.getByLabelText("Add a task"), "call the clinic{Enter}");

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("quick_add_close", undefined);
    });
  });

  it("stays open and says so when the write fails", async () => {
    // Closing on failure would throw away what the user just typed, with no
    // record of it anywhere.
    const user = userEvent.setup();
    mockInvoke.mockImplementation((command: string) => {
      if (command === "week_current")
        return Promise.resolve({
          start: "2026-08-17",
          end: "2026-08-23",
          label: "2026-W34",
          today: TODAY,
          days: [],
        });
      if (command === "task_create") return Promise.reject(new Error("disk is full"));
      return Promise.resolve(null);
    });
    render(<QuickAddWindow />);

    await user.type(screen.getByLabelText("Add a task"), "call the clinic{Enter}");

    expect(await screen.findByRole("alert")).toHaveTextContent("disk is full");
    expect(mockInvoke).not.toHaveBeenCalledWith("quick_add_close", undefined);
  });

  it("closes on Escape without creating anything", async () => {
    const user = userEvent.setup();
    respond();
    render(<QuickAddWindow />);

    await user.type(screen.getByLabelText("Add a task"), "never mind{Escape}");

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("quick_add_close", undefined);
    });
    expect(mockInvoke).not.toHaveBeenCalledWith("task_create", expect.anything());
  });

  it("does not capture an empty title", async () => {
    const user = userEvent.setup();
    respond();
    render(<QuickAddWindow />);

    await user.type(screen.getByLabelText("Add a task"), "   {Enter}");

    expect(mockInvoke).not.toHaveBeenCalledWith("task_create", expect.anything());
  });

  it("starts no inference process", async () => {
    // Spec §6.8, and the reason Quick Add is not one of the disabled AI
    // entries: capturing a task must not load a model. No command other than
    // the date lookup, the create, and the close may be called.
    const user = userEvent.setup();
    respond();
    render(<QuickAddWindow />);

    await user.type(screen.getByLabelText("Add a task"), "call the clinic{Enter}");
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("quick_add_close", undefined);
    });

    const called = mockInvoke.mock.calls.map((c) => c[0]);
    expect(new Set(called)).toEqual(
      new Set(["week_current", "task_create", "quick_add_close"]),
    );
  });
});
