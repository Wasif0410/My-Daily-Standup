import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AddTaskHere } from "@/features/boards/components/AddTaskHere";

describe("AddTaskHere", () => {
  it("names the group it files into", () => {
    // Every group header carries one of these, so "Add a task" alone would
    // give a screen reader a board full of identical buttons.
    render(<AddTaskHere label="Job Search" onAdd={vi.fn()} />);

    expect(
      screen.getByRole("button", { name: "Add a task to Job Search" }),
    ).toBeInTheDocument();
  });

  it("keeps the field out of the way until the plus is pressed", async () => {
    const user = userEvent.setup();
    render(<AddTaskHere label="Job Search" onAdd={vi.fn()} />);

    // A standing field under every heading is a column of empty boxes down a
    // 340px board.
    expect(screen.queryByLabelText("Add to Job Search")).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Add a task to Job Search" }));
    expect(screen.getByLabelText("Add to Job Search")).toBeInTheDocument();
  });

  it("adds on Enter and clears the field for the next one", async () => {
    const user = userEvent.setup();
    const onAdd = vi.fn();
    render(<AddTaskHere label="Job Search" onAdd={onAdd} />);

    await user.click(screen.getByRole("button", { name: "Add a task to Job Search" }));
    const field = screen.getByLabelText("Add to Job Search");
    await user.type(field, "email two contacts{Enter}");

    expect(onAdd).toHaveBeenCalledWith("email two contacts");
    expect(field).toHaveValue("");
  });

  it("stays open so a run of tasks is one Enter each", async () => {
    const user = userEvent.setup();
    const onAdd = vi.fn();
    render(<AddTaskHere label="Job Search" onAdd={onAdd} />);

    await user.click(screen.getByRole("button", { name: "Add a task to Job Search" }));
    await user.type(screen.getByLabelText("Add to Job Search"), "One{Enter}Two{Enter}");

    expect(onAdd).toHaveBeenNthCalledWith(1, "One");
    expect(onAdd).toHaveBeenNthCalledWith(2, "Two");
  });

  it("trims what it is given rather than filing the padding", async () => {
    const user = userEvent.setup();
    const onAdd = vi.fn();
    render(<AddTaskHere label="Job Search" onAdd={onAdd} />);

    await user.click(screen.getByRole("button", { name: "Add a task to Job Search" }));
    await user.type(screen.getByLabelText("Add to Job Search"), "  call back  {Enter}");

    expect(onAdd).toHaveBeenCalledWith("call back");
  });

  it("refuses a blank title rather than making a nameless row", async () => {
    const user = userEvent.setup();
    const onAdd = vi.fn();
    render(<AddTaskHere label="Job Search" onAdd={onAdd} />);

    await user.click(screen.getByRole("button", { name: "Add a task to Job Search" }));
    await user.type(screen.getByLabelText("Add to Job Search"), "   {Enter}");

    expect(onAdd).not.toHaveBeenCalled();
    // Still open: refusing is not the same as being finished with.
    expect(screen.getByLabelText("Add to Job Search")).toBeInTheDocument();
  });

  it("closes on Escape without adding anything", async () => {
    const user = userEvent.setup();
    const onAdd = vi.fn();
    render(<AddTaskHere label="Job Search" onAdd={onAdd} />);

    await user.click(screen.getByRole("button", { name: "Add a task to Job Search" }));
    await user.type(screen.getByLabelText("Add to Job Search"), "never mind{Escape}");

    expect(onAdd).not.toHaveBeenCalled();
    expect(screen.queryByLabelText("Add to Job Search")).not.toBeInTheDocument();
  });

  it("forgets an abandoned draft rather than reopening onto it", async () => {
    const user = userEvent.setup();
    render(<AddTaskHere label="Job Search" onAdd={vi.fn()} />);

    await user.click(screen.getByRole("button", { name: "Add a task to Job Search" }));
    await user.type(screen.getByLabelText("Add to Job Search"), "half a thought");
    await user.keyboard("{Escape}");
    await user.click(screen.getByRole("button", { name: "Add a task to Job Search" }));

    expect(screen.getByLabelText("Add to Job Search")).toHaveValue("");
  });

  it("works for a group whose name has spaces and odd casing", async () => {
    const user = userEvent.setup();
    const onAdd = vi.fn();
    render(<AddTaskHere label="No project" onAdd={onAdd} />);

    await user.click(screen.getByRole("button", { name: "Add a task to No project" }));
    await user.type(screen.getByLabelText("Add to No project"), "a thing{Enter}");

    expect(onAdd).toHaveBeenCalledWith("a thing");
  });
});
