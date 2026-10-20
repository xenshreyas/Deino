//! From real fixture dumps to won segments (CAP-05): a recording polls
//! Zoom's call window (`deino-speaker-tags`' anonymised real captures, with
//! the Zoom Workplace window beside it) every 250 ms. Zoom shows no talking
//! indicator: Speaker View's large tile or the one unmuted remote names the
//! speaker, and the remote roster is kept as spans. The spans are aligned
//! with system-track segments by the card's rule, more than half wins
//! (`speaker_tags::align`); a call with one remote person throughout names
//! every segment after them (`speaker_tags::one_to_one`).

#![allow(
    clippy::unwrap_used,
    reason = "an integration-test crate: helpers fail the test by panicking"
)]

use std::path::PathBuf;
use std::sync::Arc;

use deino_core::speaker_tags::Timed;
use deino_core::speaker_tags::align::align;
use deino_core::speaker_tags::one_to_one::{only_remote, won_by};
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

/// What a recording saved: its speaker spans and its roster spans.
struct Saved {
    spans: Vec<TagSpan>,
    roster: Vec<TagSpan>,
}

/// Records one poll per dump, 250 ms apart, and returns what was saved.
fn record(frames: &[&str]) -> Saved {
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
    Saved {
        spans: sink.spans(),
        roster: sink.roster(),
    }
}

/// The segments, timed, with fresh IDs.
fn timed(segments: &[(u32, u32)]) -> Vec<Timed> {
    let clock = SystemClock;
    segments
        .iter()
        .map(|&(start, end)| Timed {
            id: SegmentId::generate(&clock),
            start: Millis(start),
            end: Millis(end),
        })
        .collect()
}

/// The name winning each `(start, end)` segment, if any.
fn winners(spans: &[TagSpan], segments: &[(u32, u32)]) -> Vec<Option<String>> {
    let timed = timed(segments);
    let wins = align(spans, &timed);
    timed
        .iter()
        .map(|segment| {
            let win = wins.iter().find(|win| win.segment == segment.id)?;
            Some(win.name.to_string())
        })
        .collect()
}

/// Speaker View, the remote's large tile (frames as the capture tool saw them).
const TALKING: &str = "real-speaker-view-remote-talking";
/// Gallery View, the remote unmuted.
const GALLERY: &str = "real-gallery-remote-unmuted";
/// Everyone muted, no frames: nobody is named.
const SILENT: &str = "real-silent-all-muted";

#[test]
fn the_remote_speaker_becomes_spans_halfway_between_polls() {
    let saved = record(&[
        SILENT, SILENT, TALKING, TALKING, TALKING, SILENT, GALLERY, GALLERY,
    ]);

    let drawn: Vec<(u32, u32, &str)> = saved
        .spans
        .iter()
        .map(|span| (span.start.0, span.end.0, span.name.as_str()))
        .collect();
    assert_eq!(
        drawn,
        [(375, 1_125, "Bea Ruiz"), (1_375, 1_875, "Bea Ruiz")]
    );
    assert!(saved.spans.iter().all(|span| span.app == CallApp::Zoom));
    let roster: Vec<(u32, u32, &str)> = saved
        .roster
        .iter()
        .map(|span| (span.start.0, span.end.0, span.name.as_str()))
        .collect();
    // The user (Alex Chen, signed in to Zoom Workplace) is never listed.
    assert_eq!(roster, [(0, 1_875, "Bea Ruiz")]);
}

#[test]
fn segments_go_to_the_name_shown_for_more_than_half_of_them() {
    let saved = record(&[
        SILENT, SILENT, TALKING, TALKING, TALKING, SILENT, GALLERY, GALLERY,
    ]);

    let named = winners(
        &saved.spans,
        &[
            (300, 1_200),   // Bea 750 of 900
            (1_100, 1_400), // Bea 25 + 25 of 300
            (0, 375),       // nobody shown
        ],
    );

    assert_eq!(named, [Some("Bea Ruiz".into()), None, None]);
}

#[test]
fn a_one_to_one_call_names_every_remote_segment_even_with_everyone_muted() {
    // A minute and a quarter with the mics muted: no speaker span at all.
    let saved = record(&[SILENT; 300]);
    assert!(saved.spans.is_empty());

    let only = only_remote(&saved.roster).unwrap();
    let segments = timed(&[(1_000, 4_000), (30_000, 31_000)]);
    let wins = won_by(&only, &segments);

    assert_eq!(only.as_str(), "Bea Ruiz");
    assert_eq!(wins.len(), 2);
    assert!(only_remote(&record(&[SILENT; 200]).roster).is_none());
}
