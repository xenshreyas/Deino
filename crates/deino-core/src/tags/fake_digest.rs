//! The digest half of `FakeTagRepo`.

use async_trait::async_trait;
use deino_db::DbError;
use deino_domain::{MeetingId, TagId};

use super::digest_repo::opening;
use super::{FakeTagRepo, MeetingDigest, MeetingDigestReader, MeetingTagRepo};
use crate::meetings::Meeting;
use crate::testing::lock;

impl FakeTagRepo {
    fn digest_of(&self, meeting: &Meeting) -> MeetingDigest {
        MeetingDigest {
            meeting_id: meeting.id.clone(),
            title: meeting.title.clone(),
            summary: lock(&self.summaries).get(&meeting.id).cloned(),
        }
    }
}

#[async_trait]
impl MeetingDigestReader for FakeTagRepo {
    async fn digest(&self, meeting: &MeetingId) -> Result<Option<MeetingDigest>, DbError> {
        let known = self.meetings.iter().find(|known| known.id == *meeting);
        Ok(known.map(|known| self.digest_of(known)))
    }

    async fn transcript_opening(
        &self,
        meeting: &MeetingId,
        max_chars: usize,
    ) -> Result<String, DbError> {
        let transcript = lock(&self.transcripts).get(meeting).cloned();
        Ok(opening(&Vec::from_iter(transcript), max_chars))
    }

    async fn examples(
        &self,
        tag: &TagId,
        except: &MeetingId,
        limit: u32,
    ) -> Result<Vec<MeetingDigest>, DbError> {
        let tagged = self.meetings_tagged(tag).await?;
        let limit = usize::try_from(limit).unwrap_or(usize::MAX);
        Ok(tagged
            .iter()
            .filter(|meeting| meeting.id != *except)
            .take(limit)
            .map(|meeting| self.digest_of(meeting))
            .collect())
    }
}
