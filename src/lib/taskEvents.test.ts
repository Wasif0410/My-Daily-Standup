import { emit, listen } from "@tauri-apps/api/event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { emitTaskChanged, onTaskChanged, TASK_CHANGED } from "@/lib/taskEvents";

vi.mock("@tauri-apps/api/event", () => ({
  emit: vi.fn().mockResolvedValue(undefined),
  listen: vi.fn().mockResolvedValue(() => {}),
}));

const mockEmit = vi.mocked(emit);
const mockListen = vi.mocked(listen);

beforeEach(() => {
  vi.clearAllMocks();
});

describe("task change announcements", () => {
  it("broadcasts on the shared channel", () => {
    emitTaskChanged();

    expect(mockEmit).toHaveBeenCalledWith(TASK_CHANGED);
  });

  it("is fire-and-forget, so a failure cannot break the mutation that caused it", async () => {
    // The write already succeeded. A board that failed to hear about it is a
    // stale board; a rejected promise here would be a broken checkbox.
    mockEmit.mockRejectedValueOnce(new Error("no event permission"));

    expect(() => emitTaskChanged()).not.toThrow();
    await Promise.resolve();
  });

  it("subscribes to the same channel it emits on", async () => {
    await onTaskChanged(vi.fn());

    expect(mockListen).toHaveBeenCalledWith(TASK_CHANGED, expect.any(Function));
  });

  it("runs the handler when the event arrives", async () => {
    const handler = vi.fn();
    mockListen.mockImplementationOnce((_event, callback) => {
      (callback as () => void)();
      return Promise.resolve(() => {});
    });

    await onTaskChanged(handler);

    expect(handler).toHaveBeenCalled();
  });

  it("returns an unsubscribe, so a closing board stops listening", async () => {
    const unlisten = vi.fn();
    mockListen.mockResolvedValueOnce(unlisten);

    const stop = await onTaskChanged(vi.fn());
    stop();

    expect(unlisten).toHaveBeenCalled();
  });
});
