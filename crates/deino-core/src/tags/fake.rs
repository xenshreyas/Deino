//! An in-memory store for the tag ports.

use std::sync::Mutex;

use async_trait::async_trait;
use deino_db::DbError;
use deino_domain::{MeetingId, TagId};

use super::{AppliedTag, MeetingTagRepo, Tag, TagApplication, TagRepo};
use crate::meetings::Meeting;
use crate::testing::lock;

/// Keeps tags and applications in `Vec`s and honours the [`TagRepo`] and
/// [`MeetingTagRepo`] contracts: case-insensitive names, first applier kept,
/// orderings. `meetings_tagged` answers from the meetings given to
/// [`FakeTagRepo::with_meetings`], all of them outside the trash.
#[derive(Debug, Default)]
pub struct FakeTagRepo {
    tags: Mutex<Vec<Tag>>,
    applied: Mutex<Vec<TagApplication>>,
    meetings: Vec<Meeting>,
}

impl FakeTagRepo {
    /// A repository that knows `meetings`, for `meetings_tagged`.
    pub fn with_meetings(meetings: impl IntoIterator<Item = Meeting>) -> Self {
        Self {
            meetings: meetings.into_iter().collect(),
            ..Self::default()
        }
    }
}

#[async_trait]
impl TagRepo for FakeTagRepo {
    async fn create_tag(&self, tag: &Tag) -> Result<(), DbError> {
        lock(&self.tags).push(tag.clone());
        Ok(())
    }

    async fn tag(&self, id: &TagId) -> Result<Option<Tag>, DbError> {
        Ok(lock(&self.tags).iter().find(|tag| tag.id == *id).cloned())
    }

    async fn tag_by_name(&self, name: &str) -> Result<Option<Tag>, DbError> {
        let tags = lock(&self.tags);
        Ok(tags
            .iter()
            .find(|tag| tag.name.eq_ignore_ascii_case(name))
            .cloned())
    }

    async fn tags(&self) -> Result<Vec<Tag>, DbError> {
        let mut tags = lock(&self.tags).clone();
        tags.sort_by_key(|tag| tag.name.to_ascii_lowercase());
        Ok(tags)
    }

    async fn update_tag(&self, tag: &Tag) -> Result<bool, DbError> {
        let mut tags = lock(&self.tags);
        let stored = tags.iter_mut().find(|stored| stored.id == tag.id);
        Ok(stored.map(|stored| *stored = tag.clone()).is_some())
    }

    async fn delete_tag(&self, id: &TagId) -> Result<bool, DbError> {
        let mut tags = lock(&self.tags);
        let before = tags.len();
        tags.retain(|tag| tag.id != *id);
        lock(&self.applied).retain(|application| application.tag_id != *id);
        Ok(tags.len() < before)
    }
}

#[async_trait]
impl MeetingTagRepo for FakeTagRepo {
    async fn apply_tag(&self, application: &TagApplication) -> Result<(), DbError> {
        let mut applied = lock(&self.applied);
        let exists = applied.iter().any(|stored| {
            stored.meeting_id == application.meeting_id && stored.tag_id == application.tag_id
        });
        if !exists {
            applied.push(application.clone());
        }
        Ok(())
    }

    async fn remove_tag(&self, meeting: &MeetingId, tag: &TagId) -> Result<bool, DbError> {
        let mut applied = lock(&self.applied);
        let before = applied.len();
        applied.retain(|stored| !(stored.meeting_id == *meeting && stored.tag_id == *tag));
        Ok(applied.len() < before)
    }

    async fn tags_of(&self, meeting: &MeetingId) -> Result<Vec<AppliedTag>, DbError> {
        let tags = self.tags().await?;
        let applied = lock(&self.applied);
        let on_meeting = |tag: Tag| {
            let found = applied
                .iter()
                .find(|a| a.meeting_id == *meeting && a.tag_id == tag.id)?;
            Some(AppliedTag {
                applied_by: found.applied_by,
                applied_at: found.applied_at,
                tag,
            })
        };
        Ok(tags.into_iter().filter_map(on_meeting).collect())
    }

    async fn meetings_tagged(&self, tag: &TagId) -> Result<Vec<Meeting>, DbError> {
        let applied = lock(&self.applied);
        let tagged = |meeting: &&Meeting| {
            applied
                .iter()
                .any(|a| a.meeting_id == meeting.id && a.tag_id == *tag)
        };
        let mut meetings: Vec<Meeting> = self.meetings.iter().filter(tagged).cloned().collect();
        meetings.sort_by(|a, b| (&b.created_at, &b.id).cmp(&(&a.created_at, &a.id)));
        Ok(meetings)
    }
}
