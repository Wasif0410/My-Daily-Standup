-- Sticky-note window state (spec §6.7).
--
-- One row per board. Geometry lives here rather than in a config file so it
-- shares the database's crash safety: a board's position survives an
-- unexpected shutdown, which is what makes §23's "boards restore their
-- positions after restarting" true rather than aspirational.

CREATE TABLE board_windows (
    kind            TEXT PRIMARY KEY NOT NULL
                        CHECK (kind IN ('priority', 'weekly-tasks',
                                        'weekly-progress', 'monthly-progress')),

    -- Null until the window has been placed once; the OS picks the first spot.
    x               INTEGER,
    y               INTEGER,
    width           INTEGER NOT NULL CHECK (width  >= 200),
    height          INTEGER NOT NULL CHECK (height >= 120),

    -- Monitor name, so a board pinned to a second screen returns there rather
    -- than to the same coordinates on whichever display is primary today.
    monitor         TEXT,

    visible         INTEGER NOT NULL DEFAULT 1 CHECK (visible   IN (0, 1)),
    collapsed       INTEGER NOT NULL DEFAULT 0 CHECK (collapsed IN (0, 1)),

    -- Floor of 0.2: a fully transparent board would be unrecoverable by
    -- clicking, since the user could not find it.
    opacity         REAL NOT NULL DEFAULT 1.0 CHECK (opacity BETWEEN 0.2 AND 1.0),

    always_on_top   INTEGER NOT NULL DEFAULT 0 CHECK (always_on_top IN (0, 1)),
    locked          INTEGER NOT NULL DEFAULT 0 CHECK (locked        IN (0, 1)),

    updated_at      TEXT NOT NULL
);
