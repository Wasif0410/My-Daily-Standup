import { render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ErrorBoundary } from "@/components/ErrorBoundary";

function Boom({ message = "something broke" }: { message?: string }): React.ReactNode {
  throw new Error(message);
}

// React logs caught errors to the console; that noise is expected here.
beforeEach(() => {
  vi.spyOn(console, "error").mockImplementation(() => {});
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe("ErrorBoundary", () => {
  it("renders its children when nothing goes wrong", () => {
    render(
      <ErrorBoundary>
        <p>the board</p>
      </ErrorBoundary>,
    );

    expect(screen.getByText("the board")).toBeInTheDocument();
  });

  it("shows something instead of a blank window when a child throws", () => {
    // The failure this exists for. Without a boundary React unmounts the whole
    // tree, and a frameless transparent window renders as an opaque white
    // rectangle with no way to tell what happened.
    render(
      <ErrorBoundary>
        <Boom />
      </ErrorBoundary>,
    );

    expect(screen.getByRole("alert")).toBeInTheDocument();
  });

  it("shows the error message, so the failure can be diagnosed", () => {
    // A board that says "something went wrong" and nothing else is barely
    // better than the blank window it replaced.
    render(
      <ErrorBoundary>
        <Boom message="Cannot read properties of undefined (reading 'metadata')" />
      </ErrorBoundary>,
    );

    expect(screen.getByRole("alert")).toHaveTextContent(
      "Cannot read properties of undefined (reading 'metadata')",
    );
  });

  it("offers a way to recover", () => {
    render(
      <ErrorBoundary>
        <Boom />
      </ErrorBoundary>,
    );

    expect(screen.getByRole("button", { name: /reload/i })).toBeInTheDocument();
  });

  it("names what failed when told", () => {
    // Four board windows look alike; one saying "Priority Tasks could not
    // load" is worth more than four saying the same thing.
    render(
      <ErrorBoundary label="Priority Tasks">
        <Boom />
      </ErrorBoundary>,
    );

    expect(screen.getByRole("alert")).toHaveTextContent("Priority Tasks");
  });

  it("survives a thrown value that is not an Error", () => {
    // A rejected IPC call throws a plain object. Reading `.message` off
    // something that has none must not crash the boundary itself.
    function ThrowObject(): React.ReactNode {
      // Throwing a non-Error is the whole point of this test: a rejected IPC
      // call does exactly this, and the boundary must survive it.
      // eslint-disable-next-line @typescript-eslint/only-throw-error
      throw { kind: "internal" };
    }

    render(
      <ErrorBoundary>
        <ThrowObject />
      </ErrorBoundary>,
    );

    expect(screen.getByRole("alert")).toBeInTheDocument();
  });
});
