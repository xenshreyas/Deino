//! What a suggested follow-up is: the draft the editor shows, Home's
//! summary of it, the user's edit, and the stored row.

use chrono::{DateTime, Utc};
use deino_domain::{Email, MeetingId};
use deino_llm::followup::FollowUpVerdict;
use serde::{Deserialize, Serialize};

/// Who a draft goes to. Pre-filled with the external attendees in To.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Recipients {
    /// The To addresses.
    pub to: Vec<Email>,
    /// The CC addresses.
    pub cc: Vec<Email>,
    /// The BCC addresses.
    pub bcc: Vec<Email>,
}

/// A ready draft, as the editor shows it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FollowUpDraft {
    /// The meeting it follows up.
    pub meeting_id: MeetingId,
    /// The subject line.
    pub subject: String,
    /// The plain-text body, with `[placeholders]` to fill.
    pub body: String,
    /// Who it goes to.
    pub recipients: Recipients,
    /// When the model wrote it.
    pub generated_at: DateTime<Utc>,
    /// When the user last changed it.
    pub edited_at: Option<DateTime<Utc>>,
}

/// One entry of Home's "Suggested follow-ups".
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FollowUpSummary {
    /// The meeting it follows up.
    pub meeting_id: MeetingId,
    /// The meeting's title.
    pub meeting_title: String,
    /// The draft's subject line.
    pub subject: String,
    /// When the model wrote it; Home shows it for three days from then.
    pub generated_at: DateTime<Utc>,
}

/// The editor's fields, as the user left them. Addresses are checked by
/// [`FollowUpService::save`](super::FollowUpService::save).
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, specta::Type)]
pub struct FollowUpEdit {
    /// The subject line.
    pub subject: String,
    /// The body.
    pub body: String,
    /// The To addresses, as typed.
    pub to: Vec<String>,
    /// The CC addresses, as typed.
    pub cc: Vec<String>,
    /// The BCC addresses, as typed.
    pub bcc: Vec<String>,
}

/// Where a meeting's follow-up stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, sqlx::Type)]
#[sqlx(rename_all = "snake_case")]
pub enum FollowUpStatus {
    /// A draft to show.
    Ready,
    /// The model decided no follow-up was needed.
    Quiet,
    /// The user dismissed it; it never comes back.
    Dismissed,
}

/// Why the model stayed quiet.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, specta::Type, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum QuietReason {
    /// A colleague owns the follow-up.
    ColleagueOwns,
    /// The conversation moved to Slack or another chat.
    MovedToChat,
    /// A blocker comes before any next step.
    BlockerFirst,
    /// Nothing was agreed that an email would move forward.
    NothingToFollowUp,
}

impl QuietReason {
    /// Why the model stayed quiet, or `None` when it said to send.
    pub fn of(verdict: FollowUpVerdict) -> Option<Self> {
        match verdict {
            FollowUpVerdict::Send => None,
            FollowUpVerdict::ColleagueOwns => Some(Self::ColleagueOwns),
            FollowUpVerdict::MovedToChat => Some(Self::MovedToChat),
            FollowUpVerdict::BlockerFirst => Some(Self::BlockerFirst),
            FollowUpVerdict::NothingToFollowUp => Some(Self::NothingToFollowUp),
        }
    }

    /// The pipeline's skip reason, shown in the stage's status.
    pub fn reason(self) -> &'static str {
        match self {
            Self::ColleagueOwns => "A colleague owns the follow-up.",
            Self::MovedToChat => "The conversation moved to chat.",
            Self::BlockerFirst => "A blocker comes before any next step.",
            Self::NothingToFollowUp => "Nothing to follow up on by email.",
        }
    }
}

/// One `follow_up_drafts` row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredFollowUp {
    /// The meeting.
    pub meeting_id: MeetingId,
    /// Ready, quiet or dismissed.
    pub status: FollowUpStatus,
    /// Set exactly when the status is quiet.
    pub quiet_reason: Option<QuietReason>,
    /// The subject; empty unless ready.
    pub subject: String,
    /// The body; empty unless ready.
    pub body: String,
    /// Who it goes to.
    pub recipients: Recipients,
    /// When the stage wrote the row.
    pub generated_at: DateTime<Utc>,
    /// When the user last changed the draft.
    pub edited_at: Option<DateTime<Utc>>,
    /// When the user dismissed it.
    pub dismissed_at: Option<DateTime<Utc>>,
}

impl StoredFollowUp {
    /// The draft to show, when the row is ready.
    pub fn draft(self) -> Option<FollowUpDraft> {
        (self.status == FollowUpStatus::Ready).then_some(FollowUpDraft {
            meeting_id: self.meeting_id,
            subject: self.subject,
            body: self.body,
            recipients: self.recipients,
            generated_at: self.generated_at,
            edited_at: self.edited_at,
        })
    }
}
