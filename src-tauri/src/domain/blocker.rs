//! Blockers and comments.
//!
//! Both exist for spec §6.6, and both are here rather than in the repository
//! because each carries a rule the storage layer has no business knowing: a
//! blocker drags the task's status with it, and a comment accumulates instead
//! of overwriting.

use chrono::{Local, NaiveDate};
use rusqlite::Connection;

use crate::storage::{StorageError, Task, TaskPatch, TaskRepo, TaskStatus};

/// Sets or clears a task's blocker, keeping its status in step.
///
/// Setting one marks the task `blocked`; clearing one returns it to `planned`.
/// The two are changed together because a `blocked` status that does not track
/// the blocker text is decoration — and once they can drift apart, every view
/// has to decide which of the two to believe.
///
/// A completed task keeps its status when a blocker is cleared. Resolving a
/// blocker must not un-finish work.
///
/// Blank text counts as clearing. An empty string reaching the database would
/// leave a task "blocked" by nothing, which no view could explain.
///
/// Never touches `rollover_count` — only [`super::reschedule`] may (spec §10.3).
pub fn set_blocker(
    repo: &TaskRepo,
    conn: &Connection,
    id: &str,
    blocker: Option<&str>,
) -> Result<Task, StorageError> {
    let task = repo
        .get(conn, id)?
        .ok_or_else(|| StorageError::TaskNotFound { id: id.to_string() })?;

    let text = blocker.map(str::trim).filter(|t| !t.is_empty());

    let status = match (text, task.status) {
        (Some(_), _) => Some(TaskStatus::Blocked),
        // Only lift the block. A cancelled or completed task keeps its state.
        (None, TaskStatus::Blocked) => Some(TaskStatus::Planned),
        (None, _) => None,
    };

    repo.update(
        conn,
        id,
        TaskPatch {
            blocker: Some(text.map(str::to_string)),
            status,
            ..Default::default()
        },
    )
}

/// Appends a dated comment to a task's notes.
///
/// Appends rather than replaces: a field that overwrites the previous comment
/// is not a comment. Each line carries its date, because a note is read a week
/// after it was written and an undated one cannot be placed.
///
/// Blank text is a no-op rather than an empty dated line.
pub fn add_comment(
    repo: &TaskRepo,
    conn: &Connection,
    id: &str,
    comment: &str,
    today: NaiveDate,
) -> Result<Task, StorageError> {
    let text = comment.trim();

    let task = repo
        .get(conn, id)?
        .ok_or_else(|| StorageError::TaskNotFound { id: id.to_string() })?;

    if text.is_empty() {
        return Ok(task);
    }

    let line = format!("{}: {}", today.format("%Y-%m-%d"), text);
    let notes = match task
        .notes
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
    {
        Some(existing) => format!("{existing}\n{line}"),
        None => line,
    };

    repo.update(
        conn,
        id,
        TaskPatch {
            notes: Some(Some(notes)),
            ..Default::default()
        },
    )
}

/// Today's date, for callers that do not supply one.
///
/// `Local`, matching [`super::current_week`]: a comment stamped with tomorrow's
/// date because the user is west of Greenwich would be wrong in the one place
/// the date matters.
pub fn today() -> NaiveDate {
    Local::now().date_naive()
}
