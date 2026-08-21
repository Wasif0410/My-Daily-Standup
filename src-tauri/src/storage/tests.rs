//! Tests for the database layer.
//!
//! Every test runs against an in-memory database, so they are independent,
//! parallel-safe, and leave nothing behind.

use super::*;
use rusqlite::Connection;

fn table_exists(conn: &Connection, name: &str) -> bool {
    conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
        [name],
        |row| row.get::<_, i64>(0),
    )
    .expect("query sqlite_master")
        > 0
}

fn user_version(conn: &Connection) -> i64 {
    conn.query_row("PRAGMA user_version", [], |row| row.get(0))
        .expect("read user_version")
}

#[test]
fn fresh_database_creates_the_tasks_table() {
    let db = Db::open_in_memory().expect("open in-memory database");
    assert!(
        table_exists(db.conn(), "tasks"),
        "migrations should create the tasks table"
    );
}

#[test]
fn migrations_advance_user_version_to_latest() {
    let db = Db::open_in_memory().expect("open in-memory database");
    assert_eq!(
        user_version(db.conn()),
        LATEST_VERSION as i64,
        "user_version should equal the highest migration number"
    );
}

#[test]
fn running_migrations_twice_is_a_no_op() {
    let db = Db::open_in_memory().expect("open in-memory database");
    let before = user_version(db.conn());

    // Applying an already-applied migration must not error or duplicate work.
    run_migrations(db.conn()).expect("second migration run should succeed");

    assert_eq!(user_version(db.conn()), before);
    assert!(table_exists(db.conn(), "tasks"));
}

#[test]
fn foreign_keys_are_enforced() {
    let db = Db::open_in_memory().expect("open in-memory database");

    // parent_task_id references tasks(id); a dangling reference must fail.
    let result = db.conn().execute(
        "INSERT INTO tasks (id, title, horizon, status, source_type, parent_task_id,
                            rollover_count, created_at, updated_at)
         VALUES ('a', 'orphan', 'daily', 'planned', 'manual', 'does-not-exist', 0, '2026-08-20', '2026-08-20')",
        [],
    );

    assert!(
        result.is_err(),
        "inserting a task with a non-existent parent must be rejected"
    );
}

#[test]
fn deleting_a_parent_nulls_the_child_rather_than_deleting_it() {
    // A child task outliving its parent is valid: a daily action stays
    // meaningful even if the weekly milestone above it is removed.
    let db = Db::open_in_memory().expect("open in-memory database");
    let conn = db.conn();

    conn.execute(
        "INSERT INTO tasks (id, title, horizon, status, source_type, rollover_count, created_at, updated_at)
         VALUES ('parent', 'weekly milestone', 'weekly', 'planned', 'manual', 0, '2026-08-20', '2026-08-20')",
        [],
    )
    .expect("insert parent");

    conn.execute(
        "INSERT INTO tasks (id, title, horizon, status, source_type, parent_task_id, rollover_count, created_at, updated_at)
         VALUES ('child', 'daily action', 'daily', 'planned', 'manual', 'parent', 0, '2026-08-20', '2026-08-20')",
        [],
    )
    .expect("insert child");

    conn.execute("DELETE FROM tasks WHERE id = 'parent'", [])
        .expect("delete parent");

    let surviving: i64 = conn
        .query_row("SELECT COUNT(*) FROM tasks WHERE id = 'child'", [], |r| {
            r.get(0)
        })
        .expect("count child");
    assert_eq!(surviving, 1, "child must survive its parent's deletion");

    let parent_ref: Option<String> = conn
        .query_row(
            "SELECT parent_task_id FROM tasks WHERE id = 'child'",
            [],
            |r| r.get(0),
        )
        .expect("read child's parent_task_id");
    assert_eq!(parent_ref, None, "child's parent_task_id must be nulled");
}

#[test]
fn status_is_constrained_to_known_values() {
    let db = Db::open_in_memory().expect("open in-memory database");

    let result = db.conn().execute(
        "INSERT INTO tasks (id, title, horizon, status, source_type, rollover_count, created_at, updated_at)
         VALUES ('x', 'bad status', 'daily', 'not-a-real-status', 'manual', 0, '2026-08-20', '2026-08-20')",
        [],
    );

    assert!(
        result.is_err(),
        "an unknown status value must be rejected by the schema, not stored"
    );
}

#[test]
fn horizon_is_constrained_to_known_values() {
    let db = Db::open_in_memory().expect("open in-memory database");

    let result = db.conn().execute(
        "INSERT INTO tasks (id, title, horizon, status, source_type, rollover_count, created_at, updated_at)
         VALUES ('x', 'bad horizon', 'yearly', 'planned', 'manual', 0, '2026-08-20', '2026-08-20')",
        [],
    );

    assert!(result.is_err(), "an unknown horizon value must be rejected");
}

#[test]
fn rollover_count_defaults_to_zero() {
    let db = Db::open_in_memory().expect("open in-memory database");
    let conn = db.conn();

    conn.execute(
        "INSERT INTO tasks (id, title, horizon, status, source_type, created_at, updated_at)
         VALUES ('t', 'no explicit rollover', 'daily', 'planned', 'manual', '2026-08-20', '2026-08-20')",
        [],
    )
    .expect("insert without rollover_count");

    let count: i64 = conn
        .query_row("SELECT rollover_count FROM tasks WHERE id = 't'", [], |r| {
            r.get(0)
        })
        .expect("read rollover_count");
    assert_eq!(count, 0);
}

#[test]
fn opening_a_file_database_persists_across_connections() {
    let dir = std::env::temp_dir().join(format!("mds-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("persist.db");
    let _ = std::fs::remove_file(&path);

    {
        let db = Db::open(&path).expect("open file database");
        db.conn()
            .execute(
                "INSERT INTO tasks (id, title, horizon, status, source_type, rollover_count, created_at, updated_at)
                 VALUES ('persisted', 'survives', 'daily', 'planned', 'manual', 0, '2026-08-20', '2026-08-20')",
                [],
            )
            .expect("insert");
    }

    let reopened = Db::open(&path).expect("reopen file database");
    let found: i64 = reopened
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM tasks WHERE id = 'persisted'",
            [],
            |r| r.get(0),
        )
        .expect("count");
    assert_eq!(found, 1, "data must survive closing and reopening");

    drop(reopened);
    let _ = std::fs::remove_dir_all(&dir);
}
