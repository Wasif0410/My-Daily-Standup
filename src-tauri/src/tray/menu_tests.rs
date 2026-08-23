//! Tests for the tray menu's shape.
//!
//! The menu is a pure list of descriptors precisely so this file can exist: a
//! menu assembled against a live `AppHandle` could only be checked by clicking
//! it.

use super::*;

fn ids(entries: &[MenuEntry]) -> Vec<&str> {
    entries.iter().map(|e| e.id).collect()
}

fn find<'a>(entries: &'a [MenuEntry], id: &str) -> &'a MenuEntry {
    entries
        .iter()
        .find(|e| e.id == id)
        .unwrap_or_else(|| panic!("no tray entry with id {id}"))
}

#[test]
fn the_menu_follows_the_order_in_the_spec() {
    let entries = menu_entries(false, false);

    // §6.8's list, in its order. Extra entries are allowed after it; the nine
    // the spec names are not allowed to move or disappear.
    let spec_order = [
        "open-boards",
        "daily-standup",
        "evening-check-in",
        "plan-week",
        "monthly-review",
        "quick-add",
        "pause-reminders",
        "settings",
    ];
    let found: Vec<&str> = ids(&entries)
        .into_iter()
        .filter(|id| spec_order.contains(id))
        .collect();

    assert_eq!(found, spec_order);
    assert_eq!(
        ids(&entries).last(),
        Some(&"quit"),
        "Quit belongs at the bottom, where every tray menu puts it"
    );
}

#[test]
fn opening_boards_is_available_now() {
    let entries = menu_entries(false, false);

    assert!(find(&entries, "open-boards").enabled);
}

#[test]
fn quick_add_is_available_now() {
    // The definition of done requires it to work.
    let entries = menu_entries(false, false);

    assert!(find(&entries, "quick-add").enabled);
}

#[test]
fn quit_is_always_available() {
    // Once the main window only hides, this is the only way out of the app.
    // A disabled Quit would strand the user in the tray.
    let entries = menu_entries(true, true);

    assert!(find(&entries, "quit").enabled);
}

#[test]
fn the_four_ai_entries_are_disabled() {
    // Wave 5. Present so the shape of the app is visible, disabled so they
    // cannot silently do nothing.
    let entries = menu_entries(false, false);

    for id in [
        "daily-standup",
        "evening-check-in",
        "plan-week",
        "monthly-review",
    ] {
        assert!(
            !find(&entries, id).enabled,
            "{id} should not be available yet"
        );
    }
}

#[test]
fn settings_is_disabled_until_pr_18() {
    let entries = menu_entries(false, false);

    assert!(!find(&entries, "settings").enabled);
}

#[test]
fn every_disabled_entry_says_why() {
    // A greyed-out row with no explanation reads as a bug rather than as a
    // feature that has not arrived.
    let entries = menu_entries(false, false);

    for entry in entries.iter().filter(|e| !e.enabled) {
        assert!(
            entry.label.contains(COMING_SOON.trim()),
            "{} is disabled but does not say why: {:?}",
            entry.id,
            entry.label
        );
    }
}

#[test]
fn no_available_entry_claims_to_be_coming_soon() {
    // The converse, so a label and its state cannot drift apart when Wave 5
    // enables one of these.
    let entries = menu_entries(false, false);

    for entry in entries.iter().filter(|e| e.enabled) {
        assert!(
            !entry.label.contains(COMING_SOON.trim()),
            "{} works but claims otherwise: {:?}",
            entry.id,
            entry.label
        );
    }
}

#[test]
fn pause_reminders_shows_its_current_state() {
    assert_eq!(
        find(&menu_entries(false, false), "pause-reminders").checked,
        Some(false)
    );
    assert_eq!(
        find(&menu_entries(true, false), "pause-reminders").checked,
        Some(true)
    );
}

#[test]
fn unlock_all_boards_is_present() {
    // PR 16 built the escape hatch and named this as its surface.
    let entries = menu_entries(false, false);

    let unlock = find(&entries, "unlock-boards");
    assert!(unlock.enabled);
    assert!(unlock.label.to_lowercase().contains("unlock"));
}

#[test]
fn start_with_windows_reflects_the_stored_setting() {
    assert_eq!(
        find(&menu_entries(false, false), "autostart").checked,
        Some(false)
    );
    assert_eq!(
        find(&menu_entries(false, true), "autostart").checked,
        Some(true)
    );
}

#[test]
fn plain_entries_are_not_checkable() {
    // A tick on "Quit" would suggest a state it does not have.
    let entries = menu_entries(false, false);

    assert_eq!(find(&entries, "quit").checked, None);
    assert_eq!(find(&entries, "open-boards").checked, None);
}

#[test]
fn every_id_is_unique() {
    // Two rows sharing an id means one of them silently runs the other's
    // action.
    let entries = menu_entries(false, false);
    let mut seen = ids(&entries);
    seen.sort_unstable();
    let count = seen.len();
    seen.dedup();

    assert_eq!(seen.len(), count, "duplicate tray menu id");
}
