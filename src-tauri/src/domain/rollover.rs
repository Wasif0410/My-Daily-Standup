//! Rescheduling and period statistics.
//!
//! The rollover counter exists for one purpose: spec §10.3's reflection
//! prompt, *"you have moved this five times — is it blocked, too large, or no
//! longer important?"* That question is only worth asking if the number counts
//! deferrals and nothing else, so the boundaries below are the substance of
//! this module, not an implementation detail.

use chrono::NaiveDate;
use rusqlite::Connection;

use crate::storage::{StorageError, Task, TaskPatch, TaskRepo, TaskStatus};

/// Reschedules a task, counting the move as a deferral only when it pushes the
/// date later.
///
/// Three cases deliberately do **not** increment:
///
/// - **Pulling work earlier.** Moving a task forward is planning, not
///   avoidance; counting it would penalise the behaviour the app wants.
/// - **Moving to the same day.** Nothing changed.
/// - **Scheduling a previously unscheduled task.** Giving a backlog item its
///   first date is scheduling, not deferral.
///
/// The counter records history and never decreases: pulling a task forward
/// after deferring it does not erase the deferral.
///
/// This is the only path that touches `rollover_count`. Editing a title,
/// adding a blocker, or completing a task all leave it alone.
pub fn reschedule(
    repo: &TaskRepo,
    conn: &Connection,
    id: &str,
    to: &str,
) -> Result<Task, StorageError> {
    // Reject anything that is not a real date before touching the database, so
    // a bad value cannot leave a task half-updated.
    let new_date = parse_date(to)?;

    let task = repo
        .get(conn, id)?
        .ok_or_else(|| StorageError::TaskNotFound { id: id.to_string() })?;

    let is_deferral = match task.scheduled_date.as_deref() {
        Some(existing) => match parse_date(existing) {
            Ok(current) => new_date > current,
            // A malformed date already in the database should not block the
            // user from fixing it; treat the repair as a plain reschedule.
            Err(_) => false,
        },
        None => false,
    };

    let patch = TaskPatch {
        scheduled_date: Some(Some(to.to_string())),
        rollover_count: is_deferral.then(|| task.rollover_count + 1),
        ..Default::default()
    };

    repo.update(conn, id, patch)
}

fn parse_date(value: &str) -> Result<NaiveDate, StorageError> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|_| StorageError::InvalidDate {
        value: value.to_string(),
    })
}

/// Planned-versus-completed figures for a set of tasks.
///
/// Feeds the weekly and monthly reviews (spec §13.2, §13.3). Computed here so
/// the model never does the arithmetic it would be asked to summarise.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PeriodStats {
    /// Tasks that were actually expected to happen — cancelled work excluded.
    pub planned: usize,
    pub completed: usize,
    /// Still outstanding, and deferred at least once.
    pub carried: usize,
    pub cancelled: usize,
    /// `completed / planned`, or zero when nothing was planned.
    pub completion_rate: f64,
}

/// Summarises a period.
///
/// Cancelled tasks are excluded from `planned`: deliberately dropping
/// something is a decision, not a failure to finish it, and counting it as
/// incomplete would make an honest week look like a bad one.
pub fn period_stats(tasks: &[Task]) -> PeriodStats {
    let cancelled = tasks
        .iter()
        .filter(|t| t.status == TaskStatus::Cancelled)
        .count();
    let planned = tasks.len() - cancelled;
    let completed = tasks
        .iter()
        .filter(|t| t.status == TaskStatus::Completed)
        .count();
    let carried = tasks
        .iter()
        .filter(|t| {
            t.rollover_count > 0
                && !matches!(t.status, TaskStatus::Completed | TaskStatus::Cancelled)
        })
        .count();

    PeriodStats {
        planned,
        completed,
        carried,
        cancelled,
        completion_rate: if planned == 0 {
            0.0
        } else {
            completed as f64 / planned as f64
        },
    }
}
