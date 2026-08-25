import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { DaySummary } from "@/features/boards/components/DaySummary";

describe("DaySummary", () => {
  it("shows completed against total", () => {
    render(<DaySummary totals={{ completed: 2, total: 3, minutes: 105 }} />);

    expect(screen.getByText("2/3")).toBeInTheDocument();
  });

  it("shows the recorded time", () => {
    render(<DaySummary totals={{ completed: 2, total: 3, minutes: 105 }} />);

    expect(screen.getByText("1h 45m")).toBeInTheDocument();
  });

  it("shows a dash when nothing was recorded", () => {
    // Never "0m": nobody measured the day, which is not the same as it taking
    // no time.
    render(<DaySummary totals={{ completed: 0, total: 2, minutes: null }} />);

    expect(screen.getByText("—")).toBeInTheDocument();
  });

  it("shows 0/0 for an empty day rather than nothing", () => {
    // A blank row would read as broken. An empty day is a fact about the week.
    render(<DaySummary totals={{ completed: 0, total: 0, minutes: null }} />);

    expect(screen.getByText("0/0")).toBeInTheDocument();
  });

  it("describes itself in words for screen readers", () => {
    // "2/3" and a dash are shorthand a screen reader cannot make sense of.
    render(<DaySummary totals={{ completed: 2, total: 3, minutes: 105 }} />);

    expect(
      screen.getByLabelText("2 of 3 done, 1h 45m, 67 percent"),
    ).toBeInTheDocument();
  });

  it("says nothing was recorded rather than reading out a dash", () => {
    render(<DaySummary totals={{ completed: 0, total: 1, minutes: null }} />);

    expect(
      screen.getByLabelText("0 of 1 done, no time recorded, 0 percent"),
    ).toBeInTheDocument();
  });

  it("shows the completion percentage", () => {
    render(<DaySummary totals={{ completed: 2, total: 3, minutes: 105 }} />);

    expect(screen.getByText("67%")).toBeInTheDocument();
  });

  it("keeps the count and the time alongside the percentage", () => {
    // The percentage is an addition, not a replacement. The raw counts are what
    // the user acts on; the percentage only tells them where to look first.
    render(<DaySummary totals={{ completed: 2, total: 3, minutes: 105 }} />);

    expect(screen.getByText("2/3")).toBeInTheDocument();
    expect(screen.getByText("1h 45m")).toBeInTheDocument();
    expect(screen.getByText("67%")).toBeInTheDocument();
  });

  it("marks a finished day as complete for the stylesheet", () => {
    render(<DaySummary totals={{ completed: 3, total: 3, minutes: 105 }} />);

    expect(screen.getByText("100%")).toHaveAttribute("data-complete", "true");
  });

  it("does not mark an unfinished day as complete", () => {
    render(<DaySummary totals={{ completed: 2, total: 3, minutes: 105 }} />);

    expect(screen.getByText("67%")).toHaveAttribute("data-complete", "false");
  });

  it("shows 0% for an empty day rather than 100%", () => {
    // Nothing planned is not everything done.
    render(<DaySummary totals={{ completed: 0, total: 0, minutes: null }} />);

    expect(screen.getByText("0%")).toBeInTheDocument();
  });
});
