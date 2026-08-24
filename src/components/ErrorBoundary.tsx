import { Component } from "react";
import type { ErrorInfo, ReactNode } from "react";

interface ErrorBoundaryProps {
  children: ReactNode;
  /** What this boundary is protecting, e.g. "Priority Tasks". Four board
   *  windows look alike; one that names itself is worth four that do not. */
  label?: string | undefined;
}

interface ErrorBoundaryState {
  message: string | null;
}

/** Anything can be thrown in JavaScript, and a rejected IPC call throws a plain
 *  object. Reading `.message` off something that has none must not crash the
 *  boundary itself. */
function describe(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === "string") return error;
  return "An unexpected error occurred.";
}

/**
 * Stops one broken component from blanking a whole window.
 *
 * Without this, React unmounts the entire tree when anything throws — and a
 * frameless transparent board renders as an opaque white rectangle with no
 * title, no controls, and nothing to say what happened. That is the worst
 * failure mode this app has: a sticky note that is simply gone.
 *
 * A class component because error boundaries have no hook equivalent; React
 * still offers no other way to catch a render-time throw.
 */
export class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  override state: ErrorBoundaryState = { message: null };

  static getDerivedStateFromError(error: unknown): ErrorBoundaryState {
    return { message: describe(error) };
  }

  override componentDidCatch(error: unknown, info: ErrorInfo) {
    // To the webview console, where the dev log and devtools can both reach
    // it. The rendered message below is for the user; this is for whoever is
    // debugging.
    console.error("Board failed to render:", error, info.componentStack);
  }

  override render() {
    const { children, label } = this.props;
    const { message } = this.state;

    if (message === null) return children;

    return (
      <div className="board-failure" role="alert">
        <p className="board-failure-title">
          {label ? `${label} could not load.` : "This window could not load."}
        </p>
        <p className="board-failure-message">{message}</p>
        <button
          type="button"
          className="board-failure-retry"
          // A reload rather than clearing the error state: whatever threw did
          // so during setup, and re-rendering the same broken tree would only
          // throw again.
          onClick={() => window.location.reload()}
        >
          Reload
        </button>
      </div>
    );
  }
}
