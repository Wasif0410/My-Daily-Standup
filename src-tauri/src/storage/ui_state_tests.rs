//! Tests for the presentation-state store.

use super::*;

fn store() -> (Db, UiStateRepo) {
    let db = Db::open_in_memory().expect("open in-memory database");
    (db, UiStateRepo::new())
}

#[test]
fn a_missing_key_reads_as_none() {
    // Absence is not an error: a board opening for the first time is the
    // normal case, not a failure.
    let (db, repo) = store();

    let found = repo.get(db.conn(), "weekly-progress.expanded").unwrap();

    assert_eq!(found, None);
}

#[test]
fn a_value_round_trips() {
    let (db, repo) = store();

    repo.set(db.conn(), "weekly-progress.expanded", "[\"2026-08-19\"]")
        .unwrap();
    let found = repo.get(db.conn(), "weekly-progress.expanded").unwrap();

    assert_eq!(found.as_deref(), Some("[\"2026-08-19\"]"));
}

#[test]
fn setting_an_existing_key_replaces_it() {
    // An upsert, not a second row: a primary-key collision would otherwise
    // make the second save fail rather than take effect.
    let (db, repo) = store();

    repo.set(db.conn(), "k", "first").unwrap();
    repo.set(db.conn(), "k", "second").unwrap();

    assert_eq!(repo.get(db.conn(), "k").unwrap().as_deref(), Some("second"));
}

#[test]
fn an_empty_value_is_stored_rather_than_treated_as_absent() {
    // "Nothing expanded" is a real choice and must survive a restart. Reading
    // it back as absent would re-apply the default and re-open today.
    let (db, repo) = store();

    repo.set(db.conn(), "weekly-progress.expanded", "[]")
        .unwrap();

    assert_eq!(
        repo.get(db.conn(), "weekly-progress.expanded")
            .unwrap()
            .as_deref(),
        Some("[]")
    );
}

#[test]
fn keys_are_independent() {
    let (db, repo) = store();

    repo.set(db.conn(), "a", "1").unwrap();
    repo.set(db.conn(), "b", "2").unwrap();

    assert_eq!(repo.get(db.conn(), "a").unwrap().as_deref(), Some("1"));
    assert_eq!(repo.get(db.conn(), "b").unwrap().as_deref(), Some("2"));
}
