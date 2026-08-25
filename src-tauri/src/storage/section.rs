//! User-created sections on a board.
//!
//! A section is a heading the user typed and a list of free-text bullets
//! underneath it. Deliberately not modelled as tasks: a task carries a
//! horizon, a status and a rollover count because the planning engine reasons
//! about it, and none of that is true of a line someone wanted written on a
//! sticky note. Keeping them apart means neither has to pretend to be the
//! other, and no board query has to filter out invented statuses.
//!
//! Like [`super::task_repo`], this is pure data access with no Tauri coupling,
//! which is what makes it unit-testable (spec §3.6).

use rusqlite::{Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};

use super::{BoardKind, BoardRepo, BoardWindow, StorageError};

/// Longest section title, in characters.
///
/// Eighty is roughly a line of prose. A heading longer than its own board is
/// not a heading, and the width it would need is width taken from the items
/// it exists to introduce.
pub const MAX_TITLE_CHARS: usize = 80;

/// Longest item text, in characters.
///
/// Generous enough for a sentence or two of context — the point of a
/// free-text bullet — while still bounding what a single board has to render.
pub const MAX_ITEM_CHARS: usize = 500;

/// One free-text bullet inside a section.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SectionItem {
    pub id: String,
    pub section_id: String,
    pub text: String,
    pub position: i64,
}

/// A named section on one board, with the items it holds.
///
/// Items travel with the section rather than being fetched separately: a
/// board renders a section and its bullets together, and two round trips are
/// two chances to draw a heading whose contents have not arrived yet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardSection {
    pub id: String,
    pub board_kind: BoardKind,
    pub title: String,
    pub position: i64,
    pub items: Vec<SectionItem>,
}

/// Reads and writes board sections and their items.
#[derive(Debug, Default, Clone, Copy)]
pub struct SectionRepo;

impl SectionRepo {
    pub fn new() -> Self {
        Self
    }

    /// Every section on one board, in the order they are shown, each carrying
    /// its own items in theirs.
    ///
    /// Two queries rather than a join: a join repeats each section's title
    /// once per item and returns nothing at all for a section that is still
    /// empty, which is exactly the state a section spends its first moments in.
    pub fn list(
        &self,
        conn: &Connection,
        board_kind: BoardKind,
    ) -> Result<Vec<BoardSection>, StorageError> {
        let mut stmt = conn.prepare(
            "SELECT id, board_kind, title, position FROM board_sections \
             WHERE board_kind = ?1 ORDER BY position",
        )?;
        let mut rows = stmt.query(rusqlite::params![board_kind])?;

        let mut sections = Vec::new();
        while let Some(row) = rows.next()? {
            sections.push(section_from_row(row)?);
        }

        for section in &mut sections {
            section.items = self.items_of(conn, &section.id)?;
        }

        Ok(sections)
    }

    /// One section with its items, or `None` if it has been deleted.
    ///
    /// A missing section is not an error here: a board holding a stale id
    /// after a delete elsewhere is a normal race, not a fault.
    pub fn get(&self, conn: &Connection, id: &str) -> Result<Option<BoardSection>, StorageError> {
        let mut stmt = conn
            .prepare("SELECT id, board_kind, title, position FROM board_sections WHERE id = ?1")?;
        let mut rows = stmt.query([id])?;

        let Some(row) = rows.next()? else {
            return Ok(None);
        };
        let mut section = section_from_row(row)?;
        drop(rows);
        drop(stmt);

        section.items = self.items_of(conn, id)?;
        Ok(Some(section))
    }

    /// Creates an empty section at the end of a board.
    ///
    /// Appended rather than inserted at the top: the user is adding to a list
    /// they are already looking at, and pushing everything else down would
    /// move the thing they were reading.
    pub fn create(
        &self,
        conn: &Connection,
        board_kind: BoardKind,
        title: &str,
    ) -> Result<BoardSection, StorageError> {
        let title = validated(title, MAX_TITLE_CHARS, "A section title")?;
        ensure_board_row(conn, board_kind)?;

        let id = uuid::Uuid::new_v4().to_string();
        let position = next_position(
            conn,
            "SELECT MAX(position) FROM board_sections WHERE board_kind = ?1",
            rusqlite::params![board_kind],
        )?;

        conn.execute(
            "INSERT INTO board_sections (id, board_kind, title, position, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![id, board_kind, title, position, now_iso8601()],
        )?;

        self.get(conn, &id)?
            .ok_or_else(|| StorageError::SectionNotFound { id })
    }

    /// Retitles a section.
    ///
    /// Held to the same rules as [`Self::create`]: a rename is the same act as
    /// naming, so a title that could not have been created must not be
    /// reachable by editing one that could.
    pub fn rename(
        &self,
        conn: &Connection,
        id: &str,
        title: &str,
    ) -> Result<BoardSection, StorageError> {
        let title = validated(title, MAX_TITLE_CHARS, "A section title")?;

        let affected = conn.execute(
            "UPDATE board_sections SET title = ?2 WHERE id = ?1",
            rusqlite::params![id, title],
        )?;
        if affected == 0 {
            return Err(StorageError::SectionNotFound { id: id.to_string() });
        }

        self.get(conn, id)?
            .ok_or_else(|| StorageError::SectionNotFound { id: id.to_string() })
    }

    /// Deletes a section and everything in it.
    ///
    /// The items go by the schema's `ON DELETE CASCADE`, which fires because
    /// every connection this app opens sets `PRAGMA foreign_keys = ON` (see
    /// [`super::db`]). An item outside its section can never be rendered, so
    /// leaving them would only grow the file with rows nothing can reach.
    pub fn delete(&self, conn: &Connection, id: &str) -> Result<(), StorageError> {
        let affected = conn.execute("DELETE FROM board_sections WHERE id = ?1", [id])?;

        if affected == 0 {
            return Err(StorageError::SectionNotFound { id: id.to_string() });
        }

        Ok(())
    }

    /// Appends one bullet to a section.
    pub fn add_item(
        &self,
        conn: &Connection,
        section_id: &str,
        text: &str,
    ) -> Result<SectionItem, StorageError> {
        let text = validated(text, MAX_ITEM_CHARS, "An item")?;

        // Checked here rather than left to the foreign key so the caller is
        // told which thing was missing, instead of being handed SQLite's
        // wording for a constraint it never knew about.
        if !section_exists(conn, section_id)? {
            return Err(StorageError::SectionNotFound {
                id: section_id.to_string(),
            });
        }

        let id = uuid::Uuid::new_v4().to_string();
        let position = next_position(
            conn,
            "SELECT MAX(position) FROM board_section_items WHERE section_id = ?1",
            rusqlite::params![section_id],
        )?;

        conn.execute(
            "INSERT INTO board_section_items (id, section_id, text, position, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![id, section_id, text, position, now_iso8601()],
        )?;

        self.item(conn, &id)?
            .ok_or_else(|| StorageError::ItemNotFound { id })
    }

    /// Rewrites one bullet's text, leaving its place in the list alone.
    pub fn update_item(
        &self,
        conn: &Connection,
        id: &str,
        text: &str,
    ) -> Result<SectionItem, StorageError> {
        let text = validated(text, MAX_ITEM_CHARS, "An item")?;

        let affected = conn.execute(
            "UPDATE board_section_items SET text = ?2 WHERE id = ?1",
            rusqlite::params![id, text],
        )?;
        if affected == 0 {
            return Err(StorageError::ItemNotFound { id: id.to_string() });
        }

        self.item(conn, id)?
            .ok_or_else(|| StorageError::ItemNotFound { id: id.to_string() })
    }

    /// Deletes one bullet.
    pub fn delete_item(&self, conn: &Connection, id: &str) -> Result<(), StorageError> {
        let affected = conn.execute("DELETE FROM board_section_items WHERE id = ?1", [id])?;

        if affected == 0 {
            return Err(StorageError::ItemNotFound { id: id.to_string() });
        }

        Ok(())
    }

    fn item(&self, conn: &Connection, id: &str) -> Result<Option<SectionItem>, StorageError> {
        let mut stmt = conn.prepare(
            "SELECT id, section_id, text, position FROM board_section_items WHERE id = ?1",
        )?;
        let mut rows = stmt.query([id])?;

        match rows.next()? {
            Some(row) => Ok(Some(item_from_row(row)?)),
            None => Ok(None),
        }
    }

    fn items_of(
        &self,
        conn: &Connection,
        section_id: &str,
    ) -> Result<Vec<SectionItem>, StorageError> {
        let mut stmt = conn.prepare(
            "SELECT id, section_id, text, position FROM board_section_items \
             WHERE section_id = ?1 ORDER BY position",
        )?;
        let mut rows = stmt.query([section_id])?;

        let mut items = Vec::new();
        while let Some(row) = rows.next()? {
            items.push(item_from_row(row)?);
        }

        Ok(items)
    }
}

/// Trims `value` and rejects it if nothing useful survives, or if it is longer
/// than `max` characters.
///
/// Over-length is refused rather than shortened. Truncating would store words
/// the user did not write and drop ones they did, without telling them which.
///
/// Counted in characters, not bytes: a byte cap would reject a note of
/// perfectly ordinary length written in any language that does not fit in
/// ASCII.
fn validated(value: &str, max: usize, subject: &str) -> Result<String, StorageError> {
    let trimmed = value.trim();

    if trimmed.is_empty() {
        return Err(StorageError::Validation {
            message: format!("{subject} cannot be blank"),
        });
    }

    let length = trimmed.chars().count();
    if length > max {
        return Err(StorageError::Validation {
            message: format!("{subject} cannot be longer than {max} characters (got {length})"),
        });
    }

    Ok(trimmed.to_string())
}

/// The position a newly appended row takes.
///
/// Derived from the current maximum rather than from a count, so deleting
/// something in the middle cannot make the next addition collide with a
/// position that is already taken.
fn next_position(
    conn: &Connection,
    sql: &str,
    params: &[&dyn rusqlite::ToSql],
) -> Result<i64, StorageError> {
    let highest: Option<i64> = conn.query_row(sql, params, |row| row.get(0))?;
    Ok(highest.map_or(0, |value| value + 1))
}

fn section_exists(conn: &Connection, id: &str) -> Result<bool, StorageError> {
    let found: Option<i64> = conn
        .query_row("SELECT 1 FROM board_sections WHERE id = ?1", [id], |row| {
            row.get(0)
        })
        .optional()?;

    Ok(found.is_some())
}

/// Makes sure the board this section will point at has a row to point at.
///
/// A board has no `board_windows` row until its window has been placed once,
/// but sections reference that table. Without this, adding a section on a
/// fresh install would fail on a foreign key the user has no way to satisfy —
/// and the only way to satisfy it would be to open and move the window first,
/// which is not a thing anyone would guess.
fn ensure_board_row(conn: &Connection, kind: BoardKind) -> Result<(), StorageError> {
    let found: Option<i64> = conn
        .query_row(
            "SELECT 1 FROM board_windows WHERE kind = ?1",
            rusqlite::params![kind],
            |row| row.get(0),
        )
        .optional()?;

    if found.is_none() {
        // The same defaults the board itself would have been saved with, so
        // this cannot quietly become a second definition of them.
        BoardRepo::new().save(conn, &BoardWindow::default_for(kind))?;
    }

    Ok(())
}

fn section_from_row(row: &Row<'_>) -> Result<BoardSection, rusqlite::Error> {
    Ok(BoardSection {
        id: row.get(0)?,
        board_kind: row.get(1)?,
        title: row.get(2)?,
        position: row.get(3)?,
        items: Vec::new(),
    })
}

fn item_from_row(row: &Row<'_>) -> Result<SectionItem, rusqlite::Error> {
    Ok(SectionItem {
        id: row.get(0)?,
        section_id: row.get(1)?,
        text: row.get(2)?,
        position: row.get(3)?,
    })
}

fn now_iso8601() -> String {
    chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S%.6fZ")
        .to_string()
}
