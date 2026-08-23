//! The IPC boundary between the frontend and Rust.
//!
//! Commands are thin wrappers over [`AppState`], which owns the database and
//! holds every method the frontend can reach. Keeping the logic on `AppState`
//! rather than in the command functions means it can be tested directly,
//! without constructing a Tauri runtime.

pub mod boards;
pub mod tasks;

use std::path::Path;
use std::sync::Mutex;

use serde::Serialize;

use crate::storage::{
    BoardKind, BoardRepo, BoardWindow, Db, NewTask, StorageError, Task, TaskHorizon, TaskPatch,
    TaskRepo, DATABASE_FILENAME,
};

/// How an error is reported across the IPC boundary.
///
/// A tagged shape rather than a stringified panic, so the frontend can branch
/// on `kind` (show a "not found" toast, retry, surface a bug report) instead of
/// pattern-matching on English prose that may change.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub kind: ErrorKind,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ErrorKind {
    /// The requested task does not exist.
    NotFound,
    /// The request itself was malformed — a date that is not a date, say.
    InvalidInput,
    /// The database rejected the operation — a constraint, or a bad value.
    Storage,
    /// A lock was poisoned by a panic elsewhere. Not recoverable in place.
    Internal,
}

impl From<StorageError> for CommandError {
    fn from(error: StorageError) -> Self {
        let kind = match error {
            StorageError::TaskNotFound { .. } => ErrorKind::NotFound,
            StorageError::InvalidDate { .. } => ErrorKind::InvalidInput,
            StorageError::Sqlite(_) | StorageError::Migration { .. } => ErrorKind::Storage,
            StorageError::Io(_) => ErrorKind::Internal,
        };

        Self {
            kind,
            message: error.to_string(),
        }
    }
}

/// Everything the frontend can reach.
///
/// The connection lives behind a mutex because `rusqlite::Connection` is `Send`
/// but not `Sync`, and Tauri shares state across command invocations.
pub struct AppState {
    db: Mutex<Db>,
    repo: TaskRepo,
    boards: BoardRepo,
}

impl AppState {
    /// Opens the application database inside `app_data_dir`.
    pub fn new(app_data_dir: &Path) -> Result<Self, StorageError> {
        let db = Db::open(&app_data_dir.join(DATABASE_FILENAME))?;

        Ok(Self {
            db: Mutex::new(db),
            repo: TaskRepo::new(),
            boards: BoardRepo::new(),
        })
    }

    /// An in-memory instance for tests.
    pub fn in_memory() -> Result<Self, StorageError> {
        Ok(Self {
            db: Mutex::new(Db::open_in_memory()?),
            repo: TaskRepo::new(),
            boards: BoardRepo::new(),
        })
    }

    /// Runs `operation` against the connection.
    ///
    /// A poisoned lock means another thread panicked mid-write. The database
    /// itself is fine — WAL rolls back the incomplete transaction — so report
    /// it rather than propagating the panic.
    fn with_conn<T>(
        &self,
        operation: impl FnOnce(&TaskRepo, &rusqlite::Connection) -> Result<T, StorageError>,
    ) -> Result<T, CommandError> {
        let guard = self.db.lock().map_err(|_| CommandError {
            kind: ErrorKind::Internal,
            message: "database lock was poisoned by an earlier panic".to_string(),
        })?;

        operation(&self.repo, guard.conn()).map_err(CommandError::from)
    }

    pub fn create_task(&self, input: NewTask) -> Result<Task, CommandError> {
        self.with_conn(|repo, conn| repo.create(conn, input))
    }

    pub fn get_task(&self, id: &str) -> Result<Option<Task>, CommandError> {
        self.with_conn(|repo, conn| repo.get(conn, id))
    }

    pub fn update_task(&self, id: &str, patch: TaskPatch) -> Result<Task, CommandError> {
        self.with_conn(|repo, conn| repo.update(conn, id, patch))
    }

    pub fn delete_task(&self, id: &str) -> Result<(), CommandError> {
        self.with_conn(|repo, conn| repo.delete(conn, id))
    }

    pub fn list_by_horizon(&self, horizon: TaskHorizon) -> Result<Vec<Task>, CommandError> {
        self.with_conn(|repo, conn| repo.list_by_horizon(conn, horizon))
    }

    pub fn list_for_date(&self, date: &str) -> Result<Vec<Task>, CommandError> {
        self.with_conn(|repo, conn| repo.list_for_date(conn, date))
    }

    pub fn list_for_period(&self, start: &str, end: &str) -> Result<Vec<Task>, CommandError> {
        self.with_conn(|repo, conn| repo.list_for_period(conn, start, end))
    }

    /// Records how long a task took, in minutes.
    ///
    /// `None` clears the value back to unrecorded, which is distinct from
    /// zero: an unmeasured task must contribute nothing to a total rather
    /// than dragging an average down.
    pub fn set_time_spent(&self, id: &str, minutes: Option<i64>) -> Result<Task, CommandError> {
        self.with_conn(|repo, conn| {
            repo.update(
                conn,
                id,
                TaskPatch {
                    time_spent_minutes: Some(minutes),
                    ..Default::default()
                },
            )
        })
    }

    /// Moves a task to a new date through the rollover engine.
    ///
    /// Deliberately not a plain `update_task` carrying a new `scheduled_date`:
    /// that would bypass rollover counting entirely and silently break the
    /// reflection prompt in spec §10.3. Every date move must come through here.
    pub fn reschedule_task(&self, id: &str, to: &str) -> Result<Task, CommandError> {
        self.with_conn(|repo, conn| crate::domain::reschedule(repo, conn, id, to))
    }

    pub fn children_of(&self, parent_id: &str) -> Result<Vec<Task>, CommandError> {
        self.with_conn(|repo, conn| repo.children_of(conn, parent_id))
    }
}

#[cfg(test)]
mod tests;

impl AppState {
    /// One board's saved window state, or its defaults.
    pub fn board(&self, kind: BoardKind) -> Result<BoardWindow, CommandError> {
        let guard = self.db.lock().map_err(poisoned)?;
        self.boards
            .get(guard.conn(), kind)
            .map_err(CommandError::from)
    }

    /// Every board's state, for restoring the layout at startup.
    pub fn boards(&self) -> Result<Vec<BoardWindow>, CommandError> {
        let guard = self.db.lock().map_err(poisoned)?;
        self.boards.all(guard.conn()).map_err(CommandError::from)
    }

    pub fn save_board(&self, window: &BoardWindow) -> Result<(), CommandError> {
        let guard = self.db.lock().map_err(poisoned)?;
        self.boards
            .save(guard.conn(), window)
            .map_err(CommandError::from)
    }
}

fn poisoned<T>(_: T) -> CommandError {
    CommandError {
        kind: ErrorKind::Internal,
        message: "database lock was poisoned by an earlier panic".to_string(),
    }
}
