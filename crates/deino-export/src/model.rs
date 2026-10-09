//! [`MeetingExport`]: everything an export can say about one meeting.

use chrono::{DateTime, FixedOffset, NaiveDate};
use deino_domain::{MeetingId, Millis, Seconds};
use serde::Serialize;

use crate::{ExportSections, Section};

/// The scheme of the links that open a meeting in Deino (DEV-05 registers it).
const LINK_PREFIX: &str = "deino://meetings/";

/// One meeting, as every export format sees it.
///
/// `deino-core` assembles it from the repositories; renderers only read it.
/// Times are already in the user's local zone, so a renderer never needs one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingExport {
    /// The meeting's ID.
    pub id: MeetingId,
    /// Its title.
    pub title: String,
    /// When it started (or was created), in the user's zone.
    pub started_at: DateTime<FixedOffset>,
    /// How long it recorded, when known.
    pub duration: Option<Seconds>,
    /// Who took part: invitees first, as the attendee list orders them.
    pub attendees: Vec<Participant>,
    /// Tag names, by name.
    pub tags: Vec<String>,
    /// Names of the folders the meeting sits in, by name.
    pub folders: Vec<String>,
    /// The AI notes (the summary today, the enhanced notes after NOT-02), as Markdown.
    pub notes: Option<String>,
    /// What the user typed during the meeting, as Markdown.
    pub my_notes: Option<String>,
    /// The meeting's action items, in the order they were created.
    pub action_items: Vec<ActionItemExport>,
    /// The transcript, in time order, with speaker names.
    pub transcript: Vec<Utterance>,
}

impl MeetingExport {
    /// The `deino://` link that opens the meeting in the app.
    pub fn link(&self) -> String {
        format!("{LINK_PREFIX}{}", self.id)
    }

    /// The meeting's calendar date in the user's zone.
    pub fn date(&self) -> NaiveDate {
        self.started_at.date_naive()
    }

    /// The chosen sections that have something to show, in the order every
    /// format writes them.
    pub fn sections(&self, chosen: ExportSections) -> Vec<Section> {
        Self::ORDER
            .into_iter()
            .filter(|section| chosen.includes(*section) && self.has(*section))
            .collect()
    }

    /// The order sections appear in a document.
    const ORDER: [Section; 6] = [
        Section::Attendees,
        Section::Notes,
        Section::MyNotes,
        Section::ActionItems,
        Section::Insights,
        Section::Transcript,
    ];

    fn has(&self, section: Section) -> bool {
        let written =
            |notes: &Option<String>| notes.as_deref().is_some_and(|text| !text.trim().is_empty());
        match section {
            Section::Attendees => !self.attendees.is_empty(),
            Section::Notes => written(&self.notes),
            Section::MyNotes => written(&self.my_notes),
            Section::ActionItems => !self.action_items.is_empty(),
            // AI-06 adds insights to the export.
            Section::Insights => false,
            Section::Transcript => !self.transcript.is_empty(),
        }
    }
}

/// Someone who attended the meeting.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Participant {
    /// Their display name.
    pub name: String,
    /// Their email address, when known.
    pub email: Option<String>,
    /// True for the user.
    pub is_me: bool,
}

/// An action item of the meeting.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionItemExport {
    /// What to do.
    pub title: String,
    /// Context; may be empty.
    pub description: String,
    /// Who should do it, by name.
    pub assignee: Option<String>,
    /// When it is due, in the user's zone.
    pub due: Option<NaiveDate>,
    /// True once done.
    pub done: bool,
}

/// One stretch of speech in the transcript.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Utterance {
    /// Who said it: a person's name, "Me", or a speaker label.
    pub speaker: String,
    /// True when the user said it.
    pub is_me: bool,
    /// Offset from the start of the recording.
    pub start: Millis,
    /// Offset of its end; never before `start`.
    pub end: Millis,
    /// What was said.
    pub text: String,
}
