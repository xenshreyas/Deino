//! An in-memory store for the tag ports.

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use deino_db::DbError;
use deino_domain::{MeetingId, TagId};

use super::{AppliedTag, MeetingTagLink, MeetingTagRepo, Tag, TagApplication, TagRepo};
use crate::meetings::Meeting;
use crate::testing::lock;

/// Keeps tags and applications in `Vec`s and honours the [`TagRepo`],
/// [`MeetingTagRepo`] and [`MeetingDigestReader`](super::MeetingDigestReader)
/// contracts: case-insensitive names, first applier kept, orderings.
///
/// It knows only the meetings given to [`FakeTagRepo::with_meetings`], all
/// outside the trash, with no summary or transcript until
/// [`FakeTagRepo::set_summary`] and [`FakeTagRepo::set_transcript`] add
/// them. Unlike SQLite's foreign keys, it does not refuse tags on unknown
/// meetings.
#[derive(Debug, Default)]
pub struct FakeTagRepo {
    tags: Mutex<Vec<Tag>>,
    applied: Mutex<Vec<TagApplication>>,
    pub(super) meetings: Vec<Meeting>,
    pub(super) summaries: Mutex<HashMap<MeetingId, String>>,
    pub(super) transcripts: Mutex<HashMap<MeetingId, String>>,
}

impl FakeTagRepo {
    /// A repository that knows `meetings`.
    pub fn with_meetings(meetings: impl IntoIterator<Item = Meeting>) -> Self {
        Self {
            meetings: meetings.into_iter().collect(),
            ..Self::default()
        }
    }

    /// Gives a known meeting a finished summary.
    pub fn set_summary(&self, meeting: &MeetingId, summary: &str) {
        lock(&self.summaries).insert(meeting.clone(), summary.to_owned());
    }

    /// Gives a known meeting a transcript.
    pub fn set_transcript(&self, meeting: &MeetingId, transcript: &str) {
        lock(&self.transcripts).insert(meeting.clone(), transcript.to_owned());
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
    async fn apply_tag(&self, application: &TagApplication) -> Result<bool, DbError> {
        let mut applied = lock(&self.applied);
        let exists = applied.iter().any(|stored| {
            stored.meeting_id == application.meeting_id && stored.tag_id == application.tag_id
        });
        if !exists {
            applied.push(application.clone());
        }
        Ok(!exists)
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

    async fn links(&self) -> Result<Vec<MeetingTagLink>, DbError> {
        let mut links: Vec<MeetingTagLink> = lock(&self.applied)
            .iter()
            .map(|application| MeetingTagLink {
                meeting_id: application.meeting_id.clone(),
                tag_id: application.tag_id,
                applied_by: application.applied_by,
            })
            .collect();
        links.sort_by(|a, b| (&a.meeting_id, a.tag_id).cmp(&(&b.meeting_id, b.tag_id)));
        Ok(links)
    }
}
