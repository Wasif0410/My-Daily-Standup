import { useState } from "react";
import type { FormEvent } from "react";
import type { ChatReply, ChatStatus } from "@/types/chat";
import type { CommandError } from "@/types/task";

interface ChatPanelProps {
  /** `null` before the status has ever been queried — treated as not running,
   *  because offering to start a model that already runs is recoverable and
   *  claiming one runs when none does is not. */
  status: ChatStatus | null;
  /** The model is loading. Seconds, not milliseconds. */
  starting: boolean;
  /** The model is composing an answer. About a second. */
  sending: boolean;
  lastReply: ChatReply | null;
  error: CommandError | null;
  onStart: () => void;
  onStop: () => void;
  onSend: (message: string) => void;
}

/**
 * A test chat against the local model.
 *
 * The first AI surface in the app, and deliberately the smallest one that
 * proves the chain works end to end: one question, one answer, and the numbers
 * behind it. No history, no streaming, no context from the database.
 *
 * Two rules shape the layout. First, the message box exists only while a model
 * is running — a field that cannot send is worse than no field, because it
 * invites a paragraph of typing and then throws it away, which reads as lost
 * work rather than a stopped model. Second, both waits are stated in words.
 * The load is a few seconds of reading 2.4 GB from disk, and a surface that
 * merely goes quiet through that is indistinguishable from one that has hung
 * (spec §7.5).
 *
 * Presentational: it renders what it is given and reports what was asked for.
 * Whether a model actually started is the store's business, and only the
 * backend's answer settles it.
 */
export function ChatPanel({
  status,
  starting,
  sending,
  lastReply,
  error,
  onStart,
  onStop,
  onSend,
}: ChatPanelProps) {
  // The one piece of local state: an unsent draft belongs to the box it is
  // being typed into, not to a store that other windows subscribe to.
  const [draft, setDraft] = useState("");

  const running = status?.running === true;

  function submit(event: FormEvent) {
    event.preventDefault();

    // Whitespace is not a question. Sending it would cost a second of model
    // time and come back with something arbitrary.
    const message = draft.trim();
    if (!message || sending) return;

    setDraft("");
    onSend(message);
  }

  return (
    <section className="chat-panel" aria-label="Local model chat">
      {error && (
        <p className="chat-error" role="alert">
          {error.message}
        </p>
      )}

      {running ? (
        <>
          <div className="chat-status">
            <p className="chat-hint">
              Answering with {status.model} on port {status.port}.
            </p>
            <button type="button" aria-label="Stop model" onClick={onStop}>
              Stop model
            </button>
          </div>

          <form className="chat-compose" onSubmit={submit}>
            <input
              type="text"
              aria-label="Message"
              value={draft}
              disabled={sending}
              placeholder="Ask the model something"
              onChange={(event) => setDraft(event.target.value)}
            />
            <button type="submit" aria-label="Send" disabled={sending}>
              Send
            </button>
          </form>

          {/* Said in words rather than shown as a spinner alone. A spinner
              does not distinguish a model composing an answer from a window
              that has stopped responding. */}
          {sending && <p className="chat-hint">The model is thinking…</p>}
        </>
      ) : (
        <>
          <button
            type="button"
            aria-label="Start model"
            disabled={starting}
            onClick={onStart}
          >
            Start model
          </button>

          {starting ? (
            <p className="chat-hint">
              Loading the model… This takes a few seconds — several gigabytes are read
              from disk before it can answer anything.
            </p>
          ) : (
            <p className="chat-hint">
              The model is loaded on demand and is not held in memory until you ask for
              it. Starting it takes a few seconds.
            </p>
          )}
        </>
      )}

      {lastReply && (
        <article className="chat-reply" aria-label="Model reply">
          <p className="chat-reply-content">{lastReply.content}</p>
          {/* The numbers are the point of this surface: whether a local model
              is usable at all on this machine is a question about tokens and
              seconds, and there is nowhere else to read that. */}
          <p className="chat-stats" aria-label="Reply statistics">
            {lastReply.promptTokens} prompt tokens · {lastReply.completionTokens}{" "}
            completion tokens · {lastReply.elapsedMs} ms
          </p>
        </article>
      )}
    </section>
  );
}
