//! Progress computation.
//!
//! Pure functions over immutable data. Rust owns every calculation the app
//! displays; the language model never computes a number (spec §3.6, §24).

use crate::storage::{Task, TaskStatus};

/// How far along a task is.
///
/// The shape depends on what the task actually tracks — "12 of 20
/// applications" and "2 of 3 subtasks" are different statements and the UI
/// renders them differently.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Progress {
    /// Done or not. No finer measure available.
    Binary { completed: bool },
    /// Counting toward a target the user set, e.g. 12 of 20 applications.
    Numeric { current: f64, target: f64 },
    /// Inferred from child tasks.
    Subtasks { completed: usize, total: usize },
}

impl Progress {
    /// Progress as a fraction in `0.0..=1.0`, for rendering a bar.
    ///
    /// Always clamped: overshooting a target must not overflow the track, and
    /// an empty denominator reports zero rather than dividing by zero.
    pub fn fraction(self) -> f64 {
        let raw = match self {
            Self::Binary { completed } => {
                return if completed { 1.0 } else { 0.0 };
            }
            Self::Numeric { current, target } => {
                if target <= 0.0 {
                    return 0.0;
                }
                current / target
            }
            Self::Subtasks { completed, total } => {
                if total == 0 {
                    return 0.0;
                }
                completed as f64 / total as f64
            }
        };

        raw.clamp(0.0, 1.0)
    }
}

/// Computes a task's progress from its own fields and its direct children.
///
/// Precedence: an explicit numeric target beats counting children, because
/// "12 of 20 applications" says more than "1 of 3 subtasks". With neither, the
/// task is simply done or not.
///
/// Takes `&Task` deliberately: computing progress must never change a task's
/// status. Completing the last child makes the parent *look* finished, but
/// marking it finished stays a decision the caller makes (spec §10.1).
pub fn compute_progress(task: &Task, children: &[Task]) -> Progress {
    if let Some(target) = task.progress_target {
        return Progress::Numeric {
            current: task.progress_current.unwrap_or(0.0),
            target,
        };
    }

    if !children.is_empty() {
        // A cancelled subtask is work deliberately dropped, not work
        // outstanding. Leaving it in the denominator would cap the parent
        // below 100% forever.
        let counted: Vec<&Task> = children
            .iter()
            .filter(|c| c.status != TaskStatus::Cancelled)
            .collect();

        return Progress::Subtasks {
            completed: counted
                .iter()
                .filter(|c| c.status == TaskStatus::Completed)
                .count(),
            total: counted.len(),
        };
    }

    Progress::Binary {
        completed: task.status == TaskStatus::Completed,
    }
}

/// Whether a task's completion rule is satisfied.
///
/// Reports only. Nothing here writes, so a parent is never silently completed
/// behind the user's back — the caller decides whether to act on this.
pub fn is_complete_by_rule(task: &Task, children: &[Task]) -> bool {
    match compute_progress(task, children) {
        Progress::Binary { completed } => completed,
        Progress::Numeric { current, target } => target > 0.0 && current >= target,
        // An all-cancelled parent has no real work behind it, so `total == 0`
        // must not read as "everything done".
        Progress::Subtasks { completed, total } => total > 0 && completed == total,
    }
}
