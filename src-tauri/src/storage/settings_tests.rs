//! Tests for the settings row.
//!
//! Every rule here is one the schema also enforces. That is deliberate: the
//! CHECK constraints are the backstop, and these cover the wording and the
//! behaviour a user would meet before ever reaching them.

use super::*;

fn setup() -> (Db, SettingsRepo) {
    (
        Db::open_in_memory().expect("open in-memory database"),
        SettingsRepo::new(),
    )
}

/// The stored timestamp, which `Settings` deliberately does not carry.
///
/// Read straight from the column rather than through the repository: nothing
/// in the app needs this value, and exposing it as API to satisfy a test would
/// be the same mistake as shipping a setting nothing reads.
fn stored_updated_at(db: &Db) -> String {
    db.conn()
        .query_row("SELECT updated_at FROM settings WHERE id = 1", [], |row| {
            row.get(0)
        })
        .expect("the migration seeds the single row")
}

#[test]
fn a_fresh_database_reads_the_seeded_defaults() {
    // The migration seeds row 1, so the very first read is an ordinary read
    // rather than a special case anyone has to remember.
    let (db, repo) = setup();

    let settings = repo.get(db.conn()).unwrap();

    assert_eq!(settings, Settings::default());
    assert_eq!(settings.priority_threshold, 5);
    assert_eq!(settings.week_starts_on, "monday");
    assert!(!settings.launch_at_login);
}

#[test]
fn reading_with_the_row_missing_still_yields_the_defaults() {
    // Nothing in the app deletes this row, which is exactly why the read has
    // to survive its absence: a database that lost it for any reason must
    // still open every board rather than failing at startup.
    let (db, repo) = setup();

    db.conn()
        .execute("DELETE FROM settings", [])
        .expect("delete the settings row");

    assert_eq!(repo.get(db.conn()).unwrap(), Settings::default());
}

#[test]
fn a_priority_threshold_round_trips() {
    let (db, repo) = setup();

    let applied = repo
        .apply(
            db.conn(),
            SettingsPatch {
                priority_threshold: Some(8),
                ..Default::default()
            },
        )
        .unwrap();

    assert_eq!(applied.priority_threshold, 8);
    assert_eq!(repo.get(db.conn()).unwrap().priority_threshold, 8);
}

#[test]
fn both_ends_of_the_priority_range_are_accepted() {
    // The bounds are inclusive. An off-by-one here would refuse "show me
    // everything" and "show me only the top", which are the two settings
    // anybody actually reaches for.
    let (db, repo) = setup();

    for threshold in [0, 10] {
        let applied = repo
            .apply(
                db.conn(),
                SettingsPatch {
                    priority_threshold: Some(threshold),
                    ..Default::default()
                },
            )
            .unwrap();

        assert_eq!(applied.priority_threshold, threshold);
    }
}

#[test]
fn an_out_of_range_priority_threshold_is_rejected_rather_than_clamped() {
    // Silently changing the number is worse than refusing it: the user typed
    // something specific, and a board quietly filtered at 10 when they asked
    // for 50 looks empty for no reason they can see.
    let (db, repo) = setup();

    for threshold in [-1, 11, 50] {
        let error = repo
            .apply(
                db.conn(),
                SettingsPatch {
                    priority_threshold: Some(threshold),
                    ..Default::default()
                },
            )
            .unwrap_err();

        assert!(
            matches!(error, StorageError::Validation { .. }),
            "{threshold} should be refused as validation, not as a constraint failure: {error}"
        );
        assert_eq!(
            repo.get(db.conn()).unwrap().priority_threshold,
            5,
            "a refused patch must leave the stored value alone"
        );
    }
}

#[test]
fn every_valid_week_start_day_round_trips() {
    let (db, repo) = setup();

    for day in ["monday", "sunday", "saturday"] {
        let applied = repo
            .apply(
                db.conn(),
                SettingsPatch {
                    week_starts_on: Some(day.to_string()),
                    ..Default::default()
                },
            )
            .unwrap();

        assert_eq!(applied.week_starts_on, day);
        assert_eq!(repo.get(db.conn()).unwrap().week_starts_on, day);
    }
}

#[test]
fn an_unknown_week_start_day_is_rejected() {
    let (db, repo) = setup();

    for day in ["tuesday", "", "mon"] {
        let error = repo
            .apply(
                db.conn(),
                SettingsPatch {
                    week_starts_on: Some(day.to_string()),
                    ..Default::default()
                },
            )
            .unwrap_err();

        assert!(
            matches!(error, StorageError::Validation { .. }),
            "{day:?} should be refused as validation: {error}"
        );
    }

    assert_eq!(repo.get(db.conn()).unwrap().week_starts_on, "monday");
}

#[test]
fn a_week_start_day_is_normalised_before_it_is_stored() {
    // The three values are also a CHECK constraint, so a stray capital would
    // otherwise reach SQLite and come back as its wording rather than ours.
    // Folding is friendlier than refusing "Monday", which is how anyone would
    // write it.
    let (db, repo) = setup();

    let applied = repo
        .apply(
            db.conn(),
            SettingsPatch {
                week_starts_on: Some("  Sunday ".to_string()),
                ..Default::default()
            },
        )
        .unwrap();

    assert_eq!(applied.week_starts_on, "sunday");
}

#[test]
fn launch_at_login_round_trips() {
    let (db, repo) = setup();

    let applied = repo
        .apply(
            db.conn(),
            SettingsPatch {
                launch_at_login: Some(true),
                ..Default::default()
            },
        )
        .unwrap();

    assert!(applied.launch_at_login);
    assert!(repo.get(db.conn()).unwrap().launch_at_login);
}

#[test]
fn an_omitted_field_is_left_alone() {
    // The whole point of a patch: a settings window saving one control must
    // not reset the two the user was not looking at.
    let (db, repo) = setup();

    repo.apply(
        db.conn(),
        SettingsPatch {
            priority_threshold: Some(9),
            week_starts_on: Some("saturday".to_string()),
            launch_at_login: Some(true),
        },
    )
    .unwrap();

    let applied = repo
        .apply(
            db.conn(),
            SettingsPatch {
                priority_threshold: Some(2),
                ..Default::default()
            },
        )
        .unwrap();

    assert_eq!(applied.priority_threshold, 2);
    assert_eq!(applied.week_starts_on, "saturday");
    assert!(applied.launch_at_login);
}

#[test]
fn an_empty_patch_is_a_no_op_that_still_returns_the_current_settings() {
    let (db, repo) = setup();

    repo.apply(
        db.conn(),
        SettingsPatch {
            priority_threshold: Some(7),
            ..Default::default()
        },
    )
    .unwrap();
    let before = stored_updated_at(&db);

    let returned = repo.apply(db.conn(), SettingsPatch::default()).unwrap();

    assert_eq!(returned, repo.get(db.conn()).unwrap());
    assert_eq!(returned.priority_threshold, 7);
    assert_eq!(
        stored_updated_at(&db),
        before,
        "nothing changed, so nothing was updated"
    );
}

#[test]
fn applying_a_change_moves_updated_at() {
    let (db, repo) = setup();
    let before = stored_updated_at(&db);

    repo.apply(
        db.conn(),
        SettingsPatch {
            launch_at_login: Some(true),
            ..Default::default()
        },
    )
    .unwrap();

    assert!(
        stored_updated_at(&db) > before,
        "updated_at must move on every applied change"
    );
}

#[test]
fn a_rejected_patch_does_not_apply_its_valid_fields() {
    // A patch is one edit. Half-applying it would leave the user looking at a
    // rejection message beside a board that had already changed.
    let (db, repo) = setup();

    repo.apply(
        db.conn(),
        SettingsPatch {
            priority_threshold: Some(99),
            week_starts_on: Some("sunday".to_string()),
            launch_at_login: Some(true),
        },
    )
    .unwrap_err();

    assert_eq!(repo.get(db.conn()).unwrap(), Settings::default());
}

#[test]
fn applying_to_a_missing_row_writes_the_defaults_back() {
    // The single row is the schema's invariant, not something a write may
    // assume is already there. Recreating it is what keeps a database that
    // somehow lost it repairable by using the app.
    let (db, repo) = setup();
    db.conn()
        .execute("DELETE FROM settings", [])
        .expect("delete the settings row");

    let applied = repo
        .apply(
            db.conn(),
            SettingsPatch {
                priority_threshold: Some(3),
                ..Default::default()
            },
        )
        .unwrap();

    assert_eq!(applied.priority_threshold, 3);
    assert_eq!(
        applied.week_starts_on, "monday",
        "the fields the patch did not name come back as their defaults"
    );
    assert_eq!(repo.get(db.conn()).unwrap(), applied);
}

#[test]
fn settings_are_deserialised_from_camel_case() {
    // The frontend speaks camelCase across the IPC boundary; Rust and SQL
    // speak snake_case. A mismatch here fails at runtime, not compile time.
    let patch: SettingsPatch = serde_json::from_str(
        r#"{"priorityThreshold": 4, "weekStartsOn": "sunday", "launchAtLogin": true}"#,
    )
    .expect("parse a camelCase patch");

    assert_eq!(patch.priority_threshold, Some(4));
    assert_eq!(patch.week_starts_on.as_deref(), Some("sunday"));
    assert_eq!(patch.launch_at_login, Some(true));
}

#[test]
fn an_absent_json_field_deserialises_as_leave_alone() {
    let patch: SettingsPatch = serde_json::from_str("{}").expect("parse an empty patch");

    assert_eq!(patch.priority_threshold, None);
    assert_eq!(patch.week_starts_on, None);
    assert_eq!(patch.launch_at_login, None);
}
