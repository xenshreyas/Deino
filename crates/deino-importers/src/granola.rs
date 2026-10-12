//! Granola (G §9.2): `GET /v1/notes` pages through the user's notes
//! (`{notes, hasMore, cursor}`), then `GET /v1/notes/{id}?include=transcript`
//! gives each one. Auth: `Bearer grn_…`. A transcript too large to inline
//! (413) imports the note without it.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_core::importers::{Importer, ImporterError, SourceItem, SourcePage};
use deino_core::transcript_import::{ImportAttendee, ImportUtterance, TranscriptImport};
use deino_domain::ImportSource;
use deino_net::{NetClient, NetError};
use serde::Deserialize;

use crate::http::ProviderApi;
use crate::mapping::{self, Parts};

/// Granola's public API root.
pub const BASE_URL: &str = "https://public-api.granola.ai/v1";
/// Notes per list page.
const PAGE_SIZE: u32 = 30;
/// The speaker name of the user's own microphone.
const ME: &str = "Me";
/// The speaker name of other people when Granola has no better one.
const THEM: &str = "Them";

/// Imports from Granola.
pub struct GranolaImporter {
    api: ProviderApi,
}

impl GranolaImporter {
    /// An importer over `client` (scoped to `granola`) with `key`.
    pub fn new(client: Arc<dyn NetClient>, key: &str) -> Self {
        let auth = format!("Bearer {key}");
        Self {
            api: ProviderApi::new(client, ImportSource::Granola, "authorization", auth),
        }
    }
}

#[async_trait]
impl Importer for GranolaImporter {
    fn source(&self) -> ImportSource {
        ImportSource::Granola
    }

    async fn page(&self, cursor: Option<&str>) -> Result<SourcePage, ImporterError> {
        let mut url = format!("{BASE_URL}/notes?page_size={PAGE_SIZE}");
        if let Some(cursor) = cursor {
            url.push_str("&cursor=");
            url.extend(url::form_urlencoded::byte_serialize(cursor.as_bytes()));
        }
        let list: NoteList = self.api.get(&url).await?.body;
        Ok(SourcePage {
            items: list
                .notes
                .into_iter()
                .map(|note| SourceItem::listed(note.id))
                .collect(),
            next: list.cursor.filter(|_| list.has_more),
        })
    }

    async fn fetch(&self, item: SourceItem) -> Result<TranscriptImport, ImporterError> {
        let id = url::form_urlencoded::byte_serialize(item.id.as_bytes()).collect::<String>();
        let url = format!("{BASE_URL}/notes/{id}");
        match self
            .api
            .get::<Note>(&format!("{url}?include=transcript"))
            .await
        {
            Ok(fetched) => Ok(fetched.body.into_import()),
            Err(ImporterError::Net(NetError::Status(status))) if status.as_u16() == 413 => {
                log::info!(target: "importers", "a Granola transcript is too large to inline; importing the note alone");
                Ok(self.api.get::<Note>(&url).await?.body.into_import())
            }
            Err(error) => Err(error),
        }
    }
}

/// `GET /v1/notes`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NoteList {
    notes: Vec<NoteSummary>,
    #[serde(default)]
    has_more: bool,
    cursor: Option<String>,
}

#[derive(Deserialize)]
struct NoteSummary {
    id: String,
}

/// `GET /v1/notes/{id}`, the fields Deino keeps.
#[derive(Deserialize)]
struct Note {
    title: Option<String>,
    created_at: Option<DateTime<Utc>>,
    calendar_event: Option<CalendarEvent>,
    #[serde(default)]
    attendees: Vec<User>,
    summary_text: Option<String>,
    summary_markdown: Option<String>,
    private_notes_markdown: Option<String>,
    transcript: Option<Vec<Line>>,
}

#[derive(Deserialize)]
struct CalendarEvent {
    organiser: Option<String>,
}

#[derive(Deserialize)]
struct User {
    name: Option<String>,
    email: Option<String>,
}

#[derive(Deserialize)]
struct Line {
    speaker: Speaker,
    text: String,
    start_time: DateTime<Utc>,
    end_time: Option<DateTime<Utc>>,
}

#[derive(Deserialize)]
struct Speaker {
    /// `microphone` (the user) or `speaker` (everyone else).
    source: String,
    attribution: Option<String>,
    diarization_label: Option<String>,
    name: Option<String>,
}

impl Note {
    fn into_import(self) -> TranscriptImport {
        let organiser = self
            .calendar_event
            .and_then(|event| event.organiser)
            .map(|email| email.to_lowercase());
        Parts {
            title: self.title,
            created_at: self.created_at,
            notes: notes(
                self.summary_markdown.or(self.summary_text),
                self.private_notes_markdown,
            ),
            transcript: transcript(self.transcript.as_deref().unwrap_or_default()),
            attendees: self
                .attendees
                .iter()
                .filter_map(|user| attendee(user, organiser.as_deref()))
                .collect(),
            action_items: Vec::new(),
        }
        .into_import()
    }
}

/// The summary, then the user's own notes under their own heading.
fn notes(summary: Option<String>, private: Option<String>) -> Option<String> {
    let private = private.filter(|text| !text.trim().is_empty());
    match (summary, private) {
        (Some(summary), Some(private)) => Some(format!("{summary}\n\n## My notes\n\n{private}")),
        (summary, None) => summary,
        (None, Some(private)) => Some(private),
    }
}

/// Lines timed from the first one: Granola stamps them with wall-clock times.
fn transcript(lines: &[Line]) -> Vec<ImportUtterance> {
    let Some(origin) = lines.iter().map(|line| line.start_time).min() else {
        return Vec::new();
    };
    let seconds = |at: DateTime<Utc>| (at - origin).as_seconds_f64();
    lines
        .iter()
        .filter_map(|line| {
            let speaker = speaker(&line.speaker);
            let end = line.end_time.map(seconds);
            mapping::utterance(Some(speaker), &line.text, seconds(line.start_time), end)
        })
        .collect()
}

fn speaker(speaker: &Speaker) -> &str {
    let mine = speaker.source == "microphone" || speaker.attribution.as_deref() == Some("me");
    if mine {
        return ME;
    }
    speaker
        .name
        .as_deref()
        .or(speaker.diarization_label.as_deref())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or(THEM)
}

fn attendee(user: &User, organiser: Option<&str>) -> Option<ImportAttendee> {
    let mut attendee = mapping::attendee(user.name.as_deref(), user.email.as_deref())?;
    attendee.is_calendar_event_organizer = attendee
        .email
        .as_deref()
        .is_some_and(|email| Some(email.to_lowercase().as_str()) == organiser);
    Some(attendee)
}
