import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { BoardEmpty } from "@/features/boards/components/BoardEmpty";

describe("BoardEmpty", () => {
  it("says what the board is for and how to fill it", () => {
    render(<BoardEmpty title="Plan your week" hint="Add what you want to get done." />);

    expect(screen.getByText("Plan your week")).toBeInTheDocument();
    expect(screen.getByText("Add what you want to get done.")).toBeInTheDocument();
  });

  it("holds the board's first action when it has one", () => {
    render(
      <BoardEmpty title="Plan your week" hint="Add what you want to get done.">
        <button type="button">Add a task</button>
      </BoardEmpty>,
    );

    expect(screen.getByRole("button", { name: "Add a task" })).toBeInTheDocument();
  });
});
