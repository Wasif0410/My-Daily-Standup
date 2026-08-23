//! Week boundaries.
//!
//! The only place in the app a week's first and last day are worked out. Rust
//! owns dates (spec §3.6), so the frontend asks for a week rather than deriving
//! one from the browser's clock — which would disagree with the database the
//! moment a user crossed a timezone.

use chrono::{Datelike, Local, NaiveDate, Weekday};
use serde::Serialize;

/// One day of a week: the date to match tasks against, and the name to show.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeekDay {
    /// ISO-8601, e.g. "2026-08-17".
    pub date: String,
    /// English day name, e.g. "Monday".
    pub name: String,
}

/// A week: its boundaries, its ISO label, and its seven days.
///
/// One type answering one question, rather than three commands whose answers
/// could disagree about which week it is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Week {
    pub start: String,
    pub end: String,
    /// The ISO week, e.g. "2026-W34".
    pub label: String,
    /// The seven days, in display order.
    pub days: Vec<WeekDay>,
    /// The date this week was derived from. Via [`current_week`] — the only
    /// path the frontend uses — that is today, which is what lets a board mark
    /// today without consulting a clock of its own.
    pub today: String,
}

/// The week `date` falls in, given the day weeks start on.
///
/// A date that *is* the start day belongs to the week it opens, not the one it
/// follows — the boundary case that decides whether Monday's work shows up on
/// Monday.
///
/// The label is always the **ISO** week, even when weeks are configured to
/// start on Sunday. "2026-W34" means one fixed thing; renumbering it to match a
/// private start day would publish a different calendar under a standard name.
pub fn week_containing(date: NaiveDate, starts_on: Weekday) -> Week {
    let week = date.week(starts_on);
    let first = week.first_day();
    let last = week.last_day();

    let mut days = Vec::with_capacity(7);
    let mut cursor = first;
    while cursor <= last {
        days.push(WeekDay {
            date: cursor.format("%Y-%m-%d").to_string(),
            name: cursor.format("%A").to_string(),
        });

        // The last representable date has no successor. Stopping is correct:
        // there is no eighth day to add.
        match cursor.succ_opt() {
            Some(next) => cursor = next,
            None => break,
        }
    }

    let iso = date.iso_week();

    Week {
        start: first.format("%Y-%m-%d").to_string(),
        end: last.format("%Y-%m-%d").to_string(),
        // Zero-padded, because these labels are read in lists and "2026-W5"
        // would sort before "2026-W34" as text.
        label: format!("{}-W{:02}", iso.year(), iso.week()),
        days,
        today: date.format("%Y-%m-%d").to_string(),
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
