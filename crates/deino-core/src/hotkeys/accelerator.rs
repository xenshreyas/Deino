//! [`Accelerator`]: a validated hotkey such as `Ctrl+Alt+Cmd+R`.
//!
//! The canonical text (`Ctrl+Alt+Shift+Cmd+<key>`) is what settings store
//! and what `tauri-plugin-global-shortcut` parses. A system-wide hotkey
//! takes the keys from every app, so plain ones are refused: a hotkey needs
//! two modifiers, one of them ⌘, ⌃ or ⌥, unless its key is F1–F20; and the
//! shortcuts macOS keeps for itself are refused too.

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

/// One validated hotkey.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Accelerator {
    ctrl: bool,
    alt: bool,
    shift: bool,
    cmd: bool,
    key: String,
}

impl Accelerator {
    fn modifier_count(&self) -> usize {
        [self.ctrl, self.alt, self.shift, self.cmd]
            .into_iter()
            .filter(|on| *on)
            .count()
    }

    fn is_function_key(&self) -> bool {
        function_key(&self.key).is_some()
    }

    fn check(self) -> Result<Self, HotkeyError> {
        let strong = self.ctrl || self.alt || self.cmd;
        if !self.is_function_key() && (!strong || self.modifier_count() < 2) {
            return Err(HotkeyError::TooPlain);
        }
        if RESERVED.contains(&self.to_string().as_str()) {
            return Err(HotkeyError::Reserved);
        }
        Ok(self)
    }

    fn set_modifier(&mut self, token: &str) -> bool {
        let flag = match token.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => &mut self.ctrl,
            "alt" | "option" => &mut self.alt,
            "shift" => &mut self.shift,
            "cmd" | "command" | "super" | "meta" => &mut self.cmd,
            _ => return false,
        };
        *flag = true;
        true
    }
}

impl FromStr for Accelerator {
    type Err = HotkeyError;

    /// Reads `Ctrl+Alt+Cmd+R`, in any case and modifier order.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let mut accelerator = Self {
            ctrl: false,
            alt: false,
            shift: false,
            cmd: false,
            key: String::new(),
        };
        for token in text.split('+').map(str::trim) {
            if accelerator.set_modifier(token) {
                continue;
            }
            if !accelerator.key.is_empty() {
                return Err(HotkeyError::Malformed);
            }
            accelerator.key = key(token).ok_or(HotkeyError::Malformed)?;
        }
        if accelerator.key.is_empty() {
            return Err(HotkeyError::Malformed);
        }
        accelerator.check()
    }
}

impl fmt::Display for Accelerator {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let modifiers = [
            (self.ctrl, "Ctrl+"),
            (self.alt, "Alt+"),
            (self.shift, "Shift+"),
            (self.cmd, "Cmd+"),
        ];
        for (_, name) in modifiers.into_iter().filter(|(on, _)| *on) {
            formatter.write_str(name)?;
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
