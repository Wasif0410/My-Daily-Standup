//! Tests for board sections — the named task groups a board renders.
//!
//! A section is a heading over real tasks, so most of what follows guards the
//! tasks rather than the heading: that renaming a group carries its tasks with
//! it instead of splitting them off under a name nothing points at any more,
//! and that deleting a heading never deletes the work filed beneath it.

use super::*;

fn setup() -> (Db, SectionRepo) {
    (
        Db::open_in_memory().expect("open in-memory database"),
        SectionRepo::new(),
    )
}

/// A task filed under `area` — the column the Priority board groups by.
fn task_in_area(db: &Db, title: &str, area: &str) -> Task {
    let mut input = NewTask::new(title, TaskHorizon::Weekly, TaskSource::Manual);
    input.area = Some(area.to_string());
    TaskRepo::new()
        .create(db.conn(), input)
        .expect("create task")
}

/// A task filed under `project` — the column the Weekly Tasks board groups by.
fn task_in_project(db: &Db, title: &str, project: &str) -> Task {
    let mut input = NewTask::new(title, TaskHorizon::Weekly, TaskSource::Manual);
    input.project = Some(project.to_string());
    TaskRepo::new()
        .create(db.conn(), input)
        .expect("create task")
}

fn reload(db: &Db, id: &str) -> Task {
    TaskRepo::new()
        .get(db.conn(), id)
        .expect("read task")
        .expect("the task should still exist")
}

#[test]
fn a_new_section_is_a_declared_group_on_its_board() {
    let (db, repo) = setup();

    let section = repo
        .create(db.conn(), BoardKind::Priority, "Job Search")
        .unwrap();

    assert_eq!(section.board_kind, BoardKind::Priority);
    assert_eq!(section.title, "Job Search");
}

#[test]
fn a_declared_group_is_listed_before_any_task_is_filed_under_it() {
    // This is the whole reason the table still exists. A group derived from
    // tasks vanishes the moment nothing carries its name, so a heading the
    // user has just typed would disappear before they could fill it.
    let (db, repo) = setup();

    repo.create(db.conn(), BoardKind::Priority, "Job Search")
        .unwrap();

    let titles: Vec<String> = repo
        .list(db.conn(), BoardKind::Priority)
        .unwrap()
        .into_iter()
        .map(|section| section.title)
        .collect();
    assert_eq!(titles, ["Job Search"]);
}

#[test]
fn a_section_can_be_created_on_a_board_that_has_never_been_opened() {
    // board_sections references board_windows, and a board has no row there
    // until its window is placed. Without the repository inserting the
    // board's defaults first, adding a section to a fresh install would fail
    // on a foreign key the user has no way to satisfy.
    let (db, repo) = setup();

    let created = repo.create(db.conn(), BoardKind::WeeklyTasks, "Standup App");

    assert!(created.is_ok(), "got {created:?}");
}

#[test]
fn the_boards_that_group_by_day_and_by_commitment_refuse_sections() {
    // Weekly Progress is seven fixed days and Monthly Progress is a list of
    // commitments. Neither groups by a name the user writes, so a section
    // there would be a heading with no column it could ever be stored in.
    let (db, repo) = setup();

    for kind in [BoardKind::WeeklyProgress, BoardKind::MonthlyProgress] {
        let created = repo.create(db.conn(), kind, "Job Search");

        assert!(
            matches!(created, Err(StorageError::Validation { .. })),
            "{kind:?} should refuse a section, got {created:?}"
        );
    }
}

#[test]
fn a_whitespace_only_title_is_rejected() {
    let (db, repo) = setup();

    let created = repo.create(db.conn(), BoardKind::Priority, "   \t\n ");

    assert!(matches!(created, Err(StorageError::Validation { .. })));
}

#[test]
fn a_title_is_trimmed_before_it_is_stored() {
    // Otherwise two groups the user reads as the same name sort and display
    // differently, and neither of them looks wrong on screen.
    let (db, repo) = setup();

    let section = repo
        .create(db.conn(), BoardKind::Priority, "  Job Search  ")
        .unwrap();

    assert_eq!(section.title, "Job Search");
}

#[test]
fn an_over_length_title_is_rejected_rather_than_truncated() {
    let (db, repo) = setup();
    let too_long = "x".repeat(MAX_TITLE_CHARS + 1);

    let created = repo.create(db.conn(), BoardKind::Priority, &too_long);

    assert!(matches!(created, Err(StorageError::Validation { .. })));
}

#[test]
fn a_title_of_exactly_the_maximum_length_is_accepted() {
    // The cap is a limit, not a fence one character short of one.
    let (db, repo) = setup();
    let at_limit = "x".repeat(MAX_TITLE_CHARS);

    let section = repo
        .create(db.conn(), BoardKind::Priority, &at_limit)
        .unwrap();

    assert_eq!(section.title.chars().count(), MAX_TITLE_CHARS);
}

#[test]
fn a_title_is_measured_in_characters_not_bytes() {
    // A cap counted in bytes would reject an eighty-character heading written
    // in any language that does not fit in ASCII.
    let (db, repo) = setup();
    let accented = "é".repeat(MAX_TITLE_CHARS);

    let section = repo.create(db.conn(), BoardKind::Priority, &accented);

    assert!(section.is_ok(), "got {section:?}");
}

#[test]
fn sections_are_appended_in_the_order_they_were_created() {
    let (db, repo) = setup();

    for title in ["First", "Second", "Third"] {
        repo.create(db.conn(), BoardKind::Priority, title).unwrap();
    }

    let titles: Vec<String> = repo
        .list(db.conn(), BoardKind::Priority)
        .unwrap()
        .into_iter()
        .map(|section| section.title)
        .collect();

    assert_eq!(titles, ["First", "Second", "Third"]);
}

#[test]
fn a_section_added_after_a_deletion_still_lands_at_the_end() {
    // Positions come from the current maximum, not from a count, so removing
    // a section in the middle must not make the next one collide with a
    // position that is already taken.
    let (db, repo) = setup();

    let first = repo
        .create(db.conn(), BoardKind::Priority, "First")
        .unwrap();
    repo.create(db.conn(), BoardKind::Priority, "Second")
        .unwrap();
    repo.delete(db.conn(), &first.id).unwrap();

    repo.create(db.conn(), BoardKind::Priority, "Third")
        .unwrap();

    let titles: Vec<String> = repo
        .list(db.conn(), BoardKind::Priority)
        .unwrap()
        .into_iter()
        .map(|section| section.title)
        .collect();
    assert_eq!(titles, ["Second", "Third"]);
}

#[test]
fn each_board_has_its_own_sections() {
    // Boards are separate notes on the desktop, and they group by different
    // columns. A heading declared on one appearing on another would read as
    // the app losing track of where things were put.
    let (db, repo) = setup();

    repo.create(db.conn(), BoardKind::Priority, "Only on priority")
        .unwrap();

    let elsewhere = repo.list(db.conn(), BoardKind::WeeklyTasks).unwrap();

    assert!(elsewhere.is_empty());
}

#[test]
fn renaming_a_section_keeps_its_id_and_gives_it_the_new_title() {
    let (db, repo) = setup();
    let section = repo
        .create(db.conn(), BoardKind::Priority, "Job Serch")
        .unwrap();

    let renamed = repo
        .rename(db.conn(), &section.id, "  Job Search  ")
        .unwrap();

    assert_eq!(renamed.id, section.id);
    assert_eq!(renamed.title, "Job Search");
    assert_eq!(renamed.position, section.position, "renaming is not moving");
}

#[test]
fn renaming_a_priority_section_rewrites_the_area_of_its_tasks() {
    // Without this the group splits in two: the declared heading says one
    // thing while every task under it still says the old name, and the board
    // renders both.
    let (db, repo) = setup();
    let section = repo
        .create(db.conn(), BoardKind::Priority, "Job Search")
        .unwrap();
    let task = task_in_area(&db, "Rewrite the CV", "Job Search");

    repo.rename(db.conn(), &section.id, "Job Hunt").unwrap();

    assert_eq!(reload(&db, &task.id).area.as_deref(), Some("Job Hunt"));
}

#[test]
fn renaming_a_weekly_tasks_section_rewrites_the_project_of_its_tasks() {
    // The Weekly Tasks board groups by project, not by area, so the same
    // rename has to reach a different column.
    let (db, repo) = setup();
    let section = repo
        .create(db.conn(), BoardKind::WeeklyTasks, "Standup App")
        .unwrap();
    let task = task_in_project(&db, "Ship the tray", "Standup App");

    repo.rename(db.conn(), &section.id, "Daily Standup")
        .unwrap();

    assert_eq!(
        reload(&db, &task.id).project.as_deref(),
        Some("Daily Standup")
    );
}

#[test]
fn renaming_matches_its_tasks_ignoring_case_and_surrounding_whitespace() {
    // "Job Search" and "job search " are one group on screen, so they have to
    // be one group here too — otherwise a rename leaves half of them behind.
    let (db, repo) = setup();
    let section = repo
        .create(db.conn(), BoardKind::Priority, "Job Search")
        .unwrap();
    let shouted = task_in_area(&db, "Rewrite the CV", "JOB SEARCH");
    let padded = task_in_area(&db, "Email the recruiter", "  job search  ");

    repo.rename(db.conn(), &section.id, "Job Hunt").unwrap();

    assert_eq!(reload(&db, &shouted.id).area.as_deref(), Some("Job Hunt"));
    assert_eq!(reload(&db, &padded.id).area.as_deref(), Some("Job Hunt"));
}

#[test]
fn renaming_a_section_leaves_tasks_in_other_groups_alone() {
    let (db, repo) = setup();
    let section = repo
        .create(db.conn(), BoardKind::Priority, "Job Search")
        .unwrap();
    let elsewhere = task_in_area(&db, "Book the dentist", "Health");
    let unfiled = TaskRepo::new()
        .create(
            db.conn(),
            NewTask::new("Read a book", TaskHorizon::Weekly, TaskSource::Manual),
        )
        .unwrap();

    repo.rename(db.conn(), &section.id, "Job Hunt").unwrap();

    assert_eq!(reload(&db, &elsewhere.id).area.as_deref(), Some("Health"));
    assert_eq!(reload(&db, &unfiled.id).area, None);
}

#[test]
fn renaming_a_priority_section_does_not_touch_the_project_column() {
    // A board names one column and one only. Rewriting both would rename a
    // group the user was not looking at, on a board they did not open.
    let (db, repo) = setup();
    let section = repo
        .create(db.conn(), BoardKind::Priority, "Job Search")
        .unwrap();
    let task = task_in_project(&db, "Rewrite the CV", "Job Search");

    repo.rename(db.conn(), &section.id, "Job Hunt").unwrap();

    assert_eq!(reload(&db, &task.id).project.as_deref(), Some("Job Search"));
}

#[test]
fn renaming_applies_the_same_rules_as_creating() {
    // A rename is the same act as naming, so a title that could not have been
    // created must not be reachable by editing one that could.
    let (db, repo) = setup();
    let section = repo
        .create(db.conn(), BoardKind::Priority, "Job Search")
        .unwrap();

    let blank = repo.rename(db.conn(), &section.id, "  ");
    let too_long = repo.rename(db.conn(), &section.id, &"x".repeat(MAX_TITLE_CHARS + 1));

    assert!(matches!(blank, Err(StorageError::Validation { .. })));
    assert!(matches!(too_long, Err(StorageError::Validation { .. })));
}

#[test]
fn a_rejected_rename_leaves_both_the_title_and_the_tasks_alone() {
    let (db, repo) = setup();
    let section = repo
        .create(db.conn(), BoardKind::Priority, "Job Search")
        .unwrap();
    let task = task_in_area(&db, "Rewrite the CV", "Job Search");

    let _ = repo.rename(db.conn(), &section.id, "   ");

    assert_eq!(
        repo.get(db.conn(), &section.id).unwrap().unwrap().title,
        "Job Search",
        "a rejected rename must leave the old title in place"
    );
    assert_eq!(reload(&db, &task.id).area.as_deref(), Some("Job Search"));
}

#[test]
fn renaming_a_section_that_does_not_exist_is_an_error() {
    let (db, repo) = setup();

    let renamed = repo.rename(db.conn(), "no-such-section", "Job Search");

    assert!(matches!(renamed, Err(StorageError::SectionNotFound { .. })));
}

#[test]
fn deleting_a_section_unfiles_its_tasks_instead_of_deleting_them() {
    // Someone tidying a heading away is tidying a heading, not throwing out
    // the work under it. The tasks survive and fall into the board's Unsorted
    // group, where they can be filed again.
    let (db, repo) = setup();
    let section = repo
        .create(db.conn(), BoardKind::Priority, "Job Search")
        .unwrap();
    let task = task_in_area(&db, "Rewrite the CV", "Job Search");

    repo.delete(db.conn(), &section.id).unwrap();

    let survivor = reload(&db, &task.id);
    assert_eq!(survivor.title, "Rewrite the CV");
    assert_eq!(survivor.area, None, "the task should be unfiled, not gone");
}

#[test]
fn deleting_a_weekly_tasks_section_unfiles_by_project() {
    let (db, repo) = setup();
    let section = repo
        .create(db.conn(), BoardKind::WeeklyTasks, "Standup App")
        .unwrap();
    let task = task_in_project(&db, "Ship the tray", "standup app ");

    repo.delete(db.conn(), &section.id).unwrap();

    assert_eq!(reload(&db, &task.id).project, None);
}

#[test]
fn deleting_a_section_leaves_tasks_in_other_groups_alone() {
    let (db, repo) = setup();
    let section = repo
        .create(db.conn(), BoardKind::Priority, "Job Search")
        .unwrap();
    let elsewhere = task_in_area(&db, "Book the dentist", "Health");

    repo.delete(db.conn(), &section.id).unwrap();

    assert_eq!(reload(&db, &elsewhere.id).area.as_deref(), Some("Health"));
}

#[test]
fn deleting_a_section_leaves_the_other_sections_alone() {
    let (db, repo) = setup();
    let doomed = repo
        .create(db.conn(), BoardKind::Priority, "Doomed")
        .unwrap();
    repo.create(db.conn(), BoardKind::Priority, "Kept").unwrap();

    repo.delete(db.conn(), &doomed.id).unwrap();

    let sections = repo.list(db.conn(), BoardKind::Priority).unwrap();
    assert_eq!(sections.len(), 1);
    assert_eq!(sections[0].title, "Kept");
}

#[test]
fn a_section_serialises_with_exactly_the_fields_the_frontend_reads() {
    // The TypeScript side is written against these exact names. A rename here
    // is a silent breakage there: the field simply arrives as undefined. An
    // extra field is just as bad — `items` used to be one, and a frontend
    // still reading it would render an empty list under every heading.
    let section = BoardSection {
        id: "section-1".into(),
        board_kind: BoardKind::WeeklyTasks,
        title: "Job Search".into(),
        position: 0,
    };

    let json = serde_json::to_value(&section).unwrap();

    assert_eq!(json["id"], "section-1");
    assert_eq!(json["boardKind"], "weekly-tasks");
    assert_eq!(json["title"], "Job Search");
    assert_eq!(json["position"], 0);
    let mut keys: Vec<&String> = json.as_object().unwrap().keys().collect();
    keys.sort();
    assert_eq!(keys, ["boardKind", "id", "position", "title"]);
}

#[test]
fn deleting_a_section_that_does_not_exist_is_an_error() {
    let (db, repo) = setup();

    let deleted = repo.delete(db.conn(), "no-such-section");

    assert!(matches!(deleted, Err(StorageError::SectionNotFound { .. })));
}
