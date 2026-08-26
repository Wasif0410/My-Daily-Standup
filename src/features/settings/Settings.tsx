import { useEffect } from "react";
import { SettingsPanel } from "@/features/settings/SettingsPanel";
import { useSettingsStore } from "@/stores/settingsStore";

/**
 * Settings, wired to the store.
 *
 * Lives in the main window rather than a window of its own. A new window needs
 * its label added to `src-tauri/capabilities/default.json`, and a wrong
 * identifier there is dropped silently rather than failing the build — this
 * project has already lost an afternoon to exactly that. The main window was a
 * development shell with nothing real to do; now it has a job.
 *
 * Renders nothing until the settings have loaded. There are no placeholder
 * defaults on purpose: showing a threshold of 5 and then snapping to the
 * user's actual 7 means having shown them something untrue.
 */
export function Settings() {
  const settings = useSettingsStore((state) => state.settings);
  const error = useSettingsStore((state) => state.error);
  const load = useSettingsStore((state) => state.load);
  const update = useSettingsStore((state) => state.update);

  useEffect(() => {
    void load();
  }, [load]);

  if (!settings) {
    return error ? (
      <p className="settings-error" role="alert">
        {error.message}
      </p>
    ) : null;
  }

  return (
    <SettingsPanel
      settings={settings}
      error={error}
      onChange={(patch) => void update(patch)}
    />
  );
}
