//! Meeting lists a page at a time (DEV-03): a [`MeetingCursor`] marks the
//! last meeting of a page in the list's own order (newest saved first, then
//! by ID), so the next page starts right after it even when meetings were
//! added or trashed in between.

use chrono::{DateTime, Utc};
use deino_domain::MeetingId;
use serde::{Deserialize, Serialize};

use super::{MeetingFilter, MeetingQueries};
use crate::meetings::Meeting;
use crate::query::{MeetingSummary, QueryError};

/// Where a meeting list continues: after this meeting.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeetingCursor {
    /// When the meeting was saved.
    pub created_at: DateTime<Utc>,
    /// The meeting's ID.
    pub id: MeetingId,
}

impl MeetingCursor {
    fn of(meeting: &Meeting) -> Self {
        Self {
            created_at: meeting.created_at,
            id: meeting.id.clone(),
        }
    }

    /// True when `meeting` comes after the cursor in the list's order.
    fn precedes(&self, meeting: &Meeting) -> bool {
        (meeting.created_at, &meeting.id) < (self.created_at, &self.id)
    }
}

/// One page of meetings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeetingPage {
    /// The meetings, newest first.
    pub meetings: Vec<MeetingSummary>,
    /// Where the next page starts; `None` on the last page.
    pub next: Option<MeetingCursor>,
}

impl MeetingQueries {
    /// The meetings `filter` matches after `after` (from the newest when
    /// `None`), at most the filter's limit, outside the trash.
    pub async fn page(
        &self,
        filter: &MeetingFilter,
        after: Option<&MeetingCursor>,
    ) -> Result<MeetingPage, QueryError> {
        if filter
            .from
            .zip(filter.to)
            .is_some_and(|(from, to)| from > to)
        {
            return Err(QueryError::InvalidRange);
        }
        let scope = self.scope(filter).await?;
        let candidates = self.ports.meeting.meetings.list().await?.into_iter();
        let mut candidates = candidates
            .filter(|meeting| after.is_none_or(|cursor| cursor.precedes(meeting)))
            .filter(|meeting| scope.as_ref().is_none_or(|ids| ids.contains(&meeting.id)))
            .peekable();
        let mut page = MeetingPage {
            meetings: Vec::new(),
            next: None,
        };
        while let Some(meeting) = candidates.next() {
            let cursor = MeetingCursor::of(&meeting);
            let state = self.state(&meeting.id).await?;
            let summary = self.summary(meeting, &state).await?;
            if filter.admits(summary.held_at.date_naive()) {
                page.meetings.push(summary);
            }
            if page.meetings.len() == filter.limit() {
                page.next = candidates.peek().map(|_| cursor);
                break;
            }
        }
        Ok(page)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::testing::library;

    #[tokio::test]
    async fn pages_join_into_the_whole_list_without_repeats() {
        let library = library().await;
        let all = library
            .meetings
            .list(&MeetingFilter::default())
            .await
            .unwrap();
        let filter = MeetingFilter {
            limit: 3,
            ..MeetingFilter::default()
        };

        let mut joined = Vec::new();
        let mut after = None;
        loop {
            let page = library
                .meetings
                .page(&filter, after.as_ref())
                .await
                .unwrap();
            assert!(page.meetings.len() <= 3);
            joined.extend(page.meetings);
            match page.next {
                Some(next) => after = Some(next),
                None => break,
            }
        }

        assert_eq!(joined, all);
        assert!(all.len() > 3, "the sample has more than one page");
    }
}
