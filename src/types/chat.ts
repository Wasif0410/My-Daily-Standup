/**
 * The local model bridge, mirroring the `chat_*` commands in
 * `src-tauri/src/commands/`.
 *
 * Rust serialises with `rename_all = "camelCase"`, so these names must match
 * the Rust field names exactly. A mismatch does not fail to compile — the
 * frontend just silently reads `undefined`.
 *
 * Deliberately the smallest thing that proves the chain works: one message in,
 * one reply out. No conversation history, no streaming, no context from the
 * database — each of those is a later change, and folding them in now would
 * mean debugging four things at once the first time a reply comes back wrong.
 */

/**
 * Whether a model is loaded, and which one.
 *
 * Every field except `running` is nullable because nothing is loaded until it
 * is asked for: with no process there is no model name and no port. The three
 * travel together so a caller never has to correlate two answers that could
 * disagree.
 */
export interface ChatStatus {
  running: boolean;
  /** The loaded model's identifier, or `null` when nothing is loaded. */
  model: string | null;
  /** The port the local server bound to, or `null` when nothing is loaded. */
  port: number | null;
}

/**
 * One answer, with what it cost.
 *
 * The counts and the timing are not diagnostics bolted on — they are the point
 * of this first surface. Whether a local model is usable at all is a question
 * about tokens per second on this machine, and there is nowhere else to read
 * that.
 */
export interface ChatReply {
  content: string;
  promptTokens: number;
  completionTokens: number;
  /** Wall-clock time for the round-trip, in milliseconds. */
  elapsedMs: number;
}
