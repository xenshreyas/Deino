//! Every chat adapter on its app's fixture dumps: an open, empty composer
//! and its send button; a draft; a closed panel and the toggle among the
//! call's controls; and composers outside a call (Zoom's Team Chat, Teams'
//! chat window) that must never be used.

#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "an integration-test crate: helpers fail the test by panicking"
)]

#[allow(dead_code, reason = "shared support: this test reads views only")]
#[path = "support/dumps.rs"]
mod dumps;

use deino_consent::{ChatView, Composer};
use deino_domain::CallApp;
use deino_speaker_tags::AxNode;
use dumps::views;

/// The open composer of a dump's call window, and that window.
fn open(app: CallApp, case: &str) -> (AxNode, Composer) {
    views(app, case)
        .into_iter()
        .find_map(|(window, view)| match view {
            ChatView::Open(composer) => Some((window, composer)),
            _ => None,
        })
        .unwrap_or_else(|| panic!("{app}/{case}: no open composer"))
}

fn label(window: &AxNode, composer: &Composer) -> (String, String) {
    let field = composer.field.resolve(window).unwrap();
    let send = composer.send.as_ref().unwrap().resolve(window).unwrap();
    (
        field.description.clone().unwrap(),
        send.description.clone().unwrap(),
    )
}

#[test]
fn each_app_finds_its_empty_composer_and_send_button_in_the_call_window() {
    let cases = [
        (CallApp::Zoom, "Type message here...", "Send"),
        (CallApp::Teams, "Type a message", "Send (Enter)"),
        (CallApp::Meet, "Send a message", "Send a message"),
    ];
    for (app, field, send) in cases {
        let (window, composer) = open(app, "chat-open-empty");

        assert!(!composer.draft, "{app}");
        assert_eq!(composer.role, "AXTextArea", "{app}");
        assert_eq!(
            label(&window, &composer),
            (field.into(), send.into()),
            "{app}"
        );
        assert_eq!(composer.window, window.summary(), "{app}");
    }
}

#[test]
fn a_composer_holding_text_is_a_draft() {
    for app in CallApp::ALL {
        let (_, composer) = open(*app, "chat-open-drafting");

        assert!(composer.draft, "{app}");
    }
}

#[test]
fn a_closed_chat_is_opened_from_the_calls_controls() {
    let cases = [
        (CallApp::Zoom, "gallery-alex-talking", "Chat"),
        (CallApp::Teams, "call-alex-speaking", "Chat"),
        (CallApp::Meet, "call-alex-speaking", "Chat with everyone"),
    ];
    for (app, case, label) in cases {
        let (window, toggle) = views(app, case)
            .into_iter()
            .find_map(|(window, view)| match view {
                ChatView::Closed { toggle, .. } => Some((window, toggle)),
                _ => None,
            })
            .unwrap_or_else(|| panic!("{app}/{case}: no chat toggle"));
        let button = toggle.resolve(&window).unwrap();
        let controls = toggle.parent().unwrap().resolve(&window).unwrap();

        assert_eq!(button.description.as_deref(), Some(label), "{app}");
        // Teams' app bar has a "Chat" button too; the toggle is the call's.
        assert_ne!(controls.description.as_deref(), Some("App bar"), "{app}");
    }
}

#[test]
fn composers_outside_a_call_are_never_used() {
    let cases = [
        (CallApp::Zoom, "home-no-call"),
        (CallApp::Teams, "chat-no-call"),
        (CallApp::Meet, "lobby-no-call"),
    ];
    for (app, case) in cases {
        for (_, view) in views(app, case) {
            assert_eq!(view, ChatView::NotInCall, "{app}/{case}");
        }
    }
    // Zoom's home window has a Team Chat composer beside the call window:
    // only the call window's composer is found.
    let (window, _) = open(CallApp::Zoom, "chat-open-empty");
    assert_eq!(window.title.as_deref(), Some("Zoom Meeting"));
    // Teams' main chat window (a composer, no call) is not the call.
    let teams = views(CallApp::Teams, "chat-open-empty");
    assert_eq!(
        teams.first().map(|(_, view)| view),
        Some(&ChatView::NotInCall)
    );
}

#[test]
fn a_call_without_a_chat_or_its_button_shows_none() {
    for (_, view) in views(CallApp::Zoom, "screen-share-no-indicator") {
        assert!(
            !matches!(view, ChatView::Open(_)),
            "a screen share without the chat panel has no composer"
        );
    }
}
