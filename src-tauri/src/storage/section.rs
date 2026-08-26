//! Named task groups on a board.
//!
//! The boards already group tasks by a name written on them — `tasks.area` on
//! Priority, `tasks.project` on Weekly Tasks — and those groups are the
//! "JOB SEARCH" and "HEALTH" headings the user reads. A section is one of
//! those groups. It is not a note pinned beside them, which is what an earlier
//! version made it: two ways to write something under one heading, neither of
//! them the other's list.
//!
//! What this table holds is only the *declaration*. A group derived from tasks
//! exists exactly as long as some task carries its name, so a heading the user
//! has just typed and not yet filled would disappear before they could file
//! anything under it. A row here says the name was meant, and keeps it on the
//! board while it is still empty.
//!
//! Like [`super::task_repo`], this is pure data access with no Tauri coupling,
//! which is what makes it unit-testable (spec §3.6).

use rusqlite::{Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};

use super::{BoardKind, BoardRepo, BoardWindow, StorageError};

/// Longest section title, in characters.
///
/// Eighty is roughly a line of prose. A heading longer than its own board is
/// not a heading, and the width it would need is width taken from the tasks
/// it exists to introduce.
pub const MAX_TITLE_CHARS: usize = 80;

/// A named task group declared on one board.
///
/// The tasks are deliberately absent: they are read by the board's own query
/// over `tasks`, which is the only place that knows the horizon, status and
/// dates a board sorts by. Shipping a second copy of them under the heading
/// would mean two lists to keep in step, and the stale one would be the one
/// with the user's name on it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardSection {
    pub id: String,
    pub board_kind: BoardKind,
    pub title: String,
    pub position: i64,
}

/// Reads and writes the groups declared on a board.
#[derive(Debug, Default, Clone, Copy)]
pub struct SectionRepo;

impl SectionRepo {
    pub fn new() -> Self {
        Self
    }

    /// Every group declared on one board, in the order they are shown.
    ///
    /// Groups that tasks alone would produce are not listed here — the board
    /// derives those from the tasks it is already reading. This is the set
    /// that must render whether or not anything is filed under it.
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

        Ok(sections)
    }

    /// One group, or `None` if it has been deleted.
    ///
    /// A missing section is not an error here: a board holding a stale id
    /// after a delete elsewhere is a normal race, not a fault.
    pub fn get(&self, conn: &Connection, id: &str) -> Result<Option<BoardSection>, StorageError> {
        let mut stmt = conn
            .prepare("SELECT id, board_kind, title, position FROM board_sections WHERE id = ?1")?;
        let mut rows = stmt.query([id])?;

        match rows.next()? {
            Some(row) => Ok(Some(section_from_row(row)?)),
            None => Ok(None),
        }
    }

    /// Declares an empty group at the end of a board.
    ///
    /// Appended rather than inserted at the top: the user is adding to a list
    /// they are already looking at, and pushing everything else down would
    /// move the thing they were reading.
    ///
    /// No task is touched. Declaring a group says the heading should render;
    /// filing work under it is the user's next move, not this one's.
    pub fn create(
        &self,
        conn: &Connection,
        board_kind: BoardKind,
        title: &str,
    ) -> Result<BoardSection, StorageError> {
        let title = validated_title(title)?;

        // Refused as validation rather than silently stored, because a section
        // on a board that groups by day has no column to be written to: it
        // would list as a heading nothing could ever be filed under.
        if board_kind.section_field().is_none() {
            return Err(StorageError::Validation {
                message: format!(
                    "the {} board groups by its own rows, so it cannot take named sections",
                    board_kind.title()
                ),
            });
        }

        ensure_board_row(conn, board_kind)?;

        let id = uuid::Uuid::new_v4().to_string();
        let position = next_position(conn, board_kind)?;

        conn.execute(
            "INSERT INTO board_sections (id, board_kind, title, position, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![id, board_kind, title, position, now_iso8601()],
        )?;

        self.get(conn, &id)?
            .ok_or_else(|| StorageError::SectionNotFound { id })
    }

    /// Renames a group: the declared row **and** every task filed under it.
    ///
    /// Both or neither, in one transaction. Renaming only the row splits the
    /// group in two — the heading reads "Job Hunt" while the tasks still say
    /// "Job Search", and the board renders both, one of them empty.
    ///
    /// The rewrite is not restricted to the tasks a board happens to show
    /// today. `area` and `project` are single fields on a task, so a task
    /// filtered off the board this moment is still in the group; leaving it
    /// behind would split it back out the moment it returned.
    ///
    /// Held to the same title rules as [`Self::create`]: a rename is the same
    /// act as naming, so a title that could not have been created must not be
    /// reachable by editing one that could.
    pub fn rename(
        &self,
        conn: &Connection,
        id: &str,
        title: &str,
    ) -> Result<BoardSection, StorageError> {
        let title = validated_title(title)?;
        let Some(section) = self.get(conn, id)? else {
            return Err(StorageError::SectionNotFound { id: id.to_string() });
        };

        let tx = conn.unchecked_transaction()?;

        tx.execute(
            "UPDATE board_sections SET title = ?2 WHERE id = ?1",
            rusqlite::params![id, title],
        )?;

        if let Some(field) = section.board_kind.section_field() {
            let column = field.column();
            tx.execute(
                &format!(
                    "UPDATE tasks SET {column} = :title, updated_at = :now \
                     WHERE {}",
                    group_matches(column)
                ),
                rusqlite::named_params! {
                    ":title": title,
                    ":now": now_iso8601(),
                    ":group": section.title,
                },
            )?;
        }

        tx.commit()?;

        self.get(conn, id)?
            .ok_or_else(|| StorageError::SectionNotFound { id: id.to_string() })
    }

    /// Removes a group's heading and **unfiles** its tasks.
    ///
    /// The tasks are never deleted. Someone tidying a heading away is tidying
    /// a heading, and losing a fortnight of work to it would be indefensible.
    /// Their `area` or `project` is cleared instead, which drops them into the
    /// board's Unsorted group where they can be filed again.
    ///
    /// One transaction, for the same reason as [`Self::rename`]: a heading
    /// removed while its tasks still name it leaves the group standing with
    /// nothing declaring it.
    pub fn delete(&self, conn: &Connection, id: &str) -> Result<(), StorageError> {
        let Some(section) = self.get(conn, id)? else {
            return Err(StorageError::SectionNotFound { id: id.to_string() });
        };

        let tx = conn.unchecked_transaction()?;

        if let Some(field) = section.board_kind.section_field() {
            let column = field.column();
            tx.execute(
                &format!(
                    "UPDATE tasks SET {column} = NULL, updated_at = :now \
                     WHERE {}",
                    group_matches(column)
                ),
                rusqlite::named_params! {
                    ":now": now_iso8601(),
                    ":group": section.title,
                },
            )?;
        }

        tx.execute("DELETE FROM board_sections WHERE id = ?1", [id])?;
        tx.commit()?;

        Ok(())
    }
}

/// The `WHERE` clause matching every task filed under one group name.
///
/// Case-insensitive and whitespace-trimmed, because "Job Search" and
/// "job search " are one heading on screen and have to be one group here too.
/// A rename that matched exactly would carry half a group across and strand
/// the rest under a name nothing declares any more.
///
/// The old name is bound as `:group`, which every caller must supply.
///
/// SQLite's `lower` folds ASCII only, so a group named in a script with its
/// own case rules matches exactly rather than loosely. That is the same
/// comparison the boards themselves group by, so at worst it agrees with what
/// is already on screen.
fn group_matches(column: &str) -> String {
    format!("{column} IS NOT NULL AND lower(trim({column})) = lower(trim(:group))")
}

/// Trims a title and rejects it if nothing useful survives, or if it is longer
/// than [`MAX_TITLE_CHARS`].
///
/// Over-length is refused rather than shortened. Truncating would store words
/// the user did not write and drop ones they did, without telling them which.
///
/// Counted in characters, not bytes: a byte cap would reject a heading of
/// perfectly ordinary length written in any language that does not fit in
/// ASCII.
fn validated_title(value: &str) -> Result<String, StorageError> {
    let trimmed = value.trim();

    if trimmed.is_empty() {
        return Err(StorageError::Validation {
            message: "A section title cannot be blank".to_string(),
        });
    }

    let length = trimmed.chars().count();
    if length > MAX_TITLE_CHARS {
        return Err(StorageError::Validation {
            message: format!(
                "A section title cannot be longer than {MAX_TITLE_CHARS} characters (got {length})"
            ),
        });
    }

    Ok(trimmed.to_string())
}

/// The position a newly declared group takes.
///
/// Derived from the current maximum rather than from a count, so deleting
/// something in the middle cannot make the next addition collide with a
/// position that is already taken.
fn next_position(conn: &Connection, board_kind: BoardKind) -> Result<i64, StorageError> {
    let highest: Option<i64> = conn.query_row(
        "SELECT MAX(position) FROM board_sections WHERE board_kind = ?1",
        rusqlite::params![board_kind],
        |row| row.get(0),
    )?;

    Ok(highest.map_or(0, |value| value + 1))
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
    })
}

fn now_iso8601() -> String {
    chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S%.6fZ")
        .to_string()
}
