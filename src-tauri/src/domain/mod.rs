//! Business rules.
//!
//! Progress arithmetic, completion rules, and rollover accounting live here
//! rather than in the repository below or the UI above. Rust owns every
//! calculation the app displays; the language model contributes language, never
//! numbers (spec §3.6).

mod blocker;
mod commitment;
mod month;
mod progress;
mod rollover;
mod time_spent;
mod week;

#[cfg(test)]
mod blocker_tests;
#[cfg(test)]
mod commitment_tests;
#[cfg(test)]
mod month_tests;
#[cfg(test)]
mod progress_tests;
#[cfg(test)]
mod rollover_tests;
#[cfg(test)]
mod time_spent_tests;
#[cfg(test)]
mod week_tests;

pub use blocker::{add_comment, set_blocker, today};
pub use commitment::{commitment_progress, Commitment};
pub use month::{current_month, month_containing, Month};
pub use progress::{compute_progress, is_complete_by_rule, Progress};
pub use rollover::{move_to_period, period_stats, reschedule, PeriodStats};
pub use time_spent::{
    average_minutes, format_minutes, implausible_durations, minutes_by_area, minutes_by_project,
    minutes_in_period, UNASSIGNED,
};
pub use week::{current_week, parse_weekday, week_containing, Week, WeekDay};
