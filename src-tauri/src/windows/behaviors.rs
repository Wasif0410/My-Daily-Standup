//! Window behaviours (spec §6.7).
//!
//! Split deliberately in two. [`apply_to_state`] folds a change into a saved
//! `BoardWindow` and is pure, so every rule that must not be representable —
//! the clamps, the mutual exclusion — is unit-testable without a window.
//! [`apply_to_window`] is the thin Tauri call no test can reach, and holds no
//! rules of its own.

use tauri::{AppHandle, Manager, WebviewWindow};

use crate::commands::{AppState, CommandError, ErrorKind};
use crate::storage::{BoardKind, BoardTheme, BoardWindow};

/// The lowest opacity a board may be set to.
///
/// Matches the schema's CHECK. Below this a board is invisible *and*
/// unclickable, so the floor is the difference between a feature and a trap.
const MIN_OPACITY: f64 = 0.2;

/// Font size limits, matching the schema. Below the floor a board is an
/// illegible smudge; above the ceiling a 340px window shows one word and the
/// menu that would undo it is off-screen.
const MIN_FONT_SIZE: f64 = 10.0;
const MAX_FONT_SIZE: f64 = 24.0;

/// One behaviour change. Exactly one variant per §6.7 toggle.
#[derive(Debug, Clone, Copy, serde::Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
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

/// Folds a change into a board's saved state.
///
/// Two rules live here rather than in the UI, because a UI can only *avoid*
/// producing a bad state while this makes one unrepresentable:
///
/// - **Always-on-top and desktop level are mutually exclusive.** Setting either
///   clears the other. Turning one *off* leaves the other alone — a board taken
///   off the top layer belongs in the ordinary stack, not behind everything.
/// - **Opacity and font size are clamped**, including against `NaN` and
///   infinity. A non-finite value fails every comparison, so a naive clamp
///   would pass it through to a schema CHECK that rejects the entire save —
///   losing whatever else the user changed in the same click.
pub fn apply_to_state(board: &mut BoardWindow, change: Behavior) {
    match change {
        Behavior::AlwaysOnTop(on) => {
            board.always_on_top = on;
            if on {
                board.desktop_level = false;
            }
        }
        Behavior::DesktopLevel(on) => {
            board.desktop_level = on;
            if on {
                board.always_on_top = false;
            }
        }
        Behavior::Locked(locked) => board.locked = locked,
        Behavior::Opacity(value) => board.opacity = clamp(value, MIN_OPACITY, 1.0, 1.0),
        Behavior::FontSize(value) => {
            board.font_size = clamp(value, MIN_FONT_SIZE, MAX_FONT_SIZE, 13.0);
        }
        Behavior::Theme(theme) => board.theme = theme,
        Behavior::Compact(compact) => board.compact = compact,
        Behavior::Visible(visible) => board.visible = visible,
    }
}

/// Clamps `value`, falling back to `default` when it is not a number at all.
fn clamp(value: f64, min: f64, max: f64, default: f64) -> f64 {
    if value.is_finite() {
        value.clamp(min, max)
    } else {
        default
    }
}

/// Pushes a board's saved state onto its live window.
///
/// Click-through is derived from `locked` rather than stored beside it: the
/// spec requires click-through only while locked, and two independent flags
/// could produce "click-through but unlocked" — precisely the state with no way
/// out.
///
/// Opacity, font size, theme, and density are not here. They are CSS custom
/// properties the frontend reads from the same saved row, which is why
/// `BoardShell` has set them since PR 10.
pub fn apply_to_window(window: &WebviewWindow, board: &BoardWindow) -> Result<(), CommandError> {
    window
        .set_always_on_top(board.always_on_top)
        .map_err(window_error)?;
    window
        .set_always_on_bottom(board.desktop_level)
        .map_err(window_error)?;
    window
        .set_ignore_cursor_events(board.locked)
        .map_err(window_error)?;

    Ok(())
}

/// Unlocks every board and lets the cursor back in.
///
/// The escape hatch from §6.7's one genuinely dangerous state: every board
/// click-through, with no surface left to click. Reachable from the main
/// window, from `Ctrl+Alt+Shift+U` with nothing visible at all, and from PR
/// 17's tray.
///
/// Deliberately unconditional — it does not check whether anything is locked
/// first. Someone reaching for this is already lost, and "nothing was locked"
/// is not a useful thing to tell them.
pub fn unlock_all(app: &AppHandle) -> Result<(), CommandError> {
    let state = app.state::<AppState>();

    for &kind in BoardKind::ALL {
        let mut board = state.board(kind)?;
        board.locked = false;
        state.save_board(&board)?;

        // A board that is not currently open has nothing to unlock on screen;
        // the saved row above is what matters when it reopens.
        if let Some(window) = app.get_webview_window(&kind.window_label()) {
            window
                .set_ignore_cursor_events(false)
                .map_err(window_error)?;
        }
    }

    Ok(())
}

fn window_error(error: tauri::Error) -> CommandError {
    CommandError {
        kind: ErrorKind::Internal,
        message: error.to_string(),
    }
}
