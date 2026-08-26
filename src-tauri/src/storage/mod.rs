//! Local operational storage.
//!
//! SQLite holds the app's working state: current tasks, their hierarchy, and
//! their progress. Long-term knowledge lives in the user's Obsidian vault
//! (spec §3.2), never here.

mod board;
mod db;
mod migrations;
mod section;
mod task;
mod task_repo;
mod ui_state;

#[cfg(test)]
mod board_tests;
#[cfg(test)]
mod patch_tests;
#[cfg(test)]
mod section_tests;
#[cfg(test)]
mod task_repo_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod ui_state_tests;

pub use board::{BoardKind, BoardRepo, BoardTheme, BoardWindow, SectionField};
pub use db::Db;
pub use migrations::{run_migrations, schema_version, LATEST_VERSION};
pub use section::{BoardSection, SectionRepo, MAX_TITLE_CHARS};
pub use task::{NewTask, Task, TaskHorizon, TaskPatch, TaskSource, TaskStatus};
pub use task_repo::TaskRepo;
pub use ui_state::UiStateRepo;

/// Filename of the application database inside the app data directory.
///
/// Defined once here so the path is never spelled out at a call site.
pub const DATABASE_FILENAME: &str = "standup.db";

/// Anything that can go wrong in the storage layer.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("could not create the database directory: {0}")]
    Io(#[from] std::io::Error),

    #[error("not a valid ISO-8601 date: {value}")]
    InvalidDate { value: String },

    #[error("no task with id {id}")]
    TaskNotFound { id: String },

    #[error("no section with id {id}")]
    SectionNotFound { id: String },

    /// Input the user could correct, caught before it reaches SQLite.
    ///
    /// Separate from [`Self::Sqlite`] even where a CHECK constraint would
    /// also have rejected the value, because a constraint failure carries
    /// SQLite's wording rather than an explanation anyone would want read
    /// back to them.
    #[error("{message}")]
    Validation { message: String },

    #[error("migration {version} failed: {source}")]
    Migration {
        version: u32,
        #[source]
        source: rusqlite::Error,
    },
}
