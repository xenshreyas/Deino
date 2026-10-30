use chrono::DateTime;
use deino_domain::{Clock, FakeClock, MeetingId};

use super::*;

fn line(channel: Channel, text: &str) -> Segment {
    Segment {
        id: SegmentId::generate(&FakeClock::new(DateTime::UNIX_EPOCH) as &dyn Clock),
        meeting_id: MeetingId::new("meeting"),
        channel,
        speaker_id: None,
        start: Millis(1_000),
        end: Millis(4_000),
        text: text.into(),
        words: Vec::new(),
        is_deleted: false,
        edited_at: None,
        original_text: None,
    }
}

fn word(text: &str, start: u32, end: u32) -> WordTiming {
    WordTiming {
        text: text.into(),
        start: Millis(start),
        end: Millis(end),
    }
}

#[test]
fn only_untimed_unedited_lines_of_the_track_are_asked_about() {
    let mut edited = line(Channel::System, "edited by hand");
    edited.edited_at = Some(DateTime::UNIX_EPOCH);
    let mut timed = line(Channel::System, "already timed");
    timed.words = vec![word("already", 1_000, 1_500), word("timed", 1_500, 2_000)];
    let mine = line(Channel::Mic, "my line");
    let call = line(Channel::System, "not sure I want the process automated");
    let segments = [edited, timed, mine, call.clone()];

    let lines = untimed(&segments, Channel::System);

    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].segment, call.id);
}

#[test]
fn a_line_takes_its_words_only_when_every_word_was_timed() {
    let whole = line(Channel::System, "well I was");
    let partial = line(Channel::System, "two words");
    let lines = untimed(&[whole.clone(), partial.clone()], Channel::System);
    let timed = vec![
        vec![
            word("well", 1_000, 1_200),
            word("I", 1_300, 1_400),
            word("was", 1_400, 1_700),
        ],
        vec![word("two", 1_000, 1_300)],
    ];

    let segments = with_words(vec![whole, partial], &lines, timed);

    assert_eq!(segments[0].words.len(), 3);
    assert!(
        segments[1].words.is_empty(),
        "a line the aligner could not finish stays whole"
    );
}
