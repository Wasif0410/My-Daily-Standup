//! My Daily Standup — application library.
//!
//! Rust owns all state, file access, date arithmetic, and process lifecycle.
//! The frontend renders and collects input; the language model contributes
//! language only. See `docs/spec.md` §3.6.

pub mod commands;
pub mod domain;
pub mod storage;

use tauri::Manager;

use commands::AppState;

/// Builds and runs the Tauri application.
///
/// Kept in the library rather than `main.rs` so integration tests and the
/// binary share one definition.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // The database lives in the OS-designated app data directory, not
            // beside the executable, so it survives reinstalls and respects
            // per-user separation.
            let app_data_dir = app.path().app_data_dir()?;
            let state = AppState::new(&app_data_dir)?;
            app.manage(state);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::tasks::task_create,
            commands::tasks::task_get,
            commands::tasks::task_update,
            commands::tasks::task_delete,
            commands::tasks::task_list_by_horizon,
            commands::tasks::task_list_for_date,
            commands::tasks::task_list_for_period,
            commands::tasks::task_children_of,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
