//! Tests for the IPC boundary.
//!
//! These exercise `AppState` directly rather than through a Tauri runtime,
//! which is the point of keeping the command functions thin.

use super::*;
use crate::storage::{TaskSource, TaskStatus};

fn state() -> AppState {
    AppState::in_memory().expect("open in-memory state")
}

fn daily(title: &str) -> NewTask {
    NewTask::new(title, TaskHorizon::Daily, TaskSource::Manual)
}

#[test]
fn a_task_round_trips_through_the_command_layer() {
    let state = state();

    let created = state.create_task(daily("call the clinic")).unwrap();
    let fetched = state.get_task(&created.id).unwrap();

    assert_eq!(fetched, Some(created));
}

#[test]
fn a_missing_task_is_none_rather_than_an_error() {
    let state = state();

    assert_eq!(state.get_task("no-such-task").unwrap(), None);
}

#[test]
fn updating_a_missing_task_reports_not_found() {
    let state = state();

    let error = state
        .update_task("no-such-task", TaskPatch::default())
        .unwrap_err();

    assert_eq!(
        error.kind,
        ErrorKind::NotFound,
        "the frontend branches on kind, so it must be NotFound rather than a generic failure"
    );
}

#[test]
fn deleting_a_missing_task_reports_not_found() {
    let state = state();

    let error = state.delete_task("no-such-task").unwrap_err();

    assert_eq!(error.kind, ErrorKind::NotFound);
}

#[test]
fn a_constraint_violation_reports_storage_not_not_found() {
    // A dangling parent is a schema rejection, not a missing task. Collapsing
    // the two would make the frontend show the wrong message.
    let state = state();

    let mut orphan = daily("orphan");
    orphan.parent_task_id = Some("no-such-parent".into());

    let error = state.create_task(orphan).unwrap_err();

    assert_eq!(error.kind, ErrorKind::Storage);
}

#[test]
fn errors_serialise_with_a_kind_the_frontend_can_branch_on() {
    let state = state();
    let error = state.delete_task("no-such-task").unwrap_err();

    let json = serde_json::to_value(&error).unwrap();

    assert_eq!(json["kind"], "not-found");
    assert!(
        json["message"].as_str().is_some_and(|m| !m.is_empty()),
        "an error must carry a human-readable message alongside its kind"
    );
}

#[test]
fn update_applies_a_patch_through_the_command_layer() {
    let state = state();
    let created = state.create_task(daily("original")).unwrap();

    let updated = state
        .update_task(
            &created.id,
            TaskPatch {
                title: Some("renamed".into()),
                status: Some(TaskStatus::Completed),
                ..Default::default()
            },
        )
        .unwrap();

    assert_eq!(updated.title, "renamed");
    assert_eq!(updated.status, TaskStatus::Completed);
}

#[test]
fn a_patch_from_json_can_clear_a_field_end_to_end() {
    // The doubled-option handling has to survive the whole path, not just the
    // deserialiser: set a blocker, then clear it with an explicit null.
    let state = state();
    let created = state.create_task(daily("blocked")).unwrap();

    let set: TaskPatch = serde_json::from_str(r#"{"blocker": "clinic closed"}"#).unwrap();
    let blocked = state.update_task(&created.id, set).unwrap();
    assert_eq!(blocked.blocker.as_deref(), Some("clinic closed"));

    let clear: TaskPatch = serde_json::from_str(r#"{"blocker": null}"#).unwrap();
    let resolved = state.update_task(&created.id, clear).unwrap();
    assert_eq!(
        resolved.blocker, None,
        "an explicit null must clear the field"
    );
}

#[test]
fn listing_filters_by_horizon() {
    let state = state();

    state.create_task(daily("a daily task")).unwrap();
    state
        .create_task(NewTask::new(
            "a weekly task",
            TaskHorizon::Weekly,
            TaskSource::Manual,
        ))
        .unwrap();

    let dailies = state.list_by_horizon(TaskHorizon::Daily).unwrap();

    assert_eq!(dailies.len(), 1);
    assert_eq!(dailies[0].title, "a daily task");
}

#[test]
fn children_are_reachable_through_the_command_layer() {
    let state = state();

    let parent = state
        .create_task(NewTask::new(
            "weekly milestone",
            TaskHorizon::Weekly,
            TaskSource::Manual,
        ))
        .unwrap();

    let mut child = daily("daily action");
    child.parent_task_id = Some(parent.id.clone());
    state.create_task(child).unwrap();

    let children = state.children_of(&parent.id).unwrap();

    assert_eq!(children.len(), 1);
    assert_eq!(children[0].title, "daily action");
}

#[test]
fn a_task_serialises_to_camel_case_for_the_frontend() {
    // The TypeScript interface in src/types/task.ts mirrors these names. If the
    // casing drifts, the frontend silently reads undefined.
    let state = state();
    let mut input = daily("casing check");
    input.scheduled_date = Some("2026-08-21".into());
    let task = state.create_task(input).unwrap();

    let json = serde_json::to_value(&task).unwrap();

    assert!(
        json.get("scheduledDate").is_some(),
        "expected camelCase keys"
    );
    assert!(json.get("rolloverCount").is_some());
    assert!(json.get("sourceType").is_some());
    assert!(
        json.get("scheduled_date").is_none(),
        "snake_case must not leak across the boundary"
    );
}
