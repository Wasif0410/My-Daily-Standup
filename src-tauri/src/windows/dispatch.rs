//! Running window work on the main thread.
//!
//! Building a webview needs the event loop to turn: the native frame is
//! created, then the webview is attached and navigated, and each step is
//! driven by the loop. A command that calls `build()` inline is *inside* that
//! loop, so the loop cannot advance and `build()` never returns.
//!
//! The failure is quiet and looks nothing like a deadlock. Windows still
//! creates and shows the native frame, so a correctly sized, correctly
//! positioned, entirely blank rectangle appears on screen. Tauri never
//! registers the window, so `get_webview_window` reports nothing there, and
//! every line after `build()` — including the one recording the board as
//! visible — is never reached.
//!
//! Queueing the work and awaiting the result from an async command keeps the
//! loop free to do the building. `open_board` was always correct; only the way
//! the button reached it was not.

use tauri::AppHandle;

use crate::commands::{CommandError, ErrorKind};

/// Runs `work` on the main thread and waits for what it returns.
///
/// Call this from an **async** command. An async command runs on the async
/// runtime rather than the event loop, so awaiting here parks that task
/// instead of stalling the loop the work depends on.
pub async fn on_main_thread<T, F>(app: AppHandle, work: F) -> Result<T, CommandError>
where
    F: FnOnce(&AppHandle) -> Result<T, CommandError> + Send + 'static,
    T: Send + 'static,
{
    // Capacity one, one message, one receiver: `try_send` cannot find the
    // buffer full, so the main thread never blocks handing the result back.
    let (tx, mut rx) = tauri::async_runtime::channel(1);

    let handle = app.clone();
    app.run_on_main_thread(move || {
        let _ = tx.try_send(work(&handle));
    })
    .map_err(|error| CommandError {
        kind: ErrorKind::Internal,
        message: format!("could not reach the main thread: {error}"),
    })?;

    rx.recv().await.unwrap_or_else(|| {
        Err(CommandError {
            kind: ErrorKind::Internal,
            message: "the main thread dropped the window operation".into(),
        })
    })
}
