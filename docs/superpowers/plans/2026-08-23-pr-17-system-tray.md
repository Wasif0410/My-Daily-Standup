# PR 17 — System Tray & Quick Add

> **For agentic workers:** Execute task-by-task with `superpowers:executing-plans` or
> `superpowers:subagent-driven-development`. Strict TDD where a test can reach the
> behaviour; tray interaction itself needs a running desktop and is covered by the manual
> matrix in Task 7.

**Goal:** A tray icon carrying §6.8's menu, so the app is one click away with no window
taking space — and a Quick Add window that captures a task without loading anything.

**Architecture:** The menu is described by a pure function returning descriptors, so its
shape, its disabled entries, and its labels are unit-testable without a Tauri runtime —
the same split PR 16 used for window behaviours. A thin builder turns descriptors into
real menu items. Quick Add is a fourth window kind on the existing single bundle, routed
by query parameter like the boards.

**Tech Stack:** Rust · Tauri 2 tray APIs · `tauri-plugin-autostart` · React 19 ·
TypeScript 5.9.3 · Vitest + Testing Library

## Global Constraints

Inherited verbatim from `docs/superpowers/plans/2026-08-20-pr-sequence.md`.

- **Windows-first.** No platform-specific dependency without a `cfg` guard.
- **Local-first.** No feature may hard-depend on a network call.
- **No LLM.** **Opening boards and adding a task must not load a model** (§6.8, and the
  DoD). This PR spawns no sidecar and the manual matrix confirms it.
- **Rust owns state.** The frontend asks; it does not decide.
- **TypeScript pinned at 5.9.3.**
- **Tokens, never literals.**
- **The full local gate before pushing:** `npm run lint`, `npm run format:check`,
  `npx tsc --noEmit`, `npx vitest run`, `cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`, `cargo test`.

---

## Design decisions settled here

| Decision | Why |
|---|---|
| **The menu is a pure list of descriptors, built into Tauri items separately** | The sequence document asks for "a Rust test on menu construction". A menu assembled directly against an `AppHandle` cannot be tested without a runtime; a `Vec<MenuEntry>` can. Same split as PR 16's `apply_to_state`. |
| **Unavailable entries say "(coming soon)" in the label, not in a tooltip** | The requirement is that they "must not silently do nothing". Tauri 2 has no cross-platform tooltip on a menu item, and a disabled item with no explanation reads as a bug. The label is the only place the reason is guaranteed to be seen. |
| **Four AI entries and Settings are all disabled** | Daily Standup, Evening Check-In, Plan My Week, and Monthly Review are Wave 5. Settings is PR 18 — the very next one. The same rule applies to all five: present, visibly unavailable, and honest about why. |
| **"Unlock All Boards" is added, though §6.8 does not list it** | PR 16's spec names this exact entry as the escape hatch. `board_unlock_all` already exists; this is the surface it was written for. |
| **"Start with Windows" is added too** | The sequence document requires `tauri-plugin-autostart` in this PR, and the settings window that would toggle it is PR 18. Without a control here the dependency ships dead. A checkable tray item is the smallest honest home for it. |
| **Closing the main window hides it instead of quitting** | The DoD requires the tray to survive a main-window close, and §26 makes the lightweight tier outlive the planning view. Quit — from the tray — is the only thing that exits. |
| **Quick Add is a fourth window kind on the same bundle** | Boards already route by `?board=`; a `?window=quick-add` parameter costs one branch in `main.tsx` versus a second Vite entry point. |
| **Quick Add closes itself after adding** | It is a capture box, not a workspace. Leaving it open after a task is filed means it is still floating over everything an hour later. |
| **Quick Add creates a *daily* task for today** | Capture means "this, now". Asking for a horizon at capture time is the friction that stops capture happening. It can be moved from the Weekly board afterwards. |
| **Pause Reminders is stored in `ui_state`, not a new table** | Reminders themselves are PR 18, which creates `settings.rs`. Inventing a settings table one PR early would mean migrating it immediately. The key is `reminders.paused`; PR 18 reads it. |

---

## File structure

| File | Responsibility |
|---|---|
| `src-tauri/src/tray/menu.rs` (+`_tests`) | The pure descriptor list |
| `src-tauri/src/tray/mod.rs` | Building the tray, handling clicks |
| `src-tauri/src/windows/quick_add.rs` | The capture window |
| `src-tauri/src/commands/tray.rs` | `autostart_enabled`, `autostart_set`, `quick_add_close` |
| `src-tauri/src/lib.rs` | Plugin, tray setup, main-window close handler |
| `src-tauri/capabilities/default.json` | Autostart permission; the quick-add window label |
| `src/features/quick-add/QuickAddWindow.tsx` (+test) | The input |
| `src/main.tsx` | Routing the new window kind |

---

### Task 1: The menu, as data

**Files:** create `src-tauri/src/tray/menu.rs` and `menu_tests.rs`, `src-tauri/src/tray/mod.rs`.

**Interfaces:**

```rust
/// One row of the tray menu.
pub struct MenuEntry {
    pub id: &'static str,
    pub label: String,
    pub enabled: bool,
    /// Rendered with a tick when true; `None` for a plain item.
    pub checked: Option<bool>,
}

/// Every row, in §6.8's order. `paused` and `autostart` come from stored state.
pub fn menu_entries(paused: bool, autostart: bool) -> Vec<MenuEntry>;

/// Appended to anything Wave 5 or PR 18 has not built yet.
pub const COMING_SOON: &str = " (coming soon)";
```

- [ ] **Step 1: Write the failing tests:**
  - `the_menu_follows_the_order_in_the_spec` — all of §6.8's nine ids, in order
  - `opening_boards_is_available_now` — it needs nothing that does not exist
  - `quick_add_is_available_now` — the DoD requires it to work
  - `quit_is_always_available` — the one entry that must never be disabled, since it is
    how the app is closed once the main window only hides
  - `the_four_ai_entries_are_disabled` — Wave 5
  - `settings_is_disabled_until_pr_18`
  - `every_disabled_entry_says_why` — a disabled row with no explanation reads as a bug
  - `no_available_entry_claims_to_be_coming_soon` — the converse, so the label and the
    state cannot drift
  - `pause_reminders_shows_its_current_state`
  - `unlock_all_boards_is_present` — PR 16's escape hatch needs its promised surface
  - `start_with_windows_reflects_the_stored_setting`
- [ ] **Step 2: Run to verify they fail. Step 3: Implement. Step 4: Verify.**
- [ ] **Step 5: Full Rust gate. Step 6: Commit** — `feat: describe the tray menu as data`

---

### Task 2: Building the tray and handling clicks

**Files:** `src-tauri/src/tray/mod.rs`, `src-tauri/src/lib.rs`.

Builds a `TrayIcon` from the descriptors and routes each id. `open-boards` opens every
board; `unlock-all` calls the existing `windows::unlock_all`; `pause-reminders` flips the
`ui_state` key and rebuilds the menu so the tick reflects it; `quit` exits.

The main window's close request is intercepted: hide instead of exit, so the tray and the
boards survive it (§26, and the DoD).

- [ ] **Step 1: Implement** the builder, the click routing, and the close handler.
- [ ] **Step 2: Full Rust gate** — `cargo build` proves the tray APIs and ids line up.
- [ ] **Step 3: Commit** — `feat: put the app in the system tray`

---

### Task 3: Autostart

**Files:** `src-tauri/Cargo.toml`, `src-tauri/src/lib.rs`, `src-tauri/src/commands/tray.rs`,
`src-tauri/capabilities/default.json`.

Registers `tauri-plugin-autostart`, and exposes `autostart_enabled` / `autostart_set` so
PR 18's settings window can reuse them rather than reaching for the plugin directly.

- [ ] **Step 1: Add the dependency and the plugin**, add the capability permission.
- [ ] **Step 2: Add the commands** and wire the tray's checkable item to them.
- [ ] **Step 3: Full Rust gate** — the build is what proves the permission identifier
      resolves; a wrong one fails there rather than at runtime.
- [ ] **Step 4: Commit** — `feat: offer launch at login`

---

### Task 4: The Quick Add window

**Files:** create `src-tauri/src/windows/quick_add.rs`; export from `windows/mod.rs`;
add the window label to the capability file.

A small, always-on-top, undecorated, centred window at `index.html?window=quick-add`.
Re-focuses rather than duplicating if it already exists, like `open_board`.

- [ ] **Step 1: Implement**, reusing `open_board`'s shape.
- [ ] **Step 2: Full Rust gate. Step 3: Commit** — `feat: add the quick add window`

---

### Task 5: The Quick Add input

**Files:** create `src/features/quick-add/QuickAddWindow.tsx` (+test); modify
`src/main.tsx`; append to `theme.css`.

Reuses the existing `QuickAdd` field. Creates a daily task scheduled for today, announces
the change so every open board reloads, then closes the window.

- [ ] **Step 1: Write the failing tests:**
  - `captures a task on Enter`
  - `creates it as a daily task for today` — asks Rust for the date rather than reading a
    browser clock, as every other date in the app does
  - `announces the change so the boards update` — the DoD's "the boards update"
  - `closes itself after capturing` — a capture box left open is a box floating over
    everything an hour later
  - `stays open and says so when the write fails` — closing on failure would lose what
    the user typed
  - `closes on Escape without creating anything`
  - `does not capture an empty title`
- [ ] **Step 2: Run to verify they fail. Step 3: Implement. Step 4: Verify. Step 5: Style.**
- [ ] **Step 6: Commit** — `feat: capture a task from the tray`

---

### Task 6: Routing

**Files:** `src/main.tsx`.

`?window=quick-add` renders `QuickAddWindow`; `?board=` still renders a board; neither
renders the main app.

- [ ] **Step 1: Implement. Step 2: Full gate. Step 3: Commit** — `feat: route the quick add window`

---

### Task 7: Verify against the real app

- [ ] **Step 1: The full local gate** — all seven commands.
- [ ] **Step 2: Launch and read the dev log.** Tray interaction needs a real desktop:
  - The tray icon appears, and its menu matches §6.8's order.
  - The four AI entries and Settings are greyed out and say "(coming soon)".
  - **Close the main window: the tray icon and the boards are still there.** The DoD.
  - Open Boards from the tray brings all four back.
  - **Quick Add captures a task, the open boards update without a reload, and the window
    closes itself.** The DoD.
  - Pause Reminders ticks and stays ticked after reopening the menu.
  - Unlock All Boards clears a locked board.
  - Start with Windows toggles, and the registry entry appears.
  - **Quit exits everything** — no `my-daily-standup` process left behind. The DoD.
  - **No sidecar process at any point**, especially after Quick Add.
- [ ] **Step 3: Commit, push, open the PR, tick PR 17 in the sequence document.**

---

## Definition of Done

From the sequence document, verbatim:

- Tray survives main window close.
- Quick Add creates a task and the boards update.
- Quit terminates everything cleanly.

## Deliberately out of scope

- **The AI sessions behind four menu entries.** Wave 5. Present and visibly unavailable.
- **The settings window.** PR 18, the next one. Same treatment.
- **Reminders themselves.** PR 18. Pause Reminders stores the flag it will read.
