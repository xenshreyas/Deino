//! The cloud's voices as the meeting's speakers (CAP-14, pure).
//!
//! The rule is the `diarize` stage's: each voice of a track that may hold
//! several people (system audio, the mixed file) becomes "Speaker 1..N" in
//! order of first appearance, skipping labels the user's own speaker has,
//! coloured like diarization colours them. The microphone is the user, so
//! its voices are ignored. The speakers carry no voice embedding: the local
//! `diarize` stage that runs next measures voices on the Mac, re-clusters
//! with known voiceprints and names speakers (`identify_speakers`).

use deino_domain::{Channel, Clock, MeetingId, PaletteColor, SpeakerId};

use super::CloudSpan;
use crate::diarization::speaker_labels;
use crate::segments::{NewSpeaker, Speaker};

/// The speakers for one cloud transcript's voices.
#[derive(Debug, Default)]
pub struct CloudVoices {
    /// Each voice: its channel, the service's name for it, its speaker.
    voices: Vec<(Channel, String, SpeakerId)>,
    /// The speakers to store, in label order.
    speakers: Vec<NewSpeaker>,
}

impl CloudVoices {
    /// The speakers for the voices in `heard` (in time order), next to the
    /// meeting's `kept` speakers.
    pub fn of(
        meeting: &MeetingId,
        heard: &[(Channel, CloudSpan)],
        kept: &[Speaker],
        clock: &dyn Clock,
    ) -> Self {
        let mut named: Vec<(Channel, &str)> = Vec::new();
        for (channel, span) in heard {
            let Some(voice) = span.voice.as_deref().filter(|_| *channel != Channel::Mic) else {
                continue;
            };
            if !named.contains(&(*channel, voice)) {
                named.push((*channel, voice));
            }
        }
        let taken: Vec<String> = kept
            .iter()
            .filter(|speaker| speaker.is_me)
            .map(|speaker| speaker.label.clone())
            .collect();
        let labels = speaker_labels(named.len(), &taken);
        let mut voices = Self::default();
        for (index, ((channel, name), label)) in named.into_iter().zip(labels).enumerate() {
            let speaker = Speaker {
                id: SpeakerId::generate(clock),
                meeting_id: meeting.clone(),
                label,
                person_id: None,
                is_me: false,
                color: PaletteColor::cycle(index + 1),
            };
            voices.voices.push((channel, name.to_owned(), speaker.id));
            voices.speakers.push(NewSpeaker {
                speaker,
                voice: None,
            });
        }
        voices
    }

    /// True when the service told no voices apart.
    pub fn is_empty(&self) -> bool {
        self.speakers.is_empty()
    }

    /// The speakers to store.
    pub fn speakers(&self) -> &[NewSpeaker] {
        &self.speakers
    }

    /// The speaker of `voice` on `channel`, if the service named one.
    pub fn speaker_of(&self, channel: Channel, voice: Option<&str>) -> Option<SpeakerId> {
        let voice = voice?;
        self.voices
            .iter()
            .find(|(on, name, _)| *on == channel && name == voice)
            .map(|(_, _, id)| *id)
    }
}

#[cfg(test)]
mod tests {
    use deino_domain::{FakeClock, Millis};
    use deino_testkit::at;

    use super::*;
    use crate::final_transcription::HeardSpan;

    fn heard(channel: Channel, start: u32, voice: Option<&str>) -> (Channel, CloudSpan) {
        let span = HeardSpan {
            start: Millis(start),
            end: Millis(start + 500),
            text: "hi".into(),
            words: Vec::new(),
        };
        let voice = voice.map(str::to_owned);
        (channel, CloudSpan { span, voice })
    }

    fn me(label: &str) -> Speaker {
        Speaker {
            id: SpeakerId::generate(&FakeClock::new(at("2026-10-12T09:00:00Z"))),
            meeting_id: MeetingId::new("m"),
            label: label.into(),
            person_id: None,
            is_me: true,
            color: PaletteColor::cycle(0),
        }
    }

    #[test]
    fn voices_become_numbered_speakers_in_order_of_first_appearance() {
        let clock = FakeClock::new(at("2026-10-12T09:00:00Z"));
        let heard = [
            heard(Channel::Mic, 0, Some("speaker_0")),
            heard(Channel::System, 100, Some("speaker_1")),
            heard(Channel::System, 200, Some("speaker_0")),
            heard(Channel::System, 300, Some("speaker_1")),
            heard(Channel::System, 400, None),
        ];

        let voices = CloudVoices::of(&MeetingId::new("m"), &heard, &[me("Speaker 1")], &clock);

        let labels: Vec<&str> = voices
            .speakers()
            .iter()
            .map(|new| new.speaker.label.as_str())
            .collect();
        assert_eq!(labels, ["Speaker 2", "Speaker 3"]);
        assert!(voices.speakers().iter().all(|new| new.voice.is_none()));
        let first = voices.speaker_of(Channel::System, Some("speaker_1"));
        assert_eq!(first, Some(voices.speakers()[0].speaker.id));
        assert_eq!(voices.speaker_of(Channel::Mic, Some("speaker_0")), None);
        assert_eq!(voices.speaker_of(Channel::System, None), None);
    }

    #[test]
    fn a_track_without_voices_stores_no_speakers() {
        let clock = FakeClock::new(at("2026-10-12T09:00:00Z"));
        let heard = [heard(Channel::Mic, 0, Some("speaker_0"))];

        let voices = CloudVoices::of(&MeetingId::new("m"), &heard, &[], &clock);

        assert!(voices.is_empty());
    }
}
