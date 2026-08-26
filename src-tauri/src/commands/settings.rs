//! Settings commands (spec §18).
//!
//! Deliberately thin, like the rest of the boundary: the ordering rules for
//! `launch_at_login` live on [`AppState::update_settings`], where a test can
//! reach them without a Tauri runtime.

use tauri::{AppHandle, State};

use super::{AppState, CommandError};
use crate::storage::{Settings, SettingsPatch};

#[tauri::command]
pub fn settings_get(state: State<'_, AppState>) -> Result<Settings, CommandError> {
    state.settings()
}

/// Applies a settings patch. An omitted field is left alone.
///
/// Takes an `AppHandle` because `launch_at_login` is not merely stored: the
/// same autostart path the tray's "Start with Windows" uses is invoked here, so
/// the setting is real rather than a flag nothing consumes.
#[tauri::command]
pub fn settings_update(
    app: AppHandle,
    state: State<'_, AppState>,
    patch: SettingsPatch,
) -> Result<Settings, CommandError> {
    state.update_settings(patch, |enabled| {
        super::tray::autostart_set(app.clone(), enabled)
    })
}
