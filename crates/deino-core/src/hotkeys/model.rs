//! The hotkey types: [`HotkeyAction`], [`HotkeyState`] and [`Hotkey`].

use serde::{Deserialize, Serialize};

use crate::links::{Intent, Navigation};
use crate::settings::{
    DEFAULT_ASK_HOTKEY, DEFAULT_QUICK_NOTE_HOTKEY, DEFAULT_RECORD_HOTKEY, ShortcutSettings,
};

/// What a hotkey does.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, specta::Type,
)]
#[serde(rename_all = "snake_case")]
pub enum HotkeyAction {
    /// Start a recording, or stop the one running.
    Record,
    /// A new note.
    QuickNote,
    /// The meeting's Ask panel, or Deino.
    Ask,
}

impl HotkeyAction {
    /// Every action, in the order Settings lists them.
    pub const ALL: [Self; 3] = [Self::Record, Self::QuickNote, Self::Ask];

    /// Its name in Settings and in messages.
    pub fn label(self) -> &'static str {
        match self {
            Self::Record => "Start or stop recording",
            Self::QuickNote => "Quick note",
            Self::Ask => "Ask",
        }
    }

    /// What pressing it asks Deino to do.
    pub fn intent(self) -> Intent {
        match self {
            Self::Record => Intent::ToggleRecording,
            Self::QuickNote => Intent::Show(Navigation::NewNote { event_id: None }),
            Self::Ask => Intent::Show(Navigation::Ask),
        }
    }

    /// The hotkey it has out of the box.
    pub fn default_accelerator(self) -> &'static str {
        match self {
            Self::Record => DEFAULT_RECORD_HOTKEY,
            Self::QuickNote => DEFAULT_QUICK_NOTE_HOTKEY,
            Self::Ask => DEFAULT_ASK_HOTKEY,
        }
    }

    /// Its saved hotkey in `settings`.
    pub fn saved(self, settings: &ShortcutSettings) -> Option<&str> {
        match self {
            Self::Record => settings.record.as_deref(),
            Self::QuickNote => settings.quick_note.as_deref(),
            Self::Ask => settings.ask.as_deref(),
        }
    }

    /// Saves `accelerator` as its hotkey in `settings`.
    pub fn save(self, settings: &mut ShortcutSettings, accelerator: Option<String>) {
        let slot = match self {
            Self::Record => &mut settings.record,
            Self::QuickNote => &mut settings.quick_note,
            Self::Ask => &mut settings.ask,
        };
        *slot = accelerator;
    }
}

/// Whether a hotkey works now.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum HotkeyState {
    /// Registered: it works in every app.
    Active,
    /// Turned off by the user.
    Off,
    /// Saved, but not working; `reason` says why (a conflict, an invalid
    /// combination).
    Failed {
        /// The user-facing reason.
        reason: String,
    },
}

/// One hotkey as Settings → Shortcuts shows it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Hotkey {
    /// What it does.
    pub action: HotkeyAction,
    /// Its name.
    pub label: String,
    /// The saved combination (`Ctrl+Alt+Cmd+R`), or `None` when off.
    pub accelerator: Option<String>,
    /// The combination it has out of the box.
    pub default_accelerator: String,
    /// Whether it works now.
    pub state: HotkeyState,
}
