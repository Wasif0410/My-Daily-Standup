//! Recorded durations and period totals.
//!
//! Spec §10.4. Rust owns every number the weekly recap reports; the model only
//! narrates them (§3.6). A recap whose figures disagree with its prose is worse
//! than no recap.

use std::collections::HashMap;

use crate::storage::Task;

/// Bucket for tasks with no area set.
///
/// Dropping them would make the per-area figures silently disagree with the
/// overall total, which is worse than an extra row.
pub const UNASSIGNED: &str = "Unassigned";

/// Total recorded minutes across a set of tasks.
///
/// A task with no recorded duration contributes nothing. It is *unmeasured*,
/// not zero — treating the two the same would make a half-logged week read as
/// a slow one.
pub fn minutes_in_period(tasks: &[Task]) -> i64 {
    tasks.iter().filter_map(|t| t.time_spent_minutes).sum()
}

/// Mean minutes across the tasks that actually have a recorded duration.
///
/// Divides by the count of *measured* tasks, not all of them. Counting an
/// unrecorded task as zero would drag down the average of a half-logged week
/// and report the work as faster than it was.
///
/// `None` when nothing was recorded: there is no average of no measurements,
/// and reporting zero would be a claim rather than an absence.
pub fn average_minutes(tasks: &[Task]) -> Option<i64> {
    let measured: Vec<i64> = tasks.iter().filter_map(|t| t.time_spent_minutes).collect();

    if measured.is_empty() {
        return None;
    }

    Some(measured.iter().sum::<i64>() / measured.len() as i64)
}

/// Recorded minutes grouped by area, largest first.
///
/// Ordering is part of the output rather than a display concern: the weekly
/// recap leads with where the time actually went.
pub fn minutes_by_area(tasks: &[Task]) -> Vec<(String, i64)> {
    group_by(tasks, |task| {
        task.area.clone().unwrap_or_else(|| UNASSIGNED.to_string())
    })
}

/// Recorded minutes grouped by project, largest first.
pub fn minutes_by_project(tasks: &[Task]) -> Vec<(String, i64)> {
    group_by(tasks, |task| {
        task.project
            .clone()
            .unwrap_or_else(|| UNASSIGNED.to_string())
    })
}

fn group_by(tasks: &[Task], key: impl Fn(&Task) -> String) -> Vec<(String, i64)> {
    let mut totals: HashMap<String, i64> = HashMap::new();

    for task in tasks {
        // Only recorded durations contribute. A group whose work was never
        // logged is omitted entirely rather than shown as "0m", which would
        // imply the work took no time.
        if let Some(minutes) = task.time_spent_minutes {
            *totals.entry(key(task)).or_default() += minutes;
        }
    }

    let mut grouped: Vec<(String, i64)> = totals.into_iter().collect();
    // Largest first, then alphabetically so equal totals have a stable order
    // rather than shuffling between runs.
    grouped.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    grouped
}

/// Tasks whose recorded duration exceeds `threshold_minutes`.
///
/// Reports only. Eighteen hours on one task is more likely a typo than a
/// marathon, but the user may be right, so this raises a question during the
/// evening check-in rather than discarding the value (§10.4).
pub fn implausible_durations(tasks: &[Task], threshold_minutes: i64) -> Vec<&Task> {
    tasks
        .iter()
        .filter(|t| t.time_spent_minutes.is_some_and(|m| m > threshold_minutes))
        .collect()
}

/// Formats minutes for display, e.g. `1h 35m`.
pub fn format_minutes(minutes: i64) -> String {
    let hours = minutes / 60;
    let remainder = minutes % 60;

    match (hours, remainder) {
        (0, m) => format!("{m}m"),
        (h, 0) => format!("{h}h"),
        (h, m) => format!("{h}h {m}m"),
    }
}
