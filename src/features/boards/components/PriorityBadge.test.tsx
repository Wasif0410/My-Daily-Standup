import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { PriorityBadge } from "@/features/boards/components/PriorityBadge";

describe("PriorityBadge", () => {
  it("shows the number, never colour alone", () => {
    render(<PriorityBadge priority={8} />);

    expect(screen.getByText("P8")).toBeInTheDocument();
  });

  it("shows a dash for unprioritised work", () => {
    render(<PriorityBadge priority={null} />);

    expect(screen.getByText("—")).toBeInTheDocument();
  });

  it("stays inert when no change handler is given", () => {
    // The Monthly board renders priorities it must not let anyone edit, so a
    // badge is only a control when someone asks it to be.
    render(<PriorityBadge priority={5} />);

    expect(screen.queryByRole("button")).not.toBeInTheDocument();
  });
});

describe("PriorityBadge as a control", () => {
  it("opens a picker from the badge itself", async () => {
    const user = userEvent.setup();
    render(<PriorityBadge priority={5} onChange={vi.fn()} />);

    await user.click(screen.getByRole("button", { name: "Priority 5" }));

    expect(screen.getByRole("button", { name: "Priority 1" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Priority 10" })).toBeInTheDocument();
  });

  it("offers every step from 1 to 10", async () => {
    const user = userEvent.setup();
    render(<PriorityBadge priority={null} onChange={vi.fn()} />);

    await user.click(screen.getByRole("button", { name: "No priority" }));

    for (let step = 1; step <= 10; step++) {
      expect(
        screen.getByRole("button", { name: `Priority ${step}` }),
      ).toBeInTheDocument();
    }
  });

  it("reports the chosen level and closes", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<PriorityBadge priority={3} onChange={onChange} />);

    await user.click(screen.getByRole("button", { name: "Priority 3" }));
    await user.click(screen.getByRole("button", { name: "Priority 9" }));

    expect(onChange).toHaveBeenCalledWith(9);
    expect(
      screen.queryByRole("button", { name: "Priority 1" }),
    ).not.toBeInTheDocument();
  });

  it("clears back to unprioritised", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<PriorityBadge priority={6} onChange={onChange} />);

    await user.click(screen.getByRole("button", { name: "Priority 6" }));
    await user.click(screen.getByRole("button", { name: "Clear priority" }));

    // Null is not zero: unranked work is a state the app represents honestly,
    // and P0 would claim someone ranked it lowest.
    expect(onChange).toHaveBeenCalledWith(null);
  });

  it("offers no clear when there is nothing to clear", async () => {
    const user = userEvent.setup();
    render(<PriorityBadge priority={null} onChange={vi.fn()} />);

    await user.click(screen.getByRole("button", { name: "No priority" }));

    expect(
      screen.queryByRole("button", { name: "Clear priority" }),
    ).not.toBeInTheDocument();
  });

  it("marks which level is currently set", async () => {
    const user = userEvent.setup();
    render(<PriorityBadge priority={7} onChange={vi.fn()} />);

    await user.click(screen.getByRole("button", { name: "Priority 7" }));

    expect(
      screen.getByRole("button", { name: "Priority 7", current: true }),
    ).toBeInTheDocument();
  });

  it("closes on Escape without changing anything", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<PriorityBadge priority={4} onChange={onChange} />);

    await user.click(screen.getByRole("button", { name: "Priority 4" }));
    await user.keyboard("{Escape}");

    expect(onChange).not.toHaveBeenCalled();
    expect(
      screen.queryByRole("button", { name: "Priority 1" }),
    ).not.toBeInTheDocument();
  });
});
