//! Tests for week boundaries.

use chrono::{Datelike, NaiveDate, Weekday};

use super::*;

fn date(text: &str) -> NaiveDate {
    NaiveDate::parse_from_str(text, "%Y-%m-%d").expect("a valid test date")
}

#[test]
fn week_containing_a_midweek_date_spans_monday_to_sunday() {
    // 2026-08-23 is a Sunday; 2026-08-19 is the Wednesday before it.
    let week = week_containing(date("2026-08-19"), Weekday::Mon);

    assert_eq!(week.start, "2026-08-17");
    assert_eq!(week.end, "2026-08-23");
}

#[test]
fn week_containing_monday_returns_that_monday() {
    // A boundary date belongs to the week it starts, not the one before it.
    let week = week_containing(date("2026-08-17"), Weekday::Mon);

    assert_eq!(week.start, "2026-08-17");
    assert_eq!(week.end, "2026-08-23");
}

#[test]
fn week_containing_sunday_returns_the_preceding_monday() {
    let week = week_containing(date("2026-08-23"), Weekday::Mon);

    assert_eq!(week.start, "2026-08-17");
    assert_eq!(week.end, "2026-08-23");
}

#[test]
fn week_containing_respects_a_sunday_start() {
    // The setting PR 16 will drive. With a Sunday start, 2026-08-23 begins its
    // own week rather than ending the previous one.
    let week = week_containing(date("2026-08-23"), Weekday::Sun);

    assert_eq!(week.start, "2026-08-23");
    assert_eq!(week.end, "2026-08-29");
}

#[test]
fn week_containing_crosses_a_month_boundary() {
    // 2026-09-01 is a Tuesday, so its week starts in August. Weeks do not stop
    // at months, and a board that clipped them would lose Monday's work.
    let week = week_containing(date("2026-09-01"), Weekday::Mon);

    assert_eq!(week.start, "2026-08-31");
    assert_eq!(week.end, "2026-09-06");
}

#[test]
fn week_containing_crosses_a_year_boundary() {
    // 2027-01-01 is a Friday; its week starts on 2026-12-28.
    let week = week_containing(date("2027-01-01"), Weekday::Mon);

    assert_eq!(week.start, "2026-12-28");
    assert_eq!(week.end, "2027-01-03");
}

#[test]
fn current_week_is_seven_days_long() {
    let week = current_week(Weekday::Mon);

    let start = NaiveDate::parse_from_str(&week.start, "%Y-%m-%d").unwrap();
    let end = NaiveDate::parse_from_str(&week.end, "%Y-%m-%d").unwrap();

    assert_eq!(
        (end - start).num_days(),
        6,
        "a week is seven days inclusive"
    );
    assert_eq!(start.weekday(), Weekday::Mon);
}

#[test]
fn parse_weekday_accepts_the_names_the_frontend_sends() {
    assert_eq!(parse_weekday(Some("monday")), Weekday::Mon);
    assert_eq!(parse_weekday(Some("sunday")), Weekday::Sun);
}

#[test]
fn parse_weekday_defaults_to_monday() {
    // Spec §6.4's default. An unrecognised value falls back rather than
    // failing: a bad setting must not make the board unopenable.
    assert_eq!(parse_weekday(None), Weekday::Mon);
    assert_eq!(parse_weekday(Some("nonsense")), Weekday::Mon);
}
