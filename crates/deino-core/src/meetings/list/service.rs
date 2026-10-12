//! [`MeetingListService`]: pages of the Meetings list and "every match"
//! for ⌘A, over the list port, the time zone and the user's addresses.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use deino_domain::{LocalZone, MeetingId};

use super::{
    ListQuery, ListedAttendee, ListedTag, MAX_PAGE_SIZE, MeetingCursor, MeetingFilter,
    MeetingListRepo, MeetingPage, MeetingRow, RowLimit,
};
use crate::meetings::MeetingError;
use crate::profile::InternalDomains;
use crate::settings::SettingsService;

/// The user's own addresses (Settings → Profile), which tell internal
/// meetings from external ones.
#[async_trait]
pub trait OwnAddresses: Send + Sync {
    /// The organisation the user's addresses describe.
    async fn internal_domains(&self) -> InternalDomains;
}

#[async_trait]
impl OwnAddresses for SettingsService {
    async fn internal_domains(&self) -> InternalDomains {
        InternalDomains::of(&self.get().await.profile)
    }
}

/// What the list service reads through.
pub struct MeetingListDeps {
    /// The list queries.
    pub repo: Arc<dyn MeetingListRepo>,
    /// The user's time zone, for date ranges.
    pub zone: Arc<dyn LocalZone>,
    /// The user's addresses, for the audience filter.
    pub addresses: Arc<dyn OwnAddresses>,
}

/// Lists meetings for the Meetings page.
pub struct MeetingListService {
    deps: MeetingListDeps,
}

impl MeetingListService {
    /// The service over its ports.
    pub fn new(deps: MeetingListDeps) -> Self {
        Self { deps }
    }

    /// Up to `size` meetings `filter` matches, newest first, after `after`
    /// (the first page when `None`). `size` is clamped to 1..=[`MAX_PAGE_SIZE`].
    pub async fn page(
        &self,
        filter: &MeetingFilter,
        after: Option<&MeetingCursor>,
        size: u32,
    ) -> Result<MeetingPage, MeetingError> {
        let query = self.resolve(filter).await?;
        let size = size.clamp(1, MAX_PAGE_SIZE);
        let limit = RowLimit::At(size + 1);
        let mut rows = self.deps.repo.page(&query, after, limit).await?;
        let more = rows.len() > usize::try_from(size).unwrap_or(usize::MAX);
        rows.truncate(usize::try_from(size).unwrap_or(usize::MAX));
        let next = more.then(|| rows.last().map(MeetingRow::cursor)).flatten();
        Ok(MeetingPage {
            meetings: self.dress(rows).await?,
            next,
        })
    }

    /// Every meeting `filter` matches, newest first: what ⌘A selects,
    /// including rows not loaded yet (Circleback §14).
    pub async fn matching_ids(
        &self,
        filter: &MeetingFilter,
    ) -> Result<Vec<MeetingId>, MeetingError> {
        let query = self.resolve(filter).await?;
        let rows = self.deps.repo.page(&query, None, RowLimit::All).await?;
        Ok(rows.into_iter().map(|row| row.id).collect())
    }

    /// `filter` as the repository's query; the audience condition needs at
    /// least one of the user's addresses.
    async fn resolve(&self, filter: &MeetingFilter) -> Result<ListQuery, MeetingError> {
        let domains = self.deps.addresses.internal_domains().await;
        if filter.audience.is_some() && domains.own().next().is_none() {
            return Err(MeetingError::AudienceUnknown);
        }
        Ok(ListQuery::resolve(
            filter,
            self.deps.zone.as_ref(),
            &domains,
        ))
    }

    /// Attaches each row's avatars and tag chips.
    async fn dress(
        &self,
        rows: Vec<MeetingRow>,
    ) -> Result<Vec<super::ListedMeeting>, MeetingError> {
        let ids: Vec<MeetingId> = rows.iter().map(|row| row.id.clone()).collect();
        let mut attendees: HashMap<MeetingId, Vec<ListedAttendee>> = HashMap::new();
        for row in self.deps.repo.attendees(&ids).await? {
            attendees
                .entry(row.meeting_id)
                .or_default()
                .push(row.attendee);
        }
        let mut tags: HashMap<MeetingId, Vec<ListedTag>> = HashMap::new();
        for row in self.deps.repo.tags(&ids).await? {
            tags.entry(row.meeting_id).or_default().push(row.tag);
        }
        Ok(rows
            .into_iter()
            .map(|row| {
                let (people, chips) = (attendees.remove(&row.id), tags.remove(&row.id));
                row.listed(people.unwrap_or_default(), chips.unwrap_or_default())
            })
            .collect())
    }
}
