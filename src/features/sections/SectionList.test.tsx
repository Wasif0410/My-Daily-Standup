import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { SectionList } from "@/features/sections/SectionList";
import type { BoardSection } from "@/types/section";

function make(id: string, title: string, position: number): BoardSection {
  return { id, boardKind: "priority", title, position, items: [] };
}

function handlers() {
  return {
    onAddSection: vi.fn(),
    onDoneAdding: vi.fn(),
    onRenameSection: vi.fn(),
    onDeleteSection: vi.fn(),
    onAddItem: vi.fn(),
    onUpdateItem: vi.fn(),
    onDeleteItem: vi.fn(),
  };
}

describe("SectionList", () => {
  it("renders every section in its stored order", () => {
    const sections = [make("a", "Job Search", 0), make("b", "Health", 1)];
    render(<SectionList sections={sections} adding={false} {...handlers()} />);

    const headings = screen.getAllByRole("heading").map((h) => h.textContent);
    expect(headings).toEqual(["Job Search", "Health"]);
  });

  it("carries no add button of its own", () => {
    render(<SectionList sections={[]} adding={false} {...handlers()} />);

    // Adding lives in the board header beside the gear, where the board's
    // other controls are. A second one down here would be two ways to do one
    // thing, and it would sit below the tasks where nobody would look.
    expect(
      screen.queryByRole("button", { name: "Add a section" }),
    ).not.toBeInTheDocument();
  });

  it("shows the name field only while the board says it is adding", () => {
    const { rerender } = render(
      <SectionList sections={[]} adding={false} {...handlers()} />,
    );
    expect(screen.queryByLabelText("New section name")).not.toBeInTheDocument();

    rerender(<SectionList sections={[]} adding={true} {...handlers()} />);
    expect(screen.getByLabelText("New section name")).toBeInTheDocument();
  });

  it("puts the new section at the top, where the button that made it is", () => {
    const h = handlers();
    render(
      <SectionList sections={[make("a", "Job Search", 0)]} adding={true} {...h} />,
    );

    const fields = screen.getAllByRole("textbox");
    // The field is above the first section's own entry field.
    expect(fields[0]).toHaveAccessibleName("New section name");
  });

  it("adds a section on Enter and tells the board it is finished", async () => {
    const user = userEvent.setup();
    const h = handlers();
    render(<SectionList sections={[]} adding={true} {...h} />);

    await user.type(screen.getByLabelText("New section name"), "Job Search{Enter}");

    expect(h.onAddSection).toHaveBeenCalledWith("Job Search");
    expect(h.onDoneAdding).toHaveBeenCalled();
  });

  it("refuses a blank section name", async () => {
    const user = userEvent.setup();
    const h = handlers();
    render(<SectionList sections={[]} adding={true} {...h} />);

    await user.type(screen.getByLabelText("New section name"), "   {Enter}");

    expect(h.onAddSection).not.toHaveBeenCalled();
  });

  it("abandons a new section on Escape", async () => {
    const user = userEvent.setup();
    const h = handlers();
    render(<SectionList sections={[]} adding={true} {...h} />);

    await user.type(screen.getByLabelText("New section name"), "Oops{Escape}");

    expect(h.onAddSection).not.toHaveBeenCalled();
    expect(h.onDoneAdding).toHaveBeenCalled();
  });

  it("says nothing at all when a board has no sections", () => {
    render(<SectionList sections={[]} adding={false} {...handlers()} />);

    // Sections are optional. A board that never uses them must not carry an
    // empty-state message competing with the tasks that are the point of it.
    expect(screen.queryByRole("heading")).not.toBeInTheDocument();
    expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
  });

  it("passes a bullet added in one section through with that section's id", async () => {
    const user = userEvent.setup();
    const h = handlers();
    render(
      <SectionList sections={[make("a", "Job Search", 0)]} adding={false} {...h} />,
    );

    await user.click(screen.getByRole("button", { name: "Add a note to Job Search" }));
    await user.type(screen.getByLabelText("Add to Job Search"), "Call back{Enter}");

    expect(h.onAddItem).toHaveBeenCalledWith("a", "Call back");
  });
});
