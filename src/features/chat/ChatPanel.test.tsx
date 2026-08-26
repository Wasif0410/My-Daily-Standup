import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ChatPanel } from "@/features/chat/ChatPanel";
import type { ChatReply, ChatStatus } from "@/types/chat";

function stopped(): ChatStatus {
  return { running: false, model: null, port: null };
}

function running(): ChatStatus {
  return { running: true, model: "qwen2.5-3b-instruct", port: 8080 };
}

function reply(overrides: Partial<ChatReply> = {}): ChatReply {
  return {
    content: "Two things today.",
    promptTokens: 12,
    completionTokens: 34,
    elapsedMs: 980,
    ...overrides,
  };
}

/** The panel is presentational: every prop has to be handed to it. */
function renderPanel(overrides: Partial<Parameters<typeof ChatPanel>[0]> = {}) {
  const props = {
    status: stopped(),
    starting: false,
    sending: false,
    lastReply: null,
    error: null,
    onStart: vi.fn(),
    onStop: vi.fn(),
    onSend: vi.fn(),
    ...overrides,
  };
  render(<ChatPanel {...props} />);
  return props;
}

describe("when no model is running", () => {
  it("offers to start one", () => {
    renderPanel();

    expect(screen.getByRole("button", { name: /start model/i })).toBeEnabled();
  });

  it("says the model is not resident until it is asked for", () => {
    // The load is on demand by design (nothing sits in RAM), and without this
    // the several-second first wait reads as the app being broken.
    renderPanel();

    expect(screen.getByText(/on demand/i)).toBeInTheDocument();
  });

  it("offers no message box at all", () => {
    // A field that cannot send is worse than no field: it invites typing and
    // then fails, which reads as lost work rather than a stopped model.
    renderPanel();

    expect(screen.queryByRole("textbox", { name: /message/i })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /^send$/i })).not.toBeInTheDocument();
  });

  it("starts the model when asked", async () => {
    const user = userEvent.setup();
    const props = renderPanel();

    await user.click(screen.getByRole("button", { name: /start model/i }));

    expect(props.onStart).toHaveBeenCalled();
  });

  it("offers to start even before the status has ever been queried", () => {
    renderPanel({ status: null });

    expect(screen.getByRole("button", { name: /start model/i })).toBeInTheDocument();
  });
});

describe("while the model is loading", () => {
  it("says so plainly", () => {
    renderPanel({ starting: true });

    expect(screen.getByText(/loading the model/i)).toBeInTheDocument();
  });

  it("sets the expectation that this takes seconds", () => {
    // Spec §7.5's loading sequence in miniature: the wait is a few seconds for
    // 2.4 GB plus a health check, and a spinner alone does not say that.
    renderPanel({ starting: true });

    expect(screen.getByText(/few seconds/i)).toBeInTheDocument();
  });

  it("disables the start button so it cannot be asked twice", () => {
    renderPanel({ starting: true });

    expect(screen.getByRole("button", { name: /start model/i })).toBeDisabled();
  });
});

describe("when a model is running", () => {
  it("offers a message box, a send button and a stop button", () => {
    renderPanel({ status: running() });

    expect(screen.getByRole("textbox", { name: /message/i })).toBeEnabled();
    expect(screen.getByRole("button", { name: /^send$/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /stop model/i })).toBeInTheDocument();
  });

  it("names the model that is answering", () => {
    // This is a diagnostic surface. Which model replied is half the answer.
    renderPanel({ status: running() });

    expect(screen.getByText(/qwen2\.5-3b-instruct/)).toBeInTheDocument();
  });

  it("sends what was typed", async () => {
    const user = userEvent.setup();
    const props = renderPanel({ status: running() });

    await user.type(screen.getByRole("textbox", { name: /message/i }), "what is up");
    await user.click(screen.getByRole("button", { name: /^send$/i }));

    expect(props.onSend).toHaveBeenCalledWith("what is up");
  });

  it("sends on Enter", async () => {
    const user = userEvent.setup();
    const props = renderPanel({ status: running() });

    await user.type(
      screen.getByRole("textbox", { name: /message/i }),
      "what is up{Enter}",
    );

    expect(props.onSend).toHaveBeenCalledWith("what is up");
  });

  it("refuses a blank message", async () => {
    const user = userEvent.setup();
    const props = renderPanel({ status: running() });

    await user.type(screen.getByRole("textbox", { name: /message/i }), "   {Enter}");
    await user.click(screen.getByRole("button", { name: /^send$/i }));

    expect(props.onSend).not.toHaveBeenCalled();
  });

  it("clears the box after sending, so the next question starts empty", async () => {
    const user = userEvent.setup();
    renderPanel({ status: running() });

    const box = screen.getByRole("textbox", { name: /message/i });
    await user.type(box, "what is up{Enter}");

    expect(box).toHaveValue("");
  });

  it("stops the model when asked", async () => {
    const user = userEvent.setup();
    const props = renderPanel({ status: running() });

    await user.click(screen.getByRole("button", { name: /stop model/i }));

    expect(props.onStop).toHaveBeenCalled();
  });
});

describe("while the model is thinking", () => {
  it("says it is thinking", () => {
    renderPanel({ status: running(), sending: true });

    expect(screen.getByText(/thinking/i)).toBeInTheDocument();
  });

  it("disables the box and the send button", () => {
    // One question at a time: there is no conversation history yet, so a
    // second send would overwrite the answer to the first.
    renderPanel({ status: running(), sending: true });

    expect(screen.getByRole("textbox", { name: /message/i })).toBeDisabled();
    expect(screen.getByRole("button", { name: /^send$/i })).toBeDisabled();
  });
});

describe("the reply", () => {
  it("shows what the model said", () => {
    renderPanel({ status: running(), lastReply: reply({ content: "Two things." }) });

    expect(screen.getByText("Two things.")).toBeInTheDocument();
  });

  it("shows the token counts and the time it took", () => {
    // The numbers are the point. This surface exists to prove the chain works
    // and to show what it costs.
    renderPanel({
      status: running(),
      lastReply: reply({ promptTokens: 12, completionTokens: 34, elapsedMs: 980 }),
    });

    const stats = screen.getByLabelText(/reply statistics/i);
    expect(stats).toHaveTextContent(/12/);
    expect(stats).toHaveTextContent(/34/);
    expect(stats).toHaveTextContent(/980\s*ms/i);
  });

  it("shows nothing where a reply would go before one arrives", () => {
    renderPanel({ status: running() });

    expect(screen.queryByLabelText(/reply statistics/i)).not.toBeInTheDocument();
  });
});

describe("errors", () => {
  it("announces a failure rather than failing silently", () => {
    renderPanel({
      status: stopped(),
      error: { kind: "internal", message: "model file not found" },
    });

    expect(screen.getByRole("alert")).toHaveTextContent(/model file not found/);
  });

  it("keeps the last reply on screen alongside the error", () => {
    // A failed follow-up must not destroy the answer the user was reading.
    renderPanel({
      status: running(),
      lastReply: reply({ content: "Two things." }),
      error: { kind: "internal", message: "connection reset" },
    });

    expect(screen.getByText("Two things.")).toBeInTheDocument();
    expect(screen.getByRole("alert")).toHaveTextContent(/connection reset/);
  });
});
