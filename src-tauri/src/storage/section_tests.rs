//! Tests for board sections and their items.
//!
//! Sections are the one place in the app where the user's own words are the
//! whole content, so most of what follows guards those words: that they are
//! not silently trimmed away to nothing, not silently shortened, and not lost
//! when something next to them is deleted.

use super::*;

fn setup() -> (Db, SectionRepo) {
    (
        Db::open_in_memory().expect("open in-memory database"),
        SectionRepo::new(),
    )
}

#[test]
fn a_new_section_starts_empty() {
    let (db, repo) = setup();

    let section = repo
        .create(db.conn(), BoardKind::Priority, "Blockers")
        .unwrap();

    assert_eq!(section.board_kind, BoardKind::Priority);
    assert_eq!(section.title, "Blockers");
    assert!(
        section.items.is_empty(),
        "a section is a container the user fills, not one pre-filled for them"
    );
}

#[test]
fn a_section_can_be_created_on_a_board_that_has_never_been_opened() {
    // board_sections references board_windows, and a board has no row there
    // until its window is placed. Without the repository inserting the
    // board's defaults first, adding a section to a fresh install would fail
    // on a foreign key the user has no way to satisfy.
    let (db, repo) = setup();

    let created = repo.create(db.conn(), BoardKind::MonthlyProgress, "Themes");

    assert!(created.is_ok(), "got {created:?}");
}

#[test]
fn a_whitespace_only_title_is_rejected() {
    let (db, repo) = setup();

    let created = repo.create(db.conn(), BoardKind::Priority, "   \t\n ");

    assert!(matches!(created, Err(StorageError::Validation { .. })));
}

#[test]
fn a_title_is_trimmed_before_it_is_stored() {
    // Otherwise two sections the user reads as the same name sort and display
    // differently, and neither of them looks wrong on screen.
    let (db, repo) = setup();

    let section = repo
        .create(db.conn(), BoardKind::Priority, "  Blockers  ")
        .unwrap();

    assert_eq!(section.title, "Blockers");
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
    // A cap counted in bytes would reject an eighty-character note written in
    // any language that does not fit in ASCII.
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
    // Boards are separate notes on the desktop. A heading written on one
    // appearing on another would read as the app losing track of where things
    // were put.
    let (db, repo) = setup();

    repo.create(db.conn(), BoardKind::Priority, "Only on priority")
        .unwrap();

    let elsewhere = repo.list(db.conn(), BoardKind::WeeklyTasks).unwrap();

    assert!(elsewhere.is_empty());
}

#[test]
fn renaming_a_section_keeps_its_id_and_its_items() {
    let (db, repo) = setup();
    let section = repo
        .create(db.conn(), BoardKind::Priority, "Blockrs")
        .unwrap();
    repo.add_item(db.conn(), &section.id, "Waiting on review")
        .unwrap();

    let renamed = repo.rename(db.conn(), &section.id, "  Blockers  ").unwrap();

    assert_eq!(renamed.id, section.id);
    assert_eq!(renamed.title, "Blockers");
    assert_eq!(renamed.items.len(), 1);
}

#[test]
fn renaming_applies_the_same_rules_as_creating() {
    // A rename is the same act as naming, so a title that could not have been
    // created must not be reachable by editing one that could.
    let (db, repo) = setup();
    let section = repo
        .create(db.conn(), BoardKind::Priority, "Blockers")
        .unwrap();

    let blank = repo.rename(db.conn(), &section.id, "  ");
    let too_long = repo.rename(db.conn(), &section.id, &"x".repeat(MAX_TITLE_CHARS + 1));

    assert!(matches!(blank, Err(StorageError::Validation { .. })));
    assert!(matches!(too_long, Err(StorageError::Validation { .. })));
    assert_eq!(
        repo.get(db.conn(), &section.id).unwrap().unwrap().title,
        "Blockers",
        "a rejected rename must leave the old title in place"
    );
}

#[test]
fn renaming_a_section_that_does_not_exist_is_an_error() {
    let (db, repo) = setup();

    let renamed = repo.rename(db.conn(), "no-such-section", "Blockers");

    assert!(matches!(renamed, Err(StorageError::SectionNotFound { .. })));
}

#[test]
fn deleting_a_section_takes_its_items_with_it() {
    // The items are only ever seen inside their section, so leaving them
    // behind would grow the database with rows nothing can ever show.
    let (db, repo) = setup();
    let section = repo
        .create(db.conn(), BoardKind::Priority, "Blockers")
        .unwrap();
    repo.add_item(db.conn(), &section.id, "Waiting on review")
        .unwrap();
    repo.add_item(db.conn(), &section.id, "Waiting on access")
        .unwrap();

    repo.delete(db.conn(), &section.id).unwrap();

    let orphans: i64 = db
        .conn()
        .query_row("SELECT COUNT(*) FROM board_section_items", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(orphans, 0);
}

#[test]
fn deleting_a_section_leaves_the_other_sections_alone() {
    let (db, repo) = setup();
    let doomed = repo
        .create(db.conn(), BoardKind::Priority, "Doomed")
        .unwrap();
    let kept = repo.create(db.conn(), BoardKind::Priority, "Kept").unwrap();
    repo.add_item(db.conn(), &kept.id, "Still here").unwrap();
    repo.add_item(db.conn(), &doomed.id, "Going away").unwrap();

    repo.delete(db.conn(), &doomed.id).unwrap();

    let sections = repo.list(db.conn(), BoardKind::Priority).unwrap();
    assert_eq!(sections.len(), 1);
    assert_eq!(sections[0].items.len(), 1);
    assert_eq!(sections[0].items[0].text, "Still here");
}

#[test]
fn deleting_a_section_that_does_not_exist_is_an_error() {
    let (db, repo) = setup();

    let deleted = repo.delete(db.conn(), "no-such-section");

    assert!(matches!(deleted, Err(StorageError::SectionNotFound { .. })));
}

#[test]
fn items_are_appended_in_the_order_they_were_added() {
    let (db, repo) = setup();
    let section = repo
        .create(db.conn(), BoardKind::Priority, "Blockers")
        .unwrap();

    for text in ["First", "Second", "Third"] {
        repo.add_item(db.conn(), &section.id, text).unwrap();
    }

    let items: Vec<String> = repo.list(db.conn(), BoardKind::Priority).unwrap()[0]
        .items
        .iter()
        .map(|item| item.text.clone())
        .collect();

    assert_eq!(items, ["First", "Second", "Third"]);
}

#[test]
fn an_item_is_positioned_within_its_own_section() {
    // Positions are per-section. Counting across the whole board would make
    // the first item of a second section start at some arbitrary number, and
    // any later reorder arithmetic would be reasoning about the wrong list.
    let (db, repo) = setup();
    let first = repo
        .create(db.conn(), BoardKind::Priority, "First")
        .unwrap();
    let second = repo
        .create(db.conn(), BoardKind::Priority, "Second")
        .unwrap();
    repo.add_item(db.conn(), &first.id, "One").unwrap();
    repo.add_item(db.conn(), &first.id, "Two").unwrap();

    let item = repo.add_item(db.conn(), &second.id, "Alone").unwrap();

    assert_eq!(item.position, 0);
}

#[test]
fn an_items_text_is_trimmed_and_whitespace_only_text_is_rejected() {
    let (db, repo) = setup();
    let section = repo
        .create(db.conn(), BoardKind::Priority, "Blockers")
        .unwrap();

    let trimmed = repo
        .add_item(db.conn(), &section.id, "  Waiting on review \n")
        .unwrap();
    let blank = repo.add_item(db.conn(), &section.id, " \t ");

    assert_eq!(trimmed.text, "Waiting on review");
    assert!(matches!(blank, Err(StorageError::Validation { .. })));
}

#[test]
fn over_length_item_text_is_rejected_rather_than_truncated() {
    // Silently shortening someone's note loses their words, and they have no
    // way to know which ones went.
    let (db, repo) = setup();
    let section = repo
        .create(db.conn(), BoardKind::Priority, "Blockers")
        .unwrap();

    let added = repo.add_item(db.conn(), &section.id, &"x".repeat(MAX_ITEM_CHARS + 1));

    assert!(matches!(added, Err(StorageError::Validation { .. })));
}

#[test]
fn item_text_of_exactly_the_maximum_length_is_accepted() {
    let (db, repo) = setup();
    let section = repo
        .create(db.conn(), BoardKind::Priority, "Blockers")
        .unwrap();

    let added = repo.add_item(db.conn(), &section.id, &"x".repeat(MAX_ITEM_CHARS));

    assert!(added.is_ok(), "got {added:?}");
}

#[test]
fn adding_an_item_to_a_section_that_does_not_exist_is_an_error() {
    // Reported as a missing section rather than as a raw foreign key failure,
    // so the frontend can say which thing was gone.
    let (db, repo) = setup();

    let added = repo.add_item(db.conn(), "no-such-section", "Orphan");

    assert!(matches!(added, Err(StorageError::SectionNotFound { .. })));
}

#[test]
fn updating_an_item_applies_the_same_rules_as_adding_one() {
    let (db, repo) = setup();
    let section = repo
        .create(db.conn(), BoardKind::Priority, "Blockers")
        .unwrap();
    let item = repo
        .add_item(db.conn(), &section.id, "Waiting on review")
        .unwrap();

    let updated = repo
        .update_item(db.conn(), &item.id, "  Waiting on access  ")
        .unwrap();
    let blank = repo.update_item(db.conn(), &item.id, "   ");
    let too_long = repo.update_item(db.conn(), &item.id, &"x".repeat(MAX_ITEM_CHARS + 1));

    assert_eq!(updated.text, "Waiting on access");
    assert_eq!(updated.position, item.position, "editing is not reordering");
    assert!(matches!(blank, Err(StorageError::Validation { .. })));
    assert!(matches!(too_long, Err(StorageError::Validation { .. })));
}

#[test]
fn updating_an_item_that_does_not_exist_is_an_error() {
    let (db, repo) = setup();

    let updated = repo.update_item(db.conn(), "no-such-item", "Anything");

    assert!(matches!(updated, Err(StorageError::ItemNotFound { .. })));
}

#[test]
fn deleting_an_item_leaves_its_section_and_siblings_standing() {
    let (db, repo) = setup();
    let section = repo
        .create(db.conn(), BoardKind::Priority, "Blockers")
        .unwrap();
    let doomed = repo.add_item(db.conn(), &section.id, "Going away").unwrap();
    repo.add_item(db.conn(), &section.id, "Still here").unwrap();

    repo.delete_item(db.conn(), &doomed.id).unwrap();

    let sections = repo.list(db.conn(), BoardKind::Priority).unwrap();
    assert_eq!(sections.len(), 1);
    assert_eq!(sections[0].items.len(), 1);
    assert_eq!(sections[0].items[0].text, "Still here");
}

#[test]
fn deleting_an_item_that_does_not_exist_is_an_error() {
    let (db, repo) = setup();

    let deleted = repo.delete_item(db.conn(), "no-such-item");

    assert!(matches!(deleted, Err(StorageError::ItemNotFound { .. })));
}

#[test]
fn listing_carries_each_sections_own_items_and_nobody_elses() {
    let (db, repo) = setup();
    let blockers = repo
        .create(db.conn(), BoardKind::Priority, "Blockers")
        .unwrap();
    let wins = repo.create(db.conn(), BoardKind::Priority, "Wins").unwrap();
    repo.add_item(db.conn(), &blockers.id, "Waiting on review")
        .unwrap();
    repo.add_item(db.conn(), &wins.id, "Shipped the tray")
        .unwrap();

    let sections = repo.list(db.conn(), BoardKind::Priority).unwrap();

    assert_eq!(sections[0].items.len(), 1);
    assert_eq!(sections[0].items[0].text, "Waiting on review");
    assert_eq!(sections[0].items[0].section_id, blockers.id);
    assert_eq!(sections[1].items.len(), 1);
    assert_eq!(sections[1].items[0].section_id, wins.id);
}

#[test]
fn a_section_serialises_with_the_field_names_the_frontend_reads() {
    // The TypeScript side is written against these exact names. A rename here
    // is a silent breakage there: the field simply arrives as undefined.
    let section = BoardSection {
        id: "section-1".into(),
        board_kind: BoardKind::WeeklyTasks,
        title: "Blockers".into(),
        position: 0,
        items: vec![SectionItem {
            id: "item-1".into(),
            section_id: "section-1".into(),
            text: "Waiting on review".into(),
            position: 0,
        }],
    };

    let json = serde_json::to_value(&section).unwrap();

    assert_eq!(json["boardKind"], "weekly-tasks");
    assert_eq!(json["items"][0]["sectionId"], "section-1");
}
