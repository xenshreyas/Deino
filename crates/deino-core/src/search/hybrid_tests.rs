use deino_domain::{MeetingId, Millis};

use super::*;
use crate::search::Snippet;

fn hit(kind: HitKind, ref_id: &str, meeting: Option<&str>) -> Hit {
    Hit {
        kind,
        ref_id: ref_id.to_owned(),
        meeting_id: meeting.map(MeetingId::new),
        ts_ms: None,
        end_ms: None,
        label: Snippet::plain(ref_id),
        snippet: Snippet::default(),
        date: None,
        score: 0.0,
    }
}

fn moment(ref_id: &str, meeting: &str, at: u32) -> Hit {
    Hit {
        ts_ms: Some(Millis(at)),
        snippet: Snippet::plain(format!("keyword {ref_id}")),
        ..hit(HitKind::Segment, ref_id, Some(meeting))
    }
}

fn window(ref_id: &str, meeting: &str, (start, end): (u32, u32)) -> Hit {
    Hit {
        ts_ms: Some(Millis(start)),
        end_ms: Some(Millis(end)),
        snippet: Snippet::plain(format!("window {ref_id}")),
        ..hit(HitKind::Segment, ref_id, Some(meeting))
    }
}

fn refs(hits: &[Hit]) -> Vec<&str> {
    hits.iter().map(|hit| hit.ref_id.as_str()).collect()
}

#[test]
fn a_hit_found_by_both_lists_outranks_hits_found_by_one() {
    let keyword = vec![
        hit(HitKind::Meeting, "a", Some("a")),
        hit(HitKind::Meeting, "b", Some("b")),
    ];
    let semantic = vec![
        hit(HitKind::Meeting, "c", Some("c")),
        hit(HitKind::Meeting, "b", Some("b")),
    ];

    let fused = fuse(keyword, semantic);

    assert_eq!(refs(&fused), ["b", "a", "c"]);
    let expected = 2.0 / (RRF_K + 2.0);
    assert!((fused[0].score - expected).abs() < 1e-12);
}

#[test]
fn ranks_count_within_each_kind() {
    let keyword = vec![
        hit(HitKind::Person, "maya", None),
        hit(HitKind::Meeting, "a", Some("a")),
    ];

    let fused = fuse(keyword, Vec::new());

    // Both are first of their kind, so they tie; the order is kept.
    assert_eq!(refs(&fused), ["maya", "a"]);
    assert!((fused[0].score - fused[1].score).abs() < 1e-12);
}

#[test]
fn the_best_keyword_moment_in_a_window_fuses_with_it_and_keeps_its_details() {
    let keyword = vec![moment("s2", "m1", 12_000), moment("s3", "m1", 14_000)];
    let semantic = vec![window("s1", "m1", (10_000, 20_000))];

    let fused = fuse(keyword, semantic);

    assert_eq!(refs(&fused), ["s2", "s3"]);
    assert_eq!(fused[0].ts_ms, Some(Millis(12_000)));
    assert_eq!(fused[0].snippet.text, "keyword s2");
    assert!(fused[0].score > fused[1].score);
}

#[test]
fn windows_of_other_meetings_or_times_do_not_fuse() {
    let keyword = vec![moment("s2", "m1", 30_000), moment("s9", "m2", 12_000)];
    let semantic = vec![window("s1", "m1", (10_000, 20_000))];

    let fused = fuse(keyword, semantic);

    assert_eq!(fused.len(), 3);
    assert_eq!(refs(&fused), ["s2", "s1", "s9"]);
}

#[test]
fn notes_fuse_by_their_meeting() {
    let keyword = vec![hit(HitKind::Note, "m1", Some("m1"))];
    let semantic = vec![
        hit(HitKind::Note, "m2", Some("m2")),
        hit(HitKind::Note, "m1", Some("m1")),
    ];

    let fused = fuse(keyword, semantic);

    assert_eq!(refs(&fused), ["m1", "m2"]);
}

#[test]
fn empty_lists_fuse_to_nothing() {
    assert!(fuse(Vec::new(), Vec::new()).is_empty());
}
