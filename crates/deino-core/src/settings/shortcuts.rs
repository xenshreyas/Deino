//! The shortcuts section: Deino's system-wide hotkeys (DEV-05).
//!
//! Each hotkey is an accelerator such as `Ctrl+Alt+Cmd+R`, or `null` when
//! the user turned it off. `crate::hotkeys` validates and registers them;
//! a value it cannot read is reported there and left unregistered.

use serde::{Deserialize, Serialize};

/// The default "start or stop recording" hotkey: ⌃⌥⌘R.
pub const DEFAULT_RECORD_HOTKEY: &str = "Ctrl+Alt+Cmd+R";
/// The default "quick note" hotkey: ⌃⌥⌘N.
pub const DEFAULT_QUICK_NOTE_HOTKEY: &str = "Ctrl+Alt+Cmd+N";
/// The default "Ask" hotkey: ⌃⌥⌘J (⌘J opens Ask inside Deino).
pub const DEFAULT_ASK_HOTKEY: &str = "Ctrl+Alt+Cmd+J";

/// The shortcuts section of `Settings`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct ShortcutSettings {
    /// Starts a recording, or stops the one running.
    pub record: Option<String>,
    /// Opens a new note.
    pub quick_note: Option<String>,
    /// Opens the meeting's Ask panel, or Deino.
    pub ask: Option<String>,
}

impl Default for ShortcutSettings {
    fn default() -> Self {
        Self {
            record: Some(DEFAULT_RECORD_HOTKEY.to_owned()),
            quick_note: Some(DEFAULT_QUICK_NOTE_HOTKEY.to_owned()),
            ask: Some(DEFAULT_ASK_HOTKEY.to_owned()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hotkeys_default_on_and_stay_off_once_turned_off() {
        let missing: ShortcutSettings = serde_json::from_str("{}").unwrap();
        assert_eq!(missing, ShortcutSettings::default());
        assert_eq!(missing.record.as_deref(), Some(DEFAULT_RECORD_HOTKEY));

        let off: ShortcutSettings =
            serde_json::from_str(r#"{ "record": null, "ask": "Ctrl+Alt+A" }"#).unwrap();
        assert_eq!(off.record, None);
        assert_eq!(off.ask.as_deref(), Some("Ctrl+Alt+A"));
        assert_eq!(off.quick_note.as_deref(), Some(DEFAULT_QUICK_NOTE_HOTKEY));
    }
}
