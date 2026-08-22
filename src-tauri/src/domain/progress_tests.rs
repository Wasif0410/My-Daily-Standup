//! Tests for progress computation.
//!
//! These encode spec §10.1 and §10.2. Every rule here has a test that fails if
//! the rule is inverted.

use super::*;
use crate::storage::{NewTask, Task, TaskHorizon, TaskSource, TaskStatus};

/// A bare task with no progress fields set.
fn task(status: TaskStatus) -> Task {
    let input = NewTask::new("a task", TaskHorizon::Weekly, TaskSource::Manual);
    Task {
        id: "t".into(),
        title: input.title,
        description: None,
        horizon: input.horizon,
        status,
        parent_task_id: None,
        source_type: input.source_type,
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
        rollover_count: 0,
        created_at: "2026-08-20T00:00:00.000000Z".into(),
        updated_at: "2026-08-20T00:00:00.000000Z".into(),
    }
}

fn child(status: TaskStatus) -> Task {
    let mut c = task(status);
    c.id = format!("child-{status:?}");
    c
}

#[test]
fn a_task_with_no_children_and_no_target_is_binary() {
    let progress = compute_progress(&task(TaskStatus::Planned), &[]);

    assert_eq!(progress, Progress::Binary { completed: false });
}

#[test]
fn a_completed_task_with_no_children_reports_fully_done() {
    let progress = compute_progress(&task(TaskStatus::Completed), &[]);

    assert_eq!(progress, Progress::Binary { completed: true });
    assert_eq!(progress.fraction(), 1.0);
}

#[test]
fn a_numeric_target_produces_numeric_progress() {
    let mut t = task(TaskStatus::InProgress);
    t.progress_current = Some(12.0);
    t.progress_target = Some(20.0);

    let progress = compute_progress(&t, &[]);

    assert_eq!(
        progress,
        Progress::Numeric {
            current: 12.0,
            target: 20.0
        }
    );
    assert!((progress.fraction() - 0.6).abs() < f64::EPSILON);
}

#[test]
fn an_explicit_target_wins_over_counting_children() {
    // "12 of 20 applications" is more meaningful than "1 of 3 subtasks", so a
    // target the user set takes precedence over inference.
    let mut t = task(TaskStatus::InProgress);
    t.progress_current = Some(12.0);
    t.progress_target = Some(20.0);

    let children = [child(TaskStatus::Completed), child(TaskStatus::Planned)];
    let progress = compute_progress(&t, &children);

    assert!(matches!(progress, Progress::Numeric { .. }));
}

#[test]
fn children_are_counted_when_there_is_no_explicit_target() {
    let children = [
        child(TaskStatus::Completed),
        child(TaskStatus::Completed),
        child(TaskStatus::Planned),
    ];

    let progress = compute_progress(&task(TaskStatus::InProgress), &children);

    assert_eq!(
        progress,
        Progress::Subtasks {
            completed: 2,
            total: 3
        }
    );
}

#[test]
fn cancelled_children_are_excluded_from_the_denominator() {
    // Abandoning a subtask should not permanently cap the parent below 100%.
    let children = [
        child(TaskStatus::Completed),
        child(TaskStatus::Cancelled),
        child(TaskStatus::Completed),
    ];

    let progress = compute_progress(&task(TaskStatus::InProgress), &children);

    assert_eq!(
        progress,
        Progress::Subtasks {
            completed: 2,
            total: 2
        }
    );
    assert_eq!(progress.fraction(), 1.0);
}

#[test]
fn progress_beyond_the_target_is_clamped_to_one() {
    let mut t = task(TaskStatus::InProgress);
    t.progress_current = Some(25.0);
    t.progress_target = Some(20.0);

    // A progress bar must not overflow its track when you overshoot a goal.
    assert_eq!(compute_progress(&t, &[]).fraction(), 1.0);
}

#[test]
fn a_zero_target_does_not_divide_by_zero() {
    let mut t = task(TaskStatus::InProgress);
    t.progress_current = Some(0.0);
    t.progress_target = Some(0.0);

    assert_eq!(compute_progress(&t, &[]).fraction(), 0.0);
}

#[test]
fn all_children_cancelled_reports_zero_rather_than_complete() {
    let children = [child(TaskStatus::Cancelled), child(TaskStatus::Cancelled)];

    let progress = compute_progress(&task(TaskStatus::InProgress), &children);

    assert_eq!(progress.fraction(), 0.0, "no real work was done");
}

#[test]
fn completing_every_child_satisfies_the_completion_rule() {
    let children = [child(TaskStatus::Completed), child(TaskStatus::Completed)];

    assert!(is_complete_by_rule(
        &task(TaskStatus::InProgress),
        &children
    ));
}

#[test]
fn one_unfinished_child_leaves_the_rule_unsatisfied() {
    let children = [child(TaskStatus::Completed), child(TaskStatus::Planned)];

    assert!(!is_complete_by_rule(
        &task(TaskStatus::InProgress),
        &children
    ));
}

#[test]
fn reaching_a_numeric_target_satisfies_the_completion_rule() {
    let mut t = task(TaskStatus::InProgress);
    t.progress_current = Some(20.0);
    t.progress_target = Some(20.0);

    assert!(is_complete_by_rule(&t, &[]));
}

#[test]
fn computing_progress_never_completes_the_parent_itself() {
    // Spec §10.1: completing a child updates progress but must not silently
    // mark the parent done. `compute_progress` takes an immutable reference
    // precisely so it cannot; this test documents that the caller decides.
    let children = [child(TaskStatus::Completed), child(TaskStatus::Completed)];
    let parent = task(TaskStatus::InProgress);

    let progress = compute_progress(&parent, &children);

    assert_eq!(progress.fraction(), 1.0, "progress is full");
    assert_eq!(
        parent.status,
        TaskStatus::InProgress,
        "but the parent's status is untouched - completion stays a decision"
    );
}
