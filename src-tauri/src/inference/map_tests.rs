//! Tests for the commitment map.
//!
//! The map is the only tier that is always present, so most of what follows
//! guards two things: that it fits, and that nothing quietly falls out of it.
//! Those pull in opposite directions, which is why both caps and the
//! ungrouped pile are tested against each other rather than in isolation.

use super::map::*;
use crate::storage::{
    BoardKind, Db, NewTask, SectionRepo, Task, TaskHorizon, TaskRepo, TaskSource, TaskStatus,
};

/// A budget large enough that nothing is dropped for size.
const GENEROUS: usize = 100_000;

fn setup() -> (Db, TaskRepo) {
    (
        Db::open_in_memory().expect("open in-memory database"),
        TaskRepo::new(),
    )
}

/// Builds a task and lets the caller adjust it before it is stored.
fn task(db: &Db, title: &str, horizon: TaskHorizon, edit: impl FnOnce(&mut NewTask)) -> Task {
    let mut input = NewTask::new(title, horizon, TaskSource::Manual);
    edit(&mut input);
    TaskRepo::new()
        .create(db.conn(), input)
        .expect("create task")
}

fn monthly_in(db: &Db, title: &str, area: &str, priority: i64) -> Task {
    task(db, title, TaskHorizon::Monthly, |input| {
        input.area = Some(area.to_string());
        input.priority = Some(priority);
        input.status = TaskStatus::InProgress;
    })
}

fn weekly_in(db: &Db, title: &str, area: &str) -> Task {
    task(db, title, TaskHorizon::Weekly, |input| {
        input.area = Some(area.to_string());
    })
}

fn map_of(db: &Db, repo: &TaskRepo) -> CommitmentMap {
    build_map(repo, db.conn(), GENEROUS).expect("build the map")
}

#[test]
fn an_empty_database_produces_a_map_that_says_so() {
    let (db, repo) = setup();

    let rendered = map_of(&db, &repo).render();

    assert!(rendered.starts_with(HEADER));
    assert!(
        rendered.contains("nothing active"),
        "an empty map must say it is empty rather than being a bare header:\n{rendered}"
    );
}

#[test]
fn the_monthly_commitment_becomes_the_heading_line() {
    let (db, repo) = setup();
    task(&db, "applications this month", TaskHorizon::Monthly, |i| {
        i.area = Some("Job Search".to_string());
        i.priority = Some(9);
        i.status = TaskStatus::InProgress;
        i.progress_current = Some(12.0);
        i.progress_target = Some(20.0);
    });

    let rendered = map_of(&db, &repo).render();

    assert!(
        rendered.contains("Job Search [p9, in progress] — 12/20 applications this month"),
        "the heading must match the shape in spec §9.2:\n{rendered}"
    );
}

#[test]
fn weekly_milestones_indent_beneath_their_heading() {
    let (db, repo) = setup();
    monthly_in(&db, "applications this month", "Job Search", 9);
    task(&db, "Fall 2026 applications", TaskHorizon::Weekly, |i| {
        i.area = Some("Job Search".to_string());
        i.progress_current = Some(3.0);
        i.progress_target = Some(5.0);
    });

    let rendered = map_of(&db, &repo).render();

    // Deliberately NOT the spec's worked example, which reads
    // "....... weekly, 3/5 done". The spec's prose says every line carries
    // priority and status; its example forgot the priority. The prose wins,
    // because a model can only rank work whose rank it can see. This task has
    // none set, so it reads p-.
    assert!(
        rendered.contains("  Fall 2026 applications ....... p-, weekly, 3/5 done"),
        "a milestone line must indent and carry priority, horizon and detail:\n{rendered}"
    );
}

#[test]
fn a_repeatedly_deferred_child_shows_how_many_times_it_moved() {
    let (db, repo) = setup();
    let created = task(&db, "Schedule dental cleaning", TaskHorizon::Daily, |i| {
        i.area = Some("Health".to_string());
        // Giving a task its first date is scheduling, not deferral, so the
        // count only starts moving from a date it already had.
        i.scheduled_date = Some("2026-08-25".to_string());
    });
    crate::domain::reschedule(&repo, db.conn(), &created.id, "2026-09-01").unwrap();

    let rendered = map_of(&db, &repo).render();

    assert!(
        rendered.contains("daily, deferred 1×"),
        "a deferral count is the detail that makes the line worth reading:\n{rendered}"
    );
}

#[test]
fn a_blocked_child_says_blocked_rather_than_its_progress() {
    let (db, repo) = setup();
    task(&db, "Rewrite resume", TaskHorizon::Weekly, |i| {
        i.area = Some("Job Search".to_string());
        i.status = TaskStatus::Blocked;
    });

    let rendered = map_of(&db, &repo).render();

    assert!(rendered.contains("weekly, blocked"), "{rendered}");
}

#[test]
fn a_task_with_no_section_renders_ungrouped_rather_than_being_dropped() {
    let (db, repo) = setup();
    task(&db, "Call the clinic", TaskHorizon::Weekly, |_| {});

    let map = map_of(&db, &repo);
    let rendered = map.render();

    assert!(
        rendered.contains(UNGROUPED_HEADING),
        "ungrouped work needs a heading of its own:\n{rendered}"
    );
    assert!(
        rendered.contains("Call the clinic"),
        "work filed under no section must still reach the model:\n{rendered}"
    );
}

#[test]
fn a_task_whose_group_is_only_blank_space_counts_as_ungrouped() {
    let (db, repo) = setup();
    task(&db, "Call the clinic", TaskHorizon::Weekly, |i| {
        i.area = Some("   ".to_string());
    });

    let rendered = map_of(&db, &repo).render();

    assert!(rendered.contains(UNGROUPED_HEADING), "{rendered}");
    assert!(rendered.contains("Call the clinic"), "{rendered}");
}

#[test]
fn a_group_is_matched_case_insensitively_and_trimmed() {
    let (db, repo) = setup();
    SectionRepo::new()
        .create(db.conn(), BoardKind::Priority, "Job Search")
        .unwrap();
    task(&db, "Rewrite resume", TaskHorizon::Weekly, |i| {
        i.area = Some("  job search ".to_string());
    });

    let map = map_of(&db, &repo);

    let named: Vec<_> = map
        .sections
        .iter()
        .filter(|section| section.title.is_some())
        .collect();
    assert_eq!(
        named.len(),
        1,
        "one heading on screen must be one heading here: {:#?}",
        map.sections
    );
    assert_eq!(named[0].title.as_deref(), Some("Job Search"));
    assert_eq!(named[0].milestones.len(), 1);
}

#[test]
fn a_declared_section_renders_even_with_nothing_filed_under_it() {
    let (db, repo) = setup();
    SectionRepo::new()
        .create(db.conn(), BoardKind::WeeklyTasks, "Health")
        .unwrap();

    let rendered = map_of(&db, &repo).render();

    assert!(
        rendered.contains("Health"),
        "a heading the user typed is a commitment even before it is filled:\n{rendered}"
    );
}

#[test]
fn a_group_the_boards_derive_from_tasks_alone_still_gets_a_heading() {
    let (db, repo) = setup();
    weekly_in(&db, "Book eye test", "Health");

    let map = map_of(&db, &repo);

    assert!(
        map.sections
            .iter()
            .any(|section| section.title.as_deref() == Some("Health")),
        "a group with no declared row is still the user's group: {:#?}",
        map.sections
    );
}

#[test]
fn completed_and_cancelled_work_is_absent_from_an_active_map() {
    let (db, repo) = setup();
    task(&db, "Finished thing", TaskHorizon::Weekly, |i| {
        i.area = Some("Job Search".to_string());
        i.status = TaskStatus::Completed;
    });
    task(&db, "Dropped thing", TaskHorizon::Weekly, |i| {
        i.area = Some("Job Search".to_string());
        i.status = TaskStatus::Cancelled;
    });
    weekly_in(&db, "Live thing", "Job Search");

    let rendered = map_of(&db, &repo).render();

    assert!(rendered.contains("Live thing"), "{rendered}");
    assert!(!rendered.contains("Finished thing"), "{rendered}");
    assert!(!rendered.contains("Dropped thing"), "{rendered}");
}

#[test]
fn the_map_never_exceeds_its_token_budget() {
    let (db, repo) = setup();
    for index in 0..60 {
        let area = format!("Area {index}");
        monthly_in(&db, &format!("commitment {index}"), &area, 5);
        for child in 0..6 {
            weekly_in(&db, &format!("milestone {index}-{child}"), &area);
        }
    }

    let budget = 400;
    let map = build_map(&repo, db.conn(), budget).unwrap();

    assert!(
        map.estimated_tokens <= budget,
        "the map spent {} of a {budget}-token budget",
        map.estimated_tokens
    );
    assert!(
        !map.sections.is_empty(),
        "a budget this size must still carry something"
    );
}

#[test]
fn the_map_budget_holds_at_every_size_not_just_a_lucky_one() {
    // Sections are joined by a blank line, and those newlines cost tokens.
    // One budget can happen to leave enough slack to hide that; a sweep
    // cannot.
    let (db, repo) = setup();
    for index in 0..30 {
        let area = format!("A{}", "x".repeat(index % 5));
        monthly_in(&db, &format!("c{index}"), &area, 5);
    }

    // Below about 25 tokens not even the header and the truncation notice fit,
    // so there is nothing for the budget to hold.
    for budget in 30..=400 {
        let map = build_map(&repo, db.conn(), budget).unwrap();

        assert!(
            map.estimated_tokens <= budget,
            "the map spent {} of a {budget}-token budget",
            map.estimated_tokens
        );
    }
}

#[test]
fn the_map_never_exceeds_its_section_count_cap() {
    let (db, repo) = setup();
    for index in 0..MAX_SECTIONS * 3 {
        monthly_in(&db, &format!("commitment {index}"), &format!("A{index}"), 5);
    }

    let map = map_of(&db, &repo);

    assert_eq!(
        map.sections.len(),
        MAX_SECTIONS,
        "the count cap holds even when the token budget is unlimited"
    );
    assert!(map.truncated, "a capped map must admit it was capped");
}

#[test]
fn the_map_never_exceeds_its_line_count_cap() {
    let (db, repo) = setup();
    for index in 0..MAX_SECTIONS {
        let area = format!("Area {index}");
        monthly_in(&db, &format!("commitment {index}"), &area, 5);
        for child in 0..MAX_MILESTONES_PER_SECTION * 2 {
            weekly_in(&db, &format!("milestone {index}-{child}"), &area);
        }
    }

    let map = map_of(&db, &repo);

    assert!(
        map.line_count() <= MAX_LINES,
        "the map rendered {} lines against a cap of {MAX_LINES}",
        map.line_count()
    );
}

#[test]
fn one_crowded_section_cannot_crowd_out_the_others() {
    let (db, repo) = setup();
    monthly_in(&db, "big commitment", "Crowded", 9);
    for index in 0..200 {
        weekly_in(&db, &format!("milestone {index}"), "Crowded");
    }
    monthly_in(&db, "small commitment", "Quiet", 8);

    let map = map_of(&db, &repo);

    assert!(
        map.render().contains("Quiet"),
        "a section with two hundred tasks must not consume the whole map:\n{}",
        map.render()
    );
}

#[test]
fn overflow_drops_the_lowest_ranked_sections_first() {
    let (db, repo) = setup();
    monthly_in(&db, "top commitment", "Highest", 9);
    monthly_in(&db, "middle commitment", "Middle", 5);
    monthly_in(&db, "bottom commitment", "Lowest", 1);

    // Room for the header, the truncation notice, and one heading.
    let map = build_map(&repo, db.conn(), 90).unwrap();
    let rendered = map.render();

    assert!(rendered.contains("Highest"), "{rendered}");
    assert!(
        !rendered.contains("Lowest"),
        "the lowest-ranked section must go first:\n{rendered}"
    );
    assert!(map.truncated);
}

#[test]
fn a_truncated_map_tells_the_model_it_is_incomplete() {
    let (db, repo) = setup();
    for index in 0..40 {
        monthly_in(&db, &format!("commitment {index}"), &format!("A{index}"), 5);
    }

    let map = build_map(&repo, db.conn(), 120).unwrap();
    let rendered = map.render();

    assert!(map.truncated);
    assert!(map.dropped_sections > 0);
    assert!(
        rendered.contains("incomplete"),
        "a map that merely stopped would read as a complete one:\n{rendered}"
    );
}

#[test]
fn a_deep_parent_chain_renders_without_hanging() {
    let (db, repo) = setup();

    let mut parent: Option<String> = None;
    for index in 0..500 {
        let created = task(&db, &format!("link {index}"), TaskHorizon::Daily, |input| {
            input.area = Some("Chain".to_string());
            input.parent_task_id = parent.clone();
        });
        parent = Some(created.id);
    }

    let map = map_of(&db, &repo);

    assert!(
        map.render().contains("Chain"),
        "a five-hundred-deep chain must render, and quickly"
    );
}

#[test]
fn a_cyclic_parent_chain_does_not_hang_the_map() {
    let (db, repo) = setup();
    let first = weekly_in(&db, "first", "Loop");
    let second = task(&db, "second", TaskHorizon::Weekly, |input| {
        input.area = Some("Loop".to_string());
        input.parent_task_id = Some(first.id.clone());
    });
    repo.update(
        db.conn(),
        &first.id,
        crate::storage::TaskPatch {
            parent_task_id: Some(Some(second.id.clone())),
            ..Default::default()
        },
    )
    .unwrap();

    let map = map_of(&db, &repo);

    assert!(map.render().contains("Loop"));
}

#[test]
fn a_heading_with_no_commitment_still_renders_its_children() {
    let (db, repo) = setup();
    weekly_in(&db, "Book eye test", "Health");

    let rendered = map_of(&db, &repo).render();

    assert!(rendered.contains("Health"), "{rendered}");
    assert!(rendered.contains("Book eye test"), "{rendered}");
    assert!(
        !rendered.contains("Health ["),
        "a group with no monthly commitment has no bracket to show:\n{rendered}"
    );
}

#[test]
fn a_section_led_by_a_commitment_outranks_one_that_merely_contains_urgent_work() {
    let (db, repo) = setup();
    monthly_in(&db, "the month's commitment", "Committed", 6);
    task(&db, "urgent daily", TaskHorizon::Daily, |input| {
        input.area = Some("Busy".to_string());
        input.priority = Some(6);
    });

    let map = map_of(&db, &repo);

    assert_eq!(
        map.sections[0].title.as_deref(),
        Some("Committed"),
        "a commitment decides a section's rank, not the loudest task inside it"
    );
}

#[test]
fn every_child_line_carries_its_priority() {
    // The spec is explicit: "Every line carries priority and status." Priority
    // is what the boards sort by and the field the user edits most, so a map
    // without it asks the model to rank work whose rank it cannot see. The
    // spec's worked example omits it from child lines; the prose does not, and
    // the prose is what the model needs.
    let (db, repo) = setup();
    task(&db, "Tailor the resume", TaskHorizon::Weekly, |input| {
        input.area = Some("Job Search".to_string());
        input.priority = Some(9);
    });

    let rendered = map_of(&db, &repo).render();

    assert!(
        rendered.contains("p9"),
        "a P9 task must show its priority on its own line:
{rendered}"
    );
}

#[test]
fn an_unprioritised_child_says_so_rather_than_looking_like_a_zero() {
    // `priority IS NULL` means nobody ranked it, which is a different claim
    // from ranked-lowest. The boards depend on that distinction and the model
    // must not read an absent rank as a bottom one.
    let (db, repo) = setup();
    task(&db, "dishes", TaskHorizon::Daily, |input| {
        input.area = Some("Life".to_string());
        input.priority = None;
    });

    let rendered = map_of(&db, &repo).render();

    assert!(
        rendered.contains("p-"),
        "an unranked task must be visibly unranked:
{rendered}"
    );
    assert!(
        !rendered.contains("p0"),
        "and must never render as priority zero:
{rendered}"
    );
}
