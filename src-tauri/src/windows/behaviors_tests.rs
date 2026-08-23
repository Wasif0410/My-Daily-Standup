//! Tests for the rules a window behaviour change must obey.
//!
//! Every one runs against `apply_to_state`, which is pure over `BoardWindow`.
//! The Tauri calls in `apply_to_window` need a running runtime and are covered
//! by the manual matrix in the PR 16 plan.

use super::*;
use crate::storage::{BoardKind, BoardTheme, BoardWindow};

fn board() -> BoardWindow {
    BoardWindow::default_for(BoardKind::Priority)
}

#[test]
fn always_on_top_clears_desktop_level() {
    // Both at once is meaningless. Enforced here so no UI can produce it.
    let mut b = board();
    apply_to_state(&mut b, Behavior::DesktopLevel(true));

    apply_to_state(&mut b, Behavior::AlwaysOnTop(true));

    assert!(b.always_on_top);
    assert!(!b.desktop_level);
}

#[test]
fn desktop_level_clears_always_on_top() {
    let mut b = board();
    apply_to_state(&mut b, Behavior::AlwaysOnTop(true));

    apply_to_state(&mut b, Behavior::DesktopLevel(true));

    assert!(b.desktop_level);
    assert!(!b.always_on_top);
}

#[test]
fn turning_always_on_top_off_leaves_desktop_level_alone() {
    // Clearing one must not set the other. A board switched off the top layer
    // belongs in the ordinary stack, not behind everything.
    let mut b = board();

    apply_to_state(&mut b, Behavior::AlwaysOnTop(false));

    assert!(!b.always_on_top);
    assert!(!b.desktop_level);
}

#[test]
fn turning_desktop_level_off_leaves_always_on_top_alone() {
    let mut b = board();
    apply_to_state(&mut b, Behavior::AlwaysOnTop(true));

    apply_to_state(&mut b, Behavior::DesktopLevel(false));

    assert!(b.always_on_top, "an unrelated flag must survive");
}

#[test]
fn opacity_is_clamped_to_the_floor() {
    // A fully transparent board is invisible *and* unclickable. The floor is
    // not a preference; it is the difference between a feature and a trap.
    let mut b = board();

    apply_to_state(&mut b, Behavior::Opacity(0.0));

    assert_eq!(b.opacity, 0.2);
}

#[test]
fn opacity_is_clamped_to_one() {
    let mut b = board();

    apply_to_state(&mut b, Behavior::Opacity(1.8));

    assert_eq!(b.opacity, 1.0);
}

#[test]
fn font_size_is_clamped_at_both_ends() {
    let mut b = board();

    apply_to_state(&mut b, Behavior::FontSize(4.0));
    assert_eq!(b.font_size, 10.0);

    apply_to_state(&mut b, Behavior::FontSize(99.0));
    assert_eq!(b.font_size, 24.0);
}

#[test]
fn a_font_size_inside_the_range_is_kept_exactly() {
    let mut b = board();

    apply_to_state(&mut b, Behavior::FontSize(17.0));

    assert_eq!(b.font_size, 17.0);
}

#[test]
fn locking_does_not_touch_anything_else() {
    let mut b = board();
    apply_to_state(&mut b, Behavior::Opacity(0.6));
    apply_to_state(&mut b, Behavior::AlwaysOnTop(true));

    apply_to_state(&mut b, Behavior::Locked(true));

    assert!(b.locked);
    assert_eq!(b.opacity, 0.6);
    assert!(b.always_on_top);
}

#[test]
fn unlocking_is_the_only_way_out_and_it_works() {
    // Click-through is derived from `locked`, never stored beside it. There is
    // no second flag that could stay set after this.
    let mut b = board();
    apply_to_state(&mut b, Behavior::Locked(true));

    apply_to_state(&mut b, Behavior::Locked(false));

    assert!(!b.locked);
}

#[test]
fn theme_and_density_round_trip() {
    let mut b = board();

    apply_to_state(&mut b, Behavior::Theme(BoardTheme::Light));
    apply_to_state(&mut b, Behavior::Compact(true));

    assert_eq!(b.theme, BoardTheme::Light);
    assert!(b.compact);
}

#[test]
fn visibility_is_a_behaviour_like_any_other() {
    let mut b = board();

    apply_to_state(&mut b, Behavior::Visible(true));

    assert!(b.visible);
}

#[test]
fn a_non_finite_opacity_falls_back_rather_than_poisoning_the_row() {
    // NaN fails every comparison, so a naive clamp would let it through and
    // the schema's CHECK would then reject the whole save — losing an
    // unrelated change the user made in the same click.
    let mut b = board();

    apply_to_state(&mut b, Behavior::Opacity(f64::NAN));

    assert!(b.opacity.is_finite());
    assert!((0.2..=1.0).contains(&b.opacity));
}

#[test]
fn a_non_finite_font_size_falls_back_too() {
    let mut b = board();

    apply_to_state(&mut b, Behavior::FontSize(f64::INFINITY));

    assert!(b.font_size.is_finite());
    assert!((10.0..=24.0).contains(&b.font_size));
}
