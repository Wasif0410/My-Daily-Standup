//! Board section commands.
//!
//! All of these are database work and nothing else — no window is created, so
//! nothing here needs the event loop to turn. That is why they are plain
//! synchronous commands rather than the `async` dance
//! [`crate::commands::boards::board_open`] has to do.

use tauri::State;

use super::{AppState, CommandError};
use crate::storage::{BoardKind, BoardSection, SectionItem};

#[tauri::command]
pub fn section_list(
    state: State<'_, AppState>,
    board_kind: BoardKind,
) -> Result<Vec<BoardSection>, CommandError> {
    state.sections(board_kind)
}

/// Creates an empty section at the end of a board.
///
/// Returns the created section rather than nothing, so the frontend renders
/// the id and position the database actually assigned instead of guessing at
/// them and drifting the first time two boards write at once.
#[tauri::command]
pub fn section_create(
    state: State<'_, AppState>,
    board_kind: BoardKind,
    title: String,
) -> Result<BoardSection, CommandError> {
    state.create_section(board_kind, &title)
}

#[tauri::command]
pub fn section_rename(
    state: State<'_, AppState>,
    id: String,
    title: String,
) -> Result<BoardSection, CommandError> {
    state.rename_section(&id, &title)
}

/// Deletes a section and every item in it.
#[tauri::command]
pub fn section_delete(state: State<'_, AppState>, id: String) -> Result<(), CommandError> {
    state.delete_section(&id)
}

#[tauri::command]
pub fn section_item_add(
    state: State<'_, AppState>,
    section_id: String,
    text: String,
) -> Result<SectionItem, CommandError> {
    state.add_section_item(&section_id, &text)
}

#[tauri::command]
pub fn section_item_update(
    state: State<'_, AppState>,
    id: String,
    text: String,
) -> Result<SectionItem, CommandError> {
    state.update_section_item(&id, &text)
}

#[tauri::command]
pub fn section_item_delete(state: State<'_, AppState>, id: String) -> Result<(), CommandError> {
    state.delete_section_item(&id)
}
