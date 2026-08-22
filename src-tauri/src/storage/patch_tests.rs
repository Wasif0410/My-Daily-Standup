//! Deserialisation tests for `TaskPatch`.
//!
//! The doubled option is the whole reason a patch can clear a field, and it is
//! easy to get wrong: by default serde maps an explicit JSON `null` onto
//! `None`, which is indistinguishable from the field being absent. That would
//! silently turn "resolve this blocker" into "change nothing".

use super::TaskPatch;

#[test]
fn an_absent_field_means_unchanged() {
    let patch: TaskPatch = serde_json::from_str(r#"{}"#).unwrap();

    assert!(
        patch.blocker.is_none(),
        "a field the client did not send must mean 'leave alone'"
    );
}

#[test]
fn an_explicit_null_means_clear() {
    let patch: TaskPatch = serde_json::from_str(r#"{"blocker": null}"#).unwrap();

    assert_eq!(
        patch.blocker,
        Some(None),
        "an explicit null must mean 'clear this field', not 'leave alone'"
    );
}

#[test]
fn a_value_means_set() {
    let patch: TaskPatch = serde_json::from_str(r#"{"blocker": "clinic closed"}"#).unwrap();

    assert_eq!(patch.blocker, Some(Some("clinic closed".to_string())));
}

#[test]
fn clearing_and_leaving_alone_are_distinguishable() {
    // The pair that matters: if these two deserialise identically, the API
    // cannot express "un-complete this task".
    let absent: TaskPatch = serde_json::from_str(r#"{}"#).unwrap();
    let explicit_null: TaskPatch = serde_json::from_str(r#"{"completedAt": null}"#).unwrap();

    assert_ne!(
        absent.completed_at, explicit_null.completed_at,
        "omitting a field and sending null must not mean the same thing"
    );
}

#[test]
fn camel_case_field_names_are_accepted() {
    // The frontend sends camelCase; Rust stores snake_case.
    let patch: TaskPatch =
        serde_json::from_str(r#"{"scheduledDate": "2026-08-21", "parentTaskId": null}"#).unwrap();

    assert_eq!(patch.scheduled_date, Some(Some("2026-08-21".to_string())));
    assert_eq!(patch.parent_task_id, Some(None));
}

#[test]
fn rollover_count_cannot_be_set_from_json() {
    // Rollover is owned by the domain layer's reschedule path (spec §10.3).
    // A client must not be able to fake the "you have moved this five times"
    // signal by patching the counter directly.
    let result: Result<TaskPatch, _> = serde_json::from_str(r#"{"rolloverCount": 99}"#);

    assert!(
        result.is_err(),
        "rollover_count is not part of the wire format and must be rejected outright"
    );
}

#[test]
fn an_unknown_field_is_rejected() {
    // A typo in a field name should fail loudly rather than silently doing
    // nothing, which is otherwise very hard to debug from the UI side.
    let result: Result<TaskPatch, _> = serde_json::from_str(r#"{"titel": "typo"}"#);

    assert!(result.is_err(), "unknown fields must be rejected");
}
