# PR 16 — Window Behaviors

> **For agentic workers:** Execute task-by-task with `superpowers:executing-plans` or
> `superpowers:subagent-driven-development`. Strict TDD where a test can reach the
> behaviour; the OS-level ones are covered by persistence tests plus the manual matrix in
> Task 8.

**Goal:** Every behaviour in spec §6.7 — always on top, desktop level, locked, see-through,
resizable text, pinned to a monitor, themed, compact — toggling, persisting, and restoring
after a reboot. Plus the escape hatch that stops a locked board from being lost forever.

**Architecture:** Four new columns on `board_windows`, one Rust module that applies a
saved state to a live window, and one command that writes a change and applies it in the
same call so the two can never drift. The frontend gains a board menu and drives font
size, theme, and density entirely through CSS custom properties the shell already sets —
which is why PR 10 put them there.

**Tech Stack:** Rust · rusqlite · Tauri 2 (`set_always_on_top`, `set_always_on_bottom`,
`set_ignore_cursor_events`) · `tauri-plugin-global-shortcut` · React 19 · TypeScript
5.9.3 · Vitest + Testing Library

## Global Constraints

Inherited verbatim from `docs/superpowers/plans/2026-08-20-pr-sequence.md`.

- **Windows-first.** No platform-specific dependency without a `cfg` guard.
- **Local-first.** No feature may hard-depend on a network call.
- **No LLM.** Window behaviours must not start an inference process.
- **Rust owns state.** The frontend asks; it does not decide.
- **TypeScript pinned at 5.9.3.**
- **Tokens, never literals.** Font size, theme, and density are all token changes.
- **Hover-revealed controls stay keyboard-reachable.**
- **The full local gate before pushing:** `npm run lint`, `npm run format:check`,
  `npx tsc --noEmit`, `npx vitest run`, `cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`, `cargo test`.

---

## Design decisions settled here

| Decision | Why |
|---|---|
| **One `board_set_behavior` command that writes *and* applies** | Two commands — one to persist, one to apply — is two chances for the window and the database to disagree. A board that says "locked" in its menu but still accepts drags is worse than one that cannot lock at all. |
| **A global shortcut is the escape hatch, not just a button** | The spec's hazard is real: all boards click-through, main window closed, and there is no surface left to click. The tray does not arrive until PR 17, and an escape hatch that lives inside the thing that can break is not an escape hatch. `Ctrl+Alt+Shift+U` works with nothing visible. **This adds `tauri-plugin-global-shortcut`** — a first-party plugin, and the only new dependency. |
| **A button in the main window too** | Belt and braces, and discoverable — nobody finds a shortcut they were never told about. PR 17's tray entry calls the same command. |
| **Click-through is derived from `locked`, never stored separately** | The spec says click-through must only activate while locked. Two independent flags could produce "click-through but unlocked", which is exactly the unrecoverable state. One flag makes it unrepresentable. |
| **Locking also removes the drag region** | `set_ignore_cursor_events` stops clicks reaching the webview, but a board that is locked while the pointer is already over it must not still be draggable. Both halves, or "locked" is a half-truth. |
| **Font size, theme, and density are CSS custom properties** | `BoardShell` already sets `--board-font-size` and `--board-opacity` per window, precisely so PR 16 could drive them without touching a component. Honouring that is the whole point of the token layer. |
| **Opacity keeps its 0.2 floor, enforced in SQL** | Already in the schema. A fully transparent board is invisible *and* unclickable, so the floor is not a preference. |
| **Font size is clamped 10–24px in SQL too** | An 800px font is the same class of accident: the board becomes one illegible character with no way back to the menu. |
| **`desktop_level` is a separate flag from `always_on_top`, and they are mutually exclusive** | Both set at once is meaningless. Setting either clears the other, in Rust, so no UI can produce the contradiction. |
| **Monitor is recorded but reopening does not force the window onto it** | Saved coordinates already put a board back on the right screen when that screen exists. Forcing a window onto a monitor that has been unplugged is how a board ends up off-screen — the failure this feature is meant to prevent. The column stays for PR 17's "pin" UI to build on. |

---

## File structure

| File | Responsibility |
|---|---|
| `src-tauri/migrations/005_board_appearance.sql` | `font_size`, `theme`, `compact`, `desktop_level` |
| `src-tauri/src/storage/board.rs` (+`_tests`) | The new fields |
| `src-tauri/src/windows/behaviors.rs` (+`_tests`) | Applying a saved state to a live window; `unlock_all` |
| `src-tauri/src/commands/boards.rs` | `board_set_behavior`, `board_unlock_all` |
| `src-tauri/src/lib.rs` | Plugin, shortcut registration, handlers |
| `src-tauri/capabilities/default.json` | Global-shortcut permission |
| `src/types/board.ts` | `BoardTheme`, `BoardBehavior` |
| `src/lib/ipc.ts` | `setBoardBehavior`, `unlockAllBoards` |
| `src/features/boards/components/BoardMenu.tsx` (+test) | The controls |
| `src/components/BoardShell.tsx` (+test) | Font size, theme, density |
| `src/features/boards/BoardRoot.tsx` | Wiring, and the locked drag region |
| `src/app/App.tsx` | The unlock button |
| `src/styles/tokens.css` | A light palette; compact density |

---

### Task 1: The new columns

**Files:** create `src-tauri/migrations/005_board_appearance.sql`; modify
`src-tauri/src/storage/board.rs`, `board_tests.rs`, `migrations.rs`.

```sql
ALTER TABLE board_windows ADD COLUMN font_size REAL NOT NULL DEFAULT 13.0
    CHECK (font_size BETWEEN 10.0 AND 24.0);
ALTER TABLE board_windows ADD COLUMN theme TEXT NOT NULL DEFAULT 'dark'
    CHECK (theme IN ('dark', 'light'));
ALTER TABLE board_windows ADD COLUMN compact INTEGER NOT NULL DEFAULT 0
    CHECK (compact IN (0, 1));
ALTER TABLE board_windows ADD COLUMN desktop_level INTEGER NOT NULL DEFAULT 0
    CHECK (desktop_level IN (0, 1));
```

`BoardWindow` gains `font_size: f64`, `theme: BoardTheme`, `compact: bool`,
`desktop_level: bool`. `BoardTheme` is a two-variant enum with the same
`as_str`/`parse`/`ToSql`/`FromSql` treatment `BoardKind` already has.

- [x] **Step 1: Write the failing tests:**
  - `migration_005_brings_the_schema_to_version_five`
  - `a_new_board_defaults_to_dark_at_thirteen_pixels`
  - `appearance_round_trips`
  - `an_existing_row_gains_the_defaults` — the migration must not strand boards saved
    before it; `ALTER TABLE … DEFAULT` is what makes that true
  - `a_font_size_below_the_floor_is_rejected` — 8px; the board becomes one illegible
    character with no way back to the menu
  - `a_font_size_above_the_ceiling_is_rejected` — 40px
  - `an_unknown_theme_is_rejected`
- [x] **Step 2: Run to verify they fail. Step 3: Implement. Step 4: Verify.**
- [x] **Step 5: Full Rust gate. Step 6: Commit** — `feat: store how a board looks`

---

### Task 2: Applying behaviours to a live window

**Files:** create `src-tauri/src/windows/behaviors.rs` and `behaviors_tests.rs`; export
from `windows/mod.rs`.

**Interfaces:**

```rust
/// One behaviour change. Exactly one variant per §6.7 toggle.
pub enum Behavior {
    AlwaysOnTop(bool),
    DesktopLevel(bool),
    Locked(bool),
    Opacity(f64),
    FontSize(f64),
    Theme(BoardTheme),
    Compact(bool),
    Visible(bool),
}

/// Folds a change into a saved state, enforcing the rules that must not be
/// representable — mutual exclusion, and the clamps.
pub fn apply_to_state(board: &mut BoardWindow, change: Behavior);

/// Pushes a saved state onto a live window.
pub fn apply_to_window(window: &WebviewWindow, board: &BoardWindow) -> Result<(), CommandError>;

/// Clears `locked` on every board and lets the cursor back in. The escape hatch.
pub fn unlock_all(app: &AppHandle) -> Result<(), CommandError>;
```

`apply_to_state` is pure over `BoardWindow`, so every rule below is unit-testable without
a window. `apply_to_window` is the thin Tauri call that no test can reach.

- [x] **Step 1: Write the failing tests** (all against `apply_to_state`):
  - `always_on_top_clears_desktop_level` — both at once is meaningless
  - `desktop_level_clears_always_on_top`
  - `turning_always_on_top_off_leaves_desktop_level_alone` — clearing one must not set
    the other
  - `opacity_is_clamped_to_the_floor` — 0.0 becomes 0.2; an invisible, unclickable board
    is not a preference
  - `opacity_is_clamped_to_one`
  - `font_size_is_clamped_at_both_ends`
  - `locking_does_not_touch_anything_else`
  - `unlocking_is_the_only_way_click_through_turns_off` — there is no separate flag to
    get out of step with
- [x] **Step 2: Run to verify they fail. Step 3: Implement both functions. Step 4: Verify.**
- [x] **Step 5: Full Rust gate. Step 6: Commit** — `feat: apply window behaviours`

---

### Task 3: The commands and the escape hatch

**Files:** `src-tauri/src/commands/boards.rs`, `src-tauri/Cargo.toml`,
`src-tauri/src/lib.rs`, `src-tauri/capabilities/default.json`.

**Interfaces:**
- `board_set_behavior { kind, behavior } -> BoardWindow` — writes and applies in one call.
- `board_unlock_all -> ()`

Registers `tauri-plugin-global-shortcut` and binds **`Ctrl+Alt+Shift+U`** to `unlock_all`
at startup. A failure to register must not stop the app launching — a missing shortcut is
a degraded escape hatch, not a broken app — so it is logged and swallowed.

- [x] **Step 1: Add the dependency and the plugin**, register the shortcut, add
      `global-shortcut:default` to the capability file.
- [x] **Step 2: Add both commands** and register the handlers.
- [x] **Step 3: Full Rust gate** — `cargo build` proves the permission identifier resolves.
- [x] **Step 4: Commit** — `feat: add the unlock escape hatch`

---

### Task 4: Frontend types and wrappers

**Files:** `src/types/board.ts`, `src/lib/ipc.ts`.

```typescript
export type BoardTheme = "dark" | "light";
export type BoardBehavior =
  | { kind: "alwaysOnTop"; value: boolean }
  | { kind: "desktopLevel"; value: boolean }
  | { kind: "locked"; value: boolean }
  | { kind: "opacity"; value: number }
  | { kind: "fontSize"; value: number }
  | { kind: "theme"; value: BoardTheme }
  | { kind: "compact"; value: boolean }
  | { kind: "visible"; value: boolean };

export function setBoardBehavior(kind: BoardKind, behavior: BoardBehavior): Promise<BoardWindow>;
export function unlockAllBoards(): Promise<void>;
```

`BoardWindow` gains `fontSize`, `theme`, `compact`, `desktopLevel`.

- [x] **Step 1: Add them. Step 2: Typecheck. Step 3: Commit** — `feat: type window behaviours`

---

### Task 5: `BoardMenu`

**Files:** create `src/features/boards/components/BoardMenu.tsx` (+test); append to
`theme.css`.

**Interfaces:** `<BoardMenu board={BoardWindow} onChange={(b: BoardBehavior) => void} onClose={() => void} />`

A `role="menu"` opened from a header button, carrying every §6.7 toggle.

- [x] **Step 1: Write the failing tests:**
  - `offers every behaviour the spec lists`
  - `reflects the current state` — a board already on top shows the toggle *on*, not a
    dead switch
  - `reports a toggle to its owner` — one test per behaviour
  - `steps the font size rather than offering a free number` — a spinner in a 340px board
    is a mis-click waiting to happen
  - `will not step the font size past its limits`
  - `steps opacity in tenths and stops at the floor`
  - `warns that locking makes the board click-through, and names the way out` — the user
    must not discover this by losing a board
  - `closes on Escape` and `closes on an outside click`
- [x] **Step 2: Run to verify they fail. Step 3: Implement. Step 4: Verify. Step 5: Style.**
- [x] **Step 6: Commit** — `feat: add the board menu`

---

### Task 6: Shell, tokens, and the locked drag region

**Files:** `src/components/BoardShell.tsx` (+test), `src/components/BoardHeader.tsx`,
`src/features/boards/BoardRoot.tsx` (+test), `src/styles/tokens.css`, `theme.css`.

`BoardShell` gains `fontSize`, `theme`, `compact` and sets `--board-font-size`,
`data-theme`, `data-compact`. `BoardHeader` drops `data-tauri-drag-region` when locked.

`tokens.css` gains a light palette under `[data-theme="light"]` and a tighter spacing
scale under `[data-compact="true"]`.

- [x] **Step 1: Write the failing tests:**
  - shell: `applies a saved font size as a custom property`;
    `marks its theme so tokens can switch`; `marks compact density`;
    `defaults to dark and comfortable`
  - header: `is draggable when unlocked`; `is not draggable when locked` — the second
    half of "locked", without which the board still moves
  - root: `opens the board menu from the header`; `sends a behaviour change to Rust`
- [x] **Step 2: Run to verify they fail. Step 3: Implement. Step 4: Verify.**
- [x] **Step 5: Commit** — `feat: drive board appearance from saved state`

---

### Task 7: The visible escape hatch

**Files:** `src/app/App.tsx` (+test).

A plainly labelled "Unlock all boards" control in the main window, explaining what it is
for. Nobody finds a keyboard shortcut they were never told about.

- [x] **Step 1: Write the failing tests** — `offers a way to unlock every board`;
      `explains the shortcut, so it can be used when no window is open`;
      `calls the unlock command`.
- [x] **Step 2: Run to verify they fail. Step 3: Implement. Step 4: Verify.**
- [x] **Step 5: Commit** — `feat: surface the unlock escape hatch`

---

### Task 8: Verify against the real app

- [x] **Step 1: The full local gate** — all seven commands.
- [ ] **Step 2: Launch and read the dev log.** Most of §6.7 is OS behaviour no test can
      reach, so this is the matrix the DoD actually rests on.

      **Done so far:** the app launched and built clean; the dev log carries no error,
      permission denial, or panic, and **no "could not register the unlock shortcut"
      line** — the registration succeeded, which is the one part of the escape hatch that
      could have silently failed. `global-shortcut:default` resolved into the generated
      `capabilities.json`. No sidecar process. Migration 005 applied to a database that
      already held four board rows: every one gained the defaults rather than failing to
      load. The schema rejects all four unrecoverable states — 4px and 99px font, zero
      opacity, an unknown theme — verified against the real file.

      **Still needs a human at the keyboard.** None of the following can be reached
      without a real compositor. For one board:
  - Always-on-top: on, covers another window; off, stops covering.
  - Desktop level: sinks below other windows; turning always-on-top on clears it.
  - Opacity: steps down to 0.2 and no further; the board stays visible.
  - Font size: steps between 10 and 24; the row layout survives both ends.
  - Theme: light and dark both legible; no hard-coded colour survives the switch.
  - Compact: rows tighten; nothing overlaps.
  - **Lock: the board stops dragging *and* clicks pass through to what is behind it.**
  - **Unlock via the main-window button, then lock again and unlock via
    `Ctrl+Alt+Shift+U` with the main window closed.** This is the DoD's escape hatch and
    the reason the plugin is here.
  - Close the app with a board locked, on top, light, and compact; reopen; every one
    survives.
  - No sidecar process.
- [x] **Step 3: Commit, push, open the PR, tick PR 16 in the sequence document.**

---

## Definition of Done

From the sequence document, verbatim:

- Every behavior in §6.7 toggles, persists, and restores after reboot.
- The unlock escape hatch works.

## Deliberately out of scope

- **The tray's "Unlock All Boards" entry.** PR 17; it calls `board_unlock_all`, which
  exists after this PR.
- **Forcing a window onto its saved monitor.** The column is recorded and saved
  coordinates already restore the right screen. Forcing a window onto a monitor that has
  been unplugged causes the very problem this feature prevents.
- **A settings window.** PR 16 puts these controls on each board, where they apply. Global
  defaults are a later concern.
