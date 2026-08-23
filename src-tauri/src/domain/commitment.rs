//! Rolling a commitment's progress up from the work beneath it.
//!
//! [`super::compute_progress`] looks one level down: a task and its direct
//! children. The Monthly board needs one level further, because its
//! commitments sit above weekly milestones which sit above daily actions
//! (spec §4), and finishing a daily has to move the monthly figure.

use rusqlite::Connection;
use serde::Serialize;

use super::{compute_progress, is_complete_by_rule, Progress};
use crate::storage::{StorageError, Task, TaskRepo, TaskStatus};

/// A commitment as the Monthly board renders it.
///
/// Carries the fraction as well as the raw progress so no component ever
/// divides. "Progress values come from `compute_progress`, never computed in
/// the component" is only enforceable if the component is never handed a
/// numerator and a denominator and trusted not to.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Commitment {
    pub task: Task,
    pub progress: Progress,
    /// `0.0..=1.0`, already clamped. Over-target reports `1.0` so a bar cannot
    /// overflow its track, while `progress` keeps the true figure.
    pub fraction: f64,
    pub complete: bool,
}

/// Computes a commitment's progress, counting a child as done when its own
/// rule is satisfied.
///
/// The difference from [`compute_progress`] is exactly one level. A weekly
/// milestone whose dailies are all finished counts as complete here, even
/// though its stored status still says `planned` — because nothing in this app
/// completes a task on the user's behalf (§10.1). Reading the rule is the only
/// way the monthly figure can be truthful without a silent write on every
/// board render.
///
/// A numeric target short-circuits the whole thing, for the same reason
/// [`compute_progress`] prefers one: "12 of 20 applications" is the user's own
/// measure and counting subtasks must not override it.
///
/// Stops at grandchildren. Monthly → weekly → daily is the hierarchy the spec
/// defines; unbounded recursion would buy nothing real and would let a cyclic
/// `parent_task_id` hang the board.
pub fn commitment_progress(
    repo: &TaskRepo,
    conn: &Connection,
    task: &Task,
) -> Result<Commitment, StorageError> {
    let children = repo.children_of(conn, &task.id)?;

    let progress = if task.progress_target.is_some() || children.is_empty() {
        // Nothing below to consult, or an explicit target that outranks it.
        compute_progress(task, &children)
    } else {
        // A cancelled child is work deliberately dropped, not work
        // outstanding. Leaving it in the denominator would cap the commitment
        // below 100% for the rest of the month.
        let counted: Vec<&Task> = children
            .iter()
            .filter(|c| c.status != TaskStatus::Cancelled)
            .collect();

        let mut done = 0;
        for child in &counted {
            let grandchildren = repo.children_of(conn, &child.id)?;
            if is_complete_by_rule(child, &grandchildren) {
                done += 1;
            }
        }

        Progress::Subtasks {
            completed: done,
            total: counted.len(),
        }
    };

    Ok(Commitment {
        task: task.clone(),
        progress,
        fraction: progress.fraction(),
        complete: match progress {
            Progress::Binary { completed } => completed,
            Progress::Numeric { current, target } => target > 0.0 && current >= target,
            Progress::Subtasks { completed, total } => total > 0 && completed == total,
        },
    })
}
