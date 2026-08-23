//! The system tray (spec §6.8).
//!
//! Split the same way the window behaviours are: [`menu`] describes the menu as
//! data a test can inspect, and this module turns that description into real
//! Tauri items and routes their clicks. Nothing here decides what the menu
//! contains.

pub mod menu;

pub use menu::{menu_entries, MenuEntry, COMING_SOON};

use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{TrayIcon, TrayIconBuilder};
use tauri::{AppHandle, Manager};
use tauri_plugin_autostart::ManagerExt;

use crate::commands::{AppState, CommandError, ErrorKind};
use crate::storage::BoardKind;
use crate::windows;

/// Where the Pause Reminders flag lives until PR 18 builds a settings table.
///
/// `ui_state` rather than a new table: reminders themselves arrive in the very
/// next PR, and inventing a settings schema one PR early would mean migrating
/// it immediately.
pub const PAUSED_KEY: &str = "reminders.paused";

/// Builds the tray icon and its menu.
pub fn create(app: &AppHandle) -> Result<TrayIcon, CommandError> {
    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or_else(|| internal("the bundle has no window icon to use in the tray"))?;

    let tray = TrayIconBuilder::with_id("main")
        .icon(icon)
        .tooltip("My Daily Standup")
        .menu(&build_menu(app)?)
        .on_menu_event(|app, event| {
            if let Err(error) = handle(app, event.id().as_ref()) {
                eprintln!("tray action {:?} failed: {}", event.id(), error.message);
            }
        })
        .build(app)
        .map_err(tauri_error)?;

    Ok(tray)
}

/// Rebuilds the menu so a checkable row reflects the state it just changed.
pub fn refresh(app: &AppHandle) -> Result<(), CommandError> {
    let Some(tray) = app.tray_by_id("main") else {
        return Ok(());
    };

    tray.set_menu(Some(build_menu(app)?)).map_err(tauri_error)
}

fn build_menu(app: &AppHandle) -> Result<Menu<tauri::Wry>, CommandError> {
    let state = app.state::<AppState>();
    let paused = state.ui_state(PAUSED_KEY)?.as_deref() == Some("true");
    let autostart = app.autolaunch().is_enabled().unwrap_or(false);

    let menu = Menu::new(app).map_err(tauri_error)?;

    for entry in menu_entries(paused, autostart) {
        // A separator before Quit, so the one irreversible action is not
        // adjacent to the toggles above it.
        if entry.id == "quit" {
            menu.append(&PredefinedMenuItem::separator(app).map_err(tauri_error)?)
                .map_err(tauri_error)?;
        }

        match entry.checked {
            Some(checked) => {
                let item = CheckMenuItem::with_id(
                    app,
                    entry.id,
                    &entry.label,
                    entry.enabled,
                    checked,
                    None::<&str>,
                )
                .map_err(tauri_error)?;
                menu.append(&item).map_err(tauri_error)?;
            }
            None => {
                let item =
                    MenuItem::with_id(app, entry.id, &entry.label, entry.enabled, None::<&str>)
                        .map_err(tauri_error)?;
                menu.append(&item).map_err(tauri_error)?;
            }
        }
    }

    Ok(menu)
}

/// Routes a menu click.
///
/// The disabled entries are absent on purpose: an id that cannot be clicked
/// needs no arm, and adding one would make it look as though Wave 5 had landed.
fn handle(app: &AppHandle, id: &str) -> Result<(), CommandError> {
    match id {
        "open-boards" => {
            // Every board, not just the ones last visible: someone reaching for
            // this has lost track of them.
            for &kind in BoardKind::ALL {
                windows::open_board(app, kind)?;
            }
            Ok(())
        }
        "quick-add" => windows::open_quick_add(app),
        "unlock-boards" => windows::unlock_all(app),
        "pause-reminders" => {
            let state = app.state::<AppState>();
            let paused = state.ui_state(PAUSED_KEY)?.as_deref() == Some("true");
            state.set_ui_state(PAUSED_KEY, if paused { "false" } else { "true" })?;
            refresh(app)
        }
        "autostart" => {
            let manager = app.autolaunch();
            let enabled = manager.is_enabled().unwrap_or(false);

            let changed = if enabled {
                manager.disable()
            } else {
                manager.enable()
            };
            changed.map_err(|error| internal(&error.to_string()))?;

            refresh(app)
        }
        "quit" => {
            // The only way out, now that closing the main window merely hides
            // it. Everything the app owns is in this process, so exiting it is
            // the whole of "terminates cleanly" — no sidecar exists to reap.
            app.exit(0);
            Ok(())
        }
        _ => Ok(()),
    }
}

fn tauri_error(error: tauri::Error) -> CommandError {
    internal(&error.to_string())
}

fn internal(message: &str) -> CommandError {
    CommandError {
        kind: ErrorKind::Internal,
        message: message.to_string(),
    }
}

#[cfg(test)]
mod menu_tests;
