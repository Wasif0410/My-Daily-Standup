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
    render(<SectionList sections={sections} {...handlers()} />);

    const headings = screen.getAllByRole("heading").map((h) => h.textContent);
    expect(headings).toEqual(["Job Search", "Health"]);
  });

  it("adds a section from the add field", async () => {
    const user = userEvent.setup();
    const h = handlers();
    render(<SectionList sections={[]} {...h} />);

    await user.click(screen.getByRole("button", { name: "Add a section" }));
    await user.type(screen.getByLabelText("New section name"), "Job Search{Enter}");

    expect(h.onAddSection).toHaveBeenCalledWith("Job Search");
  });

  it("refuses a blank section name", async () => {
    const user = userEvent.setup();
    const h = handlers();
    render(<SectionList sections={[]} {...h} />);

    await user.click(screen.getByRole("button", { name: "Add a section" }));
    await user.type(screen.getByLabelText("New section name"), "   {Enter}");

    expect(h.onAddSection).not.toHaveBeenCalled();
  });

  it("abandons a new section on Escape and puts the button back", async () => {
    const user = userEvent.setup();
    const h = handlers();
    render(<SectionList sections={[]} {...h} />);

    await user.click(screen.getByRole("button", { name: "Add a section" }));
    await user.type(screen.getByLabelText("New section name"), "Oops{Escape}");

    expect(h.onAddSection).not.toHaveBeenCalled();
    expect(screen.getByRole("button", { name: "Add a section" })).toBeInTheDocument();
  });

  it("says nothing at all when a board has no sections", () => {
    render(<SectionList sections={[]} {...handlers()} />);

    // Sections are optional. A board that never uses them must not carry an
    // empty-state message competing with the tasks that are the point of it.
    expect(screen.queryByRole("heading")).not.toBeInTheDocument();
  });

  it("passes a bullet added in one section through with that section's id", async () => {
    const user = userEvent.setup();
    const h = handlers();
    render(<SectionList sections={[make("a", "Job Search", 0)]} {...h} />);

    await user.type(screen.getByLabelText("Add to Job Search"), "Call back{Enter}");

    expect(h.onAddItem).toHaveBeenCalledWith("a", "Call back");
  });
});
