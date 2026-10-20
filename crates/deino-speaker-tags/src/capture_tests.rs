use std::sync::Arc;

use deino_domain::{CallApp, MeetingId, Millis};

use super::*;
use crate::fake::{FakeMac, MemoryCapture, MemorySink, ScriptedClock};
use crate::ports::{Pid, Position};
use crate::status::LayoutHealth;
use crate::watch::{Step, Watch};

const ZOOM: Pid = Pid(101);
const CHROME: Pid = Pid(202);

fn window(title: &str, description: &str) -> AxNode {
    AxNode {
        role: "AXWindow".into(),
        title: Some(title.into()),
        children: vec![AxNode {
            role: "AXTabGroup".into(),
            description: Some(description.into()),
            ..AxNode::default()
        }],
        ..AxNode::default()
    }
}

/// A Zoom call window whose layout no adapter knows.
fn zoom_unknown() -> AxNode {
    window("Zoom Meeting", "Gallery")
}

/// Zoom's home window: Zoom runs, but no call window shows.
fn zoom_home() -> AxNode {
    window("Zoom Workplace", "Home")
}

/// Zoom's call window with Alex talking.
fn zoom_call() -> AxNode {
    window("Zoom Meeting", "Alex Chen, Computer audio unmuted")
}

struct Rig {
    mac: Arc<FakeMac>,
    capture: Arc<MemoryCapture>,
}

impl Rig {
    fn new(wanted: bool) -> Self {
        let mac = FakeMac::granted();
        mac.run("us.zoom.xos", ZOOM);
        let capture = Arc::new(MemoryCapture::default());
        capture.set_wanted(wanted);
        Self { mac, capture }
    }

    /// Records `meeting`: one poll per `(at, window)`, Zoom showing that
    /// window, then the recording ends.
    fn record(&self, meeting: &str, polls: &[(u32, AxNode)]) {
        let positions = polls.iter().map(|(at, _)| Position::At(Millis(*at)));
        let mut watch = Watch::new(WatchParts {
            meeting: MeetingId::new(meeting),
            platform: self.mac.platform(),
            clock: ScriptedClock::new(positions.chain([Position::Over])),
            sink: Arc::new(MemorySink::default()),
            health: Arc::new(LayoutHealth::default()),
            capture: self.capture.clone(),
        });
        for (_, shown) in polls {
            self.mac.show(ZOOM, Ok(vec![shown.clone()]));
            assert_eq!(watch.tick(), Step::Continue);
        }
        assert_eq!(watch.tick(), Step::Done);
    }

    /// Every dump saved: its meeting, app and window titles.
    fn dumps(&self) -> Vec<(String, CallApp, Vec<String>)> {
        self.capture
            .saved()
            .into_iter()
            .map(|(meeting, app, windows)| {
                let titles = windows.into_iter().filter_map(|w| w.title).collect();
                (meeting.as_str().to_owned(), app, titles)
            })
            .collect()
    }
}

/// `count` polls 250 ms apart from `from`, all showing `shown`.
fn polls(from: u32, count: u32, shown: &AxNode) -> Vec<(u32, AxNode)> {
    (0..count)
        .map(|index| (from + 250 * index, shown.clone()))
        .collect()
}

#[test]
fn an_unrecognised_layout_is_dumped_once_per_meeting() {
    let rig = Rig::new(true);

    rig.record("first", &polls(0, 4, &zoom_unknown()));
    rig.record("second", &polls(0, 2, &zoom_unknown()));

    let call = vec!["Zoom Meeting".to_owned()];
    assert_eq!(
        rig.dumps(),
        [
            ("first".into(), CallApp::Zoom, call.clone()),
            ("second".into(), CallApp::Zoom, call),
        ]
    );
    assert_eq!(rig.mac.full_reads(), 2);
    assert_eq!(rig.capture.saved()[0].2, [zoom_unknown()]);
}

#[test]
fn a_running_app_without_a_call_window_is_dumped_once_twenty_seconds_in() {
    let rig = Rig::new(true);
    // Chrome runs without a Meet room: its pages are never dumped.
    rig.mac.run("com.google.Chrome", CHROME);
    let gmail = AxNode {
        document: Some("https://mail.google.com/".into()),
        ..window("Inbox", "Mail")
    };
    rig.mac.show(CHROME, Ok(vec![gmail]));
    let home = zoom_home();

    rig.record("short", &[(0, home.clone()), (19_999, home.clone())]);
    assert!(rig.dumps().is_empty());
    rig.record("long", &polls(CAPTURE_AFTER.0, 4, &home));

    let home = vec!["Zoom Workplace".to_owned()];
    assert_eq!(rig.dumps(), [("long".into(), CallApp::Zoom, home)]);
}

#[test]
fn a_call_window_found_once_means_no_timeout_dump() {
    let rig = Rig::new(true);

    rig.record("meeting", &[(0, zoom_call()), (25_000, zoom_home())]);

    assert!(rig.dumps().is_empty());
    assert_eq!(rig.mac.full_reads(), 0);
}

#[test]
fn nothing_is_read_or_saved_when_dumps_are_not_wanted() {
    let rig = Rig::new(false);

    rig.record("unknown", &polls(0, 4, &zoom_unknown()));
    rig.record("home", &polls(CAPTURE_AFTER.0, 4, &zoom_home()));

    assert!(rig.dumps().is_empty());
    assert_eq!(rig.mac.full_reads(), 0);
}

#[test]
fn failed_reads_give_up_after_three() {
    let rig = Rig::new(true);
    rig.mac.show(ZOOM, Err(crate::error::AxError::Api(-25204)));
    let mut captures = Captures::default();
    let parts = WatchParts {
        meeting: MeetingId::new("meeting"),
        platform: rig.mac.platform(),
        clock: ScriptedClock::new([]),
        sink: Arc::new(MemorySink::default()),
        health: Arc::new(LayoutHealth::default()),
        capture: rig.capture.clone(),
    };

    for _ in 0..10 {
        captures.unrecognised(&parts, CallApp::Zoom);
    }

    assert_eq!(rig.mac.full_reads(), MAX_FAILED_READS);
    assert!(rig.dumps().is_empty());
}
