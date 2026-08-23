//! Tests for rolling a commitment's progress up from the work beneath it.

use super::*;
use crate::storage::{Db, NewTask, TaskHorizon, TaskPatch, TaskRepo, TaskSource, TaskStatus};

fn setup() -> (Db, TaskRepo) {
    let db = Db::open_in_memory().expect("open in-memory database");
    (db, TaskRepo::new())
}

/// Creates a task under `parent`, returning its id.
fn child_of(
    db: &Db,
    repo: &TaskRepo,
    parent: Option<&str>,
    title: &str,
    horizon: TaskHorizon,
) -> String {
    repo.create(
        db.conn(),
        NewTask {
            parent_task_id: parent.map(str::to_string),
            ..NewTask::new(title, horizon, TaskSource::Manual)
        },
    )
    .unwrap()
    .id
}

fn complete(db: &Db, repo: &TaskRepo, id: &str) {
    repo.update(
        db.conn(),
        id,
        TaskPatch {
            status: Some(TaskStatus::Completed),
            ..Default::default()
        },
    )
    .unwrap();
}

fn roll_up(db: &Db, repo: &TaskRepo, id: &str) -> Commitment {
    let task = repo.get(db.conn(), id).unwrap().unwrap();
    commitment_progress(repo, db.conn(), &task).unwrap()
}

#[test]
fn a_commitment_with_a_numeric_target_reports_it() {
    let (db, repo) = setup();
    let id = repo
        .create(
            db.conn(),
            NewTask {
                progress_current: Some(12.0),
                progress_target: Some(20.0),
                progress_unit: Some("applications".to_string()),
                ..NewTask::new("Job search", TaskHorizon::Monthly, TaskSource::Manual)
            },
        )
        .unwrap()
        .id;

    let rolled = roll_up(&db, &repo, &id);

    assert_eq!(
        rolled.progress,
        Progress::Numeric {
            current: 12.0,
            target: 20.0
        }
    );
    assert!((rolled.fraction - 0.6).abs() < f64::EPSILON);
    assert!(!rolled.complete);
}

#[test]
fn a_numeric_target_ignores_children() {
    // The number the user set is not overridden by counting subtasks. "12 of
    // 20 applications" says more than "0 of 1 subtasks".
    let (db, repo) = setup();
    let id = repo
        .create(
            db.conn(),
            NewTask {
                progress_current: Some(12.0),
                progress_target: Some(20.0),
                ..NewTask::new("Job search", TaskHorizon::Monthly, TaskSource::Manual)
            },
        )
        .unwrap()
        .id;
    child_of(&db, &repo, Some(&id), "a weekly", TaskHorizon::Weekly);

    let rolled = roll_up(&db, &repo, &id);

    assert_eq!(
        rolled.progress,
        Progress::Numeric {
            current: 12.0,
            target: 20.0
        }
    );
}

#[test]
fn a_commitment_with_no_target_counts_its_children() {
    let (db, repo) = setup();
    let monthly = child_of(&db, &repo, None, "MVP", TaskHorizon::Monthly);
    let first = child_of(&db, &repo, Some(&monthly), "week one", TaskHorizon::Weekly);
    child_of(&db, &repo, Some(&monthly), "week two", TaskHorizon::Weekly);
    complete(&db, &repo, &first);

    let rolled = roll_up(&db, &repo, &monthly);

    assert_eq!(
        rolled.progress,
        Progress::Subtasks {
            completed: 1,
            total: 2
        }
    );
}

#[test]
fn a_completed_daily_task_moves_the_monthly_number() {
    // The definition of done. Monthly → weekly → daily: finishing the daily
    // satisfies the weekly's rule, which the monthly counts. Nothing writes a
    // status the user did not set (§10.1) — the roll-up reads the rule.
    let (db, repo) = setup();
    let monthly = child_of(&db, &repo, None, "MVP", TaskHorizon::Monthly);
    let weekly = child_of(&db, &repo, Some(&monthly), "milestone", TaskHorizon::Weekly);
    let daily = child_of(&db, &repo, Some(&weekly), "the work", TaskHorizon::Daily);

    let before = roll_up(&db, &repo, &monthly);
    assert_eq!(
        before.progress,
        Progress::Subtasks {
            completed: 0,
            total: 1
        },
        "nothing is finished yet"
    );

    complete(&db, &repo, &daily);
    let after = roll_up(&db, &repo, &monthly);

    assert_eq!(
        after.progress,
        Progress::Subtasks {
            completed: 1,
            total: 1
        },
        "finishing the daily must reach the monthly figure"
    );
    assert!(after.complete);
}

#[test]
fn a_partly_finished_daily_set_does_not_count_its_weekly_as_done() {
    let (db, repo) = setup();
    let monthly = child_of(&db, &repo, None, "MVP", TaskHorizon::Monthly);
    let weekly = child_of(&db, &repo, Some(&monthly), "milestone", TaskHorizon::Weekly);
    let first = child_of(&db, &repo, Some(&weekly), "part one", TaskHorizon::Daily);
    child_of(&db, &repo, Some(&weekly), "part two", TaskHorizon::Daily);
    complete(&db, &repo, &first);

    let rolled = roll_up(&db, &repo, &monthly);

    assert_eq!(
        rolled.progress,
        Progress::Subtasks {
            completed: 0,
            total: 1
        }
    );
}

#[test]
fn rolling_up_never_writes_a_status() {
    // The whole design rests on this. Computing progress must never complete a
    // task behind the user's back (§10.1); if it did, the "satisfied by rule"
    // reading above would become a silent write on every board render.
    let (db, repo) = setup();
    let monthly = child_of(&db, &repo, None, "MVP", TaskHorizon::Monthly);
    let weekly = child_of(&db, &repo, Some(&monthly), "milestone", TaskHorizon::Weekly);
    let daily = child_of(&db, &repo, Some(&weekly), "the work", TaskHorizon::Daily);
    complete(&db, &repo, &daily);

    roll_up(&db, &repo, &monthly);

    let untouched = repo.get(db.conn(), &weekly).unwrap().unwrap();
    assert_eq!(untouched.status, TaskStatus::Planned);
}

#[test]
fn an_explicitly_completed_child_counts_without_children_of_its_own() {
    let (db, repo) = setup();
    let monthly = child_of(&db, &repo, None, "MVP", TaskHorizon::Monthly);
    let weekly = child_of(&db, &repo, Some(&monthly), "milestone", TaskHorizon::Weekly);
    complete(&db, &repo, &weekly);

    let rolled = roll_up(&db, &repo, &monthly);

    assert_eq!(
        rolled.progress,
        Progress::Subtasks {
            completed: 1,
            total: 1
        }
    );
}

#[test]
fn a_cancelled_child_leaves_the_denominator() {
    // Work deliberately dropped is not work outstanding. Keeping it would cap
    // the commitment below 100% for the rest of the month.
    let (db, repo) = setup();
    let monthly = child_of(&db, &repo, None, "MVP", TaskHorizon::Monthly);
    let kept = child_of(&db, &repo, Some(&monthly), "kept", TaskHorizon::Weekly);
    let dropped = child_of(&db, &repo, Some(&monthly), "dropped", TaskHorizon::Weekly);
    complete(&db, &repo, &kept);
    repo.update(
        db.conn(),
        &dropped,
        TaskPatch {
            status: Some(TaskStatus::Cancelled),
            ..Default::default()
        },
    )
    .unwrap();

    let rolled = roll_up(&db, &repo, &monthly);

    assert_eq!(
        rolled.progress,
        Progress::Subtasks {
            completed: 1,
            total: 1
        }
    );
    assert!(rolled.complete);
}

#[test]
fn a_commitment_with_nothing_under_it_is_binary() {
    let (db, repo) = setup();
    let id = child_of(&db, &repo, None, "a lone commitment", TaskHorizon::Monthly);

    let rolled = roll_up(&db, &repo, &id);

    assert_eq!(rolled.progress, Progress::Binary { completed: false });
    assert_eq!(rolled.fraction, 0.0);
}

#[test]
fn the_fraction_is_clamped_over_target() {
    // 22 of 20 is a real and good outcome. The bar must not overflow its
    // track, but the figure itself is reported unchanged.
    let (db, repo) = setup();
    let id = repo
        .create(
            db.conn(),
            NewTask {
                progress_current: Some(22.0),
                progress_target: Some(20.0),
                ..NewTask::new("Job search", TaskHorizon::Monthly, TaskSource::Manual)
            },
        )
        .unwrap()
        .id;

    let rolled = roll_up(&db, &repo, &id);

    assert_eq!(rolled.fraction, 1.0);
    assert_eq!(
        rolled.progress,
        Progress::Numeric {
            current: 22.0,
            target: 20.0
        },
        "the achievement is not clamped away"
    );
    assert!(rolled.complete);
}

#[test]
fn the_fraction_of_an_empty_target_is_zero_not_a_division_by_zero() {
    let (db, repo) = setup();
    let id = repo
        .create(
            db.conn(),
            NewTask {
                progress_current: Some(0.0),
                progress_target: Some(0.0),
                ..NewTask::new("Job search", TaskHorizon::Monthly, TaskSource::Manual)
            },
        )
        .unwrap()
        .id;

    let rolled = roll_up(&db, &repo, &id);

    assert_eq!(rolled.fraction, 0.0);
}
