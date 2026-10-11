//! [`Request`]: one read, defined once and offered twice, as a `deino`
//! subcommand and as an MCP tool.
//!
//! A request is a struct that clap parses from flags, serde parses from a
//! tool call's arguments and schemars describes as the tool's input schema;
//! its doc comments are both the `--help` text and the schema's field
//! descriptions. Its [`Request::Output`] is an object that serializes as
//! the `--json` output and the tool's structured result, and renders as
//! compact [`Markdown`] for people and models.

mod directory;
mod items;
mod meetings;
mod search;
mod transcript;

use std::future::Future;

use deino_core::query::Library;
use schemars::JsonSchema;
use serde::Serialize;
use serde::de::DeserializeOwned;

pub use directory::{
    CompanyList, EventList, FolderList, ListCalendarEvents, ListCompanies, ListFolders, ListPeople,
    ListTags, PeopleList, TagList,
};
pub use items::{ActionItemList, AssigneeArg, DueArg, ListActionItems, StatusArg};
pub use meetings::{ListMeetings, MeetingDetails, MeetingList, ReadMeetings};
pub use search::{SearchLibrary, SearchResults};
pub use transcript::GetTranscript;

use crate::CliError;

/// One read the CLI and the MCP server both offer.
pub trait Request: DeserializeOwned + JsonSchema + Send + Sync + 'static {
    /// The MCP tool's name.
    const NAME: &'static str;
    /// A short human title for the tool.
    const TITLE: &'static str;
    /// What the tool does and when to use it, written for the model that
    /// picks between tools.
    const DESCRIPTION: &'static str;
    /// What it returns: always a JSON object.
    type Output: Serialize + JsonSchema + Markdown + Send + 'static;

    /// Runs the read against `library`.
    fn run(self, library: &Library) -> impl Future<Output = Result<Self::Output, CliError>> + Send;
}

/// Output as compact Markdown: what a terminal prints without `--json`,
/// and what a model reads.
pub trait Markdown {
    /// The Markdown text.
    fn markdown(&self) -> String;
}

/// Parses an ID the caller typed into a typed ID.
pub(crate) fn meeting_id(text: &str) -> Result<deino_domain::MeetingId, CliError> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(CliError::Invalid("a meeting ID cannot be empty".into()));
    }
    Ok(deino_domain::MeetingId::new(trimmed))
}
