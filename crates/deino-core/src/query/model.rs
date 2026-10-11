//! What the library reads out: meetings, their action items and their
//! transcripts, shaped for a reader outside the app (a terminal, an LLM).
//!
//! Every type serializes as camelCase JSON and describes itself with a JSON
//! schema, which the MCP server publishes. Times are in the user's zone.

use chrono::{DateTime, FixedOffset, NaiveDate};
use deino_domain::{ActionItemId, ActionItemStatus, MeetingId, MeetingStatus};
use schemars::JsonSchema;
use serde::Serialize;

/// One meeting in a list.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MeetingSummary {
    /// The meeting's ID; pass it to `read_meeting` or `get_transcript`.
    #[schemars(with = "String")]
    pub id: MeetingId,
    /// Its title.
    pub title: String,
    /// When it started (or was created), in the user's zone.
    pub held_at: DateTime<FixedOffset>,
    /// How long it recorded, in seconds, when known.
    pub duration_s: Option<u32>,
    /// `scheduled`, `recording`, `processing`, `ready` or `failed`.
    #[schemars(with = "String")]
    pub status: MeetingStatus,
    /// Who attended, by name (invitees who declined are left out).
    pub attendees: Vec<String>,
    /// Its tags, by name.
    pub tags: Vec<String>,
    /// `deino://meeting/<id>`, which opens it in the app.
    pub link: String,
}

/// Everything about one meeting except its transcript.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MeetingDetail {
    /// The meeting as a list shows it.
    #[serde(flatten)]
    pub summary: MeetingSummary,
    /// Who attended, with emails.
    pub people: Vec<AttendeeView>,
    /// The folders it is filed in, by name.
    pub folders: Vec<String>,
    /// What the user typed during the meeting, as Markdown.
    pub my_notes: Option<String>,
    /// The newest version of the AI notes (enhanced notes), as Markdown;
    /// `[12:04]` marks a cited transcript moment.
    pub ai_notes: Option<String>,
    /// Which version of the AI notes that is (1 for the first generation).
    pub ai_notes_version: Option<u32>,
    /// The transcript moments the AI notes cite, in time order.
    pub citations: Vec<CitationLink>,
    /// Its action items.
    pub action_items: Vec<ActionItemSummary>,
    /// Its recording files.
    pub audio: AudioFiles,
    /// How many transcript segments it has; read them with `get_transcript`.
    pub transcript_segments: u32,
}

/// A transcript moment the AI notes cite.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CitationLink {
    /// How the notes show it, such as `12:04`.
    pub label: String,
    /// Milliseconds from the start of the recording.
    pub at_ms: u32,
    /// `deino://meeting/<id>?t=<ms>`.
    pub link: String,
}

/// Someone who attended a meeting.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AttendeeView {
    /// Their name.
    pub name: String,
    /// Their email, when known.
    pub email: Option<String>,
    /// True for the user.
    pub is_me: bool,
}

/// Absolute paths of a meeting's recording files.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AudioFiles {
    /// Microphone and system audio mixed together.
    pub mixed: Option<String>,
    /// The microphone track (the user).
    pub microphone: Option<String>,
    /// The system audio track (everyone else on the call).
    pub system: Option<String>,
}

/// An action item.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ActionItemSummary {
    /// The item's ID.
    #[schemars(with = "String")]
    pub id: ActionItemId,
    /// What to do.
    pub title: String,
    /// Context; may be empty.
    pub description: String,
    /// `open` or `done`.
    #[schemars(with = "String")]
    pub status: ActionItemStatus,
    /// Who should do it, by name.
    pub assignee: Option<String>,
    /// True when it is the user's.
    pub assignee_is_me: bool,
    /// The day it is due, in the user's zone.
    pub due: Option<NaiveDate>,
    /// The meeting it came from; `None` for a standalone to-do.
    #[schemars(with = "Option<String>")]
    pub meeting_id: Option<MeetingId>,
    /// That meeting's title.
    pub meeting_title: Option<String>,
    /// A `deino://` link to the moment it was said, or to its meeting.
    pub link: Option<String>,
}

/// A window of a meeting's transcript.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptPage {
    /// The meeting's ID.
    #[schemars(with = "String")]
    pub meeting_id: MeetingId,
    /// Its title.
    pub title: String,
    /// The utterances in the window, in time order.
    pub lines: Vec<TranscriptLine>,
    /// Where to start the next page (`fromMs`) when this one was cut short.
    pub next_from_ms: Option<u32>,
    /// How many utterances the whole transcript has.
    pub total_lines: u32,
}

/// One stretch of speech.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptLine {
    /// Milliseconds from the start of the recording.
    pub start_ms: u32,
    /// Where it ends.
    pub end_ms: u32,
    /// Who said it: a name, "Me", "Them" or a speaker label.
    pub speaker: String,
    /// True when the user said it.
    pub is_me: bool,
    /// What was said.
    pub text: String,
}
