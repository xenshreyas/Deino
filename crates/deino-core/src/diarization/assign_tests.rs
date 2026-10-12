use deino_domain::{Channel, FakeClock, MeetingId, SegmentId};
use deino_testkit::at;

use super::*;

fn turn(start: u32, end: u32, voice: usize) -> VoiceTurn {
    VoiceTurn {
        start: Millis(start),
        end: Millis(end),
        voice,
    }
}

fn word(text: &str, start: u32, end: u32) -> WordTiming {
    WordTiming {
        text: text.into(),
        start: Millis(start),
        end: Millis(end),
    }
}

fn segment(words: Vec<WordTiming>) -> Segment {
    let clock = FakeClock::new(at("2026-10-08T09:00:00Z"));
    Segment {
        id: SegmentId::generate(&clock),
        meeting_id: MeetingId::new("meeting-a"),
        channel: Channel::System,
        speaker_id: None,
        start: words.first().map_or(Millis(0), |w| w.start),
        end: words.last().map_or(Millis(1_000), |w| w.end),
        text: "the original, punctuated text".into(),
        words,
        is_deleted: false,
        edited_at: None,
        original_text: None,
    }
}

/// Words every 500 ms, each 400 ms long.
fn words(texts: &[&str], from: u32) -> Vec<WordTiming> {
    texts
        .iter()
        .zip((from..).step_by(500))
        .map(|(text, start)| word(text, start, start + 400))
        .collect()
}

fn voices(pieces: &[Attributed]) -> Vec<Option<usize>> {
    pieces.iter().map(|piece| piece.voice).collect()
}

#[test]
fn a_word_goes_to_the_voice_overlapping_it_most() {
    let turns = [turn(0, 1_000, 0), turn(1_000, 3_000, 1)];
    assert_eq!(voice_at(Millis(900), Millis(1_400), &turns), Some(1));
    assert_eq!(voice_at(Millis(500), Millis(1_100), &turns), Some(0));
}

#[test]
fn a_word_in_a_gap_goes_to_the_nearest_turn() {
    let turns = [turn(0, 1_000, 0), turn(5_000, 6_000, 1)];
    assert_eq!(voice_at(Millis(1_200), Millis(1_400), &turns), Some(0));
    assert_eq!(voice_at(Millis(4_500), Millis(4_800), &turns), Some(1));
    assert_eq!(voice_at(Millis(0), Millis(100), &[]), None);
}

#[test]
fn a_segment_of_one_voice_stays_whole_with_its_text() {
    let segment = segment(words(&["so", "the", "plan", "is"], 0));
    let pieces = attribute(&segment, &[turn(0, 5_000, 2)]);
    assert_eq!(pieces.len(), 1);
    assert_eq!(pieces[0].voice, Some(2));
    assert_eq!(pieces[0].text, segment.text);
    assert_eq!(
        (pieces[0].start, pieces[0].end),
        (segment.start, segment.end)
    );
}

#[test]
fn a_segment_splits_where_the_speaker_changes() {
    let segment = segment(words(
        &["we", "ship", "friday", "sounds", "good", "to", "me"],
        0,
    ));
    let turns = [turn(0, 1_450, 0), turn(1_450, 4_000, 1)];
    let pieces = attribute(&segment, &turns);
    assert_eq!(voices(&pieces), [Some(0), Some(1)]);
    assert_eq!(pieces[0].text, "we ship friday");
    assert_eq!(pieces[1].text, "sounds good to me");
    assert_eq!(
        (pieces[0].start, pieces[0].end),
        (segment.start, Millis(1_400))
    );
    assert_eq!(
        (pieces[1].start, pieces[1].end),
        (Millis(1_500), segment.end)
    );
    assert_eq!(pieces[1].words.len(), 4);
}

#[test]
fn a_short_flip_joins_its_neighbour_instead_of_splitting() {
    // "yeah" (400 ms) heard as voice 1 between two runs of voice 0.
    let segment = segment(words(
        &["one", "two", "three", "yeah", "four", "five", "six"],
        0,
    ));
    let turns = [
        turn(0, 1_500, 0),
        turn(1_500, 1_900, 1),
        turn(1_900, 4_000, 0),
    ];
    let pieces = attribute(&segment, &turns);
    assert_eq!(voices(&pieces), [Some(0)]);
    assert_eq!(pieces[0].text, segment.text);
}

#[test]
fn a_short_opening_flip_joins_the_next_piece() {
    let segment = segment(words(&["uh", "so", "the", "plan", "is", "this"], 0));
    let turns = [turn(0, 450, 1), turn(450, 4_000, 0)];
    assert_eq!(voices(&attribute(&segment, &turns)), [Some(0)]);
}

#[test]
fn edited_and_wordless_segments_go_whole_to_the_majority_voice() {
    let mut edited = segment(words(
        &["we", "ship", "friday", "sounds", "good", "to", "me"],
        0,
    ));
    edited.edited_at = Some(at("2026-10-08T10:00:00Z"));
    let turns = [turn(0, 1_000, 0), turn(1_000, 4_000, 1)];
    let pieces = attribute(&edited, &turns);
    assert_eq!(voices(&pieces), [Some(1)]);
    assert_eq!(pieces[0].text, edited.text);

    let mut wordless = segment(Vec::new());
    wordless.end = Millis(3_000);
    assert_eq!(voices(&attribute(&wordless, &turns)), [Some(1)]);
}

#[test]
fn without_turns_nothing_is_attributed() {
    let segment = segment(words(&["hello"], 0));
    assert_eq!(voices(&attribute(&segment, &[])), [None]);
}
