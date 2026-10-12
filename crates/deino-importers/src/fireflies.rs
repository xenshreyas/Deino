//! Fireflies: one GraphQL query (`transcripts(limit, skip)`) pages through
//! the user's meetings with everything Deino keeps, so there is no detail
//! call. Auth: `Bearer <API key>`. GraphQL answers errors with a 200 and an
//! `errors` list; an authentication error means the key was refused.

use std::sync::Arc;

use async_trait::async_trait;
use deino_core::importers::{Importer, ImporterError, SourceItem, SourcePage};
use deino_core::transcript_import::{ImportActionItem, ImportAttendee, TranscriptImport};
use deino_domain::ImportSource;
use deino_net::NetClient;
use serde::Deserialize;
use serde_json::json;

use crate::http::ProviderApi;
use crate::mapping::{self, Parts};

/// Fireflies' GraphQL endpoint.
pub const GRAPHQL_URL: &str = "https://api.fireflies.ai/graphql";
/// Transcripts per page: the API's maximum.
pub const PAGE_SIZE: u32 = 50;

/// The query: every field the mapping reads, nothing else.
const QUERY: &str = "query Transcripts($limit: Int, $skip: Int) { transcripts(limit: $limit, skip: $skip) { \
id title date organizer_email \
meeting_attendees { displayName email name } \
sentences { speaker_name text start_time end_time } \
summary { overview shorthand_bullet action_items } } }";

/// Imports from Fireflies.
pub struct FirefliesImporter {
    api: ProviderApi,
}

impl FirefliesImporter {
    /// An importer over `client` (scoped to `fireflies`) with `key`.
    pub fn new(client: Arc<dyn NetClient>, key: &str) -> Self {
        let auth = format!("Bearer {key}");
        Self {
            api: ProviderApi::new(client, ImportSource::Fireflies, "authorization", auth),
        }
    }
}

#[async_trait]
impl Importer for FirefliesImporter {
    fn source(&self) -> ImportSource {
        ImportSource::Fireflies
    }

    async fn page(&self, cursor: Option<&str>) -> Result<SourcePage, ImporterError> {
        let skip: u32 = cursor.and_then(|skip| skip.parse().ok()).unwrap_or(0);
        let body = json!({ "query": QUERY, "variables": { "limit": PAGE_SIZE, "skip": skip } });
        let answer: Answer = self.api.post(GRAPHQL_URL, &body).await?;
        let transcripts = match (answer.data, answer.errors) {
            (Some(data), _) => data.transcripts,
            (None, errors) => return Err(graphql_error(&errors)),
        };
        let full = transcripts.len() >= PAGE_SIZE as usize;
        Ok(SourcePage {
            items: transcripts
                .into_iter()
                .map(|transcript| {
                    let id = mapping::id_text(&transcript.id);
                    SourceItem::complete(id, transcript.into_import())
                })
                .collect(),
            next: full.then(|| (skip + PAGE_SIZE).to_string()),
        })
    }
}

/// A GraphQL error as an import error.
fn graphql_error(errors: &[GraphQlError]) -> ImporterError {
    let auth = errors.iter().any(|error| {
        let code = error
            .extensions
            .as_ref()
            .and_then(|ext| ext.code.as_deref());
        code.is_some_and(|code| code.contains("auth") || code.contains("forbidden"))
            || error.message.to_lowercase().contains("api key")
    });
    if auth {
        ImporterError::Unauthorized(ImportSource::Fireflies)
    } else {
        ImporterError::Unreadable(ImportSource::Fireflies, "a GraphQL error")
    }
}

#[derive(Deserialize)]
struct Answer {
    data: Option<Data>,
    #[serde(default)]
    errors: Vec<GraphQlError>,
}

#[derive(Deserialize)]
struct Data {
    transcripts: Vec<Transcript>,
}

#[derive(Deserialize)]
struct GraphQlError {
    message: String,
    extensions: Option<Extensions>,
}

#[derive(Deserialize)]
struct Extensions {
    code: Option<String>,
}

#[derive(Deserialize)]
struct Transcript {
    id: serde_json::Value,
    title: Option<String>,
    /// Milliseconds since the Unix epoch.
    date: Option<f64>,
    organizer_email: Option<String>,
    #[serde(default)]
    meeting_attendees: Option<Vec<Attendee>>,
    #[serde(default)]
    sentences: Option<Vec<Sentence>>,
    summary: Option<Summary>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Attendee {
    display_name: Option<String>,
    email: Option<String>,
    name: Option<String>,
}

#[derive(Deserialize)]
struct Sentence {
    speaker_name: Option<String>,
    text: String,
    start_time: f64,
    end_time: Option<f64>,
}

#[derive(Deserialize)]
struct Summary {
    overview: Option<String>,
    shorthand_bullet: Option<String>,
    action_items: Option<String>,
}

impl Transcript {
    fn into_import(self) -> TranscriptImport {
        let summary = self.summary.unwrap_or(Summary {
            overview: None,
            shorthand_bullet: None,
            action_items: None,
        });
        let organiser = self.organizer_email.unwrap_or_default().to_lowercase();
        Parts {
            title: self.title,
            created_at: self.date.and_then(mapping::from_epoch_millis),
            notes: notes(&summary),
            transcript: self
                .sentences
                .unwrap_or_default()
                .iter()
                .filter_map(|line| {
                    mapping::utterance(
                        line.speaker_name.as_deref(),
                        &line.text,
                        line.start_time,
                        line.end_time,
                    )
                })
                .collect(),
            attendees: self
                .meeting_attendees
                .unwrap_or_default()
                .iter()
                .filter_map(|attendee| attendee.import(&organiser))
                .collect(),
            action_items: action_items(summary.action_items.as_deref().unwrap_or_default()),
        }
        .into_import()
    }
}

impl Attendee {
    fn import(&self, organiser: &str) -> Option<ImportAttendee> {
        let name = self.display_name.as_deref().or(self.name.as_deref());
        let mut attendee = mapping::attendee(name, self.email.as_deref())?;
        attendee.is_calendar_event_organizer = attendee
            .email
            .as_deref()
            .is_some_and(|email| email.eq_ignore_ascii_case(organiser));
        Some(attendee)
    }
}

/// The overview and the bullet notes as Markdown.
fn notes(summary: &Summary) -> Option<String> {
    let sections: Vec<String> = [
        ("Overview", summary.overview.as_deref()),
        ("Notes", summary.shorthand_bullet.as_deref()),
    ]
    .into_iter()
    .filter_map(|(heading, text)| {
        let text = text?.trim();
        (!text.is_empty()).then(|| format!("## {heading}\n\n{text}"))
    })
    .collect();
    (!sections.is_empty()).then(|| sections.join("\n\n"))
}

/// Fireflies' action items text: one item per line, grouped under
/// `**Name**` lines naming who owns them.
fn action_items(text: &str) -> Vec<ImportActionItem> {
    let mut owner: Option<String> = None;
    let mut items = Vec::new();
    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        if let Some(name) = line
            .strip_prefix("**")
            .and_then(|rest| rest.strip_suffix("**"))
        {
            owner = Some(name.trim().to_owned()).filter(|name| !name.is_empty());
            continue;
        }
        let title = line.trim_start_matches(['-', '*', '•', ' ']).trim();
        items.push(ImportActionItem {
            title: title.to_owned(),
            assignee: owner
                .as_deref()
                .and_then(|name| mapping::attendee(Some(name), None)),
            ..ImportActionItem::default()
        });
    }
    items
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_items_take_their_owner_from_the_heading_above() {
        let items =
            action_items("**Dana Lee**\nSend the deck (04:12)\n\n**Raj**\n- Book the venue\n");
        let owners: Vec<Option<String>> = items
            .iter()
            .map(|item| item.assignee.as_ref().and_then(|a| a.name.clone()))
            .collect();
        assert_eq!(items[1].title, "Book the venue");
        assert_eq!(owners, vec![Some("Dana Lee".into()), Some("Raj".into())]);
    }
}
