//! Which track to diarize, and whose the microphone is.
//!
//! Ported from anarlog `listener2-core/src/batch/diarize.rs` (MIT):
//!
//! - **A call.** The microphone is the user alone ("Me", never diarized);
//!   the system track carries every remote participant and is diarized with
//!   one speaker fewer than the calendar expects.
//! - **In person.** Two or more people were expected, yet the system track
//!   carried at most 3 s of speech: everyone sat around the microphone, so
//!   the microphone is diarized instead with the full count (anarlog's
//!   `SHARED_MIC_CHANNEL`), and nobody is assumed to be the user.
//! - **Mixed** (legacy recordings and imports): one track holds everyone and
//!   is diarized with the full count.
//!
//! Unknown attendee counts never mean "in person": a solo memo and a
//! two-person meeting sound the same to the recorder.

use deino_domain::{AttendeeRole, Channel, Millis, Rsvp};

use super::SpeakerHint;
use crate::people::Attendee;
use crate::segments::Segment;

/// A system track with no more speech than this carried no call: a stray
/// word or two of noise, not a remote participant.
pub const REMOTE_SILENCE_MAX: Millis = Millis(3_000);

/// The plan for one meeting.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ChannelPlan {
    /// The track to diarize and how many speakers to expect on it.
    pub diarize: Option<PlannedTrack>,
    /// The channel whose segments are the user's own.
    pub me: Option<Channel>,
}

/// One track to diarize.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlannedTrack {
    /// Its channel.
    pub channel: Channel,
    /// The expected speakers on it.
    pub speakers: SpeakerHint,
}

/// Plans the meeting from its transcript and its attendees.
pub fn plan_channels(segments: &[Segment], attendees: &[Attendee]) -> ChannelPlan {
    let expected = Expected::from_attendees(attendees);
    let heard = |channel: Channel| segments.iter().any(|segment| segment.channel == channel);
    if heard(Channel::Mixed) {
        return ChannelPlan {
            diarize: Some(PlannedTrack {
                channel: Channel::Mixed,
                speakers: expected.hint(0),
            }),
            me: None,
        };
    }
    if expected.count >= 2 && speech(segments, Channel::System) <= REMOTE_SILENCE_MAX {
        return ChannelPlan {
            diarize: Some(PlannedTrack {
                channel: Channel::Mic,
                speakers: expected.hint(0),
            }),
            me: None,
        };
    }
    ChannelPlan {
        diarize: heard(Channel::System).then(|| PlannedTrack {
            channel: Channel::System,
            speakers: expected.hint(1),
        }),
        me: heard(Channel::Mic).then_some(Channel::Mic),
    }
}

/// Speech heard on `channel`: the words' durations, or a segment's whole
/// span when it has no word timings.
pub fn speech(segments: &[Segment], channel: Channel) -> Millis {
    let total: u32 = segments
        .iter()
        .filter(|segment| segment.channel == channel)
        .map(|segment| {
            if segment.words.is_empty() {
                span(segment.start, segment.end)
            } else {
                segment
                    .words
                    .iter()
                    .map(|word| span(word.start, word.end))
                    .sum()
            }
        })
        .fold(0, u32::saturating_add);
    Millis(total)
}

fn span(start: Millis, end: Millis) -> u32 {
    end.get().saturating_sub(start.get())
}

/// The calendar's expectation: the attendees who did not decline, and
/// whether every one of them confirmed (accepted, or organised it).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Expected {
    count: u16,
    confirmed: bool,
}

impl Expected {
    fn from_attendees(attendees: &[Attendee]) -> Self {
        let coming: Vec<&Attendee> = attendees
            .iter()
            .filter(|attendee| attendee.rsvp != Some(Rsvp::Declined))
            .collect();
        let confirmed = coming.iter().all(|attendee| {
            attendee.rsvp == Some(Rsvp::Accepted) || attendee.role == AttendeeRole::Organizer
        });
        Self {
            count: u16::try_from(coming.len()).unwrap_or(u16::MAX),
            confirmed,
        }
    }

    /// The hint for a track that cannot hold `absent` of the attendees (the
    /// user, on a call's system track).
    fn hint(self, absent: u16) -> SpeakerHint {
        match self.count.saturating_sub(absent) {
            0 => SpeakerHint::Unknown,
            count if self.confirmed => SpeakerHint::Exactly(count),
            count => SpeakerHint::AtMost(count),
        }
    }
}

#[cfg(test)]
#[path = "plan_tests.rs"]
mod tests;
