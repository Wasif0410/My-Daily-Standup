-- How a board looks and layers (spec §6.7).
--
-- Four columns rather than a JSON blob: each has a CHECK that makes an
-- unrecoverable state unrepresentable, and a blob cannot be constrained.
--
-- Every column carries a DEFAULT so boards saved before this migration gain
-- sensible values rather than failing to load.

-- Clamped at both ends. Below 10px a board is unreadable; above 24px a 340px
-- window shows one illegible word and the menu that would fix it is off-screen.
ALTER TABLE board_windows ADD COLUMN font_size REAL NOT NULL DEFAULT 13.0
    CHECK (font_size BETWEEN 10.0 AND 24.0);

ALTER TABLE board_windows ADD COLUMN theme TEXT NOT NULL DEFAULT 'dark'
    CHECK (theme IN ('dark', 'light'));

ALTER TABLE board_windows ADD COLUMN compact INTEGER NOT NULL DEFAULT 0
    CHECK (compact IN (0, 1));

-- Desktop-level: the board sits below ordinary windows. Mutually exclusive
-- with always_on_top, which Rust enforces — both at once is meaningless, and a
-- CHECK across two columns would reject a legal intermediate state during an
-- upsert.
ALTER TABLE board_windows ADD COLUMN desktop_level INTEGER NOT NULL DEFAULT 0
    CHECK (desktop_level IN (0, 1));
