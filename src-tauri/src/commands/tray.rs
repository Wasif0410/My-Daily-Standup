//! Tray-adjacent commands.
//!
//! Exposed so PR 18's settings window can reuse them rather than reaching for
//! the autostart plugin directly, and so the Quick Add window can dismiss
//! itself after capturing.

use tauri::{AppHandle, Manager};
use tauri_plugin_autostart::ManagerExt;

use super::{CommandError, ErrorKind};
use crate::windows::QUICK_ADD_LABEL;

/// Whether the app is registered to launch at login.
#[tauri::command]
pub fn autostart_enabled(app: AppHandle) -> Result<bool, CommandError> {
    app.autolaunch()
        .is_enabled()
        .map_err(|error| internal(&error.to_string()))
}

#[tauri::command]
pub fn autostart_set(app: AppHandle, enabled: bool) -> Result<(), CommandError> {
    let manager = app.autolaunch();

    let changed = if enabled {
        manager.enable()
    } else {
        manager.disable()
    };
    changed.map_err(|error| internal(&error.to_string()))?;

    crate::tray::refresh(&app)
}

/// Closes the Quick Add window.
///
/// A command rather than the frontend calling `getCurrentWindow().close()` so
/// the capability file does not have to grant a close permission to a window
/// whose only job is one input.
#[tauri::command]
pub fn quick_add_close(app: AppHandle) -> Result<(), CommandError> {
    if let Some(window) = app.get_webview_window(QUICK_ADD_LABEL) {
        window
            .close()
            .map_err(|error| internal(&error.to_string()))?;
    }

    Ok(())
}

fn internal(message: &str) -> CommandError {
    CommandError {
        kind: ErrorKind::Internal,
        message: message.to_string(),
    }
}
