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
    TaskRepo, UiStateRepo, DATABASE_FILENAME,
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
    ui: UiStateRepo,
}

impl AppState {
    /// Opens the application database inside `app_data_dir`.
    pub fn new(app_data_dir: &Path) -> Result<Self, StorageError> {
        let db = Db::open(&app_data_dir.join(DATABASE_FILENAME))?;

        Ok(Self {
            db: Mutex::new(db),
            repo: TaskRepo::new(),
            boards: BoardRepo::new(),
            ui: UiStateRepo::new(),
        })
    }

    /// An in-memory instance for tests.
    pub fn in_memory() -> Result<Self, StorageError> {
        Ok(Self {
            db: Mutex::new(Db::open_in_memory()?),
            repo: TaskRepo::new(),
            boards: BoardRepo::new(),
            ui: UiStateRepo::new(),
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

    /// Tasks scheduled on any day in `[start, end]` — the Weekly Progress board.
    pub fn list_scheduled_between(
        &self,
        start: &str,
        end: &str,
    ) -> Result<Vec<Task>, CommandError> {
        self.with_conn(|repo, conn| repo.list_scheduled_between(conn, start, end))
    }
    /// Tasks for the Priority board — non-daily work at or above `threshold`.
    pub fn list_by_priority(&self, threshold: i64) -> Result<Vec<Task>, CommandError> {
        self.with_conn(|repo, conn| repo.list_by_priority(conn, threshold))
    }

    /// Monthly commitments overlapping `[start, end]`, each with its progress
    /// rolled up from the work beneath it.
    ///
    /// Every number the Monthly board shows is computed here. The component
    /// receives a pre-clamped fraction so it never divides (spec §3.6).
    pub fn monthly_progress(
        &self,
        start: &str,
        end: &str,
    ) -> Result<Vec<crate::domain::Commitment>, CommandError> {
        self.with_conn(|repo, conn| {
            let commitments =
                repo.list_by_horizon_in_period(conn, TaskHorizon::Monthly, start, end)?;

            commitments
                .iter()
                .map(|task| crate::domain::commitment_progress(repo, conn, task))
                .collect()
        })
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

    /// Moves a task to another week through the rollover engine.
    ///
    /// Like [`Self::reschedule_task`], never a plain update: only this path
    /// counts the move as a deferral.
    pub fn move_task_to_period(
        &self,
        id: &str,
        start: &str,
        end: &str,
    ) -> Result<Task, CommandError> {
        self.with_conn(|repo, conn| crate::domain::move_to_period(repo, conn, id, start, end))
    }

    /// Archives a task by cancelling it.
    ///
    /// The row survives; only the board loses it. That is the whole difference
    /// between archiving and deleting, and it is why this is not `delete_task`
    /// with a friendlier name.
    pub fn archive_task(&self, id: &str) -> Result<Task, CommandError> {
        self.with_conn(|repo, conn| {
            repo.update(
                conn,
                id,
                TaskPatch {
                    status: Some(crate::storage::TaskStatus::Cancelled),
                    ..Default::default()
                },
            )
        })
    }
    /// Sets or clears a blocker, keeping the task's status in step.
    ///
    /// Deliberately not a plain `update_task` carrying a `blocker`: that would
    /// let the text and the `blocked` status drift apart, and every view would
    /// then have to guess which one to believe.
    pub fn set_blocker(&self, id: &str, blocker: Option<&str>) -> Result<Task, CommandError> {
        self.with_conn(|repo, conn| crate::domain::set_blocker(repo, conn, id, blocker))
    }

    /// Appends a dated comment to a task's notes.
    pub fn add_comment(&self, id: &str, comment: &str) -> Result<Task, CommandError> {
        self.with_conn(|repo, conn| {
            crate::domain::add_comment(repo, conn, id, comment, crate::domain::today())
        })
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

impl AppState {
    /// Reads one piece of presentation state, or `None` if it was never set.
    pub fn ui_state(&self, key: &str) -> Result<Option<String>, CommandError> {
        let guard = self.db.lock().map_err(poisoned)?;
        self.ui.get(guard.conn(), key).map_err(CommandError::from)
    }

    pub fn set_ui_state(&self, key: &str, value: &str) -> Result<(), CommandError> {
        let guard = self.db.lock().map_err(poisoned)?;
        self.ui
            .set(guard.conn(), key, value)
            .map_err(CommandError::from)
    }
}

fn poisoned<T>(_: T) -> CommandError {
    CommandError {
        kind: ErrorKind::Internal,
        message: "database lock was poisoned by an earlier panic".to_string(),
    }
}
