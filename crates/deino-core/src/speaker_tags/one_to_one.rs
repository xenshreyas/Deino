//! The one-to-one rule, pure: when the call window listed exactly one
//! remote person for the whole call, every remote voice is that person.
//!
//! Zoom shows no talking indicator, and diarization often splits one remote
//! voice into several speakers. The roster (`speaker_tag_roster`) settles a
//! one-to-one call outright: every system-track segment is won by the one
//! remote person, so the seats ([`super::seats`]) merge the split speakers
//! into one named speaker (segments on a speaker the user named stay). A
//! participant listed for less than [`BRIEF_JOIN`] in all (someone who
//! joined and left, a phone line that dropped) does not count. With more
//! than one remote person, the spans vote as before ([`super::align`]).

use deino_domain::{DisplayName, Millis, TagSpan};

use super::align::{Timed, Win};

/// Participants listed for less than this in all are ignored.
pub const BRIEF_JOIN: Millis = Millis(60_000);

/// The one remote person the roster listed for at least [`BRIEF_JOIN`], if
/// exactly one was. Names are compared ignoring case and spacing.
pub fn only_remote(roster: &[TagSpan]) -> Option<DisplayName> {
    let mut totals: Vec<(&DisplayName, u64)> = Vec::new();
    for span in roster {
        let length = u64::from(span.length());
        match totals
            .iter_mut()
            .find(|(name, _)| name.key() == span.name.key())
        {
            Some((_, total)) => *total += length,
            None => totals.push((&span.name, length)),
        }
    }
    let mut stayed = totals
        .into_iter()
        .filter(|&(_, total)| total >= u64::from(BRIEF_JOIN.0));
    match (stayed.next(), stayed.next()) {
        (Some((name, _)), None) => Some(name.clone()),
        _ => None,
    }
}

/// Every segment of `segments`, won by `name` (zero-length ones excepted).
pub fn won_by(name: &DisplayName, segments: &[Timed]) -> Vec<Win> {
    segments
        .iter()
        .filter(|segment| segment.length() > 0)
        .map(|segment| Win {
            segment: segment.id,
            name: name.clone(),
            covered: Millis(segment.length()),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use deino_domain::{CallApp, Clock, FakeClock, SegmentId};

    use super::*;

    fn listed(name: &str, (start, end): (u32, u32)) -> TagSpan {
        TagSpan {
            start: Millis(start),
            end: Millis(end),
            name: DisplayName::parse(name).unwrap(),
            app: CallApp::Zoom,
        }
    }

    fn name(text: &str) -> DisplayName {
        DisplayName::parse(text).unwrap()
    }

    #[test]
    fn one_remote_person_throughout_is_the_only_one() {
        let roster = [listed("Bea Ruiz", (0, 600_000))];

        assert_eq!(only_remote(&roster), Some(name("Bea Ruiz")));
    }

    #[test]
    fn brief_joins_are_ignored_and_split_presence_adds_up() {
        let roster = [
            listed("Bea Ruiz", (0, 40_000)),
            listed("Dev Patel", (40_000, 99_000)),
            listed("bea  ruiz", (100_000, 130_000)),
        ];

        assert_eq!(only_remote(&roster), Some(name("Bea Ruiz")));
    }

    #[test]
    fn two_people_or_nobody_for_a_minute_is_no_one_to_one() {
        let two = [
            listed("Bea Ruiz", (0, 600_000)),
            listed("Cleo Martin", (0, 61_000)),
        ];
        let brief = [listed("Bea Ruiz", (0, 59_999))];

        assert_eq!(only_remote(&two), None);
        assert_eq!(only_remote(&brief), None);
        assert_eq!(only_remote(&[]), None);
    }

    #[test]
    fn the_one_person_wins_every_segment_with_speech() {
        let clock = FakeClock::new(chrono::DateTime::UNIX_EPOCH);
        let timed = |start: u32, end: u32| Timed {
            id: SegmentId::generate(&clock as &dyn Clock),
            start: Millis(start),
            end: Millis(end),
        };
        let segments = [timed(0, 1_000), timed(1_000, 1_000), timed(2_000, 5_000)];

        let wins = won_by(&name("Bea Ruiz"), &segments);

        let won: Vec<(SegmentId, u32)> = wins.iter().map(|w| (w.segment, w.covered.0)).collect();
        assert_eq!(won, [(segments[0].id, 1_000), (segments[2].id, 3_000)]);
    }
}
