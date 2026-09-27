import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ContextPreview } from "@/features/chat/ContextPreview";
import type { ContextPreview as Preview } from "@/types/context";

function preview(overrides: Partial<Preview> = {}): Preview {
  return {
    map: "COMMITMENT MAP — active\n\nJob Search [p9, in progress] — 12/20 applications",
    prompt: "You are a planning assistant.\n\nCOMMITMENT MAP — active\n…",
    mapTokens: 412,
    contextTokens: 903,
    totalTokens: 1315,
    ...overrides,
  };
}

describe("ContextPreview", () => {
  it("shows the map the model will actually be given", () => {
    render(
      <ContextPreview
        preview={preview()}
        loading={false}
        error={null}
        onRefresh={vi.fn()}
      />,
    );

    expect(screen.getByText(/Job Search \[p9, in progress\]/)).toBeInTheDocument();
  });

  it("reports the token cost of each tier and the total", () => {
    render(
      <ContextPreview
        preview={preview()}
        loading={false}
        error={null}
        onRefresh={vi.fn()}
      />,
    );

    // The budgets are the whole point of the tiering. A preview that hid them
    // would leave the one number that decides whether a prompt fits invisible.
    expect(screen.getByText("412")).toBeInTheDocument();
    expect(screen.getByText("903")).toBeInTheDocument();
    expect(screen.getByText("1315")).toBeInTheDocument();
  });

  it("refreshes on request", async () => {
    const user = userEvent.setup();
    const onRefresh = vi.fn();
    render(
      <ContextPreview
        preview={preview()}
        loading={false}
        error={null}
        onRefresh={onRefresh}
      />,
    );

    await user.click(screen.getByRole("button", { name: /refresh/i }));

    expect(onRefresh).toHaveBeenCalled();
  });

  it("says it is working rather than showing a stale map", () => {
    render(
      <ContextPreview preview={null} loading={true} error={null} onRefresh={vi.fn()} />,
    );

    // The hint specifically, not the button's own label — the point is that
    // the panel explains itself while empty rather than looking broken.
    expect(screen.getByText(/building the map from your boards/i)).toBeInTheDocument();
  });

  it("surfaces a failure instead of an empty panel", () => {
    render(
      <ContextPreview
        preview={null}
        loading={false}
        error={{ kind: "internal", message: "no tasks found" }}
        onRefresh={vi.fn()}
      />,
    );

    expect(screen.getByRole("alert")).toHaveTextContent(/no tasks found/);
  });

  it("renders nothing but the control before anything has been built", () => {
    render(
      <ContextPreview
        preview={null}
        loading={false}
        error={null}
        onRefresh={vi.fn()}
      />,
    );

    // Not an empty-state paragraph: this is a diagnostic surface, and a button
    // that has not been pressed needs no explanation.
    expect(screen.getByRole("button", { name: /refresh/i })).toBeInTheDocument();
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("keeps the map readable as preformatted text", () => {
    // The map's meaning is in its indentation — a section's milestones sit
    // under it. Collapsing whitespace would destroy the structure the model
    // is being shown.
    const { container } = render(
      <ContextPreview
        preview={preview()}
        loading={false}
        error={null}
        onRefresh={vi.fn()}
      />,
    );

    expect(container.querySelector("pre")).not.toBeNull();
  });
});
