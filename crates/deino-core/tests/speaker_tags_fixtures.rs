//! From fixture dumps to won segments (CAP-05): a recording polls Zoom's
//! call window (`deino-speaker-tags`' fixture dumps) every 250 ms while the
//! active speaker switches, and the indicator goes missing for a shared
//! screen; the spans it saves are aligned with system-track segments by the
//! card's rule, more than half wins (`speaker_tags::align`).

#![allow(
    clippy::unwrap_used,
    reason = "an integration-test crate: helpers fail the test by panicking"
)]

use std::path::PathBuf;
use std::sync::Arc;

use deino_core::speaker_tags::Timed;
use deino_core::speaker_tags::align::align;
use deino_domain::{CallApp, MeetingId, Millis, SegmentId, SystemClock, TagSpan};
use deino_speaker_tags::fake::{FakeMac, MemorySink, ScriptedClock};
use deino_speaker_tags::{AxNode, LayoutHealth, NoCapture, Pid, Position, Step, Watch, WatchParts};

const ZOOM: Pid = Pid(4_242);

/// The windows of one of `deino-speaker-tags`' Zoom fixture dumps.
fn zoom_dump(case: &str) -> Vec<AxNode> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../deino-speaker-tags/fixtures/zoom")
        .join(format!("{case}.json"));
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// Records one poll per dump, 250 ms apart, and returns the saved spans.
fn record(frames: &[&str]) -> Vec<TagSpan> {
    let mac = FakeMac::granted();
    mac.run("us.zoom.xos", ZOOM);
    let sink = Arc::new(MemorySink::default());
    let positions = (0..frames.len()).map(|index| {
        let at = 250 * u32::try_from(index).unwrap();
        Position::At(Millis(at))
    });
    let mut watch = Watch::new(WatchParts {
        meeting: MeetingId::new("meeting"),
        platform: mac.platform(),
        clock: ScriptedClock::new(positions.chain([Position::Over])),
        sink: sink.clone(),
        health: Arc::new(LayoutHealth::default()),
        capture: Arc::new(NoCapture),
    });
    for frame in frames {
        mac.show(ZOOM, Ok(zoom_dump(frame)));
        assert_eq!(watch.tick(), Step::Continue);
    }
    assert_eq!(watch.tick(), Step::Done);
    sink.spans()
}

/// The name winning each `(start, end)` segment, if any.
fn winners(spans: &[TagSpan], segments: &[(u32, u32)]) -> Vec<Option<String>> {
    let clock = SystemClock;
    let timed: Vec<Timed> = segments
        .iter()
        .map(|&(start, end)| Timed {
            id: SegmentId::generate(&clock),
            start: Millis(start),
            end: Millis(end),
        })
        .collect();
    let wins = align(spans, &timed);
    timed
        .iter()
        .map(|segment| {
            let win = wins.iter().find(|win| win.segment == segment.id)?;
            Some(win.name.to_string())
        })
        .collect()
}

const ALEX: &str = "gallery-alex-talking";
const BEA: &str = "gallery-bea-talking";
const SILENT: &str = "gallery-silent";
const SHARING: &str = "screen-share-no-indicator";

#[test]
fn active_speaker_switches_become_spans_halfway_between_polls() {
    let spans = record(&[ALEX, ALEX, ALEX, BEA, BEA, BEA, SILENT]);

    let drawn: Vec<(u32, u32, &str)> = spans
        .iter()
        .map(|span| (span.start.0, span.end.0, span.name.as_str()))
        .collect();
    assert_eq!(drawn, [(0, 625, "Alex Chen"), (625, 1_375, "Bea Ruiz")]);
    assert!(spans.iter().all(|span| span.app == CallApp::Zoom));
}

#[test]
fn segments_go_to_the_name_lit_for_more_than_half_of_them() {
    let spans = record(&[ALEX, ALEX, ALEX, BEA, BEA, BEA, SILENT]);

    let named = winners(
        &spans,
        &[
            (0, 700),       // Alex 625 of 700
            (600, 1_000),   // Bea 375 of 400
            (425, 825),     // Alex 200, Bea 200: exactly half each
            (424, 825),     // Alex 201 of 401
            (1_300, 1_700), // Bea 75 of 400
        ],
    );

    assert_eq!(
        named,
        [
            Some("Alex Chen".into()),
            Some("Bea Ruiz".into()),
            None,
            Some("Alex Chen".into()),
            None,
        ]
    );
}

#[test]
fn a_missing_indicator_leaves_its_stretch_to_voice_matching() {
    let spans = record(&[ALEX, ALEX, SHARING, SHARING, SHARING, SHARING, BEA, BEA]);

    let named = winners(&spans, &[(0, 300), (700, 1_300), (1_500, 1_800)]);

    assert_eq!(
        named,
        [Some("Alex Chen".into()), None, Some("Bea Ruiz".into())]
    );
}
