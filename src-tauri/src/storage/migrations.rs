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
const MIGRATIONS: &[(u32, &str)] = &[(1, include_str!("../../migrations/001_initial.sql"))];

/// The schema version a fully migrated database reports.
pub const LATEST_VERSION: u32 = 1;

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
