use chrono::DateTime;
use deino_domain::{CallApp, FakeClock, SegmentId};

use super::*;

fn name(text: &str) -> DisplayName {
    DisplayName::parse(text).unwrap()
}

fn span(start: u32, end: u32, who: &str) -> TagSpan {
    TagSpan {
        start: Millis(start),
        end: Millis(end),
        name: name(who),
        app: CallApp::Zoom,
    }
}

fn segment(start: u32, end: u32) -> Timed {
    Timed {
        id: SegmentId::generate(&FakeClock::new(DateTime::UNIX_EPOCH)),
        start: Millis(start),
        end: Millis(end),
    }
}

fn winners(spans: &[TagSpan], segments: &[Timed]) -> Vec<Option<String>> {
    let wins = align(spans, segments);
    segments
        .iter()
        .map(|segment| {
            wins.iter()
                .find(|win| win.segment == segment.id)
                .map(|win| win.name.to_string())
        })
        .collect()
}

#[test]
fn a_name_lit_for_the_whole_segment_wins_it() {
    let segments = [segment(1_000, 3_000)];

    let wins = align(&[span(0, 5_000, "Alex Chen")], &segments);

    assert_eq!(wins.len(), 1);
    assert_eq!(wins[0].name, name("Alex Chen"));
    assert_eq!(wins[0].covered, Millis(2_000));
}

#[test]
fn more_than_half_wins_and_exactly_half_does_not() {
    let segments = [segment(0, 2_000), segment(10_000, 12_000)];
    let spans = [
        span(0, 1_000, "Alex"),
        span(1_000, 2_000, "Bea"),
        span(10_000, 11_001, "Alex"),
        span(11_001, 12_000, "Bea"),
    ];

    assert_eq!(winners(&spans, &segments), [None, Some("Alex".into())]);
}

#[test]
fn an_active_speaker_switch_inside_a_segment_goes_to_the_longer_name() {
    let segments = [segment(0, 4_000), segment(4_000, 8_000)];
    let spans = [span(0, 2_750, "Alex"), span(2_750, 8_000, "Bea")];

    assert_eq!(
        winners(&spans, &segments),
        [Some("Alex".into()), Some("Bea".into())]
    );
}

#[test]
fn a_flickering_indicator_sums_its_spans() {
    let segments = [segment(0, 3_000)];
    let spans = [
        span(0, 900, "Alex"),
        span(900, 1_150, "Bea"),
        span(1_150, 2_000, "Alex"),
    ];

    assert_eq!(winners(&spans, &segments), [Some("Alex".into())]);
}

#[test]
fn a_segment_without_an_indicator_is_left_to_voice_matching() {
    let segments = [segment(0, 2_000), segment(5_000, 9_000)];
    let spans = [span(5_000, 6_500, "Alex")];

    assert_eq!(winners(&spans, &segments), [None, None]);
    assert!(align(&[], &segments).is_empty());
}

#[test]
fn zero_length_segments_are_never_won() {
    let segments = [segment(1_000, 1_000)];

    assert!(align(&[span(0, 5_000, "Alex")], &segments).is_empty());
}

#[test]
fn spans_in_any_order_and_long_spans_from_far_back_still_count() {
    let segments = [segment(50_000, 52_000), segment(1_000, 2_000)];
    let spans = [
        span(40_000, 60_000, "Bea"),
        span(0, 1_200, "Alex"),
        span(1_200, 30_000, "Cleo"),
    ];

    assert_eq!(
        winners(&spans, &segments),
        [Some("Bea".into()), Some("Cleo".into())]
    );
}
