//! What the transcript edits (TRN-02) take and answer: find and replace
//! requests, their preview and their outcome.

use deino_domain::MeetingId;
use serde::{Deserialize, Serialize};

/// Where find and replace looks.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ReplaceScope {
    /// One meeting's transcript.
    Meeting {
        /// The meeting.
        #[serde(rename = "meetingId")]
        meeting_id: MeetingId,
    },
    /// Every meeting outside the trash.
    All,
}

impl ReplaceScope {
    /// The one meeting, or `None` for every meeting.
    pub fn meeting(&self) -> Option<&MeetingId> {
        match self {
            Self::Meeting { meeting_id } => Some(meeting_id),
            Self::All => None,
        }
    }
}

/// What to find, and where.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ReplaceRequest {
    /// The literal text to find; not blank.
    pub find: String,
    /// Where to look.
    pub scope: ReplaceScope,
    /// Compare letters' case exactly; otherwise ASCII case is ignored.
    pub match_case: bool,
}

/// One meeting's share of a find and replace.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MeetingMatches {
    /// The meeting.
    pub meeting_id: MeetingId,
    /// Its title.
    pub title: String,
    /// How many matches it has.
    pub matches: u32,
}

/// What a find and replace would change, before it does.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ReplacePreview {
    /// Every match.
    pub matches: u32,
    /// The lines holding them.
    pub segments: u32,
    /// Each meeting with matches, newest first.
    pub meetings: Vec<MeetingMatches>,
}

/// What a find and replace changed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ReplaceOutcome {
    /// Matches replaced.
    pub matches: u32,
    /// Lines changed.
    pub segments: u32,
    /// Meetings changed.
    pub meetings: u32,
}
