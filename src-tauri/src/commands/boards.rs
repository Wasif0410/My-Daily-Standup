//! Board window commands.

use tauri::{AppHandle, Manager, State};

use super::{AppState, CommandError};
use crate::storage::{BoardKind, BoardWindow};
use crate::windows;

#[tauri::command]
pub fn board_open(app: AppHandle, kind: BoardKind) -> Result<(), CommandError> {
    windows::open_board(&app, kind)
}

#[tauri::command]
pub fn board_close(app: AppHandle, kind: BoardKind) -> Result<(), CommandError> {
    windows::close_board(&app, kind)
}

/// Records a board's position and size after a drag or resize.
///
/// The frontend debounces this; writing on every pixel of movement would
/// hammer the database to no benefit.
#[tauri::command]
pub fn board_save_geometry(
    app: AppHandle,
    kind: BoardKind,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> Result<(), CommandError> {
    windows::save_geometry(&app, kind, x, y, width, height)
}

/// Records whether a board is collapsed to its title bar.
#[tauri::command]
pub fn board_set_collapsed(
    state: tauri::State<'_, super::AppState>,
    kind: BoardKind,
    collapsed: bool,
) -> Result<(), CommandError> {
    let mut board = state.board(kind)?;
    board.collapsed = collapsed;
    state.save_board(&board)
}

#[tauri::command]
pub fn board_list(
    state: tauri::State<'_, super::AppState>,
) -> Result<Vec<BoardWindow>, CommandError> {
    state.boards()
}

// --- presentation state ------------------------------------------------------

/// Reads one piece of presentation state — which days a board has expanded, and
/// the like. `null` means it was never set, which is not an error.
#[tauri::command]
pub fn ui_state_get(
    state: State<'_, AppState>,
    key: String,
) -> Result<Option<String>, CommandError> {
    state.ui_state(&key)
}

#[tauri::command]
pub fn ui_state_set(
    state: State<'_, AppState>,
    key: String,
    value: String,
) -> Result<(), CommandError> {
    state.set_ui_state(&key, &value)
}

// --- window behaviours (spec §6.7) --------------------------------------------

/// Applies one behaviour change and persists it.
///
/// One command rather than two — persist, then apply — because two are two
/// chances for the window and the database to disagree. A board whose menu says
/// "locked" while it still accepts drags is worse than one that cannot lock.
#[tauri::command]
pub fn board_set_behavior(
    app: AppHandle,
    state: State<'_, AppState>,
    kind: BoardKind,
    behavior: windows::Behavior,
) -> Result<BoardWindow, CommandError> {
    let mut board = state.board(kind)?;
    windows::apply_to_state(&mut board, behavior);
    state.save_board(&board)?;

    // A board that is not open right now has nothing to push the change onto;
    // the saved row is what its next launch will read.
    if let Some(window) = app.get_webview_window(&kind.window_label()) {
        windows::apply_to_window(&window, &board)?;
    }

    Ok(board)
}

/// Unlocks every board. The escape hatch from §6.7's one dangerous state.
///
/// Reachable three ways on purpose: the main window's button, the global
/// shortcut registered at startup, and — from PR 17 — the tray. An escape hatch
/// that lives only inside the thing that can break is not an escape hatch.
#[tauri::command]
pub fn board_unlock_all(app: AppHandle) -> Result<(), CommandError> {
    windows::unlock_all(&app)
}
