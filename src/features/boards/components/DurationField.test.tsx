import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { DurationField } from "@/features/boards/components/DurationField";

describe("DurationField", () => {
  it("shows a recorded duration", () => {
    render(<DurationField minutes={95} onChange={vi.fn()} />);

    expect(screen.getByRole("button", { name: /1h 35m/ })).toBeInTheDocument();
  });

  it("shows a dash when nothing was recorded", () => {
    // Never "0m": zero claims the work took no time, when in fact nobody
    // measured it.
    render(<DurationField minutes={null} onChange={vi.fn()} />);

    expect(screen.getByRole("button", { name: /not recorded/i })).toHaveTextContent("—");
  });

  it("stays closed until asked", () => {
    render(<DurationField minutes={null} onChange={vi.fn()} />);

    expect(screen.queryByLabelText("Duration")).toBeNull();
  });

  it("records a preset in one click", async () => {
    // Logging has to be one gesture. If it takes longer it stops happening,
    // and the weekly recap is built on nothing.
    const onChange = vi.fn();
    const user = userEvent.setup();
    render(<DurationField minutes={null} onChange={onChange} />);

    await user.click(screen.getByRole("button", { name: /not recorded/i }));
    await user.click(screen.getByRole("button", { name: "30m" }));

    expect(onChange).toHaveBeenCalledWith(30);
  });

  it("closes after a preset is chosen", async () => {
    const user = userEvent.setup();
    render(<DurationField minutes={null} onChange={vi.fn()} />);

    await user.click(screen.getByRole("button", { name: /not recorded/i }));
    await user.click(screen.getByRole("button", { name: "1h" }));

    expect(screen.queryByLabelText("Duration")).toBeNull();
  });

  it("records free text on Enter", async () => {
    const onChange = vi.fn();
    const user = userEvent.setup();
    render(<DurationField minutes={null} onChange={onChange} />);

    await user.click(screen.getByRole("button", { name: /not recorded/i }));
    await user.type(screen.getByLabelText("Duration"), "1h30m{Enter}");

    expect(onChange).toHaveBeenCalledWith(90);
  });

  it("reads a bare number as minutes", async () => {
    // Reading it as hours would inflate every total sixtyfold.
    const onChange = vi.fn();
    const user = userEvent.setup();
    render(<DurationField minutes={null} onChange={onChange} />);

    await user.click(screen.getByRole("button", { name: /not recorded/i }));
    await user.type(screen.getByLabelText("Duration"), "45{Enter}");

    expect(onChange).toHaveBeenCalledWith(45);
  });

  it("refuses input it cannot parse instead of guessing", async () => {
    const onChange = vi.fn();
    const user = userEvent.setup();
    render(<DurationField minutes={null} onChange={onChange} />);

    await user.click(screen.getByRole("button", { name: /not recorded/i }));
    await user.type(screen.getByLabelText("Duration"), "ages{Enter}");

    expect(onChange).not.toHaveBeenCalled();
    expect(screen.getByRole("alert")).toBeInTheDocument();
    expect(screen.getByLabelText("Duration")).toBeInTheDocument();
  });

  it("clears a recorded duration back to unrecorded", async () => {
    // Back to null, not to zero — the two mean different things to every
    // total on the progress boards.
    const onChange = vi.fn();
    const user = userEvent.setup();
    render(<DurationField minutes={35} onChange={onChange} />);

    await user.click(screen.getByRole("button", { name: /35m/ }));
    await user.click(screen.getByRole("button", { name: /clear/i }));

    expect(onChange).toHaveBeenCalledWith(null);
  });

  it("offers no clear button when there is nothing to clear", async () => {
    const user = userEvent.setup();
    render(<DurationField minutes={null} onChange={vi.fn()} />);

    await user.click(screen.getByRole("button", { name: /not recorded/i }));

    expect(screen.queryByRole("button", { name: /clear/i })).toBeNull();
  });

  it("abandons the edit on Escape", async () => {
    const onChange = vi.fn();
    const user = userEvent.setup();
    render(<DurationField minutes={35} onChange={onChange} />);

    await user.click(screen.getByRole("button", { name: /35m/ }));
    await user.type(screen.getByLabelText("Duration"), "90{Escape}");

    expect(onChange).not.toHaveBeenCalled();
    expect(screen.queryByLabelText("Duration")).toBeNull();
  });
});
