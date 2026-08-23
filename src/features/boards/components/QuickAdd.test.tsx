import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { QuickAdd } from "@/features/boards/components/QuickAdd";

describe("QuickAdd", () => {
  it("adds a task on Enter", async () => {
    const onAdd = vi.fn();
    const user = userEvent.setup();
    render(<QuickAdd onAdd={onAdd} />);

    await user.type(screen.getByLabelText("Add a task"), "email two contacts{Enter}");

    expect(onAdd).toHaveBeenCalledWith("email two contacts");
  });

  it("clears the field after adding, ready for the next one", async () => {
    // Planning happens in bursts. Having to clear the field between entries
    // turns five tasks into ten gestures.
    const user = userEvent.setup();
    render(<QuickAdd onAdd={vi.fn()} />);

    await user.type(screen.getByLabelText("Add a task"), "first{Enter}");

    expect(screen.getByLabelText("Add a task")).toHaveValue("");
  });

  it("refuses a whitespace-only title", async () => {
    const onAdd = vi.fn();
    const user = userEvent.setup();
    render(<QuickAdd onAdd={onAdd} />);

    await user.type(screen.getByLabelText("Add a task"), "   {Enter}");

    expect(onAdd).not.toHaveBeenCalled();
  });

  it("does not submit an empty field", async () => {
    const onAdd = vi.fn();
    const user = userEvent.setup();
    render(<QuickAdd onAdd={onAdd} />);

    await user.click(screen.getByLabelText("Add a task"));
    await user.keyboard("{Enter}");

    expect(onAdd).not.toHaveBeenCalled();
  });

  it("trims surrounding whitespace", async () => {
    const onAdd = vi.fn();
    const user = userEvent.setup();
    render(<QuickAdd onAdd={onAdd} />);

    await user.type(screen.getByLabelText("Add a task"), "  padded  {Enter}");

    expect(onAdd).toHaveBeenCalledWith("padded");
  });

  it("clears without adding on Escape", async () => {
    const onAdd = vi.fn();
    const user = userEvent.setup();
    render(<QuickAdd onAdd={onAdd} />);

    await user.type(screen.getByLabelText("Add a task"), "never mind{Escape}");

    expect(onAdd).not.toHaveBeenCalled();
    expect(screen.getByLabelText("Add a task")).toHaveValue("");
  });

  it("takes a board-specific placeholder", async () => {
    render(<QuickAdd onAdd={vi.fn()} placeholder="Add to this week" />);

    expect(screen.getByPlaceholderText("Add to this week")).toBeInTheDocument();
  });
});
