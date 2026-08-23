//! Presentation state.
//!
//! Which days are expanded, and anything else about how the app is being looked
//! at rather than what is in it. A key-value table because the shape of that
//! state changes with every board, and a column per board would mean a
//! migration every time someone adds a disclosure triangle.
//!
//! Deliberately not the settings table PR 16 will add. A settings reset must
//! not also forget which day the user had open.

use rusqlite::Connection;

use super::StorageError;

/// Reads and writes presentation state.
#[derive(Debug, Default, Clone, Copy)]
pub struct UiStateRepo;

impl UiStateRepo {
    pub fn new() -> Self {
        Self
    }

    /// Reads one key. A key that has never been set is `None`, not an error —
    /// a board opening for the first time is the normal case.
    pub fn get(&self, conn: &Connection, key: &str) -> Result<Option<String>, StorageError> {
        let mut stmt = conn.prepare("SELECT value FROM ui_state WHERE key = ?1")?;
        let mut rows = stmt.query([key])?;

        match rows.next()? {
            Some(row) => Ok(Some(row.get(0)?)),
            None => Ok(None),
        }
    }

    /// Writes one key, replacing any existing value.
    ///
    /// An upsert rather than an insert: the same key is written every time the
    /// user toggles anything, and a primary-key collision would turn the second
    /// toggle into a failure.
    pub fn set(&self, conn: &Connection, key: &str, value: &str) -> Result<(), StorageError> {
        conn.execute(
            "INSERT INTO ui_state (key, value) VALUES (?1, ?2) \
             ON CONFLICT (key) DO UPDATE SET value = excluded.value",
            rusqlite::params![key, value],
        )?;

        Ok(())
    }
}
