//! Task commands exposed to the frontend.
//!
//! Deliberately thin. Every one delegates to [`AppState`], which is where the
//! behaviour lives and where the tests point.

use tauri::State;

use super::{AppState, CommandError};
use crate::storage::{NewTask, Task, TaskHorizon, TaskPatch};

#[tauri::command]
pub fn task_create(state: State<'_, AppState>, input: NewTask) -> Result<Task, CommandError> {
    state.create_task(input)
}

#[tauri::command]
pub fn task_get(state: State<'_, AppState>, id: String) -> Result<Option<Task>, CommandError> {
    state.get_task(&id)
}

#[tauri::command]
pub fn task_update(
    state: State<'_, AppState>,
    id: String,
    patch: TaskPatch,
) -> Result<Task, CommandError> {
    state.update_task(&id, patch)
}

#[tauri::command]
pub fn task_delete(state: State<'_, AppState>, id: String) -> Result<(), CommandError> {
    state.delete_task(&id)
}

#[tauri::command]
pub fn task_list_by_horizon(
    state: State<'_, AppState>,
    horizon: TaskHorizon,
) -> Result<Vec<Task>, CommandError> {
    state.list_by_horizon(horizon)
}

#[tauri::command]
pub fn task_list_for_date(
    state: State<'_, AppState>,
    date: String,
) -> Result<Vec<Task>, CommandError> {
    state.list_for_date(&date)
}

#[tauri::command]
pub fn task_list_for_period(
    state: State<'_, AppState>,
    start: String,
    end: String,
) -> Result<Vec<Task>, CommandError> {
    state.list_for_period(&start, &end)
}

/// Tasks for the Priority board. `threshold` is the lowest priority shown.
#[tauri::command]
pub fn task_list_priority(
    state: State<'_, AppState>,
    threshold: i64,
) -> Result<Vec<Task>, CommandError> {
    state.list_by_priority(threshold)
}

/// Records how long a task took. `minutes: null` clears it.
#[tauri::command]
pub fn task_set_time_spent(
    state: State<'_, AppState>,
    id: String,
    minutes: Option<i64>,
) -> Result<Task, CommandError> {
    state.set_time_spent(&id, minutes)
}

/// Moves a task to a new date, counting the move as a deferral when it pushes
/// the date later. The only route that may change `scheduled_date`.
#[tauri::command]
pub fn task_reschedule(
    state: State<'_, AppState>,
    id: String,
    to: String,
) -> Result<Task, CommandError> {
    state.reschedule_task(&id, &to)
}

#[tauri::command]
pub fn task_children_of(
    state: State<'_, AppState>,
    parent_id: String,
) -> Result<Vec<Task>, CommandError> {
    state.children_of(&parent_id)
}
