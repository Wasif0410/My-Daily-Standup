import { useState } from "react";
import { DURATION_PRESETS, formatMinutes, parseDuration } from "@/lib/duration";

interface DurationFieldProps {
  minutes: number | null;
  /** `null` clears the value back to unrecorded. */
  onChange: (minutes: number | null) => void;
}

/**
 * How long a task took, and the one gesture that changes it.
 *
 * Presets cover the common cases in a single click; free text covers the rest.
 * Both matter — a field that only accepted typing would stop being used, and
 * presets alone cannot record "1h 05m".
 *
 * Unparseable input is rejected rather than coerced. Recording a number the
 * user never typed is worse than recording nothing, because nothing is a state
 * the app already represents honestly.
 */
export function DurationField({ minutes, onChange }: DurationFieldProps) {
  const [open, setOpen] = useState(false);
  const [draft, setDraft] = useState("");
  const [rejected, setRejected] = useState(false);

  function close() {
    setOpen(false);
    setDraft("");
    setRejected(false);
  }

  function commit(value: number | null) {
    onChange(value);
    close();
  }

  function submitDraft() {
    const parsed = parseDuration(draft);

    if (parsed === null) {
      // Stay open with the text intact: the user is usually one character from
      // a valid entry, and clearing it would make them start over.
      setRejected(true);
      return;
    }

    commit(parsed);
  }

  const label =
    minutes === null
      ? "Time spent: not recorded"
      : `Time spent: ${formatMinutes(minutes)}`;

  return (
    <span className="duration-field">
      <button
        type="button"
        className="duration-value"
        aria-label={label}
        aria-expanded={open}
        onClick={() => setOpen((current) => !current)}
      >
        {formatMinutes(minutes)}
      </button>

      {open && (
        <div className="duration-popover">
          <div className="duration-presets">
            {DURATION_PRESETS.map((preset) => (
              <button
                key={preset.minutes}
                type="button"
                className="duration-preset"
                onClick={() => commit(preset.minutes)}
              >
                {preset.label}
              </button>
            ))}
          </div>

          <input
            className="duration-input"
            aria-label="Duration"
            placeholder="45m, 2h, 90"
            autoFocus
            value={draft}
            onChange={(event) => {
              setDraft(event.target.value);
              setRejected(false);
            }}
            onKeyDown={(event) => {
              if (event.key === "Enter") {
                event.preventDefault();
                submitDraft();
              } else if (event.key === "Escape") {
                event.preventDefault();
                close();
              }
            }}
          />

          {rejected && (
            <p className="duration-error" role="alert">
              Not a duration — try 45m, 2h, or 90.
            </p>
          )}

          {minutes !== null && (
            <button
              type="button"
              className="duration-clear"
              onClick={() => commit(null)}
            >
              Clear
            </button>
          )}
        </div>
      )}
    </span>
  );
}
