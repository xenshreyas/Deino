use super::*;
use crate::error::AxError;
use crate::fake::{FakeMac, MemorySink, ScriptedClock};
use crate::status::{TagState, statuses};

const ZOOM: Pid = Pid(101);
const CHROME: Pid = Pid(202);

fn tile(description: &str) -> AxNode {
    AxNode {
        role: "AXGroup".into(),
        description: Some(description.into()),
        ..AxNode::default()
    }
}

/// Zoom's call window with Alex, Bea and the user; `talking` is lit.
fn zoom_call(talking: Option<&str>) -> AxNode {
    let state = |name: &str| {
        let lit = if talking == Some(name) {
            ", Talking"
        } else {
            ""
        };
        format!("{name}, Computer audio unmuted, Video on{lit}")
    };
    AxNode {
        role: "AXWindow".into(),
        title: Some("Zoom Meeting".into()),
        children: vec![
            tile(&state("Alex Chen")),
            tile(&state("Bea Ruiz")),
            tile("Shreyas (Host, me), Computer audio muted, Video on"),
        ],
        ..AxNode::default()
    }
}

/// A Zoom call window whose layout no adapter knows.
fn zoom_unknown() -> AxNode {
    AxNode {
        role: "AXWindow".into(),
        title: Some("Zoom Meeting".into()),
        children: vec![tile("Gallery")],
        ..AxNode::default()
    }
}

struct Rig {
    mac: Arc<FakeMac>,
    sink: Arc<MemorySink>,
    health: Arc<LayoutHealth>,
}

impl Rig {
    fn new() -> Self {
        let mac = FakeMac::granted();
        mac.run("us.zoom.xos", ZOOM);
        Self {
            mac,
            sink: Arc::default(),
            health: Arc::default(),
        }
    }

    fn watch(&self, positions: Vec<Position>) -> Watch {
        Watch::new(WatchParts {
            meeting: MeetingId::new("meeting"),
            platform: self.mac.platform(),
            clock: ScriptedClock::new(positions),
            sink: self.sink.clone(),
            health: self.health.clone(),
            capture: Arc::new(crate::capture::NoCapture),
        })
    }

    /// Ticks at 0, 250, ... showing each frame of Zoom's window in turn,
    /// then lets the recording end.
    fn record(&self, frames: &[AxNode]) {
        let at = |index: usize| Position::At(Millis(250 * u32::try_from(index).unwrap()));
        let mut positions: Vec<Position> = (0..frames.len()).map(at).collect();
        positions.push(Position::Over);
        let mut watch = self.watch(positions);
        for frame in frames {
            self.mac.show(ZOOM, Ok(vec![frame.clone()]));
            assert_eq!(watch.tick(), Step::Continue);
        }
        assert_eq!(watch.tick(), Step::Done);
    }

    fn spans(&self) -> Vec<(u32, u32, String)> {
        self.sink
            .spans()
            .into_iter()
            .map(|span| (span.start.0, span.end.0, span.name.to_string()))
            .collect()
    }
}

#[test]
fn speaker_switches_in_a_zoom_call_become_spans_saved_at_the_end() {
    let rig = Rig::new();

    rig.record(&[
        zoom_call(Some("Alex Chen")),
        zoom_call(Some("Alex Chen")),
        zoom_call(Some("Bea Ruiz")),
        zoom_call(None),
    ]);

    assert_eq!(
        rig.spans(),
        [(0, 375, "Alex Chen".into()), (375, 625, "Bea Ruiz".into())]
    );
    assert!(
        rig.sink
            .spans()
            .iter()
            .all(|span| span.app == CallApp::Zoom)
    );
}

#[test]
fn without_permission_nothing_is_read_and_nothing_is_saved() {
    let rig = Rig::new();
    rig.mac.set_granted(false);

    rig.record(&[zoom_call(Some("Alex Chen")), zoom_call(Some("Alex Chen"))]);

    assert!(rig.spans().is_empty());
    assert!(rig.mac.prepared().is_empty());
}

#[test]
fn an_unknown_layout_degrades_and_marks_the_app_unsupported_until_it_reads_again() {
    let rig = Rig::new();

    rig.record(&[zoom_unknown(), zoom_unknown()]);

    assert!(rig.spans().is_empty());
    let zoom = statuses(true, true, &rig.health)[0];
    assert_eq!(zoom.state, TagState::Unsupported);
    rig.record(&[zoom_call(None)]);
    assert_eq!(
        statuses(true, true, &rig.health)[0].state,
        TagState::Enabled
    );
}

#[test]
fn an_unreadable_app_names_nobody() {
    let rig = Rig::new();
    let mut watch = rig.watch(vec![Position::At(Millis(0)), Position::Over]);
    rig.mac.show(ZOOM, Err(AxError::Api(-25204)));

    watch.tick();
    watch.tick();

    assert!(rig.spans().is_empty());
}

#[test]
fn a_pause_closes_the_open_span() {
    let rig = Rig::new();
    let alex = zoom_call(Some("Alex Chen"));
    rig.mac.show(ZOOM, Ok(vec![alex]));
    let mut watch = rig.watch(vec![
        Position::At(Millis(0)),
        Position::At(Millis(250)),
        Position::Paused,
        Position::At(Millis(300)),
        Position::Over,
    ]);

    while watch.tick() == Step::Continue {}

    assert_eq!(
        rig.spans(),
        [(0, 375, "Alex Chen".into()), (300, 425, "Alex Chen".into())]
    );
}

#[test]
fn spans_are_saved_every_ten_seconds_and_kept_when_a_save_fails() {
    let rig = Rig::new();
    let positions = [0, 250, 10_000, 10_250, 20_000]
        .map(|at| Position::At(Millis(at)))
        .to_vec();
    let mut watch = rig.watch(positions);
    let frames = [Some("Alex Chen"), None, Some("Bea Ruiz"), None, None];
    rig.sink.set_failing(true);

    for (index, talking) in frames.iter().enumerate() {
        rig.mac.show(ZOOM, Ok(vec![zoom_call(*talking)]));
        watch.tick();
        if index == 2 {
            assert!(rig.sink.spans().is_empty());
            rig.sink.set_failing(false);
        }
    }

    assert_eq!(rig.spans().len(), 2);
}

/// A Chrome window on `address`.
fn chrome_window(address: &str) -> AxNode {
    AxNode {
        role: "AXWindow".into(),
        title: Some("Chrome".into()),
        document: Some(address.into()),
        ..AxNode::default()
    }
}

#[test]
fn chrome_is_prepared_once_a_meet_room_shows_and_a_lobby_names_nobody() {
    let rig = Rig::new();
    rig.mac.run("com.google.Chrome", CHROME);
    rig.mac
        .show(CHROME, Ok(vec![chrome_window("https://mail.google.com/")]));
    let positions = [0, 250, 500].map(|at| Position::At(Millis(at))).to_vec();
    let mut watch = rig.watch(positions);

    watch.tick();
    assert!(rig.mac.prepared().is_empty());
    let lobby = chrome_window("https://meet.google.com/abc-defg-hij");
    rig.mac.show(CHROME, Ok(vec![lobby]));
    watch.tick();
    watch.tick();
    watch.finish();

    assert_eq!(rig.mac.prepared(), [(CallApp::Meet, CHROME)]);
    assert!(rig.spans().is_empty());
}
