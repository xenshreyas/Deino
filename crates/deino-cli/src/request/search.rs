//! `search` / `deino search <query>`.

use deino_core::query::{Library, SearchHit, SearchMode, SearchQuery};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{Request, meeting_id};
use crate::CliError;

/// Search meetings, notes, transcripts, action items and people.
#[derive(Clone, Debug, Deserialize, JsonSchema, clap::Args)]
#[serde(deny_unknown_fields)]
pub struct SearchLibrary {
    /// What to find: words, "exact phrases", @person or @tag filters, or a
    /// company's domain (as in Deino's own search).
    #[arg(required = true, num_args = 1.., value_name = "QUERY")]
    #[serde(deserialize_with = "one_or_many")]
    #[schemars(with = "String")]
    pub query: Vec<String>,
    /// Only search this meeting.
    #[arg(long)]
    #[serde(default)]
    pub meeting_id: Option<String>,
    /// The most hits (default 20, at most 100).
    #[arg(long, short = 'n')]
    #[serde(default)]
    pub limit: Option<u32>,
}

/// Search results.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct SearchResults {
    /// What was searched for.
    pub query: String,
    /// `hybrid` when the local embedding model also matched by meaning,
    /// `keyword` when only words were matched.
    pub mode: SearchMode,
    /// The best hits first.
    pub hits: Vec<SearchHit>,
}

impl Request for SearchLibrary {
    const NAME: &'static str = "search";
    const TITLE: &'static str = "Search meetings";
    const DESCRIPTION: &'static str = "Find where something was discussed across all \
        meetings, with Deino's own search: titles, the user's notes, AI notes, action items, \
        transcript moments, people, companies and folders, best first. It matches words \
        (\"phrases\" in quotes, @name to narrow to a person, tag or folder, a domain for a \
        company) and, when the local embedding model is installed, meaning too; `mode` says \
        which ran (try other words if a keyword search finds nothing). Transcript hits carry \
        atMs and a deino://meeting/<id>?t=<ms> link: read around them with get_transcript \
        (from_ms a minute before atMs) or read the meeting with read_meeting. meeting_id limits \
        the search to one meeting.";
    type Output = SearchResults;

    async fn run(self, library: &Library) -> Result<SearchResults, CliError> {
        let text = self.query.join(" ");
        let query = SearchQuery {
            text: text.clone(),
            meeting: self.meeting_id.as_deref().map(meeting_id).transpose()?,
            limit: self.limit.unwrap_or_default(),
        };
        let outcome = library.search.search(&query).await?;
        Ok(SearchResults {
            query: text,
            mode: outcome.mode,
            hits: outcome.hits,
        })
    }
}

/// The query as one string (a tool call) or several words (the terminal).
fn one_or_many<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Text {
        One(String),
        Many(Vec<String>),
    }
    Ok(match Text::deserialize(deserializer)? {
        Text::One(text) => vec![text],
        Text::Many(words) => words,
    })
}
