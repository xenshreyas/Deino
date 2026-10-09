use super::*;
use crate::testing::{
    CHROME, HANDY, PHOTO_BOOTH, SLACK, ZOOM, app, client, event, id, room, start,
};

fn found(snapshot: &SignalSnapshot) -> Option<Candidate> {
    resolve(snapshot, &HashSet::new())
}

fn capturing(clients: Vec<MicClient>) -> SignalSnapshot {
    SignalSnapshot {
        clients,
        microphone_running: true,
        ..SignalSnapshot::default()
    }
}

#[test]
fn a_meeting_app_capturing_is_a_strong_candidate_keyed_by_app() {
    let candidate = found(&capturing(vec![client(ZOOM, false)])).unwrap();

    assert_eq!(candidate.platform, Platform::Zoom);
    assert_eq!(candidate.dwell, Dwell::Short);
    assert_eq!(candidate.key, CandidateKey::app(&id(ZOOM)));
    assert_eq!(candidate.app, app(ZOOM));
}

#[test]
fn slack_counts_only_in_full_duplex() {
    assert_eq!(found(&capturing(vec![client(SLACK, false)])), None);

    let huddle = found(&capturing(vec![client(SLACK, true)])).unwrap();

    assert_eq!(huddle.platform, Platform::Slack);
    assert_eq!(huddle.platform.kind(), crate::MeetingKind::Huddle);
}

#[test]
fn a_frontmost_meeting_app_with_camera_and_microphone_counts_without_attribution() {
    let snapshot = SignalSnapshot {
        microphone_running: true,
        camera_running: true,
        frontmost: Some(app(SLACK)),
        ..SignalSnapshot::default()
    };

    assert_eq!(found(&snapshot).unwrap().platform, Platform::Slack);
    assert_eq!(
        found(&SignalSnapshot {
            camera_running: false,
            ..snapshot
        }),
        None
    );
}

#[test]
fn a_meet_room_in_a_capturing_browser_is_keyed_by_room() {
    let snapshot = SignalSnapshot {
        browser_meetings: vec![room(CHROME, "https://meet.google.com/abc-defg-hij", false)],
        ..capturing(vec![client(CHROME, true)])
    };

    let candidate = found(&snapshot).unwrap();

    assert_eq!(candidate.platform, Platform::GoogleMeet);
    assert_eq!(candidate.dwell, Dwell::Short);
    assert_eq!(candidate.app, app(CHROME));
    let room = crate::url::normalize("https://meet.google.com/abc-defg-hij").unwrap();
    assert_eq!(candidate.key, CandidateKey::room(&room));
}

#[test]
fn a_focused_room_counts_while_the_microphone_runs_even_unattributed() {
    let focused = SignalSnapshot {
        microphone_running: true,
        browser_meetings: vec![room(CHROME, "https://us02web.zoom.us/j/123", true)],
        ..SignalSnapshot::default()
    };
    let silent = SignalSnapshot {
        microphone_running: false,
        ..focused.clone()
    };

    assert_eq!(found(&focused).unwrap().platform, Platform::Zoom);
    assert_eq!(found(&silent), None);
}

#[test]
fn a_browser_room_outranks_a_meeting_app_and_a_meeting_app_outranks_browser_audio() {
    let both = capturing(vec![client(ZOOM, false), client(CHROME, true)]);
    let with_room = SignalSnapshot {
        browser_meetings: vec![room(CHROME, "https://meet.google.com/abc-defg-hij", false)],
        ..both.clone()
    };

    assert_eq!(found(&with_room).unwrap().platform, Platform::GoogleMeet);
    assert_eq!(found(&both).unwrap().platform, Platform::Zoom);
}

#[test]
fn browser_audio_without_a_room_and_unknown_apps_need_a_sustained_hold() {
    let browser = found(&capturing(vec![client(CHROME, true)])).unwrap();
    let unknown = found(&capturing(vec![client(PHOTO_BOOTH, false)])).unwrap();

    assert_eq!(
        (browser.platform, browser.dwell),
        (Platform::Other, Dwell::Sustained)
    );
    assert_eq!(
        (unknown.platform, unknown.dwell),
        (Platform::Other, Dwell::Sustained)
    );
}

#[test]
fn denied_and_ignored_apps_never_count_but_do_not_hide_others() {
    let ignored: HashSet<BundleId> = [id(ZOOM)].into();
    let snapshot = SignalSnapshot {
        browser_meetings: vec![room(CHROME, "https://meet.google.com/abc-defg-hij", true)],
        ..capturing(vec![
            client(HANDY, false),
            client(ZOOM, true),
            client(CHROME, true),
        ])
    };

    assert_eq!(
        resolve(&capturing(vec![client(HANDY, false)]), &HashSet::new()),
        None
    );
    assert_eq!(
        resolve(&capturing(vec![client(ZOOM, true)]), &ignored),
        None
    );
    assert_eq!(
        resolve(&snapshot, &ignored).unwrap().platform,
        Platform::GoogleMeet
    );
}

#[test]
fn a_calendar_event_names_and_strengthens_any_candidate() {
    let snapshot = SignalSnapshot {
        event: Some(event(start())),
        ..capturing(vec![client(PHOTO_BOOTH, false)])
    };

    let candidate = found(&snapshot).unwrap();

    assert_eq!(candidate.dwell, Dwell::Short);
    assert_eq!(candidate.event, Some(event(start())));
}

#[test]
fn a_camera_alone_or_a_calendar_alone_is_not_a_meeting() {
    let camera_only = SignalSnapshot {
        camera_running: true,
        frontmost: Some(app(ZOOM)),
        ..SignalSnapshot::default()
    };
    let calendar_only = SignalSnapshot {
        event: Some(event(start())),
        ..SignalSnapshot::default()
    };

    assert_eq!(found(&camera_only), None);
    assert_eq!(found(&calendar_only), None);
}
