//! [`MeetingFacts`]: what template selection and previews know about a
//! meeting, and the read port that gathers them.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::DbError;
use deino_domain::{Email, MeetingId, TemplateId};

/// A meeting as templates see it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeetingFacts {
    /// The meeting.
    pub id: MeetingId,
    /// Its title.
    pub title: String,
    /// The template the user chose for it, if any.
    pub template_id: Option<TemplateId>,
    /// When it started, if known.
    pub started_at: Option<DateTime<Utc>>,
    /// The linked calendar event's description; empty without one.
    pub description: String,
    /// The attendees' email addresses (attendees without one are only counted).
    pub attendee_emails: Vec<Email>,
    /// How many attendees it has, the user included.
    pub attendee_count: usize,
}

/// Reads [`MeetingFacts`].
#[async_trait]
pub trait MeetingFactsReader: Send + Sync {
    /// The facts about `meeting`, or `None` when it does not exist.
    async fn facts(&self, meeting: &MeetingId) -> Result<Option<MeetingFacts>, DbError>;
}
