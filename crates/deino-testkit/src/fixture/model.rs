//! The fixture schema: one [`FixtureMeeting`] per `fixtures/meetings/*.json`.
//!
//! Every struct denies unknown fields and every closed set is an enum, so
//! deserializing a file is the first half of the schema check; the second
//! half (cross-references, timing, evidence) is `validate.rs`.
//! `fixtures/README.md` documents the same schema for people.

use chrono::{DateTime, NaiveDate, Utc};
use deino_domain::MeetingId;
use serde::{Deserialize, Serialize};

use super::vocab::{AttendeeRole, Channel, Email, MeetingKind, Millis, Platform, Recurrence, Rsvp};

/// The user every fixture is recorded by. Fictional, like every fixture name.
pub const USER_EMAIL: &str = "shreyas@deino.dev";

/// A recorded meeting with everything the app and the AI evals need.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FixtureMeeting {
    /// The meeting's stable ID, `meeting-fixture-<slug>`.
    pub id: MeetingId,
    /// The meeting title, as the calendar names it.
    pub title: String,
    /// Which scenario this fixture covers.
    pub kind: MeetingKind,
    /// When recording started.
    pub started_at: DateTime<Utc>,
    /// How long the recording runs.
    #[serde(rename = "durationMs")]
    pub duration: Millis,
    /// The calendar event the meeting was recorded from.
    pub calendar_event: CalendarEvent,
    /// The companies the attendees belong to, by email domain.
    pub companies: Vec<Company>,
    /// Everyone on the invite.
    pub attendees: Vec<Attendee>,
    /// Every distinct voice in the transcript.
    pub speakers: Vec<Speaker>,
    /// The transcript, in spoken order.
    pub segments: Vec<Segment>,
    /// The rough notes the user typed during the meeting (markdown).
    pub user_notes: Option<String>,
    /// What an AI should extract from this meeting, for evals.
    pub ground_truth: GroundTruth,
    /// Recorded audio covering the start of the meeting, if any.
    pub audio: Option<FixtureAudio>,
}

/// The calendar event behind a meeting.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CalendarEvent {
    /// The provider's event ID.
    pub id: String,
    /// The iCalendar UID shared by every copy of the event.
    pub ical_uid: String,
    /// The event title.
    pub title: String,
    /// The invite body.
    pub description: String,
    /// The scheduled start.
    pub start_at: DateTime<Utc>,
    /// The scheduled end.
    pub end_at: DateTime<Utc>,
    /// Where the meeting happens.
    pub platform: Platform,
    /// The video-call link; `None` in person.
    pub conference_url: Option<String>,
    /// The room; `None` for calls.
    pub location: Option<String>,
    /// The organizer's email; one attendee has it and the organizer role.
    pub organizer_email: Email,
    /// How the event repeats.
    pub recurrence: Recurrence,
}

/// A company, identified by its email domain.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Company {
    /// The company's display name.
    pub name: String,
    /// The email domain its people use.
    pub domain: String,
}

/// Someone on the invite.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Attendee {
    /// The display name.
    pub name: String,
    /// The email address, unique within a meeting.
    pub email: Email,
    /// The job title, when known.
    pub title: Option<String>,
    /// Organizer or invitee.
    pub role: AttendeeRole,
    /// The invite answer.
    pub rsvp: Rsvp,
}

/// One voice in the transcript, as diarization would label it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Speaker {
    /// The diarization label, `Speaker N` in order of first appearance.
    pub label: String,
    /// The attendee who speaks with this voice; `None` for someone not invited.
    pub attendee: Option<Email>,
    /// The track this voice is heard on.
    pub channel: Channel,
}

/// One finalized transcript segment.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Segment {
    /// The speaker's label.
    pub speaker: String,
    /// The track the segment was heard on.
    pub channel: Channel,
    /// Start, from the start of the recording.
    pub start_ms: Millis,
    /// End, from the start of the recording.
    pub end_ms: Millis,
    /// The transcribed text.
    pub text: String,
    /// Word timings, in order, inside the segment.
    pub words: Vec<Word>,
}

/// One word with its timing, as token-level STT output provides it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Word {
    /// The word, with any attached punctuation.
    pub text: String,
    /// Start, from the start of the recording.
    pub start_ms: Millis,
    /// End, from the start of the recording.
    pub end_ms: Millis,
}

/// What a careful human would extract from the meeting.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GroundTruth {
    /// Decisions the meeting actually made.
    pub decisions: Vec<String>,
    /// Explicit commitments.
    pub action_items: Vec<ExpectedActionItem>,
    /// Questions the meeting alone answers.
    pub facts: Vec<ExpectedFact>,
}

/// An action item an extractor should find.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExpectedActionItem {
    /// What has to be done.
    pub title: String,
    /// The owner's email, an attendee; `None` when nobody took it.
    pub assignee: Option<Email>,
    /// The due date resolved from what was said; `None` when none was given.
    pub due: Option<NaiveDate>,
    /// An exact quote from one segment that states the commitment.
    pub evidence: String,
}

/// A question with its answer, for chat and recall evals.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExpectedFact {
    /// The question.
    pub question: String,
    /// The expected answer.
    pub answer: String,
    /// An exact quote from one segment that supports the answer.
    pub evidence: String,
}

/// Recorded audio for the start of a meeting: one file per track.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FixtureAudio {
    /// The microphone track, relative to the `fixtures/` directory.
    pub mic: String,
    /// The system-audio track, relative to the `fixtures/` directory.
    pub system: String,
    /// Where the clip ends; the segments that end by then are spoken in it.
    pub clip_end_ms: Millis,
}

impl FixtureMeeting {
    /// The speaker with `label`, if the meeting has one.
    pub fn speaker(&self, label: &str) -> Option<&Speaker> {
        self.speakers.iter().find(|speaker| speaker.label == label)
    }

    /// The attendee with `email`, if they were invited.
    pub fn attendee(&self, email: &Email) -> Option<&Attendee> {
        self.attendees
            .iter()
            .find(|attendee| attendee.email == *email)
    }

    /// When recording stopped.
    pub fn ended_at(&self) -> DateTime<Utc> {
        self.started_at + millis_delta(self.duration)
    }

    /// The wall-clock time of an offset into the recording.
    pub fn wall_clock(&self, offset: Millis) -> DateTime<Utc> {
        self.started_at + millis_delta(offset)
    }
}

/// `millis` as a chrono duration.
fn millis_delta(millis: Millis) -> chrono::TimeDelta {
    chrono::TimeDelta::milliseconds(i64::from(millis.0))
}
