//! The transcript as prompt lines, and how a long meeting is cut into parts.

use deino_domain::Millis;

use super::*;
use crate::enhance::test_support::line;

/// One line every `step` seconds for `minutes`.
fn meeting(minutes: u32, step: u32) -> Vec<TranscriptLine> {
    (0..minutes * 60 / step)
        .map(|n| line(n + 1, n * step, "Maya", "we hold the beta date"))
        .collect()
}

const HOSTED: PartLimits = PartLimits {
    span: PART_SPAN,
    chars: 120_000,
};

#[test]
fn a_line_shows_its_reference_time_and_speaker() {
    assert_eq!(
        line(12, 271, "Maya", " we hold the beta date ").render(),
        "[s12 04:31] Maya: we hold the beta date"
    );
    assert_eq!(clock(Millis(3_871_000)), "1:04:31");
}

#[test]
fn a_short_meeting_is_one_part() {
    let lines = meeting(25, 10);

    assert_eq!(parts(&lines, HOSTED).len(), 1);
    assert!(parts(&[], HOSTED).is_empty());
}

/// A budget the 90- and 43-minute test meetings (about 22k and 11k
/// characters) are too long for, but a 20-minute part fits.
const TIGHT: PartLimits = PartLimits {
    span: PART_SPAN,
    chars: 10_000,
};

#[test]
fn a_long_meeting_that_fits_is_one_part() {
    let lines = meeting(90, 10);

    assert_eq!(parts(&lines, HOSTED).len(), 1, "duration alone never splits");
}

#[test]
fn a_transcript_too_long_for_one_call_is_cut_into_twenty_minute_parts() {
    let lines = meeting(90, 10);

    let parts = parts(&lines, TIGHT);

    assert_eq!(parts.len(), 5, "20+20+20+20+10 minutes");
    assert!(
        parts
            .iter()
            .all(|part| span(part) <= Millis(20 * 60 * 1000))
    );
    let total: usize = parts.iter().map(|part| part.len()).sum();
    assert_eq!(total, lines.len(), "no line is lost or repeated");
}

#[test]
fn a_short_last_part_joins_the_one_before() {
    let lines = meeting(43, 10);

    let parts = parts(&lines, TIGHT);

    assert_eq!(parts.len(), 2, "20 + 23 minutes, not 20 + 20 + 3");
}

#[test]
fn a_small_context_window_cuts_by_characters() {
    let lines = meeting(10, 10);
    let small = PartLimits {
        span: PART_SPAN,
        chars: 2_000,
    };

    let parts = parts(&lines, small);

    assert!(parts.len() > 1);
    assert!(parts.iter().all(|part| !part.is_empty()));
}
