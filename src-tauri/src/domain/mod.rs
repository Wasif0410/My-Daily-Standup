//! Business rules.
//!
//! Progress arithmetic, completion rules, and rollover accounting live here
//! rather than in the repository below or the UI above. Rust owns every
//! calculation the app displays; the language model contributes language, never
//! numbers (spec §3.6).

mod progress;
mod rollover;

#[cfg(test)]
mod progress_tests;
#[cfg(test)]
mod rollover_tests;

pub use progress::{compute_progress, is_complete_by_rule, Progress};
pub use rollover::{period_stats, reschedule, PeriodStats};
