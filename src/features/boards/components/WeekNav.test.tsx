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
  days: [
    { date: "2026-10-12", name: "Monday" },
    { date: "2026-10-13", name: "Tuesday" },
    { date: "2026-10-14", name: "Wednesday" },
    { date: "2026-10-15", name: "Thursday" },
    { date: "2026-10-16", name: "Friday" },
    { date: "2026-10-17", name: "Saturday" },
    { date: "2026-10-18", name: "Sunday" },
  ],
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

describe("WeekNav day strip", () => {
  it("shows all seven days with their dates", () => {
    render(<WeekNav week={WEEK} offset={0} onOffset={vi.fn()} />);

    const days = screen.getAllByRole("listitem");
    expect(days).toHaveLength(7);
    expect(days[0]).toHaveTextContent("M12");
    expect(days[6]).toHaveTextContent("S18");
  });

  it("marks today so the eye lands on it", () => {
    render(<WeekNav week={WEEK} offset={0} onOffset={vi.fn()} />);

    expect(screen.getByLabelText("Wednesday 14, today")).toHaveAttribute(
      "aria-current",
      "date",
    );
  });

  it("marks no day as today on another week", () => {
    const next = { ...WEEK, today: "2026-10-21" };
    render(<WeekNav week={next} offset={1} onOffset={vi.fn()} />);

    expect(screen.queryByLabelText(/today$/)).not.toBeInTheDocument();
  });

  it("shows which days have work and which are finished", () => {
    render(
      <WeekNav
        week={WEEK}
        offset={0}
        onOffset={vi.fn()}
        marks={{ "2026-10-12": "done", "2026-10-13": "open" }}
      />,
    );

    expect(screen.getByLabelText("Monday 12, all done")).toBeInTheDocument();
    expect(screen.getByLabelText("Tuesday 13, tasks left")).toBeInTheDocument();
  });

  it("opens a day when it is chosen, where the board allows it", async () => {
    const user = userEvent.setup();
    const onDay = vi.fn();
    render(<WeekNav week={WEEK} offset={0} onOffset={vi.fn()} onDay={onDay} />);

    await user.click(screen.getByRole("button", { name: /Thursday 15/ }));

    expect(onDay).toHaveBeenCalledWith("2026-10-15");
  });

  it("keeps days as plain labels where nothing opens", () => {
    render(<WeekNav week={WEEK} offset={0} onOffset={vi.fn()} />);

    expect(
      screen.queryByRole("button", { name: /Thursday 15/ }),
    ).not.toBeInTheDocument();
  });
});
