//! `list_meetings` / `deino meetings` and `read_meeting` /
//! `deino meetings read <ids…>`.

use chrono::NaiveDate;
use deino_core::query::{Library, MeetingDetail, MeetingFilter, MeetingSummary};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{Request, meeting_id};
use crate::CliError;

/// The most meetings one `read_meeting` call reads.
const MAX_READ: usize = 10;

/// List meetings, newest first.
#[derive(Clone, Debug, Default, Deserialize, JsonSchema, clap::Args)]
#[serde(default, deny_unknown_fields)]
pub struct ListMeetings {
    /// How many meetings to return (default 20, at most 200).
    #[arg(long, short = 'n')]
    pub limit: Option<u32>,
    /// Only meetings on or after this local day (YYYY-MM-DD).
    #[arg(long)]
    pub from: Option<NaiveDate>,
    /// Only meetings on or before this local day (YYYY-MM-DD).
    #[arg(long)]
    pub to: Option<NaiveDate>,
    /// Only meetings with this tag (name or ID).
    #[arg(long)]
    pub tag: Option<String>,
    /// Only meetings in this folder or its subfolders (name or ID).
    #[arg(long)]
    pub folder: Option<String>,
    /// Only meetings this person attended (name, email or ID).
    #[arg(long)]
    pub person: Option<String>,
    /// Only meetings someone from this company attended (name, domain or ID).
    #[arg(long)]
    pub company: Option<String>,
}

/// Meetings, newest first.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct MeetingList {
    /// The meetings that matched.
    pub meetings: Vec<MeetingSummary>,
}

impl Request for ListMeetings {
    const NAME: &'static str = "list_meetings";
    const TITLE: &'static str = "List meetings";
    const DESCRIPTION: &'static str = "List the user's recorded meetings, newest first, with \
        their ID, title, date, duration, attendees and tags. Use it to find meetings by date \
        range (from/to, YYYY-MM-DD, inclusive, the user's local days), tag, folder, attendee \
        (person) or company, or just the most recent ones (limit). It returns no notes or \
        transcript: pass the IDs to read_meeting for notes and action items, or to \
        get_transcript for what was said. To find meetings by what was discussed, use search.";
    type Output = MeetingList;

    async fn run(self, library: &Library) -> Result<MeetingList, CliError> {
        let filter = MeetingFilter {
            limit: self.limit.unwrap_or_default(),
            from: self.from,
            to: self.to,
            tag: self.tag,
            folder: self.folder,
            person: self.person,
            company: self.company,
        };
        let meetings = library.meetings.list(&filter).await?;
        Ok(MeetingList { meetings })
    }
}

/// Read meetings in full: notes, AI notes, action items, attendees, files.
#[derive(Clone, Debug, Deserialize, JsonSchema, clap::Args)]
#[serde(deny_unknown_fields)]
pub struct ReadMeetings {
    /// The meeting IDs to read (from `list_meetings` or `search`), at most 10.
    #[arg(required = true, num_args = 1..)]
    pub ids: Vec<String>,
}

/// Meetings in full.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct MeetingDetails {
    /// The meetings, in the order asked for.
    pub meetings: Vec<MeetingDetail>,
}

impl Request for ReadMeetings {
    const NAME: &'static str = "read_meeting";
    const TITLE: &'static str = "Read meetings";
    const DESCRIPTION: &'static str = "Read one or more meetings in full by ID (at most 10 \
        per call): title, date, duration, attendees with emails, tags, folders, the user's own \
        notes and the AI notes (both Markdown), action items with owners, due days and links \
        to the moment they were said, and the recording's file paths. It does not include the \
        transcript; call get_transcript for that. Get IDs from list_meetings or search.";
    type Output = MeetingDetails;

    async fn run(self, library: &Library) -> Result<MeetingDetails, CliError> {
        if self.ids.is_empty() || self.ids.len() > MAX_READ {
            return Err(CliError::Invalid(format!(
                "pass between 1 and {MAX_READ} meeting IDs"
            )));
        }
        let mut meetings = Vec::with_capacity(self.ids.len());
        for id in &self.ids {
            meetings.push(library.meetings.read(&meeting_id(id)?).await?);
        }
        Ok(MeetingDetails { meetings })
    }
}
