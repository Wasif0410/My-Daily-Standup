import { useEffect, useState } from "react";
import { headingKey } from "@/features/boards/grouping";
import { getUiState, setUiState } from "@/lib/ipc";

/**
 * Which groups a board has closed, remembered across restarts.
 *
 * Keyed by the same normalised heading the grouping uses, so a group closed as
 * "Job Search" stays closed when a task files itself under "job search". Two
 * normalisers would eventually disagree and a group would reopen itself.
 *
 * Stored in `ui_state` rather than the settings table: which headings someone
 * has folded away is presentation, and a settings reset must not forget it
 * (the same rule that keeps the Weekly Progress board's open days there).
 *
 * `null` until the saved value has been read, so a default is never painted
 * over a choice that is still loading.
 */
export function useCollapsedGroups(boardKey: string) {
  const key = `${boardKey}.collapsed-groups`;
  const [collapsed, setCollapsed] = useState<Set<string> | null>(null);

  useEffect(() => {
    let ignore = false;

    void (async () => {
      try {
        const saved = await getUiState(key);
        if (ignore) return;
        setCollapsed(new Set(saved === null ? [] : (JSON.parse(saved) as string[])));
      } catch {
        // A board that cannot recall which groups were folded is a far smaller
        // problem than one that does not render, so this opens everything
        // rather than surfacing an error over the tasks.
        if (!ignore) setCollapsed(new Set());
      }
    })();

    return () => {
      ignore = true;
    };
  }, [key]);

  function isCollapsed(label: string) {
    return collapsed?.has(headingKey(label)) ?? false;
  }

  function toggle(label: string, shut: boolean) {
    const next = new Set(collapsed ?? []);
    if (shut) next.add(headingKey(label));
    else next.delete(headingKey(label));

    // Optimistic, like every other board interaction: the group folds now and
    // the write follows.
    setCollapsed(next);
    void setUiState(key, JSON.stringify([...next]));
  }

  return { isCollapsed, toggle };
}
