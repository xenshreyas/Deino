//! The read side of the library (DEV-02/04, ADR 0035): what the `deino`
//! CLI and its MCP server answer from, over the same repositories and
//! services the app uses, so a meeting reads the same everywhere.
//!
//! | Type | Answers |
//! |---|---|
//! | [`MeetingQueries`] | meetings by date, tag, folder, person, company; one meeting in full |
//! | [`TranscriptQueries`] | a transcript, a time window at a time |
//! | [`ActionItemQueries`] | action items by status, assignee, due date, meeting |
//! | [`DirectoryQueries`] | people, companies, tags, folders, calendar events |
//! | [`Search`] | the app's search ([`HybridSearch`]): keywords, and meaning when the embedding model answers |
//!
//! [`Library`] bundles them over one [`ReadPorts`]. [`Resolver`] turns the
//! names a person or an LLM types into IDs. Nothing here writes: the CLI
//! opens the database with `Database::open_read_only`. Links are
//! `deino://meeting/<id>[?t=<ms>]` ([`link`]).

mod action_items;
mod citations;
mod directory;
mod directory_model;
mod error;
mod hybrid_search;
pub mod link;
mod meetings;
mod model;
mod ports;
mod resolve;
#[cfg(any(test, feature = "test-support"))]
mod sample;
#[cfg(any(test, feature = "test-support"))]
mod sample_notes;
mod search;
#[cfg(test)]
mod testing;
mod transcript;

pub use action_items::{ActionItemQueries, Assignee, DEFAULT_ITEMS, ItemFilter, MAX_ITEMS};
pub use directory::{DEFAULT_PEOPLE, DirectoryQueries, MAX_EVENT_DAYS, MAX_PEOPLE};
pub use directory_model::{CompanyView, EventView, EventWindow, FolderView, PersonView, TagView};
pub use error::{EntityKind, QueryError};
pub use hybrid_search::{EmbedProbe, HybridSearch};
pub use meetings::{DEFAULT_MEETINGS, MAX_MEETINGS, MeetingFilter, MeetingQueries};
pub use model::{
    ActionItemSummary, AttendeeView, AudioFiles, CitationLink, MeetingDetail, MeetingSummary,
    TranscriptLine, TranscriptPage,
};
pub use ports::{ReadContext, ReadPorts};
pub use resolve::Resolver;
#[cfg(any(test, feature = "test-support"))]
pub use sample::{SAMPLE_NOW, SampleError, seed_sample_library};
pub use search::{
    DEFAULT_HITS, HitKind, MAX_HITS, Search, SearchHit, SearchMode, SearchOutcome, SearchQuery,
};
pub use transcript::{DEFAULT_LINES, MAX_LINES, TranscriptQueries, TranscriptWindow};

use std::sync::Arc;

use deino_db::Database;

/// Every read the CLI and the MCP server offer, over one set of ports.
pub struct Library {
    /// Meetings.
    pub meetings: MeetingQueries,
    /// Transcripts.
    pub transcripts: TranscriptQueries,
    /// Action items.
    pub action_items: ActionItemQueries,
    /// People, companies, tags, folders and calendar events.
    pub directory: DirectoryQueries,
    /// Text search.
    pub search: Arc<dyn Search>,
}

impl Library {
    /// The library over `ports`.
    pub fn new(ports: &ReadPorts) -> Self {
        Self {
            meetings: MeetingQueries::new(ports.clone()),
            transcripts: TranscriptQueries::new(ports.clone()),
            action_items: ActionItemQueries::new(ports.clone()),
            directory: DirectoryQueries::new(ports.clone()),
            search: ports.search.clone(),
        }
    }

    /// The library over the SQLite repositories of `database`.
    pub fn sqlite(database: &Database, context: &ReadContext) -> Self {
        Self::new(&ReadPorts::sqlite(database, context))
    }
}
