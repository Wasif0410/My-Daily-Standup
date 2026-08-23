//! Month boundaries.
//!
//! The companion to [`super::week`], and here for the same reason: Rust owns
//! dates (spec §3.6), so the Monthly board asks which month it is rather than
//! deriving one from the browser's clock.

use chrono::{Datelike, Days, Local, NaiveDate};
use serde::Serialize;

/// A month, as a pair of inclusive ISO-8601 dates plus a readable label.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Month {
    pub start: String,
    pub end: String,
    /// In words, e.g. "August 2026" — the board's heading (spec §6.5).
    pub label: String,
    /// The date this month was derived from. Via [`current_month`] — the only
    /// path the frontend uses — that is today.
    pub today: String,
}

/// The month `date` falls in.
///
/// The last day is found as "the first of next month, minus one day" rather
/// than from a table of month lengths. February is then correct in leap years
/// for free, and December crosses into January to find the 31st without
/// anybody special-casing the year boundary.
pub fn month_containing(date: NaiveDate) -> Month {
    let first = date.with_day(1).unwrap_or(date);

    let next_first = if first.month() == 12 {
        NaiveDate::from_ymd_opt(first.year() + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(first.year(), first.month() + 1, 1)
    };

    // The fallback cannot be reached for any date chrono can represent: every
    // month has a successor within range. Falling back to the first day rather
    // than panicking keeps a board openable if that ever stops being true.
    let last = next_first
        .and_then(|n| n.checked_sub_days(Days::new(1)))
        .unwrap_or(first);

    Month {
        start: first.format("%Y-%m-%d").to_string(),
        end: last.format("%Y-%m-%d").to_string(),
        label: first.format("%B %Y").to_string(),
        today: date.format("%Y-%m-%d").to_string(),
    }
}

/// The month today falls in.
///
/// `Local`, never `Utc`, matching [`super::current_week`]: a month that turned
/// over at 8pm on the 31st would be wrong on the one evening it mattered.
pub fn current_month() -> Month {
    month_containing(Local::now().date_naive())
}
