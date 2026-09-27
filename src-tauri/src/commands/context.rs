//! The context builder, across the IPC boundary.
//!
//! One command, and it exists to be *looked at*. Everything the tiers do
//! happens before a model is ever loaded, so the only way to know whether the
//! map reads like a map — and whether the budgets are being spent on the right
//! things — is to render it and show it to the person whose tasks it describes.
//!
//! Deliberately separate from [`super::inference`]: nothing here starts a
//! process, touches a port, or needs a model on disk. Previewing the context
//! on a machine that has never downloaded a model must work.

use serde::Serialize;
use tauri::State;

use super::{AppState, CommandError};

/// What the preview shows.
///
/// The token counts are the point of the panel, not decoration: a map that
/// reads beautifully and costs 1,800 tokens against a 1,500 budget is the bug
/// this command exists to make visible.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextPreview {
    /// Tier 1 on its own, as the model receives it.
    pub map: String,
    /// The whole prompt: template, map, and ranked context.
    pub prompt: String,
    pub map_tokens: u32,
    pub context_tokens: u32,
    /// The rendered prompt end to end — the tiers *plus* the template's own
    /// prose, which is charged against the context window like everything
    /// else. Deliberately not `map_tokens + context_tokens`, because that sum
    /// would under-report exactly the number a user is checking.
    pub total_tokens: u32,
}

/// Renders today's standup context without loading a model.
#[tauri::command]
pub fn context_preview(state: State<'_, AppState>) -> Result<ContextPreview, CommandError> {
    state.context_preview()
}
