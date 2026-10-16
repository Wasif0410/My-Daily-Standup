import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { DaySummary } from "@/features/boards/components/DaySummary";

/** The bar's fill, which carries the day's completion as its width. */
function fillWidth(container: HTMLElement): string {
  const fill = container.querySelector<HTMLElement>(".day-bar-fill");
  return fill?.style.width ?? "";
}

describe("DaySummary", () => {
  it("shows completed against total", () => {
    render(<DaySummary totals={{ completed: 2, total: 3, minutes: 105 }} />);

    expect(screen.getByText("2/3")).toBeInTheDocument();
  });

  it("shows the recorded time", () => {
    render(<DaySummary totals={{ completed: 2, total: 3, minutes: 105 }} />);

    expect(screen.getByText("1h 45m")).toBeInTheDocument();
  });

  it("leaves the time out when nothing was recorded", () => {
    // Never "0m": nobody measured the day, which is not the same as it taking
    // no time. A dash in every row was noise; the spoken label still says it.
    render(<DaySummary totals={{ completed: 0, total: 2, minutes: null }} />);

    expect(screen.queryByText("—")).not.toBeInTheDocument();
    expect(screen.queryByText("0m")).not.toBeInTheDocument();
  });

  it("says an empty day has no tasks rather than showing zeros", () => {
    // "0/0 — 0%" seven times over read as a broken board. An empty day is
    // still shown, in words.
    render(<DaySummary totals={{ completed: 0, total: 0, minutes: null }} />);

    expect(screen.getByText("No tasks")).toBeInTheDocument();
    expect(screen.queryByText("0/0")).not.toBeInTheDocument();
    expect(screen.queryByText("0%")).not.toBeInTheDocument();
  });

  it("describes itself in words for screen readers", () => {
    // "2/3" and a bar are shorthand a screen reader cannot make sense of.
    render(<DaySummary totals={{ completed: 2, total: 3, minutes: 105 }} />);

    expect(
      screen.getByLabelText("2 of 3 done, 1h 45m, 67 percent"),
    ).toBeInTheDocument();
  });

  it("says nothing was recorded rather than reading out nothing", () => {
    render(<DaySummary totals={{ completed: 0, total: 1, minutes: null }} />);

    expect(
      screen.getByLabelText("0 of 1 done, no time recorded, 0 percent"),
    ).toBeInTheDocument();
  });

  it("describes an empty day in words for screen readers", () => {
    render(<DaySummary totals={{ completed: 0, total: 0, minutes: null }} />);

    expect(screen.getByLabelText("No tasks")).toBeInTheDocument();
  });

  it("shows the completion as a bar", () => {
    const { container } = render(
      <DaySummary totals={{ completed: 2, total: 3, minutes: 105 }} />,
    );

    expect(fillWidth(container)).toBe("67%");
  });

  it("draws no bar for an empty day", () => {
    // Nothing planned is not everything done, and it is not 0% done either.
    const { container } = render(
      <DaySummary totals={{ completed: 0, total: 0, minutes: null }} />,
    );

    expect(container.querySelector(".day-bar")).toBeNull();
  });

  it("marks a finished day as complete for the stylesheet", () => {
    render(<DaySummary totals={{ completed: 3, total: 3, minutes: 105 }} />);

    expect(screen.getByLabelText(/3 of 3 done/)).toHaveAttribute(
      "data-complete",
      "true",
    );
  });

  it("does not mark an unfinished day as complete", () => {
    render(<DaySummary totals={{ completed: 2, total: 3, minutes: 105 }} />);

    expect(screen.getByLabelText(/2 of 3 done/)).toHaveAttribute(
      "data-complete",
      "false",
    );
  });
});
