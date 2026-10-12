//! `Accelerator`: what a hotkey may be, and its canonical text.

use super::{Accelerator, HotkeyError};

fn canonical(text: &str) -> String {
    text.parse::<Accelerator>()
        .unwrap_or_else(|error| panic!("{text}: {error}"))
        .to_string()
}

fn refused(text: &str) -> HotkeyError {
    text.parse::<Accelerator>().expect_err(text)
}

#[test]
fn any_spelling_reads_as_the_canonical_text() {
    assert_eq!(canonical("Ctrl+Alt+Cmd+R"), "Ctrl+Alt+Cmd+R");
    assert_eq!(canonical("cmd+option+control+r"), "Ctrl+Alt+Cmd+R");
    assert_eq!(canonical("Meta + Shift + KeyN"), "Shift+Cmd+N");
    assert_eq!(canonical("Ctrl+Alt+Digit1"), "Ctrl+Alt+1");
    assert_eq!(canonical("Ctrl+Alt+space"), "Ctrl+Alt+Space");
    assert_eq!(canonical("Alt+Shift+arrowup"), "Alt+Shift+ArrowUp");
}

#[test]
fn the_canonical_text_reads_back_the_same() {
    for text in ["Ctrl+Alt+Cmd+R", "Shift+Cmd+N", "Ctrl+Alt+Comma", "F13"] {
        assert_eq!(canonical(&canonical(text)), canonical(text));
    }
}

#[test]
fn function_keys_need_no_modifiers() {
    assert_eq!(canonical("F13"), "F13");
    assert_eq!(canonical("shift+f5"), "Shift+F5");
    assert!(matches!(refused("F21"), HotkeyError::Malformed));
}

#[test]
fn plain_combinations_that_would_take_a_key_from_every_app_are_refused() {
    for text in ["R", "Cmd+R", "Shift+R", "Shift+Alt", "Ctrl+N"] {
        let error = refused(text);
        assert!(
            matches!(error, HotkeyError::TooPlain | HotkeyError::Malformed),
            "{text}: {error:?}"
        );
    }
    assert!(matches!(refused("Cmd+R"), HotkeyError::TooPlain));
}

#[test]
fn macos_own_shortcuts_are_refused() {
    assert!(matches!(refused("Cmd+Shift+4"), HotkeyError::Reserved));
    assert!(matches!(refused("Ctrl+Cmd+Q"), HotkeyError::Reserved));
    assert!(matches!(refused("Cmd+Ctrl+Space"), HotkeyError::Reserved));
}

#[test]
fn malformed_text_is_refused() {
    for text in [
        "",
        "Ctrl+Alt",
        "Ctrl+Alt+R+T",
        "Ctrl+Alt+Escape",
        "Ctrl++R",
        "Ctrl+Alt+é",
    ] {
        assert!(matches!(refused(text), HotkeyError::Malformed), "{text}");
    }
}
