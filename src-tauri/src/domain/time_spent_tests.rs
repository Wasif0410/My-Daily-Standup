//! Tests for recorded durations and period totals.
//!
//! These encode spec §10.4. The distinction that matters throughout: a task
//! with no recorded duration is *unmeasured*, not zero. Treating the two the
//! same would make every average wrong.

use super::*;
use crate::storage::{Task, TaskHorizon, TaskSource, TaskStatus};

fn task(area: Option<&str>, minutes: Option<i64>) -> Task {
    Task {
        id: format!("{area:?}-{minutes:?}"),
        title: "a task".into(),
        description: None,
        horizon: TaskHorizon::Daily,
        status: TaskStatus::Completed,
        parent_task_id: None,
        source_type: TaskSource::Manual,
        source_file: None,
        source_line: None,
        area: area.map(str::to_string),
        project: None,
        priority: None,
        scheduled_date: Some("2026-08-20".into()),
        period_start: None,
        period_end: None,
        due_date: None,
        completed_at: None,
        progress_current: None,
        progress_target: None,
        progress_unit: None,
        blocker: None,
        notes: None,
        time_spent_minutes: minutes,
        rollover_count: 0,
        created_at: "2026-08-20T00:00:00.000000Z".into(),
        updated_at: "2026-08-20T00:00:00.000000Z".into(),
    }
}

// --- totals -----------------------------------------------------------------

#[test]
fn totals_sum_recorded_durations() {
    let tasks = [task(None, Some(35)), task(None, Some(25))];

    assert_eq!(minutes_in_period(&tasks), 60);
}

#[test]
fn an_unrecorded_duration_contributes_nothing() {
    // Not zero — unmeasured. The total must reflect only what was actually
    // logged, so a half-logged week reads as partial rather than as slow.
    let tasks = [task(None, Some(45)), task(None, None)];

    assert_eq!(minutes_in_period(&tasks), 45);
}

#[test]
fn a_period_with_nothing_recorded_totals_zero() {
    let tasks = [task(None, None), task(None, None)];

    assert_eq!(minutes_in_period(&tasks), 0);
}

#[test]
fn an_empty_period_totals_zero() {
    assert_eq!(minutes_in_period(&[]), 0);
}

#[test]
fn the_average_divides_by_measured_tasks_only() {
    // Two tasks logged at 60 and 30, plus one never logged. The average is 45,
    // not 30: counting the unmeasured task as zero would report the work as a
    // third faster than it actually was.
    let tasks = [task(None, Some(60)), task(None, Some(30)), task(None, None)];

    assert_eq!(average_minutes(&tasks), Some(45));
}

#[test]
fn there_is_no_average_without_measurements() {
    // Zero would be a claim about how long the work took. None is an absence.
    assert_eq!(average_minutes(&[task(None, None)]), None);
    assert_eq!(average_minutes(&[]), None);
}

// --- grouping ---------------------------------------------------------------

#[test]
fn totals_group_by_area() {
    let tasks = [
        task(Some("Job Search"), Some(60)),
        task(Some("Health"), Some(15)),
        task(Some("Job Search"), Some(30)),
    ];

    let by_area = minutes_by_area(&tasks);

    assert_eq!(by_area.len(), 2);
    assert_eq!(
        by_area.iter().find(|(a, _)| a == "Job Search").unwrap().1,
        90
    );
    assert_eq!(by_area.iter().find(|(a, _)| a == "Health").unwrap().1, 15);
}

#[test]
fn areas_are_ordered_by_time_descending() {
    // The weekly recap leads with where the time actually went, so the
    // ordering is part of the output rather than a display concern.
    let tasks = [
        task(Some("Health"), Some(15)),
        task(Some("Job Search"), Some(90)),
        task(Some("Admin"), Some(40)),
    ];

    let by_area = minutes_by_area(&tasks);

    assert_eq!(
        by_area.iter().map(|(a, _)| a.as_str()).collect::<Vec<_>>(),
        ["Job Search", "Admin", "Health"]
    );
}

#[test]
fn tasks_without_an_area_are_grouped_as_unassigned() {
    // Dropping them would make the per-area figures silently disagree with
    // the overall total, which is worse than an "Unassigned" row.
    let tasks = [task(Some("Health"), Some(15)), task(None, Some(25))];

    let by_area = minutes_by_area(&tasks);

    assert_eq!(by_area.iter().map(|(_, m)| m).sum::<i64>(), 40);
    assert!(by_area.iter().any(|(a, _)| a == UNASSIGNED));
}

#[test]
fn an_area_with_no_recorded_time_is_omitted() {
    // A row reading "Health — 0m" implies the work took no time, when in fact
    // it was simply never logged.
    let tasks = [
        task(Some("Job Search"), Some(60)),
        task(Some("Health"), None),
    ];

    let by_area = minutes_by_area(&tasks);

    assert_eq!(by_area.len(), 1);
    assert_eq!(by_area[0].0, "Job Search");
}

// --- implausible values -----------------------------------------------------

#[test]
fn a_duration_beyond_the_threshold_is_flagged() {
    let tasks = [task(None, Some(18 * 60)), task(None, Some(30))];

    let flagged = implausible_durations(&tasks, 8 * 60);

    assert_eq!(flagged.len(), 1);
    assert_eq!(flagged[0].time_spent_minutes, Some(18 * 60));
}

#[test]
fn a_duration_exactly_at_the_threshold_is_not_flagged() {
    // Eight hours is a long day, not an error. Only what exceeds it is worth
    // querying.
    let tasks = [task(None, Some(8 * 60))];

    assert!(implausible_durations(&tasks, 8 * 60).is_empty());
}

#[test]
fn flagging_never_rejects_the_value() {
    // The user may genuinely have spent eighteen hours on something. Flagging
    // asks a question; it must not discard the answer.
    let tasks = [task(None, Some(18 * 60))];

    let flagged = implausible_durations(&tasks, 8 * 60);

    assert_eq!(flagged.len(), 1);
    assert_eq!(minutes_in_period(&tasks), 18 * 60, "the value still counts");
}

// --- formatting -------------------------------------------------------------

#[test]
fn durations_format_for_reading() {
    assert_eq!(format_minutes(0), "0m");
    assert_eq!(format_minutes(35), "35m");
    assert_eq!(format_minutes(60), "1h");
    assert_eq!(format_minutes(95), "1h 35m");
    assert_eq!(format_minutes(18 * 60), "18h");
}
