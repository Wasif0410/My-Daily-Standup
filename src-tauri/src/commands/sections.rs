//! Board section commands.
//!
//! A section is a named task group — the heading a board already derives from
//! `tasks.area` or `tasks.project` — so there are no item commands here. The
//! bullets under a heading are tasks, and they are created, edited and deleted
//! through [`crate::commands::tasks`] like every other task.
//!
//! All of these are database work and nothing else — no window is created, so
//! nothing here needs the event loop to turn. That is why they are plain
//! synchronous commands rather than the `async` dance
//! [`crate::commands::boards::board_open`] has to do.

use tauri::State;

use super::{AppState, CommandError};
use crate::storage::{BoardKind, BoardSection};

#[tauri::command]
pub fn section_list(
    state: State<'_, AppState>,
    board_kind: BoardKind,
) -> Result<Vec<BoardSection>, CommandError> {
    state.sections(board_kind)
}

/// Declares an empty group at the end of a board.
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

/// Renames a group, carrying every task filed under it to the new name.
#[tauri::command]
pub fn section_rename(
    state: State<'_, AppState>,
    id: String,
    title: String,
) -> Result<BoardSection, CommandError> {
    state.rename_section(&id, &title)
}

/// Removes a group's heading and unfiles its tasks into Unsorted.
///
/// Never deletes a task: the heading is what the user asked to be rid of.
#[tauri::command]
pub fn section_delete(state: State<'_, AppState>, id: String) -> Result<(), CommandError> {
    state.delete_section(&id)
}
