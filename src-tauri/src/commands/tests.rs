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

// --- settings (spec §18) ------------------------------------------------------

/// What the autostart effect was asked to do, in order.
type Calls = std::rc::Rc<std::cell::RefCell<Vec<bool>>>;

fn calls() -> Calls {
    std::rc::Rc::new(std::cell::RefCell::new(Vec::new()))
}

/// An autostart effect that succeeds and records what it was asked for.
///
/// The plugin call needs a live `AppHandle`, so the command layer takes the
/// effect as an argument instead. That is what lets the ordering — the part
/// this PR exists to get right — be tested at all.
fn recording(into: &Calls) -> impl Fn(bool) -> Result<(), CommandError> {
    let sink = std::rc::Rc::clone(into);

    move |enabled: bool| {
        sink.borrow_mut().push(enabled);
        Ok(())
    }
}

fn refusing(_: bool) -> Result<(), CommandError> {
    Err(CommandError {
        kind: ErrorKind::Internal,
        message: "the autostart plugin refused".to_string(),
    })
}

#[test]
fn settings_start_at_their_defaults() {
    let state = state();

    let settings = state.settings().unwrap();

    assert_eq!(settings, crate::storage::Settings::default());
}

#[test]
fn updating_settings_returns_the_stored_result() {
    let state = state();
    let effect = recording(&calls());

    let updated = state
        .update_settings(
            SettingsPatch {
                priority_threshold: Some(7),
                ..Default::default()
            },
            effect,
        )
        .unwrap();

    assert_eq!(updated.priority_threshold, 7);
    assert_eq!(state.settings().unwrap(), updated);
}

#[test]
fn an_invalid_setting_reports_invalid_input_rather_than_a_storage_failure() {
    // The frontend branches on kind: a number the user can correct must not
    // arrive looking like a broken database.
    let state = state();
    let calls = calls();
    let effect = recording(&calls);

    let error = state
        .update_settings(
            SettingsPatch {
                priority_threshold: Some(42),
                launch_at_login: Some(true),
                ..Default::default()
            },
            effect,
        )
        .unwrap_err();

    assert_eq!(error.kind, ErrorKind::InvalidInput);
    assert!(
        calls.borrow().is_empty(),
        "a patch that will be refused must not switch autostart on along the way"
    );
}

#[test]
fn turning_launch_at_login_on_reaches_the_autostart_plugin() {
    let state = state();
    let calls = calls();
    let effect = recording(&calls);

    state
        .update_settings(
            SettingsPatch {
                launch_at_login: Some(true),
                ..Default::default()
            },
            effect,
        )
        .unwrap();

    assert_eq!(
        *calls.borrow(),
        vec![true],
        "a stored flag with no plugin call behind it is the bug this PR removes"
    );
}

#[test]
fn launch_at_login_is_re_asserted_even_when_it_matches_the_stored_value() {
    // Skipping a call that "changes nothing" assumes the row and the OS
    // already agree, which is the assumption this PR exists to stop making. If
    // they have drifted, the one save the user makes to fix it would be the
    // save that did nothing.
    let state = state();
    let calls = calls();
    let effect = recording(&calls);

    state
        .update_settings(
            SettingsPatch {
                launch_at_login: Some(false),
                priority_threshold: Some(3),
                ..Default::default()
            },
            effect,
        )
        .unwrap();

    assert_eq!(*calls.borrow(), vec![false]);
}

#[test]
fn a_patch_that_says_nothing_about_autostart_leaves_the_plugin_alone() {
    // Saving a threshold must not touch a login entry.
    let state = state();
    let calls = calls();
    let effect = recording(&calls);

    state
        .update_settings(
            SettingsPatch {
                priority_threshold: Some(3),
                ..Default::default()
            },
            effect,
        )
        .unwrap();

    assert!(calls.borrow().is_empty());
}

#[test]
fn a_failed_autostart_call_fails_the_whole_update() {
    // The ordering decision: the plugin runs first, and the row is only
    // written once it has succeeded. A stored `true` beside a disabled
    // autostart is exactly the lie this PR exists to remove.
    let state = state();

    let error = state
        .update_settings(
            SettingsPatch {
                launch_at_login: Some(true),
                priority_threshold: Some(9),
                ..Default::default()
            },
            refusing,
        )
        .unwrap_err();

    assert_eq!(error.kind, ErrorKind::Internal);
    assert_eq!(
        state.settings().unwrap(),
        crate::storage::Settings::default(),
        "nothing in the patch may survive a refused autostart call"
    );
}

#[test]
fn the_stored_week_start_day_reaches_week_current() {
    // The setting was accepted long before anything read it, which left every
    // week permanently Monday.
    let state = state();
    let effect = recording(&calls());

    state
        .update_settings(
            SettingsPatch {
                week_starts_on: Some("sunday".to_string()),
                ..Default::default()
            },
            effect,
        )
        .unwrap();

    assert_eq!(state.current_week(None).days[0].name, "Sunday");
}

#[test]
fn an_explicit_week_start_day_still_overrides_the_setting() {
    let state = state();
    let effect = recording(&calls());

    state
        .update_settings(
            SettingsPatch {
                week_starts_on: Some("sunday".to_string()),
                ..Default::default()
            },
            effect,
        )
        .unwrap();

    assert_eq!(
        state.current_week(Some("saturday")).days[0].name,
        "Saturday"
    );
}

#[test]
fn week_current_defaults_to_monday_before_anything_is_configured() {
    let state = state();

    assert_eq!(state.current_week(None).days[0].name, "Monday");
}

#[test]
fn settings_serialise_to_camel_case_for_the_frontend() {
    let state = state();

    let json = serde_json::to_value(state.settings().unwrap()).unwrap();

    assert!(json.get("priorityThreshold").is_some());
    assert!(json.get("weekStartsOn").is_some());
    assert!(json.get("launchAtLogin").is_some());
    assert!(
        json.get("priority_threshold").is_none(),
        "snake_case must not leak across the boundary"
    );
}
