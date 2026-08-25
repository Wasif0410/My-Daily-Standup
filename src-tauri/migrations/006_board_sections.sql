-- User-created sections on a board, each a named list of free-text bullets.
--
-- Separate from `tasks` on purpose. A task carries a horizon, a status, a
-- schedule and a rollover count because the planning engine reasons about it.
-- A section item carries none of that: it is a line someone wanted written
-- down on a particular board. Forcing it through `tasks` would mean inventing
-- a horizon and a status for every stray note, and every board query would
-- then have to filter the inventions back out.

CREATE TABLE board_sections (
    id         TEXT PRIMARY KEY NOT NULL,

    -- Pointed at board_windows rather than repeating its CHECK list, so a
    -- board added in a later migration cannot be spelled wrong here. Nothing
    -- deletes a board row today; the cascade is what keeps that true if
    -- something ever does.
    board_kind TEXT NOT NULL REFERENCES board_windows (kind) ON DELETE CASCADE,

    -- An untitled section is a rectangle nobody can name or find again. Rust
    -- trims before writing, so this CHECK catches only what would slip past it.
    title      TEXT NOT NULL CHECK (length(trim(title)) > 0),

    -- Explicit ordering rather than created_at: the user reorders sections,
    -- and creation time cannot be rewritten to match what they dragged.
    position   INTEGER NOT NULL,

    created_at TEXT NOT NULL
);

CREATE TABLE board_section_items (
    id         TEXT PRIMARY KEY NOT NULL,

    -- An item outside a section has nowhere to render, so it goes with its
    -- section rather than surviving as a hidden row.
    section_id TEXT NOT NULL REFERENCES board_sections (id) ON DELETE CASCADE,

    text       TEXT NOT NULL CHECK (length(trim(text)) > 0),
    position   INTEGER NOT NULL,
    created_at TEXT NOT NULL
);

-- Every board render asks for one board's sections; without this it is a scan
-- of every section on every board.
CREATE INDEX idx_board_sections_board ON board_sections (board_kind);

-- The same for an item lookup, and it is also what makes the cascade above
-- cheap: SQLite scans the child table to find the rows a delete takes with it.
CREATE INDEX idx_board_section_items_section ON board_section_items (section_id);
