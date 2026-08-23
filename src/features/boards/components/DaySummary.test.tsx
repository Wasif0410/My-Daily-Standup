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

    expect(screen.getByLabelText("2 of 3 done, 1h 45m")).toBeInTheDocument();
  });

  it("says nothing was recorded rather than reading out a dash", () => {
    render(<DaySummary totals={{ completed: 0, total: 1, minutes: null }} />);

    expect(screen.getByLabelText("0 of 1 done, no time recorded")).toBeInTheDocument();
  });
});
