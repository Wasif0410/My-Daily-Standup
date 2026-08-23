//! The system tray (spec §6.8).

pub mod menu;

pub use menu::{menu_entries, MenuEntry, COMING_SOON};

#[cfg(test)]
mod menu_tests;
