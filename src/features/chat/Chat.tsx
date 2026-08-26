import { useEffect } from "react";
import { ChatPanel } from "@/features/chat/ChatPanel";
import { useChatStore } from "@/stores/chatStore";

/**
 * The local model chat, wired to the store.
 *
 * The same thin container as `features/settings/Settings.tsx`, and in the same
 * window for the same reason: a new window label has to be granted permissions
 * in `src-tauri/capabilities/default.json`, where a wrong identifier is dropped
 * silently rather than failing the build.
 *
 * Unlike Settings, this renders immediately rather than waiting on the first
 * query. There is nothing to get wrong by rendering early — a model is not
 * running until one is started, so the panel's opening offer is true whether
 * or not the status has come back yet, and holding the whole surface blank
 * over a cheap status call would only make the app look slower than it is.
 */
export function Chat() {
  const status = useChatStore((state) => state.status);
  const starting = useChatStore((state) => state.starting);
  const sending = useChatStore((state) => state.sending);
  const lastReply = useChatStore((state) => state.lastReply);
  const error = useChatStore((state) => state.error);
  const refresh = useChatStore((state) => state.refresh);
  const start = useChatStore((state) => state.start);
  const send = useChatStore((state) => state.send);
  const stop = useChatStore((state) => state.stop);

  // A model can survive a window reload, so what is running has to be asked
  // rather than assumed from a fresh store.
  useEffect(() => {
    void refresh();
  }, [refresh]);

  return (
    <ChatPanel
      status={status}
      starting={starting}
      sending={sending}
      lastReply={lastReply}
      error={error}
      onStart={() => void start()}
      onStop={() => void stop()}
      onSend={(message) => void send(message)}
    />
  );
}
