//! Which windows hide when closed, rather than being destroyed.
//!
//! Destroying a transparent webview and building a replacement in the same
//! process leaves a dead drawing surface. The window is correct in every
//! observable way — right label, right position, right URL, and its bundle
//! runs and reaches IPC — and it paints blank anyway.
//!
//! PR 21 established that and made the board close button hide instead. But a
//! button is only one route to a close: Alt+F4, the window menu, and the task
//! bar all reach a window without passing through it, and the Quick Add box
//! was still being destroyed outright. The rule therefore lives here, as a
//! pure function over a label, so every route obeys the same policy.

use crate::storage::BoardKind;
use crate::windows::QUICK_ADD_LABEL;

/// The main window's label, fixed in `tauri.conf.json`.
pub const MAIN_LABEL: &str = "main";

/// Whether closing this window should hide it rather than destroy it.
///
/// True for every window the app can reopen: the main window (§26 keeps the
/// lightweight tier alive after the planning view goes), the four boards, and
/// the transparent capture box. Anything added later closes normally until it
/// opts in here.
pub fn hides_on_close(label: &str) -> bool {
    label == MAIN_LABEL || label == QUICK_ADD_LABEL || BoardKind::from_window_label(label).is_some()
}
