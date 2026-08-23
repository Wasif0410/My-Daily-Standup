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

    let patch = TaskPatch {
        scheduled_date: Some(Some(to.to_string())),
        rollover_count: moves_later(task.scheduled_date.as_deref(), new_date)
            .then(|| task.rollover_count + 1),
        ..Default::default()
    };

    repo.update(conn, id, patch)
}

/// Moves a task to another week, counting the move as a deferral only when the
/// week starts later.
///
/// The same rule as [`reschedule`], for the same reason: pushing a commitment
/// into next week is the deferral §10.3's prompt exists to notice. Sharing
/// [`moves_later`] rather than restating the comparison keeps the two paths
/// from drifting into disagreeing about what a deferral is.
pub fn move_to_period(
    repo: &TaskRepo,
    conn: &Connection,
    id: &str,
    start: &str,
    end: &str,
) -> Result<Task, StorageError> {
    // Validated before the database is touched, so a bad value cannot leave a
    // task half-updated.
    let new_start = parse_date(start)?;
    let new_end = parse_date(end)?;

    if new_end < new_start {
        return Err(StorageError::InvalidDate {
            value: format!("{start}..{end}"),
        });
    }

    let task = repo
        .get(conn, id)?
        .ok_or_else(|| StorageError::TaskNotFound { id: id.to_string() })?;

    let patch = TaskPatch {
        period_start: Some(Some(start.to_string())),
        period_end: Some(Some(end.to_string())),
        rollover_count: moves_later(task.period_start.as_deref(), new_start)
            .then(|| task.rollover_count + 1),
        ..Default::default()
    };

    repo.update(conn, id, patch)
}

/// Whether replacing `existing` with `moving_to` pushes a task later.
///
/// Three cases deliberately answer `false`, and each is a decision rather than
/// an omission:
///
/// - **Pulling work earlier.** Planning, not avoidance.
/// - **The same date.** Nothing changed.
/// - **No previous date.** Giving a backlog item its first slot is scheduling.
///
/// A malformed date already in the database also answers `false`: the user must
/// be able to repair it without the repair counting against them.
fn moves_later(existing: Option<&str>, moving_to: NaiveDate) -> bool {
    match existing {
        Some(text) => parse_date(text).is_ok_and(|current| moving_to > current),
        None => false,
    }
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
    /// Total recorded minutes. Tasks with no duration contribute nothing:
    /// unmeasured is not the same as zero (spec §10.4).
    pub minutes_tracked: i64,
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
        minutes_tracked: super::minutes_in_period(tasks),
    }
}
