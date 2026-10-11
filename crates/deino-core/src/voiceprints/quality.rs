//! How well a speaker's voice was heard: the clean speech behind its
//! centroid, which ranks a person's exemplars for eviction.
//!
//! The spike enrolled each person from their longest overlap-free spans
//! (`docs/references/spike-diarization.md`); a centroid backed by more such
//! speech is a better exemplar. Quality counts that speech with
//! [`select_speaker_spans`] on the speaker's channel (other voices on the
//! same track cut overlaps out) and saturates at [`FULL_QUALITY_SPEECH`].

use deino_domain::{Channel, Millis, SpeakerId};

use super::Quality;
use super::spans::{SpanConfig, SpanWord, select_speaker_spans};
use crate::segments::Segment;

/// Clean speech at which a voice counts as fully heard.
pub const FULL_QUALITY_SPEECH: Millis = Millis(60_000);

/// The spans quality counts: the spike's rules (merge across 400 ms, at
/// least 1.5 s, the middle 10 s of longer runs), but up to twelve of them,
/// so up to two minutes are counted.
const QUALITY_SPANS: SpanConfig = SpanConfig {
    merge_gap: Millis(400),
    min_span: Millis(1_500),
    max_span: Millis(10_000),
    max_spans_per_speaker: 12,
};

/// `speaker`'s clean speech in a meeting with `segments`: its longest
/// overlap-free spans, up to two minutes. Zero when it said nothing.
pub fn clean_speech(segments: &[Segment], speaker: SpeakerId) -> Millis {
    let channel = segments
        .iter()
        .find(|segment| segment.speaker_id == Some(speaker))
        .map(|segment| segment.channel);
    let Some(channel) = channel else {
        return Millis(0);
    };
    let words = channel_words(segments, channel);
    let clean = select_speaker_spans(&words, &QUALITY_SPANS)
        .iter()
        .filter(|span| span.speaker == speaker)
        .map(|span| span.end.get().saturating_sub(span.start.get()))
        .fold(0, u32::saturating_add);
    Millis(clean)
}

/// The quality of a voice backed by `clean` speech: the share of
/// [`FULL_QUALITY_SPEECH`] it reaches.
pub fn quality_of(clean: Millis) -> Quality {
    Quality::new(f64::from(clean.get()) / f64::from(FULL_QUALITY_SPEECH.get()))
}

/// Every attributed word on `channel`; a segment without word timings
/// counts as one word.
fn channel_words(segments: &[Segment], channel: Channel) -> Vec<SpanWord<SpeakerId>> {
    segments
        .iter()
        .filter(|segment| segment.channel == channel)
        .filter_map(|segment| segment.speaker_id.map(|speaker| (segment, speaker)))
        .flat_map(|(segment, speaker)| {
            let whole = SpanWord {
                start: segment.start,
                end: segment.end,
                speaker,
            };
            let words = segment.words.iter().map(move |word| SpanWord {
                start: word.start,
                end: word.end,
                speaker,
            });
            let timed: Vec<SpanWord<SpeakerId>> = words.collect();
            if timed.is_empty() { vec![whole] } else { timed }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use deino_domain::{FakeClock, MeetingId, SegmentId};
    use deino_testkit::at;

    use super::*;

    fn segment(speaker: SpeakerId, start: u32, end: u32) -> Segment {
        Segment {
            id: SegmentId::generate(&FakeClock::new(at("2026-10-11T09:00:00Z"))),
            meeting_id: MeetingId::new("meeting-a"),
            channel: Channel::System,
            speaker_id: Some(speaker),
            start: Millis(start),
            end: Millis(end),
            text: "words".into(),
            words: Vec::new(),
            is_deleted: false,
            edited_at: None,
            original_text: None,
        }
    }

    fn speakers() -> (SpeakerId, SpeakerId) {
        let clock = FakeClock::new(at("2026-10-11T09:00:00Z"));
        (SpeakerId::generate(&clock), SpeakerId::generate(&clock))
    }

    #[test]
    fn clean_speech_counts_until_a_minute_of_it() {
        let (alex, _) = speakers();
        let half = [segment(alex, 0, 10_000), segment(alex, 20_000, 40_000)];
        assert_eq!(clean_speech(&half, alex), Millis(20_000));
        assert!((quality_of(Millis(30_000)).get() - 0.5).abs() < 1e-9);
        let lots: Vec<Segment> = (0..10)
            .map(|i| segment(alex, i * 20_000, i * 20_000 + 10_000))
            .collect();
        assert_eq!(clean_speech(&lots, alex), Millis(100_000));
        assert!((quality_of(clean_speech(&lots, alex)).get() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn speech_over_another_voice_does_not_count() {
        let (alex, maya) = speakers();
        let overlapped = [segment(alex, 0, 6_000), segment(maya, 0, 6_000)];
        assert_eq!(clean_speech(&overlapped, alex), Millis(0));
    }

    #[test]
    fn a_silent_speaker_has_no_quality() {
        let (alex, maya) = speakers();
        assert_eq!(clean_speech(&[segment(maya, 0, 9_000)], alex), Millis(0));
    }
}
