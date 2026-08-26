//! The tray menu, as data.
//!
//! A list of descriptors rather than Tauri menu items, so the menu's shape —
//! its order, what is available, and what each row says — is testable without a
//! running desktop. The same split [`crate::windows::behaviors`] uses: the
//! rules live where a test can reach them, and the platform call does not.

/// Appended to anything a later wave has not built yet.
///
/// In the label rather than a tooltip: Tauri 2 has no cross-platform tooltip on
/// a menu item, and a greyed-out row with no explanation reads as a bug rather
/// than as a feature that has not arrived.
pub const COMING_SOON: &str = " (coming soon)";

/// One row of the tray menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuEntry {
    pub id: &'static str,
    pub label: String,
    pub enabled: bool,
    /// `Some` for a checkable row, `None` for a plain one. A tick on "Quit"
    /// would suggest a state it does not have.
    pub checked: Option<bool>,
}

impl MenuEntry {
    /// An entry that works today.
    fn ready(id: &'static str, label: &str) -> Self {
        Self {
            id,
            label: label.to_string(),
            enabled: true,
            checked: None,
        }
    }

    /// An entry whose feature has not been built. Present so the shape of the
    /// app is visible, disabled so it cannot silently do nothing, and labelled
    /// so the user knows which of the two it is.
    fn pending(id: &'static str, label: &str) -> Self {
        Self {
            id,
            label: format!("{label}{COMING_SOON}"),
            enabled: true,
            checked: None,
        }
        .disabled()
    }

    fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    fn checkable(mut self, checked: bool) -> Self {
        self.checked = Some(checked);
        self
    }
}

/// Every row, in spec §6.8's order.
///
/// Two entries follow the spec's nine:
///
/// - **Unlock All Boards**, which PR 16 names as the escape hatch's surface. A
///   locked board is click-through, and this is where the spec says the way out
///   belongs.
/// - **Start with Windows**, because this PR is required to add the autostart
///   plugin and the settings window that would otherwise toggle it is PR 18.
///   Without a control here the dependency would ship dead.
pub fn menu_entries(autostart: bool) -> Vec<MenuEntry> {
    vec![
        MenuEntry::ready("open-boards", "Open Boards"),
        // Wave 5. Each starts a model session, and none of them exists yet.
        MenuEntry::pending("daily-standup", "Start Daily Standup"),
        MenuEntry::pending("evening-check-in", "Start Evening Check-In"),
        MenuEntry::pending("plan-week", "Plan My Week"),
        MenuEntry::pending("monthly-review", "Monthly Review"),
        // Explicitly not AI: capturing a task must not load anything (§6.8).
        MenuEntry::ready("quick-add", "Quick Add Task"),
        // Wave 4, with the standup. This was a live tick writing a flag
        // nothing read, so pausing reminders did nothing while looking as
        // though it had worked — a worse failure than a row that says it is
        // not ready, because the user only finds out when a reminder they
        // paused arrives anyway.
        MenuEntry::pending("pause-reminders", "Pause Reminders"),
        // Shows the main window, which is where settings render. They are not
        // a window of their own: a new label has to be granted in the
        // capabilities file, and a wrong identifier there is dropped silently
        // rather than failing the build.
        MenuEntry::ready("settings", "Settings"),
        MenuEntry::ready("unlock-boards", "Unlock All Boards"),
        MenuEntry::ready("autostart", "Start with Windows").checkable(autostart),
        // Once the main window only hides, this is the only way out — so it is
        // the one entry that is never disabled.
        MenuEntry::ready("quit", "Quit"),
    ]
}
