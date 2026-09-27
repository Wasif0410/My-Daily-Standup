import type { ContextPreview as Preview } from "@/types/context";
import type { CommandError } from "@/types/task";

interface ContextPreviewProps {
  preview: Preview | null;
  loading: boolean;
  error: CommandError | null;
  onRefresh: () => void;
}

/**
 * What the model is about to be shown, before it is shown to it.
 *
 * A diagnostic surface, and deliberately not a pretty one. The tiered context
 * builder makes a series of judgment calls — which sections survive the token
 * budget, which tasks count as urgent, what gets dropped when it overflows —
 * and every one of them is invisible from the answer alone. A bad answer and a
 * bad prompt look identical from the outside; this is how they are told apart.
 *
 * It also serves §9.4's source transparency: the user can see exactly what was
 * sent, which is the only honest basis for trusting a local model with their
 * planning.
 *
 * Presentational and read-only. It holds no state and mutates nothing, the
 * same shape as MonthlyBoard, which reports numbers it must not let anyone
 * edit.
 */
export function ContextPreview({
  preview,
  loading,
  error,
  onRefresh,
}: ContextPreviewProps) {
  return (
    <section className="context-preview" aria-label="Model context">
      <div className="context-preview-header">
        <h2>What the model sees</h2>
        <button
          type="button"
          onClick={onRefresh}
          disabled={loading}
          aria-label="Refresh context"
        >
          {loading ? "Building…" : "Refresh"}
        </button>
      </div>

      {error && (
        <p className="context-error" role="alert">
          {error.message}
        </p>
      )}

      {loading && !preview && (
        <p className="context-hint">Building the map from your boards…</p>
      )}

      {preview && (
        <>
          {/* The budgets are the point of the tiering, so the figures are not
              buried behind a toggle. */}
          <dl className="context-tokens">
            <div>
              <dt>Map</dt>
              <dd>{preview.mapTokens}</dd>
            </div>
            <div>
              <dt>Ranked context</dt>
              <dd>{preview.contextTokens}</dd>
            </div>
            <div>
              <dt>Total</dt>
              <dd>{preview.totalTokens}</dd>
            </div>
          </dl>

          {/* Preformatted because the indentation carries the meaning: a
              section's milestones sit beneath it, and collapsing whitespace
              would destroy the structure the model is being shown. */}
          <pre className="context-map">{preview.map}</pre>
        </>
      )}
    </section>
  );
}
