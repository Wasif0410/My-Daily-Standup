//! My Daily Standup — application library.
//!
//! Rust owns all state, file access, date arithmetic, and process lifecycle.
//! The frontend renders and collects input; the language model contributes
//! language only. See `docs/spec.md` §3.6.

pub mod storage;

/// Scaffold command proving the IPC bridge works end to end.
///
/// Removed in PR 6, when the typed command layer (`commands::tasks`) replaces
/// it. It exists purely so PR 2 has something verifiable to assert.
#[tauri::command]
fn greet(name: &str) -> String {
    format!("IPC bridge connected — hello, {name}.")
}

/// Builds and runs the Tauri application.
///
/// Kept in the library rather than `main.rs` so integration tests and the
/// binary share one definition.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greet_includes_the_supplied_name() {
        let result = greet("Wasif");
        assert!(
            result.contains("Wasif"),
            "expected the greeting to contain the name, got: {result}"
        );
    }

    #[test]
    fn greet_reports_the_bridge_is_connected() {
        // The frontend surfaces this string directly, so the wording is part
        // of the contract until PR 6 removes the canary.
        assert!(greet("anyone").starts_with("IPC bridge connected"));
    }
}
