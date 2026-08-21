-- Operational planning state. Obsidian remains the long-term source of truth
-- (docs/spec.md §3.2); this table holds only what the app needs to run:
-- current commitments, their hierarchy, and their progress.

CREATE TABLE tasks (
    id                TEXT PRIMARY KEY NOT NULL,
    title             TEXT NOT NULL,
    description       TEXT,

    horizon           TEXT NOT NULL
                          CHECK (horizon IN ('daily', 'weekly', 'monthly', 'long-term')),
    status            TEXT NOT NULL
                          CHECK (status IN ('backlog', 'planned', 'in-progress',
                                            'blocked', 'completed', 'cancelled', 'deferred')),

    -- A child outliving its parent is valid: a daily action stays meaningful
    -- even if the weekly milestone above it is deleted.
    parent_task_id    TEXT REFERENCES tasks (id) ON DELETE SET NULL,

    source_type       TEXT NOT NULL
                          CHECK (source_type IN ('obsidian', 'standup', 'manual')),
    source_file       TEXT,
    source_line       INTEGER,

    area              TEXT,
    project           TEXT,
    priority          INTEGER CHECK (priority IS NULL OR priority BETWEEN 0 AND 10),

    -- Dates are ISO-8601 strings. SQLite has no date type, and text sorts
    -- correctly in this format.
    scheduled_date    TEXT,
    period_start      TEXT,
    period_end        TEXT,
    due_date          TEXT,
    completed_at      TEXT,

    progress_current  REAL,
    progress_target   REAL,
    progress_unit     TEXT,

    blocker           TEXT,
    notes             TEXT,

    -- Incremented only on reschedule (§10.3). The signal behind "you have
    -- moved this five times" during reflection.
    rollover_count    INTEGER NOT NULL DEFAULT 0 CHECK (rollover_count >= 0),

    created_at        TEXT NOT NULL,
    updated_at        TEXT NOT NULL
);

-- Board queries: "what is scheduled for this day / this horizon".
CREATE INDEX idx_tasks_horizon_scheduled ON tasks (horizon, scheduled_date);

-- Filtering completed vs open work across every board.
CREATE INDEX idx_tasks_status ON tasks (status);

-- Walking the daily -> weekly -> monthly hierarchy.
CREATE INDEX idx_tasks_parent ON tasks (parent_task_id);

-- Weekly and monthly period roll-ups.
CREATE INDEX idx_tasks_period ON tasks (period_start, period_end);
