import { useCallback, useState } from "react";
import { ContextPreview } from "@/features/chat/ContextPreview";
import { contextPreview, toCommandError } from "@/lib/ipc";
import type { ContextPreview as Preview } from "@/types/context";
import type { CommandError } from "@/types/task";

/**
 * The context preview, wired to Rust.
 *
 * Holds its own state rather than reaching for a store, the same shape as
 * MonthlyBoard: it never mutates anything, and its data is a rendered
 * snapshot rather than a collection other components also read.
 *
 * Nothing is built on mount. Assembling the map walks the whole task tree, and
 * a diagnostic panel should not do that work every time the main window opens
 * — it runs when asked.
 */
export function Context() {
  const [preview, setPreview] = useState<Preview | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<CommandError | null>(null);

  const refresh = useCallback(() => {
    setLoading(true);
    void (async () => {
      try {
        setPreview(await contextPreview());
        setError(null);
      } catch (caught) {
        // The previous preview is kept. A failed refresh should not blank the
        // thing you were reading when you pressed the button.
        setError(toCommandError(caught));
      } finally {
        setLoading(false);
      }
    })();
  }, []);

  return (
    <ContextPreview
      preview={preview}
      loading={loading}
      error={error}
      onRefresh={refresh}
    />
  );
}
