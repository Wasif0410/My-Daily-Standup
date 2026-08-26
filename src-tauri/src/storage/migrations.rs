//! Versioned schema migrations.
//!
//! Migrations are plain SQL files applied in order and tracked with SQLite's
//! built-in `user_version` pragma. No migration framework: the version is a
//! single integer the database already stores for us, and each migration runs
//! inside a transaction so a failure leaves the schema untouched.
//!
//! To add a migration: write `migrations/00N_description.sql`, add it to
//! `MIGRATIONS` below, and never edit a migration that has already shipped.

use rusqlite::Connection;

use super::StorageError;

/// Every migration, in ascending order. The tuple is `(version, sql)`.
const MIGRATIONS: &[(u32, &str)] = &[
    (1, include_str!("../../migrations/001_initial.sql")),
    (2, include_str!("../../migrations/002_board_windows.sql")),
    (3, include_str!("../../migrations/003_time_spent.sql")),
    (4, include_str!("../../migrations/004_ui_state.sql")),
    (5, include_str!("../../migrations/005_board_appearance.sql")),
    (6, include_str!("../../migrations/006_board_sections.sql")),
    (
        7,
        include_str!("../../migrations/007_sections_are_task_groups.sql"),
    ),
];

/// The schema version a fully migrated database reports.
pub const LATEST_VERSION: u32 = 7;

/// Applies every migration newer than the database's current `user_version`.
///
/// Safe to call on every startup: already-applied migrations are skipped, so
/// running it twice is a no-op.
pub fn run_migrations(conn: &Connection) -> Result<(), StorageError> {
    let current = schema_version(conn)?;

    for &(version, sql) in MIGRATIONS {
        if version <= current {
            continue;
        }

        // Each migration is atomic. A syntax error or constraint failure rolls
        // back rather than leaving a half-migrated schema behind.
        conn.execute_batch(&format!(
            "BEGIN;
             {sql}
             PRAGMA user_version = {version};
             COMMIT;"
        ))
        .map_err(|source| StorageError::Migration { version, source })?;
    }

    Ok(())
}

/// Reads the database's current schema version.
pub fn schema_version(conn: &Connection) -> Result<u32, StorageError> {
    let version: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    Ok(version as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_are_listed_in_ascending_order_with_no_gaps() {
        // A gap or a duplicate would silently skip a migration on some
        // databases and not others, which is very hard to debug later.
        for (index, &(version, _)) in MIGRATIONS.iter().enumerate() {
            assert_eq!(
                version,
                index as u32 + 1,
                "migration {version} is out of order or duplicated"
            );
        }
    }

    #[test]
    fn latest_version_matches_the_last_migration() {
        let highest = MIGRATIONS.last().map(|&(v, _)| v).unwrap_or(0);
        assert_eq!(
            LATEST_VERSION, highest,
            "LATEST_VERSION must track the final entry in MIGRATIONS"
        );
    }

    /// Brings a fresh connection up to `version` and stops there, so a
    /// migration can be tested against the schema it will actually meet on a
    /// user's machine rather than against a database that never held data.
    fn apply_up_to(conn: &Connection, version: u32) {
        for &(number, sql) in MIGRATIONS {
            if number > version {
                break;
            }

            conn.execute_batch(&format!(
                "BEGIN;
                 {sql}
                 PRAGMA user_version = {number};
                 COMMIT;"
            ))
            .unwrap_or_else(|error| panic!("migration {number} failed: {error}"));
        }
    }

    #[test]
    fn upgrading_from_version_six_clears_out_what_sections_used_to_be() {
        // 007 is the one destructive migration in the set, and a fresh install
        // never exercises it: the rows it removes can only exist on a database
        // that ran 006 while sections were free-text notes. This is that
        // database.
        let conn = Connection::open_in_memory().expect("open in-memory database");
        apply_up_to(&conn, 6);

        conn.execute(
            "INSERT INTO board_windows (kind, width, height, updated_at) \
             VALUES ('monthly-progress', 340, 460, '2026-01-01T00:00:00Z')",
            [],
        )
        .expect("seed a board");
        conn.execute(
            "INSERT INTO board_sections (id, board_kind, title, position, created_at) \
             VALUES ('s1', 'monthly-progress', 'Themes', 0, '2026-01-01T00:00:00Z')",
            [],
        )
        .expect("seed a section");
        conn.execute(
            "INSERT INTO board_section_items (id, section_id, text, position, created_at) \
             VALUES ('i1', 's1', 'Ship the tray', 0, '2026-01-01T00:00:00Z')",
            [],
        )
        .expect("seed an item");

        run_migrations(&conn).expect("upgrade to the latest version");

        assert_eq!(schema_version(&conn).unwrap(), LATEST_VERSION);
        let items_table: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master \
                 WHERE type = 'table' AND name = 'board_section_items'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(items_table, 0, "the items table should be gone");

        let stranded: i64 = conn
            .query_row("SELECT COUNT(*) FROM board_sections", [], |row| row.get(0))
            .unwrap();
        assert_eq!(
            stranded, 0,
            "a section on a board that groups by commitment names no column, \
             so it must not survive as a heading nothing can fill"
        );
    }

    #[test]
    fn every_migration_has_non_empty_sql() {
        for &(version, sql) in MIGRATIONS {
            assert!(
                !sql.trim().is_empty(),
                "migration {version} is empty - include_str! may point at the wrong path"
            );
        }
    }
}
