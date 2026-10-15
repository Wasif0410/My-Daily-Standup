//! Telling a close the user asked for apart from one something else sent.
//!
//! Tauri reports every close as the same `CloseRequested`, whether the user
//! pressed Alt+F4 or another program posted `WM_CLOSE`. Windows does keep them
//! apart one step earlier: a close the user asks for arrives first as
//! `WM_SYSCOMMAND` / `SC_CLOSE`, and only then as `WM_CLOSE`. A bare `WM_CLOSE`
//! has no such step.
//!
//! So each board window is subclassed to note when `SC_CLOSE` comes through,
//! and the close handler consumes that note. Tauri may deliver
//! `CloseRequested` after `SC_CLOSE` has already returned, which is why this
//! is a note to be taken rather than a flag that is only set while the command
//! is on the stack.

use tauri::{WebviewWindow, Window};

use super::close_policy::CloseSource;

/// Starts watching a board window for closes the user asks for.
///
/// Must run on the thread that owns the window — Windows refuses to subclass
/// across threads — which `open_board` already guarantees.
pub fn watch(window: &WebviewWindow) {
    #[cfg(windows)]
    platform::watch(window);

    #[cfg(not(windows))]
    let _ = window;
}

/// Where the close now arriving at `window` came from, consuming the note.
///
/// Off Windows there is no way to tell, so every close counts as the user's,
/// which is how boards behaved before this distinction existed.
pub fn take_source(window: &Window) -> CloseSource {
    #[cfg(windows)]
    {
        platform::take_source(window)
    }

    #[cfg(not(windows))]
    {
        let _ = window;
        CloseSource::User
    }
}

#[cfg(windows)]
mod platform {
    use std::sync::Mutex;

    use ::windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use ::windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
    use ::windows::Win32::UI::WindowsAndMessaging::WM_NCDESTROY;
    use tauri::{WebviewWindow, Window};

    use crate::windows::close_policy::{is_user_close_command, CloseSource};

    /// Identifies this subclass among any others on the same window.
    const SUBCLASS_ID: usize = 0x5354_4E44;

    /// Windows that have had an `SC_CLOSE` not yet matched by a
    /// `CloseRequested`, by handle. A list rather than one flag because each
    /// board is its own window and their closes are independent; a list
    /// rather than a set because it can be built in a `static`, and it never
    /// holds more than the four boards.
    static USER_CLOSES: Mutex<Vec<isize>> = Mutex::new(Vec::new());

    pub fn watch(window: &WebviewWindow) {
        let Ok(hwnd) = window.hwnd() else {
            return;
        };

        // A failed subclass leaves this board's Alt+F4 hiding it without
        // remembering, which is the safe direction. Not worth failing the open.
        //
        // SAFETY: `hwnd` is a live window owned by this thread, and the
        // procedure is a plain function that outlives it.
        let installed = unsafe { SetWindowSubclass(hwnd, Some(procedure), SUBCLASS_ID, 0) };
        if !installed.as_bool() {
            eprintln!("could not watch {} for closes", window.label());
        }
    }

    pub fn take_source(window: &Window) -> CloseSource {
        let Ok(hwnd) = window.hwnd() else {
            return CloseSource::Other;
        };

        if forget(hwnd) {
            CloseSource::User
        } else {
            CloseSource::Other
        }
    }

    /// Clears the note for `hwnd`, saying whether there was one.
    fn forget(hwnd: HWND) -> bool {
        USER_CLOSES
            .lock()
            .map(|mut closes| {
                let before = closes.len();
                closes.retain(|&noted| noted != hwnd.0 as isize);
                closes.len() != before
            })
            .unwrap_or(false)
    }

    unsafe extern "system" fn procedure(
        hwnd: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
        _id: usize,
        _data: usize,
    ) -> LRESULT {
        // Noted before passing the command on: the default handling of
        // SC_CLOSE is what sends WM_CLOSE, and Tauri may act on that before
        // this call returns.
        if is_user_close_command(message, wparam.0) {
            if let Ok(mut closes) = USER_CLOSES.lock() {
                if !closes.contains(&(hwnd.0 as isize)) {
                    closes.push(hwnd.0 as isize);
                }
            }
        }

        if message == WM_NCDESTROY {
            forget(hwnd);
            // SAFETY: removing this procedure from the window it was installed
            // on, while that window is still valid.
            unsafe {
                let _ = RemoveWindowSubclass(hwnd, Some(procedure), SUBCLASS_ID);
            }
        }

        // SAFETY: forwarding the message unchanged to the next procedure in
        // the chain, from inside a subclass procedure as Windows requires.
        unsafe { DefSubclassProc(hwnd, message, wparam, lparam) }
    }
}
