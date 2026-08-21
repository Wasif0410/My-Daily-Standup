//! Local operational storage.
//!
//! SQLite holds the app's working state: current tasks, their hierarchy, and
//! their progress. Long-term knowledge lives in the user's Obsidian vault
//! (spec §3.2), never here.

mod db;
mod migrations;
mod task;
mod task_repo;

#[cfg(test)]
mod task_repo_tests;
#[cfg(test)]
mod tests;

pub use db::Db;
pub use migrations::{run_migrations, schema_version, LATEST_VERSION};
pub use task::{NewTask, Task, TaskHorizon, TaskPatch, TaskSource, TaskStatus};
pub use task_repo::TaskRepo;

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

    #[error("no task with id {id}")]
    TaskNotFound { id: String },

    #[error("migration {version} failed: {source}")]
    Migration {
        version: u32,
        #[source]
        source: rusqlite::Error,
    },
}
