import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

/**
 * Scaffold shell. Its only job right now is to prove the Vite build, the React
 * render, and the Rust IPC bridge all work end to end.
 *
 * The `greet` round-trip is a deliberate canary and is removed in PR 6, when
 * the real typed command layer replaces it.
 */
export function App() {
  const [bridge, setBridge] = useState<string>("checking…");

  useEffect(() => {
    invoke<string>("greet", { name: "Wasif" })
      .then(setBridge)
      .catch((error: unknown) => {
        setBridge(`IPC bridge failed: ${String(error)}`);
      });
  }, []);

  return (
    <main className="shell">
      <h1>My Daily Standup</h1>
      <p className="tagline">
        Long-term goals in, realistic daily actions out — entirely on your own machine.
      </p>
      <p className="bridge" data-testid="bridge-status">
        {bridge}
      </p>
    </main>
  );
}
