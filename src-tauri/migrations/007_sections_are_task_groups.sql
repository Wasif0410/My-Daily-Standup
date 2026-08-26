-- Sections become named task groups instead of free-text notes.
--
-- Migration 006 built a section as a heading plus its own list of bullets,
-- stored apart from `tasks`. That was a second, parallel way to write
-- something down on a board, and the two could never agree: the bullets under
-- "JOB SEARCH" were not the tasks in JOB SEARCH, and no board query could
-- make them one list. The headings the boards already render come from
-- `tasks.area` and `tasks.project`, so that is what a section is.
--
-- The items go with nothing to replace them. Their content was never a task
-- and cannot be promoted to one without inventing a horizon, a status and a
-- schedule for every stray line; a made-up task on a planning board is worse
-- than a lost note, because the planning engine then reasons about it.
DROP TABLE board_section_items;

-- `board_sections` survives with a new job: it records *declared* groups.
--
-- A group derived from tasks exists only while some task carries its name, so
-- a heading the user has just typed and not yet filled would vanish before
-- they could file anything under it. A row here is proof the name was meant,
-- and keeps it on the board while it is empty. The columns are unchanged
-- (id, board_kind, title, position, created_at) — only the meaning is.

-- Weekly Progress groups by day and Monthly Progress by commitment, and
-- neither is a name the user can invent. Sections created on them under 006
-- name a column that does not exist, so they are removed here rather than
-- left to list as headings the app can no longer create or fill.
DELETE FROM board_sections
WHERE board_kind IN ('weekly-progress', 'monthly-progress');
