//! Which voiceprints the engine gets for a track (CAP-04).
//!
//! The engine merges clusters that match the same known person and names
//! them (anarlog's `apply_known_speakers`): diarization's most common
//! failure is one voice split into several speakers. Only people expected
//! on the track are passed, as `KnownSpeaker` asks ("a confirmed exemplar of
//! a meeting attendee"): a stranger's voiceprint merging two clusters would
//! be a guess. `identify_speakers` names the stored speakers afterwards.

use deino_domain::{Channel, Rsvp};

use super::KnownVoice;
use crate::people::Attendee;
use crate::voiceprints::Exemplar;

/// The `exemplars` of attendees who did not decline; the user's only when
/// the diarized track is not a call's remote track (the user is never on it).
pub fn known_voices(
    exemplars: Vec<Exemplar>,
    attendees: &[Attendee],
    channel: Channel,
) -> Vec<KnownVoice> {
    let expected = |exemplar: &Exemplar| {
        attendees.iter().any(|attendee| {
            attendee.person_id == exemplar.person_id && attendee.rsvp != Some(Rsvp::Declined)
        })
    };
    exemplars
        .into_iter()
        .filter(|exemplar| expected(exemplar) && !(exemplar.is_me && channel == Channel::System))
        .map(|exemplar| KnownVoice {
            person: exemplar.person_id,
            embedding: exemplar.embedding,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use deino_domain::{AttendeeRole, FakeClock, MeetingId, PersonId};
    use deino_testkit::at;

    use super::*;

    fn attendee(person: PersonId, rsvp: Option<Rsvp>) -> Attendee {
        Attendee {
            meeting_id: MeetingId::new("meeting-a"),
            person_id: person,
            role: AttendeeRole::Invitee,
            rsvp,
        }
    }

    fn exemplar(person: PersonId, is_me: bool) -> Exemplar {
        Exemplar {
            person_id: person,
            is_me,
            embedding: vec![1.0, 0.0],
        }
    }

    #[test]
    fn only_expected_attendees_voices_reach_the_engine() {
        let clock = FakeClock::new(at("2026-10-11T09:00:00Z"));
        let [alex, maya, stranger, me] = [(); 4].map(|()| PersonId::generate(&clock));
        let attendees = [
            attendee(alex, Some(Rsvp::Accepted)),
            attendee(maya, Some(Rsvp::Declined)),
            attendee(me, None),
        ];
        let exemplars = vec![
            exemplar(alex, false),
            exemplar(maya, false),
            exemplar(stranger, false),
            exemplar(me, true),
        ];

        let remote = known_voices(exemplars.clone(), &attendees, Channel::System);
        let in_person = known_voices(exemplars, &attendees, Channel::Mic);

        let people = |known: &[KnownVoice]| known.iter().map(|k| k.person).collect::<Vec<_>>();
        assert_eq!(people(&remote), [alex]);
        assert_eq!(people(&in_person), [alex, me]);
    }
}
