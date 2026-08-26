/**
 * Application settings — the Priority board's threshold, the week-start day,
 * launch-at-login.
 *
 * Shaped like `taskStore` and `sectionStore`, with one deliberate departure:
 * nothing here is optimistic. Those stores guess because a checkbox that waits
 * on a round-trip feels broken. Settings are not a checkbox on a row; they are
 * a form the user has just deliberately submitted, and a guess here can be
 * wrong in a way a task guess cannot:
 *
 * - Rust validates `priorityThreshold` and rejects an out-of-range value
 *   outright, so the number shown could be one the database refused.
 * - `launchAtLogin` is applied to the OS autostart plugin, which can fail on
 *   its own — the switch would read "on" while the OS knows nothing about it.
 *
 * So the value the backend returns is the only truth, and this store simply
 * waits for it. That also makes the rollback trivial: since nothing was
 * changed locally, a rejection leaves the previous settings untouched and only
 * records the error. `sectionStore.mutate` skips its guess for creates on the
 * same principle — where Rust owns the answer, do not invent one.
 */

import { create } from "zustand";
import { getSettings, toCommandError, updateSettings } from "@/lib/ipc";
import type { CommandError } from "@/types/task";
import type { Settings, SettingsPatch } from "@/types/settings";

interface SettingsState {
  /**
   * `null` until the first successful load — never seeded with defaults.
   *
   * A component that renders a guessed threshold of 5 and then jumps to the
   * user's real 7 has shown them a board that was briefly wrong, and the user
   * cannot tell that from a board that is right. `null` is a third answer —
   * "not known yet" — which lets a caller wait instead of showing a fiction.
   */
  settings: Settings | null;
  loading: boolean;
  error: CommandError | null;

  load: () => Promise<void>;
  update: (patch: SettingsPatch) => Promise<void>;
}

export const useSettingsStore = create<SettingsState>((set) => ({
  settings: null,
  loading: false,
  error: null,

  async load() {
    set({ loading: true });

    try {
      // Wholesale replacement, not a merge. There is one row of settings and
      // this is all of it, so a second load supersedes the first entirely —
      // merging would let a field the backend dropped survive locally.
      set({ settings: await getSettings(), error: null });
    } catch (caught) {
      // `settings` is left exactly as it was, which on a first load means
      // still `null`. Filling it in with defaults here would hand the caller
      // an object indistinguishable from a real one.
      set({ error: toCommandError(caught) });
    } finally {
      set({ loading: false });
    }
  },

  async update(patch) {
    try {
      // Not optimistic — see the note at the top of this file. What comes back
      // is what was stored, which is not always what was asked for.
      set({ settings: await updateSettings(patch), error: null });
    } catch (caught) {
      // Nothing to roll back: no local change was made, so the previous
      // settings are already the correct state to be left in.
      set({ error: toCommandError(caught) });
    }
  },
}));
