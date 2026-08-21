//! The task type and its enums.
//!
//! Mirrors spec §10. Field names are camelCase over the IPC boundary and
//! snake_case in Rust and SQL; serde bridges the two so neither side has to
//! compromise on its own conventions.

use serde::{Deserialize, Serialize};

/// Which planning horizon a task belongs to (spec §4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TaskHorizon {
    Daily,
    Weekly,
    Monthly,
    LongTerm,
}

/// Where a task currently stands (spec §10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TaskStatus {
    Backlog,
    Planned,
    InProgress,
    Blocked,
    Completed,
    Cancelled,
    Deferred,
}

/// How a task entered the system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TaskSource {
    /// Promoted from a note in the vault.
    Obsidian,
    /// Proposed by the assistant during a session, then approved by the user.
    Standup,
    /// Typed directly into a board or Quick Add.
    Manual,
}

/// Generates the SQLite text mapping for an enum.
///
/// The string values here must match the schema's CHECK constraints exactly.
/// A mismatch fails at insert time rather than compile time, so the round-trip
/// is covered by tests.
macro_rules! sql_enum {
    ($ty:ident { $($variant:ident => $text:literal),+ $(,)? }) => {
        impl $ty {
            /// The value stored in SQLite.
            pub fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $text),+
                }
            }

            /// Parses a value read back from SQLite.
            pub fn parse(value: &str) -> Option<Self> {
                match value {
                    $($text => Some(Self::$variant),)+
                    _ => None,
                }
            }

            /// Every variant, for exhaustive round-trip testing.
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];
        }

        impl rusqlite::ToSql for $ty {
            fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
                Ok(self.as_str().into())
            }
        }

        impl rusqlite::types::FromSql for $ty {
            fn column_result(
                value: rusqlite::types::ValueRef<'_>,
            ) -> rusqlite::types::FromSqlResult<Self> {
                let text = value.as_str()?;
                Self::parse(text).ok_or_else(|| {
                    rusqlite::types::FromSqlError::Other(
                        format!("unrecognised value in database: {text}").into(),
                    )
                })
            }
        }
    };
}

sql_enum!(TaskHorizon {
    Daily => "daily",
    Weekly => "weekly",
    Monthly => "monthly",
    LongTerm => "long-term",
});

sql_enum!(TaskStatus {
    Backlog => "backlog",
    Planned => "planned",
    InProgress => "in-progress",
    Blocked => "blocked",
    Completed => "completed",
    Cancelled => "cancelled",
    Deferred => "deferred",
});

sql_enum!(TaskSource {
    Obsidian => "obsidian",
    Standup => "standup",
    Manual => "manual",
});

/// A stored task.
///
/// `id`, `created_at`, and `updated_at` are owned by the repository;
/// `rollover_count` by the domain layer. Callers never set them directly.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: Option<String>,

    pub horizon: TaskHorizon,
    pub status: TaskStatus,

    pub parent_task_id: Option<String>,

    pub source_type: TaskSource,
    pub source_file: Option<String>,
    pub source_line: Option<i64>,

    pub area: Option<String>,
    pub project: Option<String>,
    pub priority: Option<i64>,

    pub scheduled_date: Option<String>,
    pub period_start: Option<String>,
    pub period_end: Option<String>,
    pub due_date: Option<String>,
    pub completed_at: Option<String>,

    pub progress_current: Option<f64>,
    pub progress_target: Option<f64>,
    pub progress_unit: Option<String>,

    pub blocker: Option<String>,
    pub notes: Option<String>,

    /// Incremented only when a task is rescheduled (spec §10.3).
    pub rollover_count: i64,

    pub created_at: String,
    pub updated_at: String,
}

/// Everything needed to create a task. The repository supplies the rest.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewTask {
    pub title: String,
    pub horizon: TaskHorizon,
    pub status: TaskStatus,
    pub source_type: TaskSource,

    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub parent_task_id: Option<String>,
    #[serde(default)]
    pub source_file: Option<String>,
    #[serde(default)]
    pub source_line: Option<i64>,
    #[serde(default)]
    pub area: Option<String>,
    #[serde(default)]
    pub project: Option<String>,
    #[serde(default)]
    pub priority: Option<i64>,
    #[serde(default)]
    pub scheduled_date: Option<String>,
    #[serde(default)]
    pub period_start: Option<String>,
    #[serde(default)]
    pub period_end: Option<String>,
    #[serde(default)]
    pub due_date: Option<String>,
    #[serde(default)]
    pub progress_current: Option<f64>,
    #[serde(default)]
    pub progress_target: Option<f64>,
    #[serde(default)]
    pub progress_unit: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

impl NewTask {
    /// A minimal task: title, horizon, and origin. Everything else defaults.
    pub fn new(title: impl Into<String>, horizon: TaskHorizon, source_type: TaskSource) -> Self {
        Self {
            title: title.into(),
            horizon,
            status: TaskStatus::Planned,
            source_type,
            description: None,
            parent_task_id: None,
            source_file: None,
            source_line: None,
            area: None,
            project: None,
            priority: None,
            scheduled_date: None,
            period_start: None,
            period_end: None,
            due_date: None,
            progress_current: None,
            progress_target: None,
            progress_unit: None,
            notes: None,
        }
    }
}

/// A partial update.
///
/// Nullable fields use a doubled option so "leave alone" and "clear this" stay
/// distinguishable: `None` means unchanged, `Some(None)` clears the column, and
/// `Some(Some(v))` sets it. Without that distinction there would be no way to
/// resolve a blocker or un-complete a task.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskPatch {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub horizon: Option<TaskHorizon>,
    #[serde(default)]
    pub status: Option<TaskStatus>,

    #[serde(default)]
    pub description: Option<Option<String>>,
    #[serde(default)]
    pub parent_task_id: Option<Option<String>>,
    #[serde(default)]
    pub area: Option<Option<String>>,
    #[serde(default)]
    pub project: Option<Option<String>>,
    #[serde(default)]
    pub priority: Option<Option<i64>>,
    #[serde(default)]
    pub scheduled_date: Option<Option<String>>,
    #[serde(default)]
    pub period_start: Option<Option<String>>,
    #[serde(default)]
    pub period_end: Option<Option<String>>,
    #[serde(default)]
    pub due_date: Option<Option<String>>,
    #[serde(default)]
    pub completed_at: Option<Option<String>>,
    #[serde(default)]
    pub progress_current: Option<Option<f64>>,
    #[serde(default)]
    pub progress_target: Option<Option<f64>>,
    #[serde(default)]
    pub progress_unit: Option<Option<String>>,
    #[serde(default)]
    pub blocker: Option<Option<String>>,
    #[serde(default)]
    pub notes: Option<Option<String>>,

    /// Set only by the domain layer's reschedule path (spec §10.3). Editing a
    /// title or adding a blocker must never touch it, so it is not part of the
    /// serialised patch the frontend can send.
    #[serde(skip)]
    pub rollover_count: Option<i64>,
}
