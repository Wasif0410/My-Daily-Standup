//! The user's settings (spec §18).
//!
//! Three settings, because three are all that anything in the app reads today:
//! the Priority board's threshold, the day a week starts on, and whether the
//! app launches at login. A fourth column would be a preference the user could
//! set and the app would then ignore, which is worse than not offering it — the
//! setting looks applied, and the behaviour that contradicts it looks like a
//! bug in something else.
//!
//! One row, not a key/value store. `ui_state` next door is the key/value table
//! and stays that way: presentation state changes shape with every board, while
//! a setting has exactly one legal shape that the schema's CHECK constraints
//! can state. The two are also kept apart so a settings reset cannot forget
//! which day the user had expanded, and clearing presentation state cannot
//! reset their week-start day.
//!
//! Like [`super::task_repo`], pure data access with no Tauri coupling. The
//! autostart side of `launch_at_login` lives in the command layer, which is
//! the only place that can reach the plugin.

use rusqlite::{params_from_iter, Connection, Row, ToSql};
use serde::{Deserialize, Serialize};

use super::StorageError;

/// The lowest and highest priority threshold the Priority board accepts.
///
/// Matches the schema's CHECK exactly. Duplicated deliberately: the constraint
/// is the backstop and this is the message, and a test asserts they agree.
pub const MIN_PRIORITY_THRESHOLD: i64 = 0;
pub const MAX_PRIORITY_THRESHOLD: i64 = 10;

/// The days a week may start on, in the wording stored and sent over IPC.
///
/// Three rather than seven because these are the three
/// [`crate::domain::parse_weekday`] can honour. Offering Wednesday would store
/// a preference the calendar quietly ignored.
pub const WEEK_START_DAYS: [&str; 3] = ["monday", "sunday", "saturday"];

/// Everything the user can configure.
///
/// `updated_at` is stored but deliberately absent here: it exists so a support
/// question about when something changed has an answer, not so a settings form
/// can render it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub priority_threshold: i64,
    /// One of [`WEEK_START_DAYS`].
    pub week_starts_on: String,
    pub launch_at_login: bool,
}

impl Default for Settings {
    /// The same values the schema defaults to.
    ///
    /// Spelled out here as well because this is what a read falls back to when
    /// the row is missing, and that path must not depend on SQLite having had
    /// a chance to apply its own defaults.
    fn default() -> Self {
        Self {
            priority_threshold: 5,
            week_starts_on: WEEK_START_DAYS[0].to_string(),
            launch_at_login: false,
        }
    }
}

/// A partial update. An omitted field means "leave alone".
///
/// No doubled option, unlike [`super::TaskPatch`]: none of these columns is
/// nullable, so there is no "clear this" to distinguish from "leave alone".
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsPatch {
    #[serde(default)]
    pub priority_threshold: Option<i64>,
    #[serde(default)]
    pub week_starts_on: Option<String>,
    #[serde(default)]
    pub launch_at_login: Option<bool>,
}

impl SettingsPatch {
    /// Checks every field the patch names, changing nothing.
    ///
    /// Separate from [`SettingsRepo::apply`] because the command layer has to
    /// know the patch is sound *before* it switches autostart on: a patch that
    /// will be refused for its threshold must not have registered a login entry
    /// on its way to being refused.
    ///
    /// Returns the values as they will be stored, so the normalising below
    /// happens exactly once.
    pub fn validate(&self) -> Result<ValidatedPatch, StorageError> {
        if let Some(threshold) = self.priority_threshold {
            if !(MIN_PRIORITY_THRESHOLD..=MAX_PRIORITY_THRESHOLD).contains(&threshold) {
                return Err(StorageError::Validation {
                    message: format!(
                        "A priority threshold must be between {MIN_PRIORITY_THRESHOLD} and \
                         {MAX_PRIORITY_THRESHOLD} (got {threshold})"
                    ),
                });
            }
        }

        let week_starts_on = match &self.week_starts_on {
            None => None,
            Some(raw) => {
                // Folded rather than refused: "Monday" is how anyone would
                // write it, and the alternative is an error message arguing
                // about a capital letter. Anything that is not one of the
                // three after folding really is a value the app cannot honour.
                let day = raw.trim().to_ascii_lowercase();

                if !WEEK_START_DAYS.contains(&day.as_str()) {
                    return Err(StorageError::Validation {
                        message: format!(
                            "A week can start on {} (got {raw:?})",
                            WEEK_START_DAYS.join(", ")
                        ),
                    });
                }

                Some(day)
            }
        };

        Ok(ValidatedPatch {
            priority_threshold: self.priority_threshold,
            week_starts_on,
            launch_at_login: self.launch_at_login,
        })
    }
}

/// A patch that has passed [`SettingsPatch::validate`].
///
/// A distinct type rather than a flag, so a caller cannot reach the write path
/// with a patch it forgot to check.
#[derive(Debug, Default)]
pub struct ValidatedPatch {
    pub priority_threshold: Option<i64>,
    pub week_starts_on: Option<String>,
    pub launch_at_login: Option<bool>,
}

impl ValidatedPatch {
    /// Whether the patch names anything at all.
    fn is_empty(&self) -> bool {
        self.priority_threshold.is_none()
            && self.week_starts_on.is_none()
            && self.launch_at_login.is_none()
    }
}

const COLUMNS: &str = "priority_threshold, week_starts_on, launch_at_login";

/// Reads and writes the single settings row.
#[derive(Debug, Default, Clone, Copy)]
pub struct SettingsRepo;

impl SettingsRepo {
    pub fn new() -> Self {
        Self
    }

    /// The current settings.
    ///
    /// A missing row reads as the defaults rather than as an error. The
    /// migration seeds it, so absence should be impossible — but this value is
    /// read on the way to opening a board, and a database that somehow lost the
    /// row must degrade to Monday and a threshold of 5 rather than to an app
    /// that will not start.
    pub fn get(&self, conn: &Connection) -> Result<Settings, StorageError> {
        let sql = format!("SELECT {COLUMNS} FROM settings WHERE id = 1");
        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query([])?;

        match rows.next()? {
            Some(row) => Ok(from_row(row)?),
            None => Ok(Settings::default()),
        }
    }

    /// Applies a partial update and returns the stored result.
    ///
    /// An empty patch writes nothing and returns the current settings, so
    /// `updated_at` still means "when a setting last changed" rather than "when
    /// a settings window was last closed". That is the opposite of
    /// [`super::TaskRepo::update`], which does refresh its timestamp on an
    /// empty patch — a task edit is a deliberate act on one row, while a
    /// settings save is a form submitting whatever it holds.
    ///
    /// Validated before anything is written, and written in one statement, so a
    /// patch is all-or-nothing: a rejected threshold cannot leave the week-start
    /// day changed beside the error message that refused it.
    pub fn apply(&self, conn: &Connection, patch: SettingsPatch) -> Result<Settings, StorageError> {
        self.apply_validated(conn, patch.validate()?)
    }

    /// [`Self::apply`] for a caller that has already validated, and acted on
    /// the result — the command layer, which must know the patch is sound
    /// before it touches the autostart plugin.
    pub fn apply_validated(
        &self,
        conn: &Connection,
        patch: ValidatedPatch,
    ) -> Result<Settings, StorageError> {
        if patch.is_empty() {
            return self.get(conn);
        }

        // The row is the schema's invariant, not something this write may
        // assume survived. Recreating it is what keeps a database that lost it
        // repairable by using the app rather than by editing SQLite by hand.
        conn.execute(
            "INSERT OR IGNORE INTO settings (id, updated_at) VALUES (1, ?1)",
            rusqlite::params![now_iso8601()],
        )?;

        let mut assignments: Vec<String> = Vec::new();
        let mut values: Vec<Box<dyn ToSql>> = Vec::new();

        macro_rules! set {
            ($field:expr, $column:literal) => {
                if let Some(value) = $field {
                    assignments.push(format!("{} = ?{}", $column, values.len() + 1));
                    values.push(Box::new(value));
                }
            };
        }

        set!(patch.priority_threshold, "priority_threshold");
        set!(patch.week_starts_on, "week_starts_on");
        set!(patch.launch_at_login, "launch_at_login");

        // Always last, and always the repository's to set rather than the
        // caller's.
        assignments.push(format!("updated_at = ?{}", values.len() + 1));
        values.push(Box::new(now_iso8601()));

        let sql = format!(
            "UPDATE settings SET {} WHERE id = 1",
            assignments.join(", ")
        );
        conn.execute(&sql, params_from_iter(values.iter().map(|v| v.as_ref())))?;

        self.get(conn)
    }
}

fn from_row(row: &Row<'_>) -> Result<Settings, rusqlite::Error> {
    Ok(Settings {
        priority_threshold: row.get(0)?,
        week_starts_on: row.get(1)?,
        launch_at_login: row.get(2)?,
    })
}

/// The current time as an ISO-8601 UTC string, matching every other table.
fn now_iso8601() -> String {
    chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S%.6fZ")
        .to_string()
}
