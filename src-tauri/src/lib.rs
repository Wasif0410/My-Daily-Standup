//! My Daily Standup — application library.
//!
//! Rust owns all state, file access, date arithmetic, and process lifecycle.
//! The frontend renders and collects input; the language model contributes
//! language only. See `docs/spec.md` §3.6.

pub mod commands;
pub mod domain;
pub mod storage;
pub mod tray;
pub mod windows;

use tauri::Manager;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use commands::AppState;

/// The escape hatch from a fully locked, click-through desktop (spec §6.7).
///
/// Deliberately awkward to hit by accident and impossible to hit by accident
/// while typing. It exists for the one state where no window can be clicked at
/// all, so it cannot depend on any window being reachable.
fn unlock_shortcut() -> Shortcut {
    Shortcut::new(
        Some(
            Modifiers::CONTROL
                .union(Modifiers::ALT)
                .union(Modifiers::SHIFT),
        ),
        Code::KeyU,
    )
}

/// Builds and runs the Tauri application.
///
/// Kept in the library rather than `main.rs` so integration tests and the
/// binary share one definition.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Launch at login. The tray's "Start with Windows" toggles it; PR 18's
        // settings window will reuse the same commands.
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    // Fire on press only. Acting on both edges would run the
                    // unlock twice for one keystroke.
                    if shortcut != &unlock_shortcut() || event.state() != ShortcutState::Pressed {
                        return;
                    }

                    if let Err(error) = windows::unlock_all(app) {
                        eprintln!("could not unlock the boards: {}", error.message);
                    }
                })
                .build(),
        )
        .setup(|app| {
            // The database lives in the OS-designated app data directory, not
            // beside the executable, so it survives reinstalls and respects
            // per-user separation.
            let app_data_dir = app.path().app_data_dir()?;
            let state = AppState::new(&app_data_dir)?;
            app.manage(state);

            // A missing shortcut is a degraded escape hatch, not a broken
            // app — another program may already own this combination — so the
            // failure is reported and swallowed rather than stopping launch.
            if let Err(error) = app.global_shortcut().register(unlock_shortcut()) {
                eprintln!("could not register the unlock shortcut: {error}");
            }

            // The tray is the app's real home (§6.8, §26): it outlives the
            // main window and is the only way to quit once closing that window
            // merely hides it. A failure here would leave the user with no way
            // out, so it is fatal rather than logged.
            tray::create(app.handle()).map_err(|error| error.message)?;

            // Reopen whatever was on the desktop when the app last closed
            // (spec §23). A board that fails to open must not stop the others.
            for (kind, error) in windows::restore_boards(app.handle()) {
                eprintln!("could not restore the {kind:?} board: {}", error.message);
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            // Every window the app can reopen hides rather than closing. The
            // main window does so because §26's lightweight tier outlives the
            // planning view and only tray Quit ends the app; the boards and the
            // capture box do so because destroying a transparent webview and
            // rebuilding it leaves a dead drawing surface that paints blank.
            //
            // This is the whole policy, not just the close buttons: Alt+F4 and
            // the window menu arrive here too.
            let tauri::WindowEvent::CloseRequested { api, .. } = event else {
                return;
            };

            let label = window.label().to_string();
            if !windows::hides_on_close(&label) {
                return;
            }

            api.prevent_close();

            // A board must also record that it is hidden, or it reopens at the
            // next launch. close_board hides and writes in one step.
            if let Some(kind) = storage::BoardKind::from_window_label(&label) {
                if let Err(error) = windows::close_board(window.app_handle(), kind) {
                    eprintln!("could not close the {kind:?} board: {}", error.message);
                }
                return;
            }

            let _ = window.hide();
        })
        .invoke_handler(tauri::generate_handler![
            commands::tasks::task_create,
            commands::tasks::task_get,
            commands::tasks::task_update,
            commands::tasks::task_delete,
            commands::tasks::task_list_by_horizon,
            commands::tasks::task_list_for_date,
            commands::tasks::task_list_for_period,
            commands::tasks::task_list_priority,
            commands::tasks::task_list_scheduled_between,
            commands::tasks::task_set_time_spent,
            commands::tasks::task_reschedule,
            commands::tasks::task_children_of,
            commands::tasks::task_move_to_period,
            commands::tasks::task_archive,
            commands::tasks::task_set_blocker,
            commands::tasks::task_add_comment,
            commands::tasks::week_current,
            commands::tasks::month_current,
            commands::tasks::task_monthly_progress,
            commands::boards::board_open,
            commands::boards::board_set_behavior,
            commands::boards::board_unlock_all,
            commands::tray::autostart_enabled,
            commands::tray::autostart_set,
            commands::tray::quick_add_close,
            commands::boards::ui_state_get,
            commands::boards::ui_state_set,
            commands::boards::board_close,
            commands::boards::board_save_geometry,
            commands::boards::board_set_collapsed,
            commands::boards::board_list,
            commands::sections::section_list,
            commands::sections::section_create,
            commands::sections::section_rename,
            commands::sections::section_delete,
            commands::settings::settings_get,
            commands::settings::settings_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
