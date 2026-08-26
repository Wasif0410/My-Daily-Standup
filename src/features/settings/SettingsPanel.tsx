import type { Settings, SettingsPatch, WeekStart } from "@/types/settings";
import type { CommandError } from "@/types/task";

/** The scale the schema allows. Ten is not a magic number here — `priority` is
 *  `CHECK (priority BETWEEN 0 AND 10)`, and the threshold is compared to it. */
const LEVELS = Array.from({ length: 11 }, (_, level) => level);

const WEEK_STARTS: { value: WeekStart; label: string }[] = [
  { value: "monday", label: "Monday" },
  { value: "sunday", label: "Sunday" },
  { value: "saturday", label: "Saturday" },
];

interface SettingsPanelProps {
  settings: Settings;
  /**
   * Given only the field that changed.
   *
   * A patch rather than the whole object: an omitted field means "leave
   * alone", so a save cannot write back values the user never touched, and
   * cannot clobber a change made elsewhere between load and save.
   */
  onChange: (patch: SettingsPatch) => void;
  error?: CommandError | null | undefined;
}

/**
 * The settings that something actually reads.
 *
 * Deliberately short. §18 of the spec lists dozens of candidates, but a
 * setting nothing consumes is worse than no setting at all — the tray's Pause
 * Reminders spent a release as a live toggle writing a flag no code read, and
 * looked for all the world like a working feature. Each control here changes
 * observable behaviour today.
 *
 * Presentational: it renders what it is given and reports what was changed.
 * Whether a change was accepted is the store's business, which is why the
 * value shown is always the stored one and never a local draft — Rust
 * validates the threshold and can refuse it outright, and launch-at-login can
 * fail at the operating system.
 */
export function SettingsPanel({ settings, onChange, error }: SettingsPanelProps) {
  return (
    <section className="settings-panel" aria-label="Settings">
      {error && (
        <p className="settings-error" role="alert">
          {error.message}
        </p>
      )}

      <div className="settings-field">
        <label htmlFor="setting-priority-threshold">Priority board shows</label>
        <select
          id="setting-priority-threshold"
          value={String(settings.priorityThreshold)}
          onChange={(event) =>
            onChange({ priorityThreshold: Number(event.target.value) })
          }
        >
          {LEVELS.map((level) => (
            <option key={level} value={String(level)}>
              P{level} and above
            </option>
          ))}
        </select>
        {/* The setting that most needs explaining. Below the threshold a task
            is simply not on that board, which reads as the task having been
            lost rather than filtered. */}
        <p className="settings-hint">
          Tasks below this never appear on the Priority board. They are still on Weekly
          Tasks.
        </p>
      </div>

      <div className="settings-field">
        <label htmlFor="setting-week-starts-on">Week starts on</label>
        <select
          id="setting-week-starts-on"
          value={settings.weekStartsOn}
          onChange={(event) =>
            onChange({ weekStartsOn: event.target.value as WeekStart })
          }
        >
          {WEEK_STARTS.map((day) => (
            <option key={day.value} value={day.value}>
              {day.label}
            </option>
          ))}
        </select>
        <p className="settings-hint">
          Changes which seven days the Weekly boards cover.
        </p>
      </div>

      <div className="settings-field settings-field-inline">
        <input
          id="setting-launch-at-login"
          type="checkbox"
          checked={settings.launchAtLogin}
          onChange={(event) => onChange({ launchAtLogin: event.target.checked })}
        />
        <label htmlFor="setting-launch-at-login">Launch at login</label>
      </div>
    </section>
  );
}
