import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { PriorityBadge } from "@/features/boards/components/PriorityBadge";

describe("PriorityBadge", () => {
  it("renders the priority as a number, not only a colour", () => {
    // Hue alone fails for colour-blind users and again at low board opacity,
    // so the digit is never optional.
    render(<PriorityBadge priority={8} />);

    expect(screen.getByText("P8")).toBeInTheDocument();
  });

  it("renders a dash when nothing is prioritised", () => {
    render(<PriorityBadge priority={null} />);

    expect(screen.getByText("—")).toBeInTheDocument();
  });

  it("names the priority for screen readers", () => {
    render(<PriorityBadge priority={8} />);

    expect(screen.getByLabelText("Priority 8")).toBeInTheDocument();
  });

  it("names an absent priority as unset rather than as zero", () => {
    render(<PriorityBadge priority={null} />);

    expect(screen.getByLabelText("No priority")).toBeInTheDocument();
  });

  it("grades the tier so colour can reinforce the number", () => {
    const { container, rerender } = render(<PriorityBadge priority={9} />);
    expect(container.querySelector(".priority-badge")).toHaveAttribute(
      "data-tier",
      "high",
    );

    rerender(<PriorityBadge priority={5} />);
    expect(container.querySelector(".priority-badge")).toHaveAttribute(
      "data-tier",
      "medium",
    );

    rerender(<PriorityBadge priority={1} />);
    expect(container.querySelector(".priority-badge")).toHaveAttribute(
      "data-tier",
      "low",
    );

    rerender(<PriorityBadge priority={null} />);
    expect(container.querySelector(".priority-badge")).toHaveAttribute(
      "data-tier",
      "none",
    );
  });
});
