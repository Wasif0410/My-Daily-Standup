//! The Quick Add capture window (spec §6.8).
//!
//! A separate always-on-top window rather than a field on the main one, because
//! the whole point is capturing a task without the planning view being open.

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::commands::{CommandError, ErrorKind};

/// The window Tauri knows this by. Also the label the capability file allows.
pub const QUICK_ADD_LABEL: &str = "quick-add";

/// Opens the capture window, focusing it if it already exists.
///
/// Small, undecorated, centred, and always on top — it appears over whatever
/// the user was doing, takes one line, and gets out of the way. Duplicating it
/// would leave two boxes fighting for the same keystrokes, so an existing one
/// is refocused instead, exactly as [`super::open_board`] does.
pub fn open_quick_add(app: &AppHandle) -> Result<(), CommandError> {
    if let Some(existing) = app.get_webview_window(QUICK_ADD_LABEL) {
        existing.show().map_err(window_error)?;
        existing.set_focus().map_err(window_error)?;
        return Ok(());
    }

    let url = WebviewUrl::App("index.html?window=quick-add".into());

    WebviewWindowBuilder::new(app, QUICK_ADD_LABEL, url)
        .title("Quick Add Task")
        .decorations(false)
        .transparent(true)
        .skip_taskbar(true)
        .resizable(false)
        .always_on_top(true)
        .inner_size(420.0, 64.0)
        .center()
        .build()
        .map_err(window_error)?;

    Ok(())
}

fn window_error(error: tauri::Error) -> CommandError {
    CommandError {
        kind: ErrorKind::Internal,
        message: error.to_string(),
    }
}
