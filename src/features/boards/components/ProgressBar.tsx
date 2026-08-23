interface ProgressBarProps {
  /** `0..1`, already clamped by Rust. Re-clamped here only as a guard. */
  fraction: number;
  /** What is being measured — the commitment's name. */
  label: string;
}

/**
 * How far along a commitment is (spec §6.5).
 *
 * A styled div rather than a run of block characters. Block characters are
 * pinned to the font's metrics and go ragged at the small sizes and low
 * opacities §6.1 calls for, and no string of `█` can carry `role="progressbar"`.
 *
 * The percentage is rendered as text as well as width. A bar at 40% opacity on
 * a bright desktop is a smudge; the number is what survives.
 *
 * Computes nothing. The fraction arrives already worked out (spec §3.6) — the
 * clamp below is a guard against a bad value, not a calculation.
 */
export function ProgressBar({ fraction, label }: ProgressBarProps) {
  const percent = Math.round(Math.min(Math.max(fraction, 0), 1) * 100);

  return (
    <div className="progress-bar" data-complete={percent >= 100}>
      <div
        className="progress-track"
        role="progressbar"
        aria-label={label}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={percent}
      >
        <div className="progress-fill" style={{ width: `${percent}%` }} />
      </div>
      <span className="progress-percent">{percent}%</span>
    </div>
  );
}
