import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { WeekNav } from "@/features/boards/components/WeekNav";
import type { Week } from "@/types/task";

const WEEK: Week = {
  label: "2026-W42",
  start: "2026-10-12",
  end: "2026-10-18",
  today: "2026-10-14",
  days: [],
};

describe("WeekNav", () => {
  it("says which week it is in words, then the dates", () => {
    render(<WeekNav week={WEEK} offset={0} onOffset={vi.fn()} />);

    expect(screen.getByText("This week")).toBeInTheDocument();
    expect(screen.getByText("Oct 12 – 18")).toBeInTheDocument();
    expect(screen.getByText("Week 42")).toBeInTheDocument();
  });

  it("steps back and forward one week at a time", async () => {
    const user = userEvent.setup();
    const onOffset = vi.fn();
    render(<WeekNav week={WEEK} offset={2} onOffset={onOffset} />);

    await user.click(screen.getByRole("button", { name: "Previous week" }));
    await user.click(screen.getByRole("button", { name: "Next week" }));

    expect(onOffset).toHaveBeenNthCalledWith(1, 1);
    expect(onOffset).toHaveBeenNthCalledWith(2, 3);
  });

  it("offers a way back to this week once you have left it", async () => {
    const user = userEvent.setup();
    const onOffset = vi.fn();
    render(<WeekNav week={WEEK} offset={-3} onOffset={onOffset} />);

    expect(screen.getByText("3 weeks ago")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Back to this week" }));

    expect(onOffset).toHaveBeenCalledWith(0);
  });

  it("does not offer it while already on this week", () => {
    render(<WeekNav week={WEEK} offset={0} onOffset={vi.fn()} />);

    expect(screen.getByRole("button", { name: "Back to this week" })).toBeDisabled();
  });
});
