//! Fathom: `GET /external/v1/meetings` with the transcript, summary and
//! action items included pages through the user's meetings
//! (`{items, next_cursor}`), so there is no detail call. Auth: `X-Api-Key`.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_core::importers::{Importer, ImporterError, SourceItem, SourcePage};
use deino_core::transcript_import::{ImportActionItem, ImportAttendee, TranscriptImport};
use deino_domain::ImportSource;
use deino_net::NetClient;
use serde::Deserialize;

use crate::http::ProviderApi;
use crate::mapping::{self, Parts};

/// Fathom's API root.
pub const BASE_URL: &str = "https://api.fathom.ai/external/v1";
/// The list's query: everything a meeting needs, in one call.
const INCLUDE: &str = "include_transcript=true&include_summary=true&include_action_items=true";
/// The speaker name of the person who recorded the meeting.
const ME: &str = "Me";

/// Imports from Fathom.
pub struct FathomImporter {
    api: ProviderApi,
}

impl FathomImporter {
    /// An importer over `client` (scoped to `fathom`) with `key`.
    pub fn new(client: Arc<dyn NetClient>, key: &str) -> Self {
        Self {
            api: ProviderApi::new(client, ImportSource::Fathom, "x-api-key", key.to_owned()),
        }
    }
}

#[async_trait]
impl Importer for FathomImporter {
    fn source(&self) -> ImportSource {
        ImportSource::Fathom
    }

    async fn page(&self, cursor: Option<&str>) -> Result<SourcePage, ImporterError> {
        let mut url = format!("{BASE_URL}/meetings?{INCLUDE}");
        if let Some(cursor) = cursor {
            url.push_str("&cursor=");
            url.extend(url::form_urlencoded::byte_serialize(cursor.as_bytes()));
        }
        let list: MeetingList = self.api.get(&url).await?.body;
        Ok(SourcePage {
            items: list
                .items
                .into_iter()
                .map(|meeting| {
                    let id = mapping::id_text(&meeting.recording_id);
                    SourceItem::complete(id, meeting.into_import())
                })
                .collect(),
            next: list.next_cursor.filter(|cursor| !cursor.is_empty()),
        })
    }
}

#[derive(Deserialize)]
struct MeetingList {
    items: Vec<Meeting>,
    next_cursor: Option<String>,
}

#[derive(Deserialize)]
struct Meeting {
    recording_id: serde_json::Value,
    title: Option<String>,
    #[serde(rename = "meeting_title")]
    calendar_title: Option<String>,
    created_at: Option<DateTime<Utc>>,
    recording_start_time: Option<DateTime<Utc>>,
    recorded_by: Option<Person>,
    #[serde(default)]
    calendar_invitees: Vec<Person>,
    transcript: Option<Vec<Line>>,
    default_summary: Option<Summary>,
    action_items: Option<Vec<ActionItem>>,
}

#[derive(Deserialize)]
struct Person {
    name: Option<String>,
    email: Option<String>,
}

#[derive(Deserialize)]
struct Line {
    speaker: Speaker,
    text: String,
    /// `HH:MM:SS` from the start of the recording.
    timestamp: String,
}

#[derive(Deserialize)]
struct Speaker {
    display_name: Option<String>,
    matched_calendar_invitee_email: Option<String>,
}

#[derive(Deserialize)]
struct Summary {
    markdown_formatted: Option<String>,
}

#[derive(Deserialize)]
struct ActionItem {
    description: String,
    #[serde(default)]
    completed: bool,
    assignee: Option<Person>,
}

impl Meeting {
    fn into_import(self) -> TranscriptImport {
        let recorder = self
            .recorded_by
            .as_ref()
            .and_then(|person| person.email.as_deref())
            .map(str::to_lowercase);
        Parts {
            title: self.title.or(self.calendar_title),
            created_at: self.recording_start_time.or(self.created_at),
            notes: self.default_summary.and_then(|summary| summary.markdown_formatted),
            transcript: self
                .transcript
                .unwrap_or_default()
                .iter()
                .filter_map(|line| line.import(recorder.as_deref()))
                .collect(),
            attendees: self
                .calendar_invitees
                .iter()
                .filter_map(Person::import)
                .collect(),
            action_items: self
                .action_items
                .unwrap_or_default()
                .into_iter()
                .map(ActionItem::import)
                .collect(),
        }
        .into_import()
    }
}

impl Line {
    fn import(&self, recorder: Option<&str>) -> Option<deino_core::transcript_import::ImportUtterance> {
        let mine = self
            .speaker
            .matched_calendar_invitee_email
            .as_deref()
            .is_some_and(|email| Some(email.to_lowercase().as_str()) == recorder);
        let speaker = if mine { Some(ME) } else { self.speaker.display_name.as_deref() };
        let start = mapping::clock_seconds(&self.timestamp)?;
        mapping::utterance(speaker, &self.text, start, None)
    }
}

impl Person {
    fn import(&self) -> Option<ImportAttendee> {
        mapping::attendee(self.name.as_deref(), self.email.as_deref())
    }
}

impl ActionItem {
    fn import(self) -> ImportActionItem {
        ImportActionItem {
            title: self.description,
            description: String::new(),
            done: self.completed,
            assignee: self.assignee.as_ref().and_then(Person::import),
        }
    }
}
