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

// ---- days and label ---------------------------------------------------------

#[test]
fn a_week_lists_seven_days_monday_first() {
    let week = week_containing(date("2026-08-19"), Weekday::Mon);

    let names: Vec<&str> = week.days.iter().map(|d| d.name.as_str()).collect();
    assert_eq!(
        names,
        vec![
            "Monday",
            "Tuesday",
            "Wednesday",
            "Thursday",
            "Friday",
            "Saturday",
            "Sunday"
        ]
    );
}

#[test]
fn a_week_lists_seven_days_sunday_first_when_it_starts_on_sunday() {
    // The names reorder too, not just the dates. A board headed "Monday" that
    // showed Sunday's tasks would be worse than no board.
    let week = week_containing(date("2026-08-19"), Weekday::Sun);

    assert_eq!(week.days[0].name, "Sunday");
    assert_eq!(week.days[0].date, "2026-08-16");
    assert_eq!(week.days[6].name, "Saturday");
}

#[test]
fn week_days_are_consecutive_dates() {
    let week = week_containing(date("2026-08-19"), Weekday::Mon);

    let dates: Vec<&str> = week.days.iter().map(|d| d.date.as_str()).collect();
    assert_eq!(
        dates,
        vec![
            "2026-08-17",
            "2026-08-18",
            "2026-08-19",
            "2026-08-20",
            "2026-08-21",
            "2026-08-22",
            "2026-08-23"
        ]
    );
}

#[test]
fn week_days_cross_a_month_boundary() {
    // 2026-09-01 is a Tuesday, so its week opens on 2026-08-31.
    let week = week_containing(date("2026-09-01"), Weekday::Mon);

    assert_eq!(week.days[0].date, "2026-08-31");
    assert_eq!(week.days[1].date, "2026-09-01");
    assert_eq!(week.days[6].date, "2026-09-06");
}

#[test]
fn the_first_and_last_day_match_the_week_boundaries() {
    let week = week_containing(date("2026-08-19"), Weekday::Mon);

    assert_eq!(week.days[0].date, week.start);
    assert_eq!(week.days[6].date, week.end);
}

#[test]
fn a_week_is_labelled_with_its_iso_week() {
    let week = week_containing(date("2026-08-19"), Weekday::Mon);

    assert_eq!(week.label, "2026-W34");
}

#[test]
fn the_iso_label_uses_the_iso_week_not_the_configured_start() {
    // "2026-W34" means one fixed thing. A Sunday-start week that renumbered
    // itself would be publishing a private calendar under a standard name.
    let monday_start = week_containing(date("2026-08-19"), Weekday::Mon);
    let sunday_start = week_containing(date("2026-08-19"), Weekday::Sun);

    assert_eq!(monday_start.label, sunday_start.label);
}

#[test]
fn an_iso_week_is_zero_padded() {
    // "2026-W5" would sort before "2026-W34" as text, and these labels are
    // read in lists.
    let week = week_containing(date("2026-02-02"), Weekday::Mon);

    assert_eq!(week.label, "2026-W06");
}
