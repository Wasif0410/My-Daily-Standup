//! Board window commands.

use tauri::AppHandle;

use super::CommandError;
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
