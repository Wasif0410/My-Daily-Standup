//! Tests for the task repository.
//!
//! Every test opens its own in-memory database, so they are independent and
//! parallel-safe.

use super::*;

fn repo() -> (Db, TaskRepo) {
    let db = Db::open_in_memory().expect("open in-memory database");
    let repo = TaskRepo::new();
    (db, repo)
}

fn daily(title: &str) -> NewTask {
    NewTask::new(title, TaskHorizon::Daily, TaskSource::Manual)
}

#[test]
fn create_returns_a_task_with_a_generated_id() {
    let (db, repo) = repo();

    let task = repo.create(db.conn(), daily("call the clinic")).unwrap();

    assert!(!task.id.is_empty(), "repository must generate an id");
    assert_eq!(task.title, "call the clinic");
    assert_eq!(task.horizon, TaskHorizon::Daily);
    assert_eq!(task.rollover_count, 0);
}

#[test]
fn generated_ids_are_unique() {
    let (db, repo) = repo();

    let a = repo.create(db.conn(), daily("first")).unwrap();
    let b = repo.create(db.conn(), daily("second")).unwrap();

    assert_ne!(a.id, b.id, "ids must not collide");
}

#[test]
fn get_returns_the_created_task() {
    let (db, repo) = repo();
    let created = repo.create(db.conn(), daily("read me back")).unwrap();

    let found = repo.get(db.conn(), &created.id).unwrap();

    assert_eq!(found, Some(created));
}

#[test]
fn get_returns_none_for_an_unknown_id() {
    let (db, repo) = repo();

    let found = repo.get(db.conn(), "no-such-task").unwrap();

    assert_eq!(found, None, "a missing task is None, not an error");
}

#[test]
fn every_enum_variant_survives_a_database_round_trip() {
    // The enums' string values must match the schema's CHECK constraints. A
    // mismatch only shows up at runtime, so exercise every variant.
    let (db, repo) = repo();

    for &horizon in TaskHorizon::ALL {
        for &status in TaskStatus::ALL {
            for &source in TaskSource::ALL {
                let mut input = NewTask::new("round trip", horizon, source);
                input.status = status;

                let created = repo.create(db.conn(), input).unwrap_or_else(|e| {
                    panic!("create failed for {horizon:?}/{status:?}/{source:?}: {e}")
                });
                let read = repo.get(db.conn(), &created.id).unwrap().unwrap();

                assert_eq!(read.horizon, horizon);
                assert_eq!(read.status, status);
                assert_eq!(read.source_type, source);
            }
        }
    }
}

#[test]
fn update_changes_only_the_supplied_fields() {
    let (db, repo) = repo();
    let created = repo.create(db.conn(), daily("original title")).unwrap();

    let patch = TaskPatch {
        title: Some("new title".into()),
        ..Default::default()
    };
    let updated = repo.update(db.conn(), &created.id, patch).unwrap();

    assert_eq!(updated.title, "new title");
    assert_eq!(
        updated.horizon, created.horizon,
        "horizon must be untouched"
    );
    assert_eq!(updated.status, created.status, "status must be untouched");
}

#[test]
fn update_can_clear_a_nullable_field() {
    let (db, repo) = repo();
    let created = repo.create(db.conn(), daily("blocked task")).unwrap();

    repo.update(
        db.conn(),
        &created.id,
        TaskPatch {
            blocker: Some(Some("clinic closed".into())),
            ..Default::default()
        },
    )
    .unwrap();

    // Resolving a blocker means clearing it, which must be expressible.
    let cleared = repo
        .update(
            db.conn(),
            &created.id,
            TaskPatch {
                blocker: Some(None),
                ..Default::default()
            },
        )
        .unwrap();

    assert_eq!(cleared.blocker, None);
}

#[test]
fn an_empty_patch_leaves_the_task_unchanged() {
    let (db, repo) = repo();
    let created = repo.create(db.conn(), daily("untouched")).unwrap();

    let updated = repo
        .update(db.conn(), &created.id, TaskPatch::default())
        .unwrap();

    assert_eq!(updated.title, created.title);
    assert_eq!(updated.status, created.status);
    assert_eq!(updated.rollover_count, created.rollover_count);
}

#[test]
fn update_never_changes_created_at() {
    let (db, repo) = repo();
    let created = repo.create(db.conn(), daily("immutable birthday")).unwrap();

    // Seed a distinctly old timestamp rather than relying on wall-clock drift.
    // Create and update otherwise complete inside the same millisecond, which
    // makes a naive comparison pass even when created_at is being overwritten.
    db.conn()
        .execute(
            "UPDATE tasks SET created_at = '2020-01-01T00:00:00.000000Z' WHERE id = ?1",
            [&created.id],
        )
        .unwrap();

    let updated = repo
        .update(
            db.conn(),
            &created.id,
            TaskPatch {
                title: Some("renamed".into()),
                ..Default::default()
            },
        )
        .unwrap();

    assert_eq!(
        updated.created_at, "2020-01-01T00:00:00.000000Z",
        "created_at is immutable and must survive an update untouched"
    );
}

#[test]
fn update_refreshes_updated_at() {
    let (db, repo) = repo();
    let created = repo.create(db.conn(), daily("touch me")).unwrap();

    // Same reasoning as above: seed a known old value so the assertion does
    // not depend on two operations landing in different milliseconds.
    db.conn()
        .execute(
            "UPDATE tasks SET updated_at = '2020-01-01T00:00:00.000000Z' WHERE id = ?1",
            [&created.id],
        )
        .unwrap();

    let updated = repo
        .update(db.conn(), &created.id, TaskPatch::default())
        .unwrap();

    assert_ne!(
        updated.updated_at, "2020-01-01T00:00:00.000000Z",
        "the repository must refresh updated_at on every write"
    );
}

#[test]
fn update_of_an_unknown_id_reports_not_found() {
    let (db, repo) = repo();

    let result = repo.update(db.conn(), "no-such-task", TaskPatch::default());

    assert!(
        matches!(result, Err(StorageError::TaskNotFound { .. })),
        "updating a missing task must be a distinct error, not a silent no-op"
    );
}

#[test]
fn delete_removes_the_task() {
    let (db, repo) = repo();
    let created = repo.create(db.conn(), daily("temporary")).unwrap();

    repo.delete(db.conn(), &created.id).unwrap();

    assert_eq!(repo.get(db.conn(), &created.id).unwrap(), None);
}

#[test]
fn delete_of_an_unknown_id_reports_not_found() {
    let (db, repo) = repo();

    let result = repo.delete(db.conn(), "no-such-task");

    assert!(matches!(result, Err(StorageError::TaskNotFound { .. })));
}

#[test]
fn deleting_a_parent_keeps_the_child_and_nulls_its_link() {
    let (db, repo) = repo();

    let parent = repo
        .create(
            db.conn(),
            NewTask::new("weekly milestone", TaskHorizon::Weekly, TaskSource::Manual),
        )
        .unwrap();

    let mut child_input = daily("daily action");
    child_input.parent_task_id = Some(parent.id.clone());
    let child = repo.create(db.conn(), child_input).unwrap();

    repo.delete(db.conn(), &parent.id).unwrap();

    let surviving = repo.get(db.conn(), &child.id).unwrap();
    assert!(surviving.is_some(), "child must outlive its parent");
    assert_eq!(
        surviving.unwrap().parent_task_id,
        None,
        "the dangling link must be nulled"
    );
}

#[test]
fn list_by_horizon_returns_only_that_horizon() {
    let (db, repo) = repo();

    repo.create(db.conn(), daily("a daily task")).unwrap();
    repo.create(
        db.conn(),
        NewTask::new("a weekly task", TaskHorizon::Weekly, TaskSource::Manual),
    )
    .unwrap();

    let daily_tasks = repo.list_by_horizon(db.conn(), TaskHorizon::Daily).unwrap();

    assert_eq!(daily_tasks.len(), 1);
    assert_eq!(daily_tasks[0].title, "a daily task");
}

#[test]
fn list_for_date_matches_the_scheduled_date_exactly() {
    let (db, repo) = repo();

    let mut today = daily("today");
    today.scheduled_date = Some("2026-08-20".into());
    repo.create(db.conn(), today).unwrap();

    let mut tomorrow = daily("tomorrow");
    tomorrow.scheduled_date = Some("2026-08-21".into());
    repo.create(db.conn(), tomorrow).unwrap();

    let found = repo.list_for_date(db.conn(), "2026-08-20").unwrap();

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].title, "today");
}

#[test]
fn list_for_period_includes_tasks_overlapping_the_range() {
    let (db, repo) = repo();

    let mut inside = NewTask::new("this week", TaskHorizon::Weekly, TaskSource::Manual);
    inside.period_start = Some("2026-08-17".into());
    inside.period_end = Some("2026-08-23".into());
    repo.create(db.conn(), inside).unwrap();

    let mut outside = NewTask::new("next month", TaskHorizon::Weekly, TaskSource::Manual);
    outside.period_start = Some("2026-09-14".into());
    outside.period_end = Some("2026-09-20".into());
    repo.create(db.conn(), outside).unwrap();

    let found = repo
        .list_for_period(db.conn(), "2026-08-17", "2026-08-23")
        .unwrap();

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].title, "this week");
}

#[test]
fn children_of_returns_direct_children_only() {
    let (db, repo) = repo();

    let parent = repo
        .create(
            db.conn(),
            NewTask::new("monthly", TaskHorizon::Monthly, TaskSource::Manual),
        )
        .unwrap();

    let mut child_input = NewTask::new("weekly", TaskHorizon::Weekly, TaskSource::Manual);
    child_input.parent_task_id = Some(parent.id.clone());
    let child = repo.create(db.conn(), child_input).unwrap();

    // A grandchild must not appear in the parent's direct children.
    let mut grandchild_input = daily("daily");
    grandchild_input.parent_task_id = Some(child.id.clone());
    repo.create(db.conn(), grandchild_input).unwrap();

    let children = repo.children_of(db.conn(), &parent.id).unwrap();

    assert_eq!(children.len(), 1);
    assert_eq!(children[0].title, "weekly");
}

#[test]
fn creating_a_task_under_a_missing_parent_is_rejected() {
    // The schema enforces this; the repository must surface it rather than
    // silently storing a dangling reference.
    let (db, repo) = repo();

    let mut orphan = daily("orphan");
    orphan.parent_task_id = Some("no-such-parent".into());

    assert!(repo.create(db.conn(), orphan).is_err());
}

#[test]
fn timestamps_are_iso_8601() {
    let (db, repo) = repo();
    let task = repo.create(db.conn(), daily("timestamped")).unwrap();

    // Dates sort correctly as text only in this format, which the board
    // queries rely on.
    assert!(
        task.created_at.len() >= 20 && task.created_at.contains('T'),
        "expected an ISO-8601 timestamp, got: {}",
        task.created_at
    );
    assert!(task.created_at.ends_with('Z'), "timestamps must be UTC");
}
