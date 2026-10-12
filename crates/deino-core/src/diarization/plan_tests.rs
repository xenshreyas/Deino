use deino_domain::{FakeClock, MeetingId, PersonId, SegmentId};
use deino_testkit::at;

use super::*;
use crate::segments::WordTiming;

fn clock() -> FakeClock {
    FakeClock::new(at("2026-10-08T09:00:00Z"))
}

/// A segment on `channel` spanning `[start, end)` with one word.
fn said(channel: Channel, start: u32, end: u32) -> Segment {
    Segment {
        id: SegmentId::generate(&clock()),
        meeting_id: MeetingId::new("meeting-a"),
        channel,
        speaker_id: None,
        start: Millis(start),
        end: Millis(end),
        text: "words".into(),
        words: vec![WordTiming {
            text: "words".into(),
            start: Millis(start),
            end: Millis(end),
        }],
        is_deleted: false,
        edited_at: None,
        original_text: None,
    }
}

fn attendee(role: AttendeeRole, rsvp: Option<Rsvp>) -> Attendee {
    Attendee {
        meeting_id: MeetingId::new("meeting-a"),
        person_id: PersonId::generate(&clock()),
        role,
        rsvp,
    }
}

fn accepted(count: usize) -> Vec<Attendee> {
    let mut attendees = vec![attendee(AttendeeRole::Organizer, None)];
    attendees.extend((1..count).map(|_| attendee(AttendeeRole::Invitee, Some(Rsvp::Accepted))));
    attendees
}

fn call() -> Vec<Segment> {
    vec![
        said(Channel::Mic, 0, 4_000),
        said(Channel::System, 4_000, 20_000),
    ]
}

#[test]
fn on_a_call_the_system_track_is_diarized_without_the_user() {
    let plan = plan_channels(&call(), &accepted(4));
    assert_eq!(
        plan,
        ChannelPlan {
            diarize: Some(PlannedTrack {
                channel: Channel::System,
                speakers: SpeakerHint::Exactly(3)
            }),
            me: Some(Channel::Mic),
        }
    );
}

#[test]
fn unconfirmed_invitees_only_cap_the_count() {
    let mut attendees = accepted(3);
    attendees.push(attendee(AttendeeRole::Invitee, Some(Rsvp::Tentative)));
    let plan = plan_channels(&call(), &attendees);
    assert_eq!(plan.diarize.unwrap().speakers, SpeakerHint::AtMost(3));
}

#[test]
fn declined_attendees_are_not_expected() {
    let mut attendees = accepted(2);
    attendees.push(attendee(AttendeeRole::Invitee, Some(Rsvp::Declined)));
    let plan = plan_channels(&call(), &attendees);
    assert_eq!(plan.diarize.unwrap().speakers, SpeakerHint::Exactly(1));
}

#[test]
fn a_silent_system_track_with_several_attendees_is_an_in_person_meeting() {
    let segments = vec![
        said(Channel::Mic, 0, 60_000),
        said(Channel::System, 70_000, 72_500),
    ];
    let plan = plan_channels(&segments, &accepted(3));
    assert_eq!(
        plan,
        ChannelPlan {
            diarize: Some(PlannedTrack {
                channel: Channel::Mic,
                speakers: SpeakerHint::Exactly(3)
            }),
            me: None,
        }
    );
}

#[test]
fn more_than_three_seconds_of_system_speech_is_a_call() {
    let segments = vec![
        said(Channel::Mic, 0, 60_000),
        said(Channel::System, 70_000, 73_100),
    ];
    let plan = plan_channels(&segments, &accepted(3));
    assert_eq!(plan.diarize.unwrap().channel, Channel::System);
    assert_eq!(plan.me, Some(Channel::Mic));
}

#[test]
fn without_attendees_a_quiet_system_track_is_still_not_in_person() {
    let segments = vec![said(Channel::Mic, 0, 60_000)];
    assert_eq!(
        plan_channels(&segments, &[]),
        ChannelPlan {
            diarize: None,
            me: Some(Channel::Mic)
        }
    );
}

#[test]
fn a_mixed_recording_is_diarized_with_everyone() {
    let segments = vec![said(Channel::Mixed, 0, 60_000)];
    let plan = plan_channels(&segments, &accepted(2));
    assert_eq!(
        plan,
        ChannelPlan {
            diarize: Some(PlannedTrack {
                channel: Channel::Mixed,
                speakers: SpeakerHint::Exactly(2)
            }),
            me: None,
        }
    );
}

#[test]
fn speech_counts_words_or_whole_segments_without_them() {
    let mut wordless = said(Channel::System, 0, 2_000);
    wordless.words.clear();
    let mut gappy = said(Channel::System, 5_000, 9_000);
    gappy.words = vec![
        WordTiming {
            text: "a".into(),
            start: Millis(5_000),
            end: Millis(5_500),
        },
        WordTiming {
            text: "b".into(),
            start: Millis(8_000),
            end: Millis(8_250),
        },
    ];
    let segments = vec![wordless, gappy, said(Channel::Mic, 0, 9_000)];
    assert_eq!(speech(&segments, Channel::System), Millis(2_750));
}

#[test]
fn a_lone_invitee_gives_the_system_track_no_count() {
    let plan = plan_channels(&call(), &accepted(1));
    assert_eq!(plan.diarize.unwrap().speakers, SpeakerHint::Unknown);
}
