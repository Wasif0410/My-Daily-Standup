-- The user's settings (spec §18).
--
-- One row with one typed column per setting, not a key/value store. `ui_state`
-- next door is already the key/value table, and it is that because the shape of
-- presentation state changes with every board. A setting is the opposite: there
-- are three of them, each has exactly one legal shape, and every one of them
-- changes what the app computes rather than how a board looks. Columns let the
-- database say so. A key/value row can hold 'priority_threshold' = 'banana' and
-- nothing finds out until a board renders empty; `priority_threshold INTEGER
-- CHECK (... BETWEEN 0 AND 10)` cannot be written wrong in the first place.
--
-- Deliberately a separate table from `ui_state` rather than a reserved prefix
-- inside it, because the two have different lifetimes. Resetting settings must
-- not also forget which day the user had expanded, and clearing presentation
-- state must not reset their week-start day. Sharing one bucket would make each
-- of those the other's blast radius.
--
-- Only the three settings something in the app actually reads are here. The
-- rest of §18 is a list of intentions, and a column nothing consumes is a
-- setting that silently does nothing — which is the bug this migration exists
-- to stop repeating.
CREATE TABLE settings (
    -- The CHECK is the whole point: there is one user and one set of settings,
    -- so a second row is not a state the app should have to choose between.
    -- Every read is `WHERE id = 1` and can never find two answers.
    id                 INTEGER PRIMARY KEY CHECK (id = 1),

    -- The lowest priority the Priority board shows. Zero is "everything that
    -- has been ranked at all"; ten is "only the top". Refused outside the
    -- range rather than clamped — see storage/settings.rs.
    priority_threshold INTEGER NOT NULL DEFAULT 5
                           CHECK (priority_threshold BETWEEN 0 AND 10),

    -- Which day a week starts on (spec §6.4). Three values because those are
    -- the three the week commands can honour; anything else would be a stored
    -- preference the calendar quietly ignored.
    week_starts_on     TEXT NOT NULL DEFAULT 'monday'
                           CHECK (week_starts_on IN ('monday','sunday','saturday')),

    -- Mirrors the autostart plugin's registration. Written only after the
    -- plugin call has succeeded, so this column can never claim a login entry
    -- that does not exist.
    launch_at_login    INTEGER NOT NULL DEFAULT 0 CHECK (launch_at_login IN (0,1)),

    updated_at         TEXT NOT NULL
);

-- Seeded here so a read never has to cope with an empty table. Every DEFAULT
-- above is spelled out again by omission: naming only `id` and `updated_at`
-- means the row is built from the schema's own defaults rather than from a
-- second copy of them that could drift.
-- Six fractional digits, padded, because Rust writes six and these are TEXT
-- columns compared lexicographically. SQLite's %f gives three, and
-- '...16.123Z' sorts ABOVE '...16.123456Z' — 'Z' is 0x5A, '4' is 0x34 — so a
-- later write would compare as earlier. Migration 001 says text "sorts
-- correctly in this format"; that is only true while every writer agrees on
-- the format.
INSERT INTO settings (id, updated_at)
VALUES (1, strftime('%Y-%m-%dT%H:%M:%f', 'now') || '000Z');
