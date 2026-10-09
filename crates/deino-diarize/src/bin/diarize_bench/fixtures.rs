//! The FND-04 fixtures with audio, scored two ways:
//!
//! - **system**: the call's system track alone, with the calendar's
//!   expectation the diarize stage would pass (`plan_channels`);
//! - **mixed**: both tracks summed, as a shared microphone or a legacy mixed
//!   recording holds them, with every attendee expected.
//!
//! The reference is the fixture's segments inside the audio clip.

use deino_core::diarization::{SpeakerHint, plan_channels};
use deino_core::people::Attendee;
use deino_core::segments::Segment;
use deino_diarize::metrics::ReferenceTurn;
use deino_diarize::track::expected;
use deino_domain::{Channel, Millis, PersonId, SegmentId};
use deino_testkit::Fixtures;
use deino_testkit::fixture::{FixtureMeeting, fixture_uuid};

use crate::{Case, RATE};

/// Every scored clip of every fixture with audio.
pub(crate) fn cases() -> Result<Vec<Case>, String> {
    let fixtures = Fixtures::load().map_err(|e| e.to_string())?;
    let mut cases = Vec::new();
    for meeting in fixtures.meetings() {
        let Some(audio) = &meeting.audio else {
            continue;
        };
        let track = |channel| -> Result<Vec<f32>, String> {
            let path = fixtures.audio_path(audio, channel).ok_or("no track")?;
            deino_audio::read_mono(&path, RATE).map_err(|e| e.to_string())
        };
        let (mic, system) = (track(Channel::Mic)?, track(Channel::System)?);
        let clip = audio.clip_end_ms;
        let plan = plan_channels(&segments(meeting), &attendees(meeting));
        let system_hint = plan
            .diarize
            .map_or(SpeakerHint::Unknown, |track| track.speakers);
        let mixed = mix(&mic, &system);
        cases.push(case(
            name(meeting, "system"),
            system,
            reference(meeting, Some(Channel::System), clip),
            system_hint,
        ));
        let everyone = hint_for_everyone(meeting);
        cases.push(case(
            name(meeting, "mixed"),
            mixed,
            reference(meeting, None, clip),
            everyone,
        ));
    }
    Ok(cases)
}

fn case(name: String, samples: Vec<f32>, reference: Vec<ReferenceTurn>, hint: SpeakerHint) -> Case {
    Case {
        name,
        samples,
        reference,
        expected: expected(hint),
    }
}

/// `<fixture kind>/<track>`.
fn name(meeting: &FixtureMeeting, track: &str) -> String {
    format!("{:?}/{track}", meeting.kind)
}

/// Every attendee who did not decline (the whole room).
fn hint_for_everyone(meeting: &FixtureMeeting) -> SpeakerHint {
    let mixed: Vec<Segment> = segments(meeting)
        .into_iter()
        .map(|segment| Segment {
            channel: Channel::Mixed,
            ..segment
        })
        .collect();
    let plan = plan_channels(&mixed, &attendees(meeting));
    plan.diarize
        .map_or(SpeakerHint::Unknown, |track| track.speakers)
}

/// The fixture's turns on `channel` (all channels for `None`) inside the clip.
fn reference(
    meeting: &FixtureMeeting,
    channel: Option<Channel>,
    clip: Millis,
) -> Vec<ReferenceTurn> {
    meeting
        .segments
        .iter()
        .filter(|segment| channel.is_none_or(|channel| segment.channel == channel))
        .filter(|segment| segment.start_ms < clip)
        .map(|segment| ReferenceTurn {
            start: segment.start_ms,
            end: segment.end_ms.min(clip),
            speaker: segment.speaker.clone(),
        })
        .collect()
}

/// The two tracks summed at −6 dB each, the length of the longer.
fn mix(mic: &[f32], system: &[f32]) -> Vec<f32> {
    let at = |track: &[f32], index: usize| track.get(index).copied().unwrap_or(0.0);
    (0..mic.len().max(system.len()))
        .map(|i| 0.5 * (at(mic, i) + at(system, i)))
        .collect()
}

/// The fixture's transcript as the stage reads it (timing and channel only).
fn segments(meeting: &FixtureMeeting) -> Vec<Segment> {
    meeting
        .segments
        .iter()
        .enumerate()
        .filter_map(|(index, segment)| {
            Some(Segment {
                id: fixture_uuid(&format!("bench-segment-{index}"))
                    .parse::<SegmentId>()
                    .ok()?,
                meeting_id: meeting.id.clone(),
                channel: segment.channel,
                speaker_id: None,
                start: segment.start_ms,
                end: segment.end_ms,
                text: String::new(),
                words: Vec::new(),
                is_deleted: false,
                edited_at: None,
            })
        })
        .collect()
}

/// The fixture's invitees as the stage reads them.
fn attendees(meeting: &FixtureMeeting) -> Vec<Attendee> {
    meeting
        .attendees
        .iter()
        .filter_map(|attendee| {
            Some(Attendee {
                meeting_id: meeting.id.clone(),
                person_id: fixture_uuid(attendee.email.as_str())
                    .parse::<PersonId>()
                    .ok()?,
                role: attendee.role,
                rsvp: Some(attendee.rsvp),
            })
        })
        .collect()
}
