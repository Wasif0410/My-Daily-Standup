import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { SectionCard } from "@/features/sections/SectionCard";
import type { BoardSection } from "@/types/section";

function section(overrides: Partial<BoardSection> = {}): BoardSection {
  return {
    id: "s1",
    boardKind: "priority",
    title: "Job Search",
    position: 0,
    items: [
      { id: "i1", sectionId: "s1", text: "Rewrite the resume", position: 0 },
      { id: "i2", sectionId: "s1", text: "Email the recruiter", position: 1 },
    ],
    ...overrides,
  };
}

function handlers() {
  return {
    onRename: vi.fn(),
    onDelete: vi.fn(),
    onAddItem: vi.fn(),
    onUpdateItem: vi.fn(),
    onDeleteItem: vi.fn(),
  };
}

describe("SectionCard", () => {
  it("shows the section name and every bullet under it", () => {
    render(<SectionCard section={section()} {...handlers()} />);

    expect(screen.getByRole("heading", { name: "Job Search" })).toBeInTheDocument();
    expect(screen.getByText("Rewrite the resume")).toBeInTheDocument();
    expect(screen.getByText("Email the recruiter")).toBeInTheDocument();
  });

  it("keeps bullets in their stored order rather than the order they arrived", () => {
    // The store sorts by position; the card must not re-sort or reverse it.
    render(<SectionCard section={section()} {...handlers()} />);

    const bullets = screen.getAllByRole("listitem").map((li) => li.textContent);
    expect(bullets[0]).toContain("Rewrite the resume");
    expect(bullets[1]).toContain("Email the recruiter");
  });

  it("adds a bullet on Enter and clears the field for the next one", async () => {
    const user = userEvent.setup();
    const h = handlers();
    render(<SectionCard section={section()} {...h} />);

    await user.click(screen.getByRole("button", { name: "Add a note to Job Search" }));
    const field = screen.getByLabelText("Add to Job Search");
    await user.type(field, "Call the agency{Enter}");

    expect(h.onAddItem).toHaveBeenCalledWith("s1", "Call the agency");
    // Planning happens in bursts: clearing by hand between bullets would turn
    // five entries into ten gestures.
    expect(field).toHaveValue("");
  });

  it("refuses a blank bullet rather than adding an unreadable empty row", async () => {
    const user = userEvent.setup();
    const h = handlers();
    render(<SectionCard section={section()} {...h} />);

    await user.click(screen.getByRole("button", { name: "Add a note to Job Search" }));
    await user.type(screen.getByLabelText("Add to Job Search"), "   {Enter}");

    expect(h.onAddItem).not.toHaveBeenCalled();
  });

  it("commits an edited bullet when focus leaves it", async () => {
    const user = userEvent.setup();
    const h = handlers();
    render(<SectionCard section={section()} {...h} />);

    await user.click(screen.getByText("Rewrite the resume"));
    const editor = screen.getByLabelText("Edit bullet");
    await user.clear(editor);
    await user.type(editor, "Rewrite the resume properly");
    await user.tab();

    expect(h.onUpdateItem).toHaveBeenCalledWith("i1", "Rewrite the resume properly");
  });

  it("abandons an edited bullet on Escape", async () => {
    const user = userEvent.setup();
    const h = handlers();
    render(<SectionCard section={section()} {...h} />);

    await user.click(screen.getByText("Rewrite the resume"));
    await user.type(screen.getByLabelText("Edit bullet"), " and more{Escape}");

    // Escape is the only cancel. Losing a rename by clicking away is the more
    // damaging default, so blur commits and only Escape discards.
    expect(h.onUpdateItem).not.toHaveBeenCalled();
    expect(screen.getByText("Rewrite the resume")).toBeInTheDocument();
  });

  it("renames the section from its heading", async () => {
    const user = userEvent.setup();
    const h = handlers();
    render(<SectionCard section={section()} {...h} />);

    await user.click(screen.getByRole("heading", { name: "Job Search" }));
    const editor = screen.getByLabelText("Edit section name");
    await user.clear(editor);
    await user.type(editor, "Job Hunt");
    await user.tab();

    expect(h.onRename).toHaveBeenCalledWith("s1", "Job Hunt");
  });

  it("does not rename a section to nothing", async () => {
    const user = userEvent.setup();
    const h = handlers();
    render(<SectionCard section={section()} {...h} />);

    await user.click(screen.getByRole("heading", { name: "Job Search" }));
    await user.clear(screen.getByLabelText("Edit section name"));
    await user.tab();

    // A nameless section cannot be identified afterwards, so the old name
    // stands rather than the board growing an unlabelled group.
    expect(h.onRename).not.toHaveBeenCalled();
  });

  it("deletes the section", async () => {
    const user = userEvent.setup();
    const h = handlers();
    render(<SectionCard section={section()} {...h} />);

    await user.click(screen.getByRole("button", { name: "Delete Job Search" }));

    expect(h.onDelete).toHaveBeenCalledWith("s1");
  });

  it("deletes a single bullet", async () => {
    const user = userEvent.setup();
    const h = handlers();
    render(<SectionCard section={section()} {...h} />);

    await user.click(screen.getByRole("button", { name: "Delete Rewrite the resume" }));

    expect(h.onDeleteItem).toHaveBeenCalledWith("i1");
  });

  it("keeps the add field out of the way until the plus is pressed", async () => {
    const user = userEvent.setup();
    render(<SectionCard section={section()} {...handlers()} />);

    // A standing field under every section is a row of empty boxes down a
    // 340px board. The plus sits in the section header instead, beside the
    // heading it belongs to.
    expect(screen.queryByLabelText("Add to Job Search")).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Add a note to Job Search" }));
    expect(screen.getByLabelText("Add to Job Search")).toBeInTheDocument();
  });

  it("offers the plus on a section with no bullets yet", () => {
    render(<SectionCard section={section({ items: [] })} {...handlers()} />);

    // A new section is empty by definition; with no way in it would be a dead
    // heading nobody could fill.
    expect(
      screen.getByRole("button", { name: "Add a note to Job Search" }),
    ).toBeInTheDocument();
  });

  it("stays open after a bullet so a run of notes is one gesture each", async () => {
    const user = userEvent.setup();
    const h = handlers();
    render(<SectionCard section={section()} {...h} />);

    await user.click(screen.getByRole("button", { name: "Add a note to Job Search" }));
    await user.type(screen.getByLabelText("Add to Job Search"), "One{Enter}Two{Enter}");

    expect(h.onAddItem).toHaveBeenNthCalledWith(1, "s1", "One");
    expect(h.onAddItem).toHaveBeenNthCalledWith(2, "s1", "Two");
  });

  it("opens the next bullet when Enter ends the one being edited", async () => {
    const user = userEvent.setup();
    const h = handlers();
    render(<SectionCard section={section()} {...h} />);

    await user.click(screen.getByText("Rewrite the resume"));
    const editor = screen.getByLabelText("Edit bullet");
    await user.clear(editor);
    await user.type(editor, "Rewrite it properly{Enter}");

    // The second route in, and the one that matters while writing: it never
    // leaves the keyboard.
    expect(h.onUpdateItem).toHaveBeenCalledWith("i1", "Rewrite it properly");
    expect(screen.getByLabelText("Add to Job Search")).toBeInTheDocument();
  });

  it("does not open a new bullet when an edit is abandoned", async () => {
    const user = userEvent.setup();
    render(<SectionCard section={section()} {...handlers()} />);

    await user.click(screen.getByText("Rewrite the resume"));
    await user.type(screen.getByLabelText("Edit bullet"), " more{Escape}");

    expect(screen.queryByLabelText("Add to Job Search")).not.toBeInTheDocument();
  });

  it("closes the add field on Escape", async () => {
    const user = userEvent.setup();
    render(<SectionCard section={section()} {...handlers()} />);

    await user.click(screen.getByRole("button", { name: "Add a note to Job Search" }));
    await user.type(screen.getByLabelText("Add to Job Search"), "{Escape}");

    expect(screen.queryByLabelText("Add to Job Search")).not.toBeInTheDocument();
  });
});
