//! Every adapter on its app's fixture dumps (`fixtures/`): who is in the
//! call, who speaks, the user's own tile, crosstalk, a layout without an
//! indicator, an unknown layout, and windows without a call.

#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "an integration-test crate: helpers fail the test by panicking"
)]

#[path = "support/dumps.rs"]
mod dumps;

use deino_domain::{CallApp, DisplayName};
use deino_speaker_tags::{Active, ReadError};
use dumps::read;

fn name(text: &str) -> DisplayName {
    DisplayName::parse(text).unwrap()
}

fn speaker(app: CallApp, case: &str) -> Active {
    read(app, case).unwrap().active
}

/// Real Zoom (the `real-*` dumps): the user is Alex Chen (signed in to
/// Zoom Workplace), the one remote participant Bea Ruiz.
#[test]
fn zoom_speaker_view_names_the_large_tile_and_leaves_out_the_user() {
    let reading = read(CallApp::Zoom, "real-speaker-view-remote-talking").unwrap();

    assert_eq!(reading.participants, [name("Bea Ruiz")]);
    assert_eq!(reading.active, Active::Speaking(name("Bea Ruiz")));
}

#[test]
fn zoom_gallery_names_the_one_unmuted_remote() {
    let reading = read(CallApp::Zoom, "real-gallery-remote-unmuted").unwrap();

    assert_eq!(reading.participants, [name("Bea Ruiz")]);
    assert_eq!(reading.active, Active::Speaking(name("Bea Ruiz")));
}

#[test]
fn zoom_with_everyone_muted_names_nobody_but_keeps_the_roster() {
    let reading = read(CallApp::Zoom, "real-silent-all-muted").unwrap();

    assert_eq!(reading.participants, [name("Bea Ruiz")]);
    assert_eq!(reading.active, Active::Nobody);
}

#[test]
fn zoom_without_a_call_or_with_an_unknown_layout_reads_nothing() {
    assert_eq!(
        read(CallApp::Zoom, "home-no-call"),
        Err(ReadError::NotInCall)
    );
    assert_eq!(
        read(CallApp::Zoom, "unknown-layout"),
        Err(ReadError::UnrecognisedLayout)
    );
    // The hand-modelled call dump kept for deino-consent's chat tests has
    // no tile Zoom really shows.
    assert_eq!(
        read(CallApp::Zoom, "gallery-alex-talking"),
        Err(ReadError::UnrecognisedLayout)
    );
}

#[test]
fn teams_reads_the_speaking_tile_past_the_chat_window() {
    let reading = read(CallApp::Teams, "call-alex-speaking").unwrap();

    assert_eq!(reading.active, Active::Speaking(name("Alex Chen")));
    assert_eq!(
        reading.participants,
        [name("Alex Chen"), name("Bea Ruiz"), name("Dev Patel")]
    );
    assert_eq!(
        speaker(CallApp::Teams, "call-bea-speaking"),
        Active::Speaking(name("Bea Ruiz"))
    );
}

#[test]
fn teams_crosstalk_the_users_tile_and_the_chat_alone() {
    assert_eq!(speaker(CallApp::Teams, "call-crosstalk"), Active::Several);
    assert_eq!(speaker(CallApp::Teams, "call-me-speaking"), Active::Me);
    assert_eq!(
        read(CallApp::Teams, "chat-no-call"),
        Err(ReadError::NotInCall)
    );
}

#[test]
fn meet_reads_the_lit_tile_in_the_meet_window_not_gmail() {
    let reading = read(CallApp::Meet, "call-alex-speaking").unwrap();

    assert_eq!(reading.active, Active::Speaking(name("Alex Chen")));
    assert_eq!(
        reading.participants,
        [name("Alex Chen"), name("Bea Ruiz"), name("Cleo Martin")]
    );
    assert_eq!(
        speaker(CallApp::Meet, "call-cleo-speaking"),
        Active::Speaking(name("Cleo Martin"))
    );
}

#[test]
fn meet_silence_the_users_tile_and_a_presentation_without_indicators() {
    assert_eq!(speaker(CallApp::Meet, "call-silent"), Active::Nobody);
    assert_eq!(speaker(CallApp::Meet, "call-me-speaking"), Active::Me);
    let presenting = read(CallApp::Meet, "presenting-no-indicator").unwrap();
    assert_eq!(presenting.active, Active::NoIndicator);
    assert_eq!(presenting.participants, [name("Bea Ruiz")]);
}

#[test]
fn meet_in_the_lobby_or_with_an_unknown_layout_reads_nothing() {
    assert_eq!(
        read(CallApp::Meet, "lobby-no-call"),
        Err(ReadError::NotInCall)
    );
    assert_eq!(
        read(CallApp::Meet, "unknown-layout"),
        Err(ReadError::UnrecognisedLayout)
    );
}

#[test]
fn every_dump_belongs_to_its_app_and_no_other_adapter_reads_it() {
    let cases = [
        (CallApp::Zoom, "real-speaker-view-remote-talking"),
        (CallApp::Teams, "call-alex-speaking"),
        (CallApp::Meet, "call-alex-speaking"),
    ];
    for (owner, case) in cases {
        for other in CallApp::ALL.iter().filter(|app| **app != owner) {
            let adapter = deino_speaker_tags::adapter_for(*other);
            let windows = dumps::windows(owner, case);
            let read_by_other = windows
                .iter()
                .filter(|window| adapter.is_call_window(&window.summary()))
                .any(|window| adapter.read(window).is_ok());
            assert!(!read_by_other, "{other} read {owner}/{case}");
        }
    }
}
