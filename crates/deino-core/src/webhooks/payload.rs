//! The JSON body of an event delivery: a Standard Webhooks envelope
//! (`type`, `timestamp`, `data`) around Circleback's meeting payload, with
//! only the fields the endpoint chose.
//!
//! The meeting comes from EXP-01's assembler (through the automations'
//! [`MeetingSource`]), so a webhook, a script's stdin and the JSON export
//! agree, and NOT-09 redaction applies to all of them while it is on.

use chrono::{DateTime, FixedOffset, Utc};
use deino_domain::{ActionItemId, MeetingId, Seconds};
use deino_export::{ActionItemExport, InsightExport, MeetingExport, Participant, Utterance};
use serde::Serialize;

use crate::automations::{MeetingSource, PayloadFields};
use crate::export::ExportError;

/// A whole event body.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct WebhookPayload {
    /// The event, for example `notes.generated`, or `test`.
    #[serde(rename = "type")]
    pub kind: &'static str,
    /// When the body was assembled.
    pub timestamp: DateTime<Utc>,
    /// The meeting.
    pub data: MeetingData,
}

/// The meeting, Circleback-style, with the endpoint's fields.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingData {
    /// Its ID.
    pub id: MeetingId,
    /// Its title.
    pub name: String,
    /// When it started, in the user's zone.
    pub created_at: DateTime<FixedOffset>,
    /// How long it recorded, in seconds.
    pub duration: Option<Seconds>,
    /// Its tags, by name.
    pub tags: Vec<String>,
    /// Its folders, by name.
    pub folders: Vec<String>,
    /// The `deino://` link that opens it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Who took part.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attendees: Option<Vec<Participant>>,
    /// The AI notes, as Markdown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    /// The action items.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_items: Option<Vec<ActionItemExport>>,
    /// The transcript.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transcript: Option<Vec<Utterance>>,
    /// The custom insights' answers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub insights: Option<Vec<InsightExport>>,
    /// Where the recording is on this Mac.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_path: Option<String>,
    /// The action item an `action_item.*` event is about.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_item: Option<SubjectItem>,
}

/// The action item an event is about (redacted like the export's).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubjectItem {
    /// Its ID.
    pub id: ActionItemId,
    /// What to do.
    pub title: String,
    /// Context; may be empty.
    pub description: String,
    /// `open` or `done`.
    pub status: &'static str,
    /// When it is due.
    pub due_at: Option<DateTime<Utc>>,
}

/// What one event body is built from.
#[derive(Clone, Copy, Debug)]
pub struct PayloadRequest<'a> {
    /// The event's wire name.
    pub kind: &'static str,
    /// The meeting.
    pub meeting: &'a MeetingId,
    /// The endpoint's fields.
    pub fields: PayloadFields,
    /// When it is assembled.
    pub at: DateTime<Utc>,
}

/// Builds the body for `request` from `source`; `item` is the subject of
/// an action item event.
pub async fn build_event_payload(
    source: &dyn MeetingSource,
    request: PayloadRequest<'_>,
    item: Option<SubjectItem>,
) -> Result<WebhookPayload, ExportError> {
    let fields = request.fields;
    let export = source.export(request.meeting).await?;
    let audio_path = if fields.audio_path {
        source.audio_path(request.meeting).await?
    } else {
        None
    };
    let mut data = meeting_data(export, fields);
    data.audio_path = audio_path;
    data.action_item = item;
    Ok(WebhookPayload {
        kind: request.kind,
        timestamp: request.at,
        data,
    })
}

/// `export` with only `fields` (the header is always there).
pub fn meeting_data(export: MeetingExport, fields: PayloadFields) -> MeetingData {
    let url = fields.link.then(|| export.link());
    MeetingData {
        url,
        attendees: fields.attendees.then_some(export.attendees),
        notes: export.notes.filter(|_| fields.notes),
        action_items: fields.action_items.then_some(export.action_items),
        transcript: fields.transcript.then_some(export.transcript),
        insights: fields.insights.then_some(export.insights),
        audio_path: None,
        action_item: None,
        id: export.id,
        name: export.title,
        created_at: export.started_at,
        duration: export.duration,
        tags: export.tags,
        folders: export.folders,
    }
}
