import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ProgressBar } from "@/features/boards/components/ProgressBar";

/** The filled portion's width, as the style attribute reports it. */
function fillWidth(container: HTMLElement): string {
  return container.querySelector(".progress-fill")?.getAttribute("style") ?? "";
}

describe("ProgressBar", () => {
  it("renders an empty bar at zero", () => {
    const { container } = render(<ProgressBar fraction={0} label="Job search" />);

    expect(fillWidth(container)).toContain("0%");
    expect(screen.getByRole("progressbar")).toHaveAttribute("aria-valuenow", "0");
  });

  it("renders a full bar at one", () => {
    const { container } = render(<ProgressBar fraction={1} label="Job search" />);

    expect(fillWidth(container)).toContain("100%");
    expect(screen.getByRole("progressbar")).toHaveAttribute("aria-valuenow", "100");
  });

  it("renders a partial bar", () => {
    const { container } = render(<ProgressBar fraction={0.6} label="Job search" />);

    expect(fillWidth(container)).toContain("60%");
  });

  it("never overflows its track", () => {
    // Rust clamps already. This asserts the component does not undo it — an
    // over-target commitment is a good outcome, not a broken layout.
    const { container } = render(<ProgressBar fraction={1.4} label="Job search" />);

    expect(fillWidth(container)).toContain("100%");
    expect(screen.getByRole("progressbar")).toHaveAttribute("aria-valuenow", "100");
  });

  it("never renders a negative width", () => {
    const { container } = render(<ProgressBar fraction={-0.5} label="Job search" />);

    expect(fillWidth(container)).toContain("0%");
  });

  it("rounds the percentage for display", () => {
    // 0.567 rather than 0.575: the latter is 0.5749999… as a double, so it
    // rounds to 57 and would be testing IEEE 754 rather than this component.
    const { container } = render(<ProgressBar fraction={0.567} label="Job search" />);

    expect(fillWidth(container)).toContain("57%");
  });

  it("rounds a third down and two thirds up", () => {
    const { container, rerender } = render(
      <ProgressBar fraction={1 / 3} label="Job search" />,
    );
    expect(fillWidth(container)).toContain("33%");

    rerender(<ProgressBar fraction={2 / 3} label="Job search" />);
    expect(fillWidth(container)).toContain("67%");
  });

  it("shows the percentage as text, not only as a width", () => {
    // A bar at low opacity and a small font size is a smudge. The number is
    // what survives (spec §6.1).
    render(<ProgressBar fraction={0.6} label="Job search" />);

    expect(screen.getByText("60%")).toBeInTheDocument();
  });

  it("is a real progressbar", () => {
    // No run of block characters can carry this, which is why the bar is a
    // styled div.
    render(<ProgressBar fraction={0.6} label="Job search" />);

    const bar = screen.getByRole("progressbar");
    expect(bar).toHaveAttribute("aria-valuemin", "0");
    expect(bar).toHaveAttribute("aria-valuemax", "100");
    expect(bar).toHaveAttribute("aria-valuenow", "60");
  });

  it("names what it is measuring", () => {
    // An unlabelled bar on a board of four says nothing to anyone navigating
    // by landmark.
    render(<ProgressBar fraction={0.6} label="Job search" />);

    expect(screen.getByRole("progressbar", { name: "Job search" })).toBeInTheDocument();
  });

  it("marks a finished commitment so it can be styled apart", () => {
    const { container } = render(<ProgressBar fraction={1} label="Job search" />);

    expect(container.querySelector(".progress-bar")).toHaveAttribute(
      "data-complete",
      "true",
    );
  });
});
