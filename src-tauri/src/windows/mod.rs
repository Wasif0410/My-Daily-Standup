//! Sticky-note desktop windows.
//!
//! Each board is its own frameless `WebviewWindow`, not a panel inside the main
//! window. That is what lets the boards stay on the desktop after the main
//! window closes — the central promise of spec §26's lightweight tier.

pub mod behaviors;
pub mod board_windows;

pub use behaviors::{apply_to_state, apply_to_window, unlock_all, Behavior};
pub use board_windows::{close_board, open_board, restore_boards, save_geometry};
#[cfg(test)]
mod behaviors_tests;
