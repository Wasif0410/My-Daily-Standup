//! Tests for blockers and comments.

use chrono::NaiveDate;

use super::*;
use crate::storage::{Db, NewTask, TaskHorizon, TaskPatch, TaskRepo, TaskSource, TaskStatus};

fn repo() -> (Db, TaskRepo) {
    let db = Db::open_in_memory().expect("open in-memory database");
    (db, TaskRepo::new())
}

fn a_task(db: &Db, repo: &TaskRepo) -> String {
    repo.create(
        db.conn(),
        NewTask::new(
            "submit applications",
            TaskHorizon::Weekly,
            TaskSource::Manual,
        ),
    )
    .unwrap()
    .id
}

fn on(text: &str) -> NaiveDate {
    NaiveDate::parse_from_str(text, "%Y-%m-%d").expect("a valid test date")
}

#[test]
fn setting_a_blocker_marks_the_task_blocked() {
    // A `blocked` status that does not track the blocker text is decoration.
    let (db, repo) = repo();
    let id = a_task(&db, &repo);

    let task = set_blocker(&repo, db.conn(), &id, Some("waiting on the portal")).unwrap();

    assert_eq!(task.blocker.as_deref(), Some("waiting on the portal"));
    assert_eq!(task.status, TaskStatus::Blocked);
}

#[test]
fn clearing_a_blocker_returns_the_task_to_planned() {
    let (db, repo) = repo();
    let id = a_task(&db, &repo);
    set_blocker(&repo, db.conn(), &id, Some("waiting")).unwrap();

    let task = set_blocker(&repo, db.conn(), &id, None).unwrap();

    assert_eq!(task.blocker, None);
    assert_eq!(task.status, TaskStatus::Planned);
}

#[test]
fn clearing_a_blocker_leaves_a_completed_task_completed() {
    // Resolving a blocker must not un-finish work.
    let (db, repo) = repo();
    let id = a_task(&db, &repo);
    repo.update(
        db.conn(),
        &id,
        TaskPatch {
            status: Some(TaskStatus::Completed),
            ..Default::default()
        },
    )
    .unwrap();

    let task = set_blocker(&repo, db.conn(), &id, None).unwrap();

    assert_eq!(task.status, TaskStatus::Completed);
}

#[test]
fn setting_a_blocker_does_not_touch_the_rollover_count() {
    // Only `reschedule` may move that number (spec §10.3).
    let (db, repo) = repo();
    let id = a_task(&db, &repo);

    let task = set_blocker(&repo, db.conn(), &id, Some("waiting")).unwrap();

    assert_eq!(task.rollover_count, 0);
}

#[test]
fn a_blank_blocker_counts_as_clearing_it() {
    // An empty string reaching the database would leave a task "blocked" by
    // nothing, which no view could explain to the user.
    let (db, repo) = repo();
    let id = a_task(&db, &repo);
    set_blocker(&repo, db.conn(), &id, Some("waiting")).unwrap();

    let task = set_blocker(&repo, db.conn(), &id, Some("   ")).unwrap();

    assert_eq!(task.blocker, None);
    assert_eq!(task.status, TaskStatus::Planned);
}

#[test]
fn a_comment_appends_rather_than_replacing() {
    // A field that overwrites the previous comment is not a comment.
    let (db, repo) = repo();
    let id = a_task(&db, &repo);

    add_comment(&repo, db.conn(), &id, "portal was down", on("2026-08-20")).unwrap();
    let task = add_comment(&repo, db.conn(), &id, "back up now", on("2026-08-21")).unwrap();

    let notes = task.notes.unwrap();
    assert!(
        notes.contains("portal was down"),
        "the first comment survives"
    );
    assert!(notes.contains("back up now"), "the second is added");
}

#[test]
fn a_comment_carries_its_date() {
    // An undated note is useless a week later, which is exactly when it is read.
    let (db, repo) = repo();
    let id = a_task(&db, &repo);

    let task = add_comment(&repo, db.conn(), &id, "portal was down", on("2026-08-20")).unwrap();

    assert_eq!(task.notes.as_deref(), Some("2026-08-20: portal was down"));
}

#[test]
fn a_comment_on_an_empty_notes_field_does_not_lead_with_a_blank_line() {
    let (db, repo) = repo();
    let id = a_task(&db, &repo);

    let task = add_comment(&repo, db.conn(), &id, "first", on("2026-08-20")).unwrap();

    let notes = task.notes.unwrap();
    assert!(!notes.starts_with('\n'), "no leading blank line: {notes:?}");
}

#[test]
fn a_blank_comment_is_refused() {
    // Appending an empty dated line would grow the note without saying anything.
    let (db, repo) = repo();
    let id = a_task(&db, &repo);

    let task = add_comment(&repo, db.conn(), &id, "   ", on("2026-08-20")).unwrap();

    assert_eq!(task.notes, None);
}
