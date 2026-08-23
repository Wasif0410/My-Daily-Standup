-- Presentation state: which days are expanded, and anything else that is about
-- how the app is being looked at rather than what is in it.
--
-- Deliberately separate from the real settings table PR 16 will add. Scroll
-- positions and disclosure states have a different lifetime and a different
-- blast radius from "which model to load" — sharing one bucket would mean a
-- settings reset also forgot which day the user had open.

CREATE TABLE ui_state (
    key   TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL
);
