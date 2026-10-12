//! [`FactsSource`]: what automations' conditions read about a meeting,
//! over the same ports as AI-06's insights (one loader, one rule engine).

use std::sync::Arc;

use deino_db::DbError;
use deino_domain::{MeetingFacts, MeetingId};

use crate::insights::{MeetingRecords, OwnDomains, PastMeeting};

/// Meetings' facts, with the audience judged against the user's domains.
#[derive(Clone)]
pub struct FactsSource {
    records: Arc<dyn MeetingRecords>,
    domains: Arc<dyn OwnDomains>,
}

impl FactsSource {
    /// The source over the stored records and the user's own domains.
    pub fn new(records: Arc<dyn MeetingRecords>, domains: Arc<dyn OwnDomains>) -> Self {
        Self { records, domains }
    }

    /// The meeting's facts; `None` when it is gone or in the trash.
    pub async fn facts(&self, meeting: &MeetingId) -> Result<Option<MeetingFacts>, DbError> {
        let Some(record) = self.records.record(meeting).await? else {
            return Ok(None);
        };
        let domains = self.domains.internal_domains().await;
        Ok(Some(record.facts(&domains)))
    }

    /// Finished meetings with a transcript, newest first.
    pub async fn past_meetings(&self) -> Result<Vec<PastMeeting>, DbError> {
        self.records.past_meetings().await
    }
}
