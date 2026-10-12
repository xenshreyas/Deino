//! [`Accelerator`]: a validated hotkey such as `Ctrl+Alt+Cmd+R`.
//!
//! The canonical text (`Ctrl+Alt+Shift+Cmd+<key>`) is what settings store
//! and what `tauri-plugin-global-shortcut` parses. A system-wide hotkey
//! takes the keys from every app, so plain ones are refused: a hotkey needs
//! two modifiers, one of them ⌘, ⌃ or ⌥, unless its key is F1–F20; and the
//! shortcuts macOS keeps for itself are refused too.

use std::collections::BTreeSet;
use std::fmt;
use std::str::FromStr;

use super::error::HotkeyError;

/// Named keys, as the global-shortcut plugin spells them.
const NAMED_KEYS: [&str; 18] = [
    "Space",
    "Enter",
    "Tab",
    "Comma",
    "Period",
    "Slash",
    "Semicolon",
    "Quote",
    "BracketLeft",
    "BracketRight",
    "Backslash",
    "Minus",
    "Equal",
    "Backquote",
    "ArrowUp",
    "ArrowDown",
    "ArrowLeft",
    "ArrowRight",
];

/// macOS's own shortcuts (screenshots, log out, lock, Spotlight and input
/// sources, the Dock, full screen), which a hotkey must not take.
const RESERVED: [&str; 10] = [
    "Shift+Cmd+3",
    "Shift+Cmd+4",
    "Shift+Cmd+5",
    "Shift+Cmd+6",
    "Shift+Cmd+Q",
    "Ctrl+Cmd+Q",
    "Ctrl+Cmd+Space",
    "Alt+Cmd+Space",
    "Alt+Cmd+D",
    "Ctrl+Cmd+F",
];

/// A modifier key, in the order the canonical text lists them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Modifier {
    Ctrl,
    Alt,
    Shift,
    Cmd,
}

impl Modifier {
    /// `token` as a modifier (`ctrl`, `option`, `command`, ...), in any case.
    fn parse(token: &str) -> Option<Self> {
        match token.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => Some(Self::Ctrl),
            "alt" | "option" => Some(Self::Alt),
            "shift" => Some(Self::Shift),
            "cmd" | "command" | "super" | "meta" => Some(Self::Cmd),
            _ => None,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Ctrl => "Ctrl",
            Self::Alt => "Alt",
            Self::Shift => "Shift",
            Self::Cmd => "Cmd",
        }
    }
}

/// One validated hotkey.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Accelerator {
    modifiers: BTreeSet<Modifier>,
    key: String,
}

impl Accelerator {
    fn check(self) -> Result<Self, HotkeyError> {
        let strong = self.modifiers.iter().any(|m| *m != Modifier::Shift);
        let function = function_key(&self.key).is_some();
        if !function && (!strong || self.modifiers.len() < 2) {
            return Err(HotkeyError::TooPlain);
        }
        if RESERVED.contains(&self.to_string().as_str()) {
            return Err(HotkeyError::Reserved);
        }
        Ok(self)
    }
}

impl FromStr for Accelerator {
    type Err = HotkeyError;

    /// Reads `Ctrl+Alt+Cmd+R`, in any case and modifier order.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let mut modifiers = BTreeSet::new();
        let mut found: Option<String> = None;
        for token in text.split('+').map(str::trim) {
            if let Some(modifier) = Modifier::parse(token) {
                modifiers.insert(modifier);
            } else if found.is_some() {
                return Err(HotkeyError::Malformed);
            } else {
                found = Some(key(token).ok_or(HotkeyError::Malformed)?);
            }
        }
        let key = found.ok_or(HotkeyError::Malformed)?;
        Self { modifiers, key }.check()
    }
}

impl fmt::Display for Accelerator {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for modifier in &self.modifiers {
            write!(formatter, "{}+", modifier.name())?;
        }
        formatter.write_str(&self.key)
    }
}

/// The canonical spelling of key `token` (`r`, `KeyR`, `digit1`, `f5`,
/// `space`), or `None` for a key a hotkey cannot use.
fn key(token: &str) -> Option<String> {
    let bare = token
        .strip_prefix("Key")
        .or_else(|| token.strip_prefix("Digit"))
        .unwrap_or(token);
    let mut chars = bare.chars();
    if let (Some(c), None) = (chars.next(), chars.next()) {
        return c
            .is_ascii_alphanumeric()
            .then(|| c.to_ascii_uppercase().to_string());
    }
    if let Some(number) = function_key(bare) {
        return Some(format!("F{number}"));
    }
    NAMED_KEYS
        .iter()
        .find(|name| name.eq_ignore_ascii_case(bare))
        .map(|name| (*name).to_owned())
}

/// `n` for `F<n>` with 1 ≤ n ≤ 20.
fn function_key(key: &str) -> Option<u8> {
    let number = key.strip_prefix(['F', 'f'])?.parse::<u8>().ok()?;
    (1..=20).contains(&number).then_some(number)
}
