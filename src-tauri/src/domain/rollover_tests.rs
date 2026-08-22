//! Tests for rollover counting and period statistics.
//!
//! The rollover counter is the sole basis for spec §10.3's reflection prompt
//! ("you have moved this five times"). If it counts the wrong events, that
//! prompt becomes either meaningless or nagging, so the boundaries matter more
//! than the mechanism.

use super::*;
use crate::storage::{Db, NewTask, TaskHorizon, TaskPatch, TaskRepo, TaskSource, TaskStatus};

fn setup() -> (Db, TaskRepo) {
    (
        Db::open_in_memory().expect("open in-memory database"),
        TaskRepo::new(),
    )
}

fn scheduled(repo: &TaskRepo, db: &Db, date: Option<&str>) -> crate::storage::Task {
    let mut input = NewTask::new("call the clinic", TaskHorizon::Daily, TaskSource::Manual);
    input.scheduled_date = date.map(str::to_string);
    repo.create(db.conn(), input).expect("create task")
}

// --- rollover ---------------------------------------------------------------

#[test]
fn pushing_a_task_later_increments_the_counter_by_exactly_one() {
    let (db, repo) = setup();
    let task = scheduled(&repo, &db, Some("2026-08-20"));

    let moved = reschedule(&repo, db.conn(), &task.id, "2026-08-21").unwrap();

    assert_eq!(moved.rollover_count, 1);
    assert_eq!(moved.scheduled_date.as_deref(), Some("2026-08-21"));
}

#[test]
fn pulling_a_task_earlier_does_not_count_as_a_rollover() {
    // Moving work forward is planning, not avoidance. Counting it would
    // punish exactly the behaviour the app wants to encourage.
    let (db, repo) = setup();
    let task = scheduled(&repo, &db, Some("2026-08-20"));

    let moved = reschedule(&repo, db.conn(), &task.id, "2026-08-18").unwrap();

    assert_eq!(moved.rollover_count, 0);
    assert_eq!(moved.scheduled_date.as_deref(), Some("2026-08-18"));
}

#[test]
fn rescheduling_to_the_same_day_does_not_count() {
    let (db, repo) = setup();
    let task = scheduled(&repo, &db, Some("2026-08-20"));

    let moved = reschedule(&repo, db.conn(), &task.id, "2026-08-20").unwrap();

    assert_eq!(moved.rollover_count, 0);
}

#[test]
fn scheduling_an_unscheduled_task_for_the_first_time_does_not_count() {
    // Giving a backlog item its first date is scheduling, not deferral.
    let (db, repo) = setup();
    let task = scheduled(&repo, &db, None);

    let moved = reschedule(&repo, db.conn(), &task.id, "2026-08-21").unwrap();

    assert_eq!(moved.rollover_count, 0);
}

#[test]
fn repeated_deferrals_accumulate() {
    let (db, repo) = setup();
    let task = scheduled(&repo, &db, Some("2026-08-20"));

    reschedule(&repo, db.conn(), &task.id, "2026-08-21").unwrap();
    reschedule(&repo, db.conn(), &task.id, "2026-08-22").unwrap();
    let third = reschedule(&repo, db.conn(), &task.id, "2026-08-25").unwrap();

    assert_eq!(
        third.rollover_count, 3,
        "three deferrals is what makes the reflection prompt meaningful"
    );
}

#[test]
fn a_deferral_then_a_pull_forward_leaves_the_count_at_one() {
    let (db, repo) = setup();
    let task = scheduled(&repo, &db, Some("2026-08-20"));

    reschedule(&repo, db.conn(), &task.id, "2026-08-25").unwrap();
    let pulled = reschedule(&repo, db.conn(), &task.id, "2026-08-21").unwrap();

    assert_eq!(
        pulled.rollover_count, 1,
        "the counter records history and never decreases"
    );
}

#[test]
fn editing_a_title_does_not_touch_the_counter() {
    let (db, repo) = setup();
    let task = scheduled(&repo, &db, Some("2026-08-20"));
    reschedule(&repo, db.conn(), &task.id, "2026-08-21").unwrap();

    let renamed = repo
        .update(
            db.conn(),
            &task.id,
            TaskPatch {
                title: Some("renamed".into()),
                ..Default::default()
            },
        )
        .unwrap();

    assert_eq!(renamed.rollover_count, 1);
}

#[test]
fn adding_a_blocker_does_not_touch_the_counter() {
    let (db, repo) = setup();
    let task = scheduled(&repo, &db, Some("2026-08-20"));

    let blocked = repo
        .update(
            db.conn(),
            &task.id,
            TaskPatch {
                blocker: Some(Some("clinic closed".into())),
                ..Default::default()
            },
        )
        .unwrap();

    assert_eq!(
        blocked.rollover_count, 0,
        "being blocked by someone else is not avoidance"
    );
}

#[test]
fn completing_a_task_does_not_touch_the_counter() {
    let (db, repo) = setup();
    let task = scheduled(&repo, &db, Some("2026-08-20"));

    let done = repo
        .update(
            db.conn(),
            &task.id,
            TaskPatch {
                status: Some(TaskStatus::Completed),
                ..Default::default()
            },
        )
        .unwrap();

    assert_eq!(done.rollover_count, 0);
}

#[test]
fn a_malformed_date_is_rejected_and_changes_nothing() {
    let (db, repo) = setup();
    let task = scheduled(&repo, &db, Some("2026-08-20"));

    let result = reschedule(&repo, db.conn(), &task.id, "next tuesday");

    assert!(result.is_err(), "only ISO-8601 dates are accepted");

    let unchanged = repo.get(db.conn(), &task.id).unwrap().unwrap();
    assert_eq!(unchanged.scheduled_date.as_deref(), Some("2026-08-20"));
    assert_eq!(unchanged.rollover_count, 0);
}

#[test]
fn rescheduling_a_missing_task_reports_not_found() {
    let (db, repo) = setup();

    let result = reschedule(&repo, db.conn(), "no-such-task", "2026-08-21");

    assert!(result.is_err());
}

// --- period statistics ------------------------------------------------------

fn stat_task(status: TaskStatus, rollover: i64) -> crate::storage::Task {
    let mut t = crate::storage::Task {
        id: format!("{status:?}-{rollover}"),
        title: "t".into(),
        description: None,
        horizon: TaskHorizon::Daily,
        status,
        parent_task_id: None,
        source_type: TaskSource::Manual,
        source_file: None,
        source_line: None,
        area: None,
        project: None,
        priority: None,
        scheduled_date: None,
        period_start: None,
        period_end: None,
        due_date: None,
        completed_at: None,
        progress_current: None,
        progress_target: None,
        progress_unit: None,
        blocker: None,
        notes: None,
        rollover_count: rollover,
        created_at: "2026-08-20T00:00:00.000000Z".into(),
        updated_at: "2026-08-20T00:00:00.000000Z".into(),
    };
    t.rollover_count = rollover;
    t
}

#[test]
fn completion_rate_excludes_cancelled_work_from_the_denominator() {
    // Spec §13.2 reports "planned 14, completed 10". Cancelling a task you
    // deliberately dropped should not read as a failure to complete it.
    let tasks = [
        stat_task(TaskStatus::Completed, 0),
        stat_task(TaskStatus::Completed, 0),
        stat_task(TaskStatus::Planned, 0),
        stat_task(TaskStatus::Cancelled, 0),
    ];

    let stats = period_stats(&tasks);

    assert_eq!(
        stats.planned, 3,
        "cancelled tasks are not counted as planned"
    );
    assert_eq!(stats.completed, 2);
    assert_eq!(stats.cancelled, 1);
    assert!((stats.completion_rate - 2.0 / 3.0).abs() < 1e-9);
}

#[test]
fn an_empty_period_reports_a_zero_rate_rather_than_dividing_by_zero() {
    let stats = period_stats(&[]);

    assert_eq!(stats.planned, 0);
    assert_eq!(stats.completion_rate, 0.0);
}

#[test]
fn a_period_of_only_cancelled_tasks_does_not_divide_by_zero() {
    let stats = period_stats(&[stat_task(TaskStatus::Cancelled, 0)]);

    assert_eq!(stats.planned, 0);
    assert_eq!(stats.completion_rate, 0.0);
}

#[test]
fn everything_completed_is_a_rate_of_one() {
    let tasks = [
        stat_task(TaskStatus::Completed, 0),
        stat_task(TaskStatus::Completed, 1),
    ];

    assert_eq!(period_stats(&tasks).completion_rate, 1.0);
}

#[test]
fn carried_counts_unfinished_tasks_that_were_deferred() {
    let tasks = [
        stat_task(TaskStatus::Planned, 3),   // deferred and still open
        stat_task(TaskStatus::Completed, 2), // deferred but finished
        stat_task(TaskStatus::Planned, 0),   // never deferred
    ];

    let stats = period_stats(&tasks);

    assert_eq!(
        stats.carried, 1,
        "only work still outstanding counts as carried forward"
    );
}
