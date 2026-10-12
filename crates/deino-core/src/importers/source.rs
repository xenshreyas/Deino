//! The [`Importer`] port: one adapter per note-taker (Open/Closed). The
//! runner pages through a source's meetings, skips the ones the ledger
//! already has, and fetches the rest one at a time.

use async_trait::async_trait;
use deino_domain::ImportSource;

use super::ImporterError;
use crate::transcript_import::TranscriptImport;

/// One meeting as a source lists it.
#[derive(Clone, Debug, PartialEq)]
pub struct SourceItem {
    /// Its ID in the source: the de-duplication key.
    pub id: String,
    /// The whole meeting, when the list already carries it (Fireflies,
    /// Fathom, folders); `None` when [`Importer::fetch`] must ask for it.
    pub meeting: Option<Box<TranscriptImport>>,
}

impl SourceItem {
    /// An item whose meeting is fetched later.
    pub fn listed(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            meeting: None,
        }
    }

    /// An item that carries its whole meeting.
    pub fn complete(id: impl Into<String>, meeting: TranscriptImport) -> Self {
        Self {
            id: id.into(),
            meeting: Some(Box::new(meeting)),
        }
    }
}

/// One page of a source's meetings.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SourcePage {
    /// The meetings on this page.
    pub items: Vec<SourceItem>,
    /// The cursor of the next page; `None` on the last.
    pub next: Option<String>,
}

/// A note-taker (or folder) Deino imports from.
#[async_trait]
pub trait Importer: Send + Sync {
    /// Which source this is.
    fn source(&self) -> ImportSource;

    /// The page at `cursor` (the first page for `None`).
    async fn page(&self, cursor: Option<&str>) -> Result<SourcePage, ImporterError>;

    /// The whole meeting of `item`. The default returns what the list
    /// carried; adapters whose lists are summaries override it.
    async fn fetch(&self, item: SourceItem) -> Result<TranscriptImport, ImporterError> {
        item.meeting.map(|meeting| *meeting).ok_or(ImporterError::Unreadable(
            self.source(),
            "a meeting without details",
        ))
    }
}
