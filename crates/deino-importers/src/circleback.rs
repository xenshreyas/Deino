//! Circleback (C §12.1): `GET /meetings` pages through the user's meetings
//! (RFC 8288 `Link` cursors), then `GET /meeting/{id}` and
//! `GET /meeting/{id}/transcript` give each one. Auth: `Bearer cb_…`.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_core::importers::{Importer, ImporterError, SourceItem, SourcePage};
use deino_core::transcript_import::{
    ImportActionItem, ImportAttendee, ImportTag, TranscriptImport,
};
use deino_domain::ImportSource;
use deino_net::NetClient;
use serde::Deserialize;

use crate::http::ProviderApi;
use crate::mapping::{self, Parts};

/// Circleback's API root.
pub const BASE_URL: &str = "https://circleback.ai/api";

/// Imports from Circleback.
pub struct CirclebackImporter {
    api: ProviderApi,
}

impl CirclebackImporter {
    /// An importer over `client` (scoped to `circleback`) with `key`.
    pub fn new(client: Arc<dyn NetClient>, key: &str) -> Self {
        let auth = format!("Bearer {key}");
        Self {
            api: ProviderApi::new(client, ImportSource::Circleback, "authorization", auth),
        }
    }
}

#[async_trait]
impl Importer for CirclebackImporter {
    fn source(&self) -> ImportSource {
        ImportSource::Circleback
    }

    async fn page(&self, cursor: Option<&str>) -> Result<SourcePage, ImporterError> {
        let url = match cursor {
            // The `Link` header's URL, on Circleback's own API only.
            Some(next) if next.starts_with(BASE_URL) => next.to_owned(),
            Some(_) => {
                return Err(ImporterError::Unreadable(
                    self.source(),
                    "a foreign next link",
                ));
            }
            None => format!("{BASE_URL}/meetings"),
        };
        let fetched = self.api.get::<Vec<Listed>>(&url).await?;
        Ok(SourcePage {
            items: fetched
                .body
                .into_iter()
                .map(|listed| SourceItem::listed(mapping::id_text(&listed.id)))
                .collect(),
            next: fetched.next,
        })
    }

    async fn fetch(&self, item: SourceItem) -> Result<TranscriptImport, ImporterError> {
        let id = url::form_urlencoded::byte_serialize(item.id.as_bytes()).collect::<String>();
        let meeting: Meeting = self
            .api
            .get(&format!("{BASE_URL}/meeting/{id}"))
            .await?
            .body;
        let transcript: Vec<Segment> = self
            .api
            .get(&format!("{BASE_URL}/meeting/{id}/transcript"))
            .await?
            .body;
        Ok(meeting.into_import(&transcript))
    }
}

/// A meeting in the list: only its ID matters here.
#[derive(Deserialize)]
struct Listed {
    id: serde_json::Value,
}

/// `GET /meeting/{id}`, the fields Deino keeps.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Meeting {
    name: Option<String>,
    created_at: Option<DateTime<Utc>>,
    notes: Option<String>,
    #[serde(default)]
    tags: Vec<Tag>,
    #[serde(default)]
    attendees: Vec<Attendee>,
    #[serde(default)]
    action_items: Vec<ActionItem>,
}

#[derive(Deserialize)]
struct Tag {
    name: String,
    description: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Attendee {
    name: Option<String>,
    email: Option<String>,
    #[serde(default)]
    is_calendar_event_organizer: bool,
    #[serde(default)]
    is_calendar_invitee: bool,
}

#[derive(Deserialize)]
struct ActionItem {
    title: String,
    description: Option<String>,
    /// `PENDING` or `DONE`.
    status: String,
    assignee: Option<Attendee>,
}

/// One line of `GET /meeting/{id}/transcript`.
#[derive(Deserialize)]
struct Segment {
    speaker: Option<String>,
    text: String,
    /// Seconds from the start.
    timestamp: f64,
}

impl Meeting {
    fn into_import(self, transcript: &[Segment]) -> TranscriptImport {
        let parts = Parts {
            title: self.name,
            created_at: self.created_at,
            notes: self.notes,
            transcript: transcript
                .iter()
                .filter_map(|line| {
                    mapping::utterance(line.speaker.as_deref(), &line.text, line.timestamp, None)
                })
                .collect(),
            attendees: self.attendees.iter().filter_map(Attendee::import).collect(),
            action_items: self
                .action_items
                .into_iter()
                .map(ActionItem::import)
                .collect(),
        };
        let mut import = parts.into_import();
        import.tags = self
            .tags
            .into_iter()
            .map(|tag| ImportTag {
                name: tag.name,
                description: tag.description.unwrap_or_default(),
            })
            .collect();
        import
    }
}

impl Attendee {
    fn import(&self) -> Option<ImportAttendee> {
        let mut attendee = mapping::attendee(self.name.as_deref(), self.email.as_deref())?;
        attendee.is_calendar_event_organizer = self.is_calendar_event_organizer;
        attendee.is_calendar_invitee = self.is_calendar_invitee;
        Some(attendee)
    }
}

impl ActionItem {
    fn import(self) -> ImportActionItem {
        ImportActionItem {
            title: self.title,
            description: self.description.unwrap_or_default(),
            done: self.status.eq_ignore_ascii_case("done"),
            assignee: self.assignee.as_ref().and_then(Attendee::import),
        }
    }
}
