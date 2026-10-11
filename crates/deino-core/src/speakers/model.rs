//! Speakers as the participants panel shows them, and the naming requests
//! it sends.

use chrono::{DateTime, Utc};
use deino_domain::{MeetingId, Millis, PaletteColor, PersonId, SegmentId, SpeakerId};
use serde::{Deserialize, Serialize};

use crate::segments::Speaker;

/// The longest name a new person can be given from the panel.
pub const MAX_NAME_CHARS: usize = 100;

/// A person a speaker is, or may be.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PersonRef {
    /// The person.
    pub id: PersonId,
    /// Their name.
    pub name: String,
}

/// "Is this Alex?": a voice that resembles a known person, short of the
/// automatic threshold.
#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Suggestion {
    /// Who it may be.
    pub person: PersonRef,
    /// Cosine similarity with their best exemplar.
    pub score: f32,
}

/// One speaker of a meeting, as the participants panel shows it.
#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Participant {
    /// The speaker.
    pub speaker_id: SpeakerId,
    /// "Speaker 2" until a person is assigned.
    pub label: String,
    /// The speaker's palette slot.
    pub color: PaletteColor,
    /// True for the user's microphone voice.
    pub is_me: bool,
    /// Who the speaker is, once named.
    pub person: Option<PersonRef>,
    /// How long they spoke (their segments' total length).
    pub speech: Millis,
    /// Their share of the meeting's attributed speech, in whole percent.
    pub share_percent: u8,
    /// A suggested name awaiting the user's answer.
    pub suggestion: Option<Suggestion>,
}

/// A participant as stored, before its share is known.
#[derive(Clone, Debug, PartialEq)]
pub struct ParticipantRow {
    /// The speaker row.
    pub speaker: Speaker,
    /// The person it is, if named.
    pub person: Option<PersonRef>,
    /// Its segments' total length.
    pub speech: Millis,
    /// The pending suggestion, if any.
    pub suggestion: Option<Suggestion>,
}

/// Who a speaker is to become.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, specta::Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SpeakerName {
    /// An existing person: an attendee, anyone from People, or the user.
    #[serde(rename_all = "camelCase")]
    Person {
        /// The person.
        person_id: PersonId,
    },
    /// Someone new: creates a person with this name.
    New {
        /// Their name, trimmed, at most [`MAX_NAME_CHARS`] characters.
        name: String,
    },
}

/// Which segments a rename covers.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, specta::Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum RenameScope {
    /// One segment moves to the named person's speaker.
    #[serde(rename_all = "camelCase")]
    Segment {
        /// The segment.
        segment_id: SegmentId,
    },
    /// The speaker becomes the person everywhere; the voice is enrolled.
    All,
}

/// "This is Alex" from the participants panel or a transcript chip.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Rename {
    /// The meeting.
    pub meeting_id: MeetingId,
    /// The speaker being named.
    pub speaker_id: SpeakerId,
    /// Who they are.
    pub name: SpeakerName,
    /// One segment or all of them.
    pub scope: RenameScope,
}

/// What a rename did, so the UI can offer to regenerate notes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Renamed {
    /// The person the speaker now is.
    pub person: PersonRef,
    /// The speaker that now holds the renamed segments (another one, when
    /// the person already had a speaker in the meeting).
    pub speaker_id: SpeakerId,
    /// Whether the voice was saved to the person's voice profile.
    pub enrolled: bool,
}

/// A name the rename picker offers: attendees first, then people.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct NameCandidate {
    /// The person.
    pub person: PersonRef,
    /// Invited to the meeting.
    pub is_attendee: bool,
    /// The user.
    pub is_me: bool,
}

/// Names every segment of a speaker as a person, in one transaction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NameSpeaker {
    /// The meeting.
    pub meeting_id: MeetingId,
    /// The speaker.
    pub speaker_id: SpeakerId,
    /// Who it was before (whose action items in the meeting move along).
    pub previous: Option<PersonId>,
    /// Who it is now.
    pub person: PersonId,
    /// When, for the moved action items' `updated_at`.
    pub at: DateTime<Utc>,
}

/// The speaker a renamed segment moves to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SegmentTarget {
    /// The person's existing speaker in the meeting.
    Existing(SpeakerId),
    /// A new speaker for the person, created in the same transaction.
    New(Speaker),
}

/// Two speakers of one meeting that are one voice.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergeSpeakers {
    /// The meeting.
    pub meeting_id: MeetingId,
    /// The speaker that goes away.
    pub from: Speaker,
    /// The speaker that keeps every segment.
    pub into: Speaker,
    /// When, for the moved action items' `updated_at`.
    pub at: DateTime<Utc>,
}

/// What `identify_speakers` decided, written in one transaction.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Identification {
    /// Speakers named outright.
    pub named: Vec<(SpeakerId, PersonId)>,
    /// "Is this …?" suggestions, replacing the meeting's previous ones.
    pub suggested: Vec<SuggestedName>,
}

/// One suggestion to store.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SuggestedName {
    /// The speaker.
    pub speaker_id: SpeakerId,
    /// Who it may be.
    pub person_id: PersonId,
    /// The similarity behind it.
    pub score: f32,
}
