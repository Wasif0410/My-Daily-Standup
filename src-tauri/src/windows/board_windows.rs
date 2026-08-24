//! Creating, positioning, and persisting board windows.

use tauri::{AppHandle, LogicalPosition, LogicalSize, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::commands::{AppState, CommandError, ErrorKind};
use crate::storage::{BoardKind, BoardWindow};

/// Opens a board window, restoring its saved geometry.
///
/// Re-focuses the existing window if the board is already open, rather than
/// creating a duplicate.
pub fn open_board(app: &AppHandle, kind: BoardKind) -> Result<(), CommandError> {
    let label = kind.window_label();

    if let Some(existing) = app.get_webview_window(&label) {
        existing.show().map_err(window_error)?;
        existing.set_focus().map_err(window_error)?;
        return Ok(());
    }

    let state = app.state::<AppState>();
    let saved = state.board(kind)?;

    // The board is routed by query parameter, so every board shares one
    // frontend bundle rather than needing a build per window.
    let url = WebviewUrl::App(format!("index.html?board={}", kind.as_str()).into());

    let mut builder = WebviewWindowBuilder::new(app, &label, url)
        .title(kind.title())
        // Frameless and transparent: the board draws its own chrome (§6.1).
        .decorations(false)
        .transparent(true)
        // A sticky note is not an app in the alt-tab sense.
        .skip_taskbar(true)
        .resizable(true)
        .shadow(true)
        .inner_size(f64::from(saved.width), f64::from(saved.height))
        .always_on_top(saved.always_on_top);

    if let (Some(x), Some(y)) = (saved.x, saved.y) {
        builder = builder.position(f64::from(x), f64::from(y));
    } else {
        // Never positioned before: let the OS choose somewhere sensible.
        builder = builder.center();
    }

    let window = builder.build().map_err(window_error)?;

    if saved.locked {
        // A locked board is scenery: clicks pass through to whatever is
        // behind it. PR 15 adds the unlock escape hatch in the tray.
        window
            .set_ignore_cursor_events(true)
            .map_err(window_error)?;
    }

    let mut visible = saved;
    visible.visible = true;
    state.save_board(&visible)?;

    Ok(())
}

/// Hides a board window and records that it is hidden.
///
/// **Hidden, not destroyed.** Closing the webview and building a replacement
/// with the same label leaves a window that is correct in every observable
/// way — right label, right position, right URL, and its bundle runs and
/// reaches IPC — but paints as an opaque white rectangle. The transparent
/// surface does not survive being recreated in the same process, and no amount
/// of frontend error handling helps, because nothing has thrown.
///
/// Hiding sidesteps the whole problem: the window is never torn down, so
/// reopening is a `show()` and the page is already loaded. It is also simply
/// better — reopening is instant and keeps scroll position and any open menu.
///
/// The cost is a hidden window per closed board for the lifetime of the
/// process, which is a few megabytes. Quit tears them all down, and a board
/// hidden at shutdown is never recreated at startup because `restore_boards`
/// skips it.
pub fn close_board(app: &AppHandle, kind: BoardKind) -> Result<(), CommandError> {
    if let Some(window) = app.get_webview_window(&kind.window_label()) {
        window.hide().map_err(window_error)?;
    }

    let state = app.state::<AppState>();
    let mut saved = state.board(kind)?;
    saved.visible = false;
    state.save_board(&saved)?;

    Ok(())
}

/// Records a board's current position and size.
///
/// Called from the frontend, debounced, as the user drags or resizes. Writing
/// on every pixel of movement would hammer the database for no benefit.
pub fn save_geometry(
    app: &AppHandle,
    kind: BoardKind,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> Result<(), CommandError> {
    let state = app.state::<AppState>();
    let mut saved = state.board(kind)?;

    saved.x = Some(x);
    saved.y = Some(y);
    saved.width = width;
    saved.height = height;
    saved.monitor = monitor_name(app, kind);

    state.save_board(&saved)
}

/// Reopens every board that was visible when the app last closed.
///
/// Called once at startup. A board that fails to open must not prevent the
/// others from appearing, so failures are collected rather than propagated.
pub fn restore_boards(app: &AppHandle) -> Vec<(BoardKind, CommandError)> {
    let boards: Vec<BoardWindow> = match app.state::<AppState>().boards() {
        Ok(boards) => boards,
        Err(error) => return vec![(BoardKind::Priority, error)],
    };

    boards
        .into_iter()
        .filter(|board| board.visible)
        .filter_map(|board| open_board(app, board.kind).err().map(|e| (board.kind, e)))
        .collect()
}

/// The monitor a board currently sits on, so it returns to the right screen.
fn monitor_name(app: &AppHandle, kind: BoardKind) -> Option<String> {
    app.get_webview_window(&kind.window_label())?
        .current_monitor()
        .ok()
        .flatten()
        .and_then(|monitor| monitor.name().cloned())
}

fn window_error(error: tauri::Error) -> CommandError {
    CommandError {
        kind: ErrorKind::Internal,
        message: format!("window operation failed: {error}"),
    }
}

/// Applies a logical size and position to an open window.
///
/// Used by the settings paths in PR 15; kept here so all geometry handling
/// lives together.
pub fn apply_geometry(app: &AppHandle, board: &BoardWindow) -> Result<(), CommandError> {
    let Some(window) = app.get_webview_window(&board.kind.window_label()) else {
        return Ok(());
    };

    window
        .set_size(LogicalSize::new(
            f64::from(board.width),
            f64::from(board.height),
        ))
        .map_err(window_error)?;

    if let (Some(x), Some(y)) = (board.x, board.y) {
        window
            .set_position(LogicalPosition::new(f64::from(x), f64::from(y)))
            .map_err(window_error)?;
    }

    window
        .set_always_on_top(board.always_on_top)
        .map_err(window_error)?;
    window
        .set_ignore_cursor_events(board.locked)
        .map_err(window_error)?;

    Ok(())
}
