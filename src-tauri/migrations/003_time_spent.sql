-- How long each task actually took, in minutes (spec §10.4).
--
-- A logged duration rather than a stopwatch reading: the user types or picks
-- it, usually when marking the task done. NULL means "not recorded", which is
-- deliberately different from zero — a task nobody timed must contribute
-- nothing to a total rather than dragging an average down.
--
-- One column suffices because a task belongs to a single day, so summing a
-- period means summing the tasks scheduled inside it. There is no work
-- spanning a week boundary that would need splitting across two totals.

ALTER TABLE tasks ADD COLUMN time_spent_minutes INTEGER
    CHECK (time_spent_minutes IS NULL OR time_spent_minutes >= 0);
