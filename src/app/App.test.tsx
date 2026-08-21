import { render, screen } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { describe, expect, it, vi } from "vitest";
import { App } from "@/app/App";

const mockInvoke = vi.mocked(invoke);

describe("App", () => {
  it("renders the application title", () => {
    render(<App />);
    expect(
      screen.getByRole("heading", { name: "My Daily Standup" }),
    ).toBeInTheDocument();
  });

  it("shows the greeting returned by the Rust bridge", async () => {
    mockInvoke.mockResolvedValueOnce("IPC bridge connected — hello, Wasif.");

    render(<App />);

    expect(await screen.findByTestId("bridge-status")).toHaveTextContent(
      "IPC bridge connected — hello, Wasif.",
    );
    expect(mockInvoke).toHaveBeenCalledWith("greet", { name: "Wasif" });
  });

  it("surfaces a failure instead of hanging on 'checking…'", async () => {
    // A rejected bridge call must be visible. Silently staying in the
    // loading state would hide a broken IPC layer.
    mockInvoke.mockRejectedValueOnce(new Error("bridge unavailable"));

    render(<App />);

    expect(await screen.findByTestId("bridge-status")).toHaveTextContent(
      /IPC bridge failed/,
    );
  });
});
