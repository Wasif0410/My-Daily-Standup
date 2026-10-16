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

/// Where a close aimed at a board came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseSource {
    /// The user asked: Alt+F4 or the window menu. Windows routes both through
    /// `WM_SYSCOMMAND` with `SC_CLOSE` before the `WM_CLOSE` arrives.
    User,
    /// A bare `WM_CLOSE` nobody pressed a key for: `taskkill` without `/F`, a
    /// script or utility closing the app's windows, anything tearing the
    /// process down. These reach a board one window at a time and in no
    /// particular order, so honouring them would save some boards as closed
    /// and leave the rest open.
    Other,
}

/// Whether closing a board should record it as closed for the next launch.
///
/// Only a close the user asked for is remembered. Any other close still hides
/// the board, but leaves its saved visibility alone, so a board the user never
/// closed comes back with the rest. Erring this way costs a board reopening
/// that someone closed with a tool; erring the other way loses boards the user
/// had open, with nothing on screen to say why.
///
/// The board's own close button never reaches this: it calls `board_close`
/// directly. Tray Quit and logoff do not either — they end the event loop
/// without asking any window to close.
pub fn remembers_close(source: CloseSource) -> bool {
    source == CloseSource::User
}

/// `WM_SYSCOMMAND`, which Windows sends for window-menu and keyboard commands.
const WM_SYSCOMMAND: u32 = 0x0112;

/// The close command inside `WM_SYSCOMMAND`.
const SC_CLOSE: usize = 0xF060;

/// Whether a window message is the user asking to close the window.
///
/// The low four bits of a `WM_SYSCOMMAND` command are Windows' own and must
/// be masked off before comparing, or a close picked from the window menu by
/// mouse can carry bits that make it look like some other command.
pub fn is_user_close_command(message: u32, wparam: usize) -> bool {
    message == WM_SYSCOMMAND && wparam & 0xFFF0 == SC_CLOSE
}
