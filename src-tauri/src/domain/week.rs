//! Week boundaries.
//!
//! The only place in the app a week's first and last day are worked out. Rust
//! owns dates (spec §3.6), so the frontend asks for a week rather than deriving
//! one from the browser's clock — which would disagree with the database the
//! moment a user crossed a timezone.

use chrono::{Local, NaiveDate, Weekday};
use serde::Serialize;

/// A week, as a pair of inclusive ISO-8601 dates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Week {
    pub start: String,
    pub end: String,
}

/// The week `date` falls in, given the day weeks start on.
///
/// A date that *is* the start day belongs to the week it opens, not the one it
/// follows — the boundary case that decides whether Monday's work shows up on
/// Monday.
pub fn week_containing(date: NaiveDate, starts_on: Weekday) -> Week {
    let week = date.week(starts_on);

    Week {
        start: week.first_day().format("%Y-%m-%d").to_string(),
        end: week.last_day().format("%Y-%m-%d").to_string(),
    }
}

/// The week today falls in.
///
/// `Local`, never `Utc`: a planner whose day rolls over at 8pm because the user
/// sits west of Greenwich is worse than no planner.
pub fn current_week(starts_on: Weekday) -> Week {
    week_containing(Local::now().date_naive(), starts_on)
}

/// Reads a week-start day sent from the frontend, defaulting to Monday.
///
/// An unrecognised value falls back rather than failing. A corrupt setting must
/// not be able to make a board unopenable.
pub fn parse_weekday(value: Option<&str>) -> Weekday {
    match value.map(str::to_ascii_lowercase).as_deref() {
        Some("sunday") | Some("sun") => Weekday::Sun,
        Some("saturday") | Some("sat") => Weekday::Sat,
        _ => Weekday::Mon,
    }
}
