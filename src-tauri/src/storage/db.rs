//! Database handle and connection setup.

use std::path::Path;

use rusqlite::Connection;

use super::{run_migrations, StorageError};

/// An open, fully migrated database.
///
/// Holds one connection. Rust owns all database access (spec §3.6), so there
/// is no pool: commands run on Tauri's async runtime and serialise through
/// this handle.
pub struct Db {
    conn: Connection,
}

impl Db {
    /// Opens (creating if absent) the database at `path` and migrates it.
    pub fn open(path: &Path) -> Result<Self, StorageError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(path)?;
        configure(&conn, true)?;
        run_migrations(&conn)?;

        Ok(Self { conn })
    }

    /// Opens a private in-memory database and migrates it.
    ///
    /// Used by tests: independent, parallel-safe, and leaves nothing on disk.
    pub fn open_in_memory() -> Result<Self, StorageError> {
        let conn = Connection::open_in_memory()?;
        configure(&conn, false)?;
        run_migrations(&conn)?;

        Ok(Self { conn })
    }

    /// Borrows the underlying connection.
    pub fn conn(&self) -> &Connection {
        &self.conn
    }
}

/// Applies the pragmas every connection needs.
///
/// `foreign_keys` is per-connection in SQLite and defaults to OFF, so without
/// this the schema's references would be documentation rather than
/// constraints.
fn configure(conn: &Connection, on_disk: bool) -> Result<(), StorageError> {
    conn.pragma_update(None, "foreign_keys", "ON")?;

    if on_disk {
        // WAL survives a crash mid-write and lets reads proceed during a
        // write, which matters because board windows read constantly.
        // It is meaningless for an in-memory database, which has no journal
        // file to write.
        conn.pragma_update(None, "journal_mode", "WAL")?;

        // NORMAL is the recommended durability level under WAL: safe against
        // application crashes, and only at risk from a power loss mid-commit.
        conn.pragma_update(None, "synchronous", "NORMAL")?;
    }

    Ok(())
}
