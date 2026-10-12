//! What can go wrong setting a hotkey. Each message is shown under the
//! hotkey in Settings → Shortcuts.

use deino_domain::{ErrorClass, ErrorCode};

use super::model::HotkeyAction;
use crate::settings::SettingsError;

/// A hotkey Deino cannot use.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum HotkeyError {
    /// Not a modifier-and-key combination Deino can read.
    #[error("that is not a key combination Deino can use")]
    Malformed,
    /// Too few modifiers: it would take a key from every app.
    #[error("use at least two modifier keys, one of them ⌘, ⌃ or ⌥ (or an F key)")]
    TooPlain,
    /// One of macOS's own shortcuts.
    #[error("macOS uses that shortcut")]
    Reserved,
    /// Another Deino hotkey has it.
    #[error("already used for “{}”", .0.label())]
    InUse(HotkeyAction),
    /// The system refused it: another app most likely holds it.
    #[error("another app is using that shortcut")]
    Unavailable,
    /// The setting could not be saved.
    #[error(transparent)]
    Settings(#[from] SettingsError),
}

impl ErrorClass for HotkeyError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::Malformed | Self::TooPlain | Self::Reserved => ErrorCode::Invalid,
            Self::InUse(_) | Self::Unavailable => ErrorCode::Conflict,
            Self::Settings(error) => error.code(),
        }
    }
}
