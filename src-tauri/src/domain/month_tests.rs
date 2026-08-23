//! Tests for month boundaries.

use chrono::NaiveDate;

use super::*;

fn date(text: &str) -> NaiveDate {
    NaiveDate::parse_from_str(text, "%Y-%m-%d").expect("a valid test date")
}

#[test]
fn a_month_runs_from_the_first_to_the_last_day() {
    let month = month_containing(date("2026-08-19"));

    assert_eq!(month.start, "2026-08-01");
    assert_eq!(month.end, "2026-08-31");
}

#[test]
fn february_in_a_common_year_ends_on_the_twenty_eighth() {
    let month = month_containing(date("2026-02-10"));

    assert_eq!(month.end, "2026-02-28");
}

#[test]
fn february_in_a_leap_year_ends_on_the_twenty_ninth() {
    // The length has to come from the calendar, not a table someone typed out.
    let month = month_containing(date("2028-02-10"));

    assert_eq!(month.end, "2028-02-29");
}

#[test]
fn a_thirty_day_month_ends_on_the_thirtieth() {
    let month = month_containing(date("2026-09-15"));

    assert_eq!(month.end, "2026-09-30");
}

#[test]
fn december_ends_on_the_thirty_first_and_does_not_roll_the_year() {
    // "First of next month, minus a day" has to cross into January to find
    // December's end. Getting it wrong would report 2027-12-31.
    let month = month_containing(date("2026-12-15"));

    assert_eq!(month.start, "2026-12-01");
    assert_eq!(month.end, "2026-12-31");
}

#[test]
fn january_starts_the_year_correctly() {
    let month = month_containing(date("2026-01-01"));

    assert_eq!(month.start, "2026-01-01");
    assert_eq!(month.end, "2026-01-31");
}

#[test]
fn a_month_is_labelled_in_words() {
    // "AUGUST 2026" heads the board (spec §6.5). "2026-08" would be precise
    // and unreadable.
    let month = month_containing(date("2026-08-19"));

    assert_eq!(month.label, "August 2026");
}

#[test]
fn a_month_remembers_the_date_it_was_derived_from() {
    let month = month_containing(date("2026-08-19"));

    assert_eq!(month.today, "2026-08-19");
}

#[test]
fn current_month_contains_today() {
    let month = current_month();

    assert!(month.start <= month.today);
    assert!(month.today <= month.end);
}
