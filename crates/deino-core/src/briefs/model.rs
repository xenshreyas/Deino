//! What a brief is: its cited bullets as the UI reads them, and its stored
//! row.

use chrono::{DateTime, Utc};
use deino_domain::{ActionItemId, CalendarEventId, CompanyId, MeetingId, PersonId, SegmentId};
use serde::{Deserialize, Serialize};

/// What a bullet rests on. Labels are stored with the IDs, so a citation of
/// something deleted later still reads sensibly.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BriefCitation {
    /// An earlier meeting.
    #[serde(rename_all = "camelCase")]
    Meeting {
        /// The meeting.
        meeting_id: MeetingId,
        /// Its title.
        title: String,
        /// When it took place.
        at: DateTime<Utc>,
    },
    /// A moment of an earlier meeting: where an open action item was agreed.
    #[serde(rename_all = "camelCase")]
    Moment {
        /// The meeting.
        meeting_id: MeetingId,
        /// The transcript line.
        segment_id: SegmentId,
        /// The action item's title.
        label: String,
    },
    /// An open action item without a meeting.
    #[serde(rename_all = "camelCase")]
    ActionItem {
        /// The item.
        action_item_id: ActionItemId,
        /// Its title.
        label: String,
    },
    /// What the user knows about a person: their summary.
    #[serde(rename_all = "camelCase")]
    Person {
        /// The person.
        person_id: PersonId,
        /// Their name.
        name: String,
    },
    /// What the user knows about a company: its summary.
    #[serde(rename_all = "camelCase")]
    Company {
        /// The company.
        company_id: CompanyId,
        /// Its name.
        name: String,
    },
    /// The calendar invitation's description.
    Event,
}

/// One sentence of a brief and what it rests on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BriefBullet {
    /// The sentence.
    pub text: String,
    /// At least one citation.
    pub citations: Vec<BriefCitation>,
}

/// The user's verdict on a brief.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum BriefFeedback {
    /// The thumbs down.
    NotUseful,
}

/// Whether a generation produced a brief.
#[derive(Clone, Copy, Debug, PartialEq, Eq, sqlx::Type)]
#[sqlx(rename_all = "snake_case")]
pub enum BriefStatus {
    /// Bullets to show.
    Ready,
    /// Too little to say: no brief is shown.
    Insufficient,
}

/// A brief ready to show: what [`BriefService::for_event`](super::BriefService::for_event)
/// returns, and what ORG-02's event note and notification show.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Brief {
    /// The upcoming event.
    pub event_id: CalendarEventId,
    /// Two to four bullets.
    pub bullets: Vec<BriefBullet>,
    /// When it was written.
    pub generated_at: DateTime<Utc>,
    /// When the user first saw it; the card collapses from then on.
    pub read_at: Option<DateTime<Utc>>,
    /// The user's verdict.
    pub feedback: Option<BriefFeedback>,
}

/// One `briefs` row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredBrief {
    /// The event.
    pub event_id: CalendarEventId,
    /// Whether there is a brief.
    pub status: BriefStatus,
    /// The SHA-256 (hex) of the prompt it was written from.
    pub inputs_hash: String,
    /// The bullets; empty unless ready.
    pub bullets: Vec<BriefBullet>,
    /// When it was written.
    pub generated_at: DateTime<Utc>,
    /// When the user first saw it.
    pub read_at: Option<DateTime<Utc>>,
    /// The user's verdict.
    pub feedback: Option<BriefFeedback>,
}

impl StoredBrief {
    /// The brief to show, unless there was too little to say.
    pub fn into_brief(self) -> Option<Brief> {
        (self.status == BriefStatus::Ready).then_some(Brief {
            event_id: self.event_id,
            bullets: self.bullets,
            generated_at: self.generated_at,
            read_at: self.read_at,
            feedback: self.feedback,
        })
    }
}

/// What one look at an event did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BriefRun {
    /// The inputs were unchanged since the last generation; nothing ran.
    Unchanged,
    /// A new brief was written.
    Written,
    /// There is too little to say, by the local check or the model's own
    /// answer; no brief is shown.
    Insufficient,
}
