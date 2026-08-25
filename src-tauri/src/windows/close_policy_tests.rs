//! Tests for which windows survive being closed.
//!
//! Destroying a transparent webview and building a replacement in the same
//! process leaves a dead drawing surface: the window is correct in every
//! observable way and paints blank anyway. PR 21 established this and fixed
//! the board close button. The rule belongs here, as a pure function over a
//! label, so every route to a close obeys it rather than only the one button.

use super::close_policy::hides_on_close;
use crate::storage::BoardKind;
use crate::windows::QUICK_ADD_LABEL;

#[test]
fn every_board_hides_rather_than_being_destroyed() {
    // Alt+F4 and the window menu reach a board without going through its
    // close button. They must not destroy it.
    for &kind in BoardKind::ALL {
        assert!(
            hides_on_close(&kind.window_label()),
            "{kind:?} must hide on close, never be destroyed"
        );
    }
}

#[test]
fn the_main_window_hides_rather_than_quitting() {
    // §26: the lightweight tier outlives the planning view.
    assert!(hides_on_close("main"));
}

#[test]
fn the_capture_box_hides_too() {
    // Quick Add is transparent as well, so recreating it hits the same dead
    // surface as a board would.
    assert!(hides_on_close(QUICK_ADD_LABEL));
}

#[test]
fn an_unknown_window_is_left_alone() {
    // Anything the app grows later closes normally until it opts in.
    assert!(!hides_on_close("some-future-window"));
}
