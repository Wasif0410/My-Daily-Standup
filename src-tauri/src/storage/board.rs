//! Sticky-note window state.
//!
//! Geometry is persisted per board so a desktop layout survives a restart
//! (spec §6.7, §23). Stored in SQLite rather than a config file to inherit the
//! same crash safety as task data.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use super::StorageError;

/// Which board a window shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BoardKind {
    Priority,
    WeeklyTasks,
    WeeklyProgress,
    MonthlyProgress,
}

impl BoardKind {
    /// The value stored in SQLite, matching the schema's CHECK constraint.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Priority => "priority",
            Self::WeeklyTasks => "weekly-tasks",
            Self::WeeklyProgress => "weekly-progress",
            Self::MonthlyProgress => "monthly-progress",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "priority" => Some(Self::Priority),
            "weekly-tasks" => Some(Self::WeeklyTasks),
            "weekly-progress" => Some(Self::WeeklyProgress),
            "monthly-progress" => Some(Self::MonthlyProgress),
            _ => None,
        }
    }

    /// Every board, for restoring the full layout on startup.
    pub const ALL: &'static [Self] = &[
        Self::Priority,
        Self::WeeklyTasks,
        Self::WeeklyProgress,
        Self::MonthlyProgress,
    ];

    /// The window label Tauri knows this board by.
    pub fn window_label(self) -> String {
        format!("board-{}", self.as_str())
    }

    /// Title shown in the taskbar and by screen readers.
    pub fn title(self) -> &'static str {
        match self {
            Self::Priority => "Priority Tasks",
            Self::WeeklyTasks => "Weekly Tasks",
            Self::WeeklyProgress => "Weekly Progress",
            Self::MonthlyProgress => "Monthly Progress",
        }
    }
}

impl rusqlite::ToSql for BoardKind {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(self.as_str().into())
    }
}

impl rusqlite::types::FromSql for BoardKind {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let text = value.as_str()?;
        Self::parse(text).ok_or_else(|| {
            rusqlite::types::FromSqlError::Other(format!("unknown board kind: {text}").into())
        })
    }
}

/// A board window's saved state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardWindow {
    pub kind: BoardKind,
    /// `None` until the window has been placed once.
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub width: u32,
    pub height: u32,
    pub monitor: Option<String>,
    pub visible: bool,
    pub collapsed: bool,
    pub opacity: f64,
    pub always_on_top: bool,
    pub locked: bool,
}

impl BoardWindow {
    /// A board that has never been positioned. Sized for a readable list
    /// without dominating the desktop.
    pub fn default_for(kind: BoardKind) -> Self {
        Self {
            kind,
            x: None,
            y: None,
            width: 340,
            height: 460,
            monitor: None,
            visible: false,
            collapsed: false,
            opacity: 1.0,
            always_on_top: false,
            locked: false,
        }
    }
}

/// Reads and writes board window state.
#[derive(Debug, Default, Clone, Copy)]
pub struct BoardRepo;

const COLUMNS: &str = "kind, x, y, width, height, monitor, visible, collapsed, \
     opacity, always_on_top, locked";

impl BoardRepo {
    pub fn new() -> Self {
        Self
    }

    /// Saves a board's state, inserting it if this is the first time.
    pub fn save(&self, conn: &Connection, window: &BoardWindow) -> Result<(), StorageError> {
        conn.execute(
            "INSERT INTO board_windows
                 (kind, x, y, width, height, monitor, visible, collapsed,
                  opacity, always_on_top, locked, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
             ON CONFLICT (kind) DO UPDATE SET
                 x = excluded.x, y = excluded.y,
                 width = excluded.width, height = excluded.height,
                 monitor = excluded.monitor,
                 visible = excluded.visible, collapsed = excluded.collapsed,
                 opacity = excluded.opacity,
                 always_on_top = excluded.always_on_top,
                 locked = excluded.locked,
                 updated_at = excluded.updated_at",
            rusqlite::params![
                window.kind,
                window.x,
                window.y,
                window.width,
                window.height,
                window.monitor,
                window.visible,
                window.collapsed,
                window.opacity,
                window.always_on_top,
                window.locked,
                now_iso8601(),
            ],
        )?;

        Ok(())
    }

    /// Loads one board's saved state, or its defaults if never saved.
    pub fn get(&self, conn: &Connection, kind: BoardKind) -> Result<BoardWindow, StorageError> {
        let sql = format!("SELECT {COLUMNS} FROM board_windows WHERE kind = ?1");
        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query(rusqlite::params![kind])?;

        match rows.next()? {
            Some(row) => Ok(from_row(row)?),
            None => Ok(BoardWindow::default_for(kind)),
        }
    }

    /// Every board, in a stable order, falling back to defaults for any that
    /// have never been saved. Used to restore the layout on startup.
    pub fn all(&self, conn: &Connection) -> Result<Vec<BoardWindow>, StorageError> {
        BoardKind::ALL
            .iter()
            .map(|&kind| self.get(conn, kind))
            .collect()
    }
}

fn from_row(row: &rusqlite::Row<'_>) -> Result<BoardWindow, rusqlite::Error> {
    Ok(BoardWindow {
        kind: row.get(0)?,
        x: row.get(1)?,
        y: row.get(2)?,
        width: row.get(3)?,
        height: row.get(4)?,
        monitor: row.get(5)?,
        visible: row.get(6)?,
        collapsed: row.get(7)?,
        opacity: row.get(8)?,
        always_on_top: row.get(9)?,
        locked: row.get(10)?,
    })
}

fn now_iso8601() -> String {
    chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S%.6fZ")
        .to_string()
}
