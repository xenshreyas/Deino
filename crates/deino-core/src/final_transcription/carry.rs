//! Who said a re-transcribed utterance (pure).
//!
//! A re-pass replaces the live segments, but the speakers they were given
//! (by the lane, diarization or the user) must survive: each new utterance
//! takes the speaker of the old segments on its channel that overlap it
//! most. With nothing to overlap, the channel decides: the microphone is the
//! user, and system audio is the lone remote speaker when there is only one.

use deino_domain::{Channel, Millis, SpeakerId};

use crate::segments::{Segment, Speaker};

/// The speaker each channel falls back to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ChannelSpeakers {
    /// The user's speaker, for the microphone.
    pub me: Option<SpeakerId>,
    /// The only other speaker, for system audio.
    pub them: Option<SpeakerId>,
}

impl ChannelSpeakers {
    /// The fallbacks among a meeting's `speakers`.
    pub fn of(speakers: &[Speaker]) -> Self {
        let me = speakers.iter().find(|speaker| speaker.is_me).map(|s| s.id);
        let mut others = speakers.iter().filter(|speaker| !speaker.is_me);
        let them = match (others.next(), others.next()) {
            (Some(only), None) => Some(only.id),
            _ => None,
        };
        Self { me, them }
    }

    /// The fallback for `channel`.
    pub fn of_channel(self, channel: Channel) -> Option<SpeakerId> {
        match channel {
            Channel::Mic => self.me,
            Channel::System => self.them,
            Channel::Mixed => None,
        }
    }
}

/// The speaker of the `old` segments on `channel` that overlap
/// `start..end` most, if any overlaps.
pub fn carried_speaker(
    channel: Channel,
    (start, end): (Millis, Millis),
    old: &[Segment],
) -> Option<SpeakerId> {
    let mut totals: Vec<(SpeakerId, u32)> = Vec::new();
    for segment in old.iter().filter(|s| s.channel == channel) {
        let (Some(speaker), Some(overlap)) = (segment.speaker_id, overlap(segment, start, end))
        else {
            continue;
        };
        match totals.iter_mut().find(|(id, _)| *id == speaker) {
            Some((_, total)) => *total += overlap,
            None => totals.push((speaker, overlap)),
        }
    }
    totals
        .into_iter()
        .max_by_key(|(_, total)| *total)
        .map(|(speaker, _)| speaker)
}

/// How long `segment` and `start..end` overlap, when they do.
fn overlap(segment: &Segment, start: Millis, end: Millis) -> Option<u32> {
    let from = segment.start.0.max(start.0);
    let to = segment.end.0.min(end.0);
    (to > from).then(|| to - from)
}

#[cfg(test)]
mod tests {
    use deino_domain::{FakeClock, MeetingId, PaletteColor, SegmentId};
    use deino_testkit::at;

    use super::*;

    fn id() -> SpeakerId {
        SpeakerId::generate(&FakeClock::new(at("2026-10-11T09:00:00Z")))
    }

    fn old(channel: Channel, (start, end): (u32, u32), speaker: Option<SpeakerId>) -> Segment {
        Segment {
            id: SegmentId::generate(&FakeClock::new(at("2026-10-11T09:00:00Z"))),
            meeting_id: MeetingId::new("m"),
            channel,
            speaker_id: speaker,
            start: Millis(start),
            end: Millis(end),
            text: "old".into(),
            words: Vec::new(),
            is_deleted: false,
            edited_at: None,
            original_text: None,
        }
    }

    fn speaker(label: &str, is_me: bool) -> Speaker {
        Speaker {
            id: id(),
            meeting_id: MeetingId::new("m"),
            label: label.into(),
            person_id: None,
            is_me,
            color: PaletteColor::cycle(1),
        }
    }

    #[test]
    fn the_speaker_overlapping_most_on_the_same_channel_wins() {
        let (alex, sam, me) = (id(), id(), id());
        let segments = [
            old(Channel::System, (0, 1_000), Some(alex)),
            old(Channel::System, (1_000, 3_000), Some(sam)),
            old(Channel::System, (3_000, 3_500), Some(alex)),
            old(Channel::Mic, (0, 5_000), Some(me)),
        ];

        let carried = carried_speaker(Channel::System, (Millis(500), Millis(3_500)), &segments);

        assert_eq!(carried, Some(sam));
        assert_eq!(
            carried_speaker(Channel::System, (Millis(6_000), Millis(7_000)), &segments),
            None
        );
    }

    #[test]
    fn channels_fall_back_to_the_user_and_a_lone_remote_speaker() {
        let (me, them) = (speaker("Me", true), speaker("Them", false));
        let lone = ChannelSpeakers::of(&[me.clone(), them.clone()]);
        let crowd = ChannelSpeakers::of(&[me.clone(), them, speaker("Speaker 2", false)]);

        assert_eq!(lone.of_channel(Channel::Mic), Some(me.id));
        assert!(lone.of_channel(Channel::System).is_some());
        assert_eq!(crowd.of_channel(Channel::System), None);
        assert_eq!(crowd.of_channel(Channel::Mixed), None);
    }
}
