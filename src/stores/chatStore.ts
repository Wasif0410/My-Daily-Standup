/**
 * The local model: whether one is loaded, and the last thing it said.
 *
 * Shaped like `settingsStore`, and non-optimistic for a stronger version of
 * the same reason. Settings can be refused by validation; a model can simply
 * fail to exist — the file is missing, the port is taken, the health check
 * never answers. Whether a model is running is a fact about a process on this
 * machine, and the only place that fact lives is the reply. So nothing here
 * guesses: a rejected start leaves `status` exactly as it was and records the
 * error, and the panel goes on offering to start a model, which is the truth.
 *
 * There is no conversation history. One question, one reply, and the next
 * question knows nothing of the last. That is the whole scope of this first AI
 * surface, and `lastReply` rather than `replies` is the type saying so.
 */

import { create } from "zustand";
import { chatSend, chatStart, chatStatus, chatStop, toCommandError } from "@/lib/ipc";
import type { ChatReply, ChatStatus } from "@/types/chat";
import type { CommandError } from "@/types/task";

interface ChatState {
  /**
   * `null` until the first successful query — never seeded with a stopped
   * status.
   *
   * "Not asked yet" and "asked, and nothing is running" look identical to a
   * component that only has a boolean, but they are different claims and only
   * one of them has been verified.
   */
  status: ChatStatus | null;

  /**
   * Set while the model loads — seconds, not milliseconds.
   *
   * Separate from {@link sending} on purpose. Loading 2.4 GB and answering a
   * question are waits of very different lengths, and the panel says different
   * things about each: one promises a few seconds, the other says the model is
   * thinking. Collapsing them into one boolean would mean showing the wrong
   * promise half the time.
   */
  starting: boolean;

  /** Set while the model is composing an answer — about a second. */
  sending: boolean;

  /** The last answer, or `null` before one has arrived. */
  lastReply: ChatReply | null;

  error: CommandError | null;

  refresh: () => Promise<void>;
  start: () => Promise<void>;
  send: (message: string) => Promise<void>;
  stop: () => Promise<void>;
}

export const useChatStore = create<ChatState>((set, get) => ({
  status: null,
  starting: false,
  sending: false,
  lastReply: null,
  error: null,

  async refresh() {
    try {
      set({ status: await chatStatus(), error: null });
    } catch (caught) {
      // `status` is left as it was, which on a first query means still `null`.
      // Writing a stopped status here would claim a fact the failed call never
      // established.
      set({ error: toCommandError(caught) });
    }
  },

  async start() {
    set({ starting: true });

    try {
      // Not optimistic. The process can die on the health check after loading
      // perfectly well, so only what comes back is worth believing.
      set({ status: await chatStart(), error: null });
    } catch (caught) {
      // Nothing to roll back: no local change was made, so the previous status
      // is already the correct state to be left in.
      set({ error: toCommandError(caught) });
    } finally {
      set({ starting: false });
    }
  },

  async send(message) {
    // Guarded here rather than left to the backend. The command would fail
    // anyway, but with an error about a missing port, which tells the user
    // nothing they can act on. This one names the fix.
    if (!get().status?.running) {
      set({
        error: {
          kind: "invalid-input",
          message: "Start the model first — nothing is loaded to answer.",
        },
      });
      return;
    }

    set({ sending: true });

    try {
      set({ lastReply: await chatSend(message), error: null });
    } catch (caught) {
      // `lastReply` survives. The user is reading that answer, and destroying
      // it because a follow-up failed loses the only copy — there is no
      // history to scroll back to. The failure is reported on its own.
      set({ error: toCommandError(caught) });
    } finally {
      set({ sending: false });
    }
  },

  async stop() {
    try {
      const status = await chatStop();
      // The reply goes with the model. With no history behind it, an answer
      // left on screen after its model is gone is text with nothing to
      // re-ask, re-run or compare it against.
      set({ status, lastReply: null, error: null });
    } catch (caught) {
      // A refused stop means the model is very likely still running, so the
      // previous status and reply remain the accurate picture.
      set({ error: toCommandError(caught) });
    }
  },
}));
