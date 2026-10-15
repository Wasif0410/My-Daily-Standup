//! Tests for which windows survive being closed.
//!
//! Destroying a transparent webview and building a replacement in the same
//! process leaves a dead drawing surface: the window is correct in every
//! observable way and paints blank anyway. PR 21 established this and fixed
//! the board close button. The rule belongs here, as a pure function over a
//! label, so every route to a close obeys it rather than only the one button.

use super::close_policy::{hides_on_close, is_user_close_command, remembers_close, CloseSource};
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

#[test]
fn a_close_the_user_asked_for_is_remembered() {
    // Alt+F4 and the window menu: the board stays shut at the next launch.
    assert!(remembers_close(CloseSource::User));
}

#[test]
fn a_close_nobody_asked_for_leaves_visibility_alone() {
    // A bare WM_CLOSE reaching the boards one by one while the process is torn
    // down. Remembering it saved some boards as closed and not others, so the
    // next launch reopened only part of the desktop.
    assert!(!remembers_close(CloseSource::Other));
}

/// `WM_SYSCOMMAND`, `WM_CLOSE`, and two system commands, spelled out so the
/// tests do not share the constants they are checking.
const WM_SYSCOMMAND: u32 = 0x0112;
const WM_CLOSE: u32 = 0x0010;
const SC_CLOSE: usize = 0xF060;
const SC_MINIMIZE: usize = 0xF020;

#[test]
fn alt_f4_and_the_window_menu_count_as_the_user() {
    assert!(is_user_close_command(WM_SYSCOMMAND, SC_CLOSE));
}

#[test]
fn the_low_bits_windows_sets_do_not_hide_a_close() {
    // Windows uses the low four bits of the command internally. A close picked
    // from the window menu can carry them, and is still a close.
    assert!(is_user_close_command(WM_SYSCOMMAND, SC_CLOSE | 0x3));
}

#[test]
fn other_system_commands_are_not_closes() {
    assert!(!is_user_close_command(WM_SYSCOMMAND, SC_MINIMIZE));
}

#[test]
fn a_bare_wm_close_is_not_the_user() {
    // What taskkill without /F sends, and what any program closing another
    // app's windows sends. No keypress or menu stands behind it.
    assert!(!is_user_close_command(WM_CLOSE, 0));
}

#[test]
fn the_close_command_number_alone_is_not_enough() {
    // SC_CLOSE's value only means "close" inside WM_SYSCOMMAND.
    assert!(!is_user_close_command(WM_CLOSE, SC_CLOSE));
}
