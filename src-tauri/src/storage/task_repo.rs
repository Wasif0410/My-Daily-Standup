//! Task persistence.
//!
//! Pure data access with no Tauri coupling, which is what makes it fully
//! unit-testable. Business rules — completion semantics, progress roll-up,
//! rollover accounting — live in the domain layer above this (spec §3.6).
//!
//! The repository owns exactly three fields: `id`, `created_at`, and
//! `updated_at`. Everything else is stored as the caller supplied it.

use rusqlite::{params_from_iter, Connection, Row, ToSql};

use super::{NewTask, StorageError, Task, TaskHorizon, TaskPatch};

/// Columns selected for every read, in the order `from_row` expects.
const COLUMNS: &str = "id, title, description, horizon, status, parent_task_id, \
     source_type, source_file, source_line, area, project, priority, \
     scheduled_date, period_start, period_end, due_date, completed_at, \
     progress_current, progress_target, progress_unit, blocker, notes, \
     rollover_count, created_at, updated_at";

/// Reads and writes tasks.
#[derive(Debug, Default, Clone, Copy)]
pub struct TaskRepo;

impl TaskRepo {
    pub fn new() -> Self {
        Self
    }

    /// Inserts a task, generating its id and timestamps.
    ///
    /// Ids are UUIDv4 generated here — never supplied by the frontend or by
    /// model output, so a proposal cannot overwrite an existing task by
    /// choosing its id (spec §17.5).
    pub fn create(&self, conn: &Connection, input: NewTask) -> Result<Task, StorageError> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = now_iso8601();

        conn.execute(
            "INSERT INTO tasks (
                 id, title, description, horizon, status, parent_task_id,
                 source_type, source_file, source_line, area, project, priority,
                 scheduled_date, period_start, period_end, due_date,
                 progress_current, progress_target, progress_unit, notes,
                 rollover_count, created_at, updated_at
             ) VALUES (
                 ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                 ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, 0, ?21, ?22
             )",
            rusqlite::params![
                id,
                input.title,
                input.description,
                input.horizon,
                input.status,
                input.parent_task_id,
                input.source_type,
                input.source_file,
                input.source_line,
                input.area,
                input.project,
                input.priority,
                input.scheduled_date,
                input.period_start,
                input.period_end,
                input.due_date,
                input.progress_current,
                input.progress_target,
                input.progress_unit,
                input.notes,
                now,
                now,
            ],
        )?;

        self.get(conn, &id)?
            .ok_or_else(|| StorageError::TaskNotFound { id: id.clone() })
    }

    /// Fetches one task. A missing task is `None`, not an error.
    pub fn get(&self, conn: &Connection, id: &str) -> Result<Option<Task>, StorageError> {
        let sql = format!("SELECT {COLUMNS} FROM tasks WHERE id = ?1");
        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query([id])?;

        match rows.next()? {
            Some(row) => Ok(Some(from_row(row)?)),
            None => Ok(None),
        }
    }

    /// Applies a partial update and returns the stored result.
    ///
    /// `updated_at` is always refreshed; `created_at` is never touched. An
    /// empty patch still refreshes `updated_at`, which is correct: the caller
    /// asked for a write.
    pub fn update(
        &self,
        conn: &Connection,
        id: &str,
        patch: TaskPatch,
    ) -> Result<Task, StorageError> {
        if self.get(conn, id)?.is_none() {
            return Err(StorageError::TaskNotFound { id: id.to_string() });
        }

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

        set!(patch.title, "title");
        set!(patch.horizon, "horizon");
        set!(patch.status, "status");
        set!(patch.description, "description");
        set!(patch.parent_task_id, "parent_task_id");
        set!(patch.area, "area");
        set!(patch.project, "project");
        set!(patch.priority, "priority");
        set!(patch.scheduled_date, "scheduled_date");
        set!(patch.period_start, "period_start");
        set!(patch.period_end, "period_end");
        set!(patch.due_date, "due_date");
        set!(patch.completed_at, "completed_at");
        set!(patch.progress_current, "progress_current");
        set!(patch.progress_target, "progress_target");
        set!(patch.progress_unit, "progress_unit");
        set!(patch.blocker, "blocker");
        set!(patch.notes, "notes");
        set!(patch.rollover_count, "rollover_count");

        // Always last, and always set by the repository rather than the caller.
        assignments.push(format!("updated_at = ?{}", values.len() + 1));
        values.push(Box::new(now_iso8601()));

        let sql = format!(
            "UPDATE tasks SET {} WHERE id = ?{}",
            assignments.join(", "),
            values.len() + 1
        );
        values.push(Box::new(id.to_string()));

        conn.execute(&sql, params_from_iter(values.iter().map(|v| v.as_ref())))?;

        self.get(conn, id)?
            .ok_or_else(|| StorageError::TaskNotFound { id: id.to_string() })
    }

    /// Deletes a task. Children survive with their `parent_task_id` nulled,
    /// which the schema handles via `ON DELETE SET NULL`.
    pub fn delete(&self, conn: &Connection, id: &str) -> Result<(), StorageError> {
        let affected = conn.execute("DELETE FROM tasks WHERE id = ?1", [id])?;

        if affected == 0 {
            return Err(StorageError::TaskNotFound { id: id.to_string() });
        }

        Ok(())
    }

    /// Every task at one planning horizon.
    pub fn list_by_horizon(
        &self,
        conn: &Connection,
        horizon: TaskHorizon,
    ) -> Result<Vec<Task>, StorageError> {
        self.query(
            conn,
            &format!(
                "SELECT {COLUMNS} FROM tasks WHERE horizon = ?1 \
                 ORDER BY priority DESC NULLS LAST, created_at"
            ),
            rusqlite::params![horizon],
        )
    }

    /// Tasks scheduled for one specific day.
    pub fn list_for_date(&self, conn: &Connection, date: &str) -> Result<Vec<Task>, StorageError> {
        self.query(
            conn,
            &format!(
                "SELECT {COLUMNS} FROM tasks WHERE scheduled_date = ?1 \
                 ORDER BY priority DESC NULLS LAST, created_at"
            ),
            rusqlite::params![date],
        )
    }

    /// Tasks whose period overlaps `[start, end]`.
    ///
    /// Overlap rather than containment: a milestone spanning a week boundary
    /// still belongs to both weeks it touches.
    pub fn list_for_period(
        &self,
        conn: &Connection,
        start: &str,
        end: &str,
    ) -> Result<Vec<Task>, StorageError> {
        self.query(
            conn,
            &format!(
                "SELECT {COLUMNS} FROM tasks \
                 WHERE period_start IS NOT NULL AND period_end IS NOT NULL \
                   AND period_start <= ?2 AND period_end >= ?1 \
                 ORDER BY period_start, priority DESC NULLS LAST"
            ),
            rusqlite::params![start, end],
        )
    }

    /// Direct children of a task. Does not recurse.
    pub fn children_of(
        &self,
        conn: &Connection,
        parent_id: &str,
    ) -> Result<Vec<Task>, StorageError> {
        self.query(
            conn,
            &format!(
                "SELECT {COLUMNS} FROM tasks WHERE parent_task_id = ?1 \
                 ORDER BY priority DESC NULLS LAST, created_at"
            ),
            rusqlite::params![parent_id],
        )
    }

    fn query(
        &self,
        conn: &Connection,
        sql: &str,
        params: &[&dyn ToSql],
    ) -> Result<Vec<Task>, StorageError> {
        let mut stmt = conn.prepare(sql)?;
        let mut rows = stmt.query(params)?;

        let mut tasks = Vec::new();
        while let Some(row) = rows.next()? {
            tasks.push(from_row(row)?);
        }

        Ok(tasks)
    }
}

/// Builds a `Task` from a row selecting `COLUMNS` in order.
fn from_row(row: &Row<'_>) -> Result<Task, rusqlite::Error> {
    Ok(Task {
        id: row.get(0)?,
        title: row.get(1)?,
        description: row.get(2)?,
        horizon: row.get(3)?,
        status: row.get(4)?,
        parent_task_id: row.get(5)?,
        source_type: row.get(6)?,
        source_file: row.get(7)?,
        source_line: row.get(8)?,
        area: row.get(9)?,
        project: row.get(10)?,
        priority: row.get(11)?,
        scheduled_date: row.get(12)?,
        period_start: row.get(13)?,
        period_end: row.get(14)?,
        due_date: row.get(15)?,
        completed_at: row.get(16)?,
        progress_current: row.get(17)?,
        progress_target: row.get(18)?,
        progress_unit: row.get(19)?,
        blocker: row.get(20)?,
        notes: row.get(21)?,
        rollover_count: row.get(22)?,
        created_at: row.get(23)?,
        updated_at: row.get(24)?,
    })
}

/// The current time as an ISO-8601 UTC string.
///
/// Text in this format sorts chronologically, which the board queries depend
/// on. Rust owns date arithmetic, never the model (spec §3.6).
fn now_iso8601() -> String {
    chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S%.6fZ")
        .to_string()
}
