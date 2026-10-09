//! An in-memory store for the segment and speaker ports.

use std::sync::Mutex;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::DbError;
use deino_domain::{MeetingId, PersonId, SegmentId, SpeakerId};

use super::{
    NewSegment, NewSpeaker, Page, Reattribution, Segment, SegmentPiece, SegmentReader,
    SegmentWriter, Speaker, SpeakerRepo, SpeakerVoice,
};
use crate::testing::lock;

/// Keeps segments and speakers in `Vec`s and honours the [`SegmentReader`],
/// [`SegmentWriter`] and [`SpeakerRepo`] contracts, including time order,
/// paging and hiding deleted segments.
#[derive(Debug, Default)]
pub struct FakeSegmentRepo {
    segments: Mutex<Vec<Segment>>,
    speakers: Mutex<Vec<Speaker>>,
    voices: Mutex<Vec<(SpeakerId, SpeakerVoice)>>,
}

impl FakeSegmentRepo {
    /// Runs `change` on the segment with `id`; `false` if missing.
    fn update(&self, id: &SegmentId, change: impl FnOnce(&mut Segment)) -> bool {
        let mut segments = lock(&self.segments);
        segments
            .iter_mut()
            .find(|segment| segment.id == *id)
            .map(change)
            .is_some()
    }

    /// Forgets the speakers `remove` selects, like `ON DELETE SET NULL`.
    fn remove_speakers_where(&self, remove: impl Fn(&Speaker) -> bool) -> usize {
        let removed: Vec<SpeakerId> = {
            let mut speakers = lock(&self.speakers);
            let removed = speakers
                .iter()
                .filter(|s| remove(s))
                .map(|s| s.id)
                .collect();
            speakers.retain(|speaker| !remove(speaker));
            removed
        };
        lock(&self.voices).retain(|(id, _)| !removed.contains(id));
        for segment in lock(&self.segments).iter_mut() {
            if segment.speaker_id.is_some_and(|id| removed.contains(&id)) {
                segment.speaker_id = None;
            }
        }
        removed.len()
    }

    /// Runs `change` on the speaker with `id`; `false` if missing.
    fn update_speaker(&self, id: &SpeakerId, change: impl FnOnce(&mut Speaker)) -> bool {
        let mut speakers = lock(&self.speakers);
        speakers
            .iter_mut()
            .find(|speaker| speaker.id == *id)
            .map(change)
            .is_some()
    }
}

/// A piece split off `original`, as the repository stores it.
fn split_off(original: &Segment, id: SegmentId, piece: &SegmentPiece) -> Segment {
    Segment {
        id,
        speaker_id: piece.speaker_id,
        start: piece.start,
        end: piece.end,
        text: piece.text.clone(),
        words: piece.words.clone(),
        is_deleted: false,
        edited_at: None,
        ..original.clone()
    }
}

fn stored(segment: &NewSegment) -> Segment {
    Segment {
        id: segment.id,
        meeting_id: segment.meeting_id.clone(),
        channel: segment.channel,
        speaker_id: segment.speaker_id,
        start: segment.start,
        end: segment.end,
        text: segment.text.clone(),
        words: segment.words.clone(),
        is_deleted: false,
        edited_at: None,
    }
}

#[async_trait]
impl SegmentReader for FakeSegmentRepo {
    async fn list(&self, meeting: &MeetingId, page: Page) -> Result<Vec<Segment>, DbError> {
        let mut segments: Vec<Segment> = lock(&self.segments)
            .iter()
            .filter(|segment| segment.meeting_id == *meeting && !segment.is_deleted)
            .cloned()
            .collect();
        segments.sort_by_key(|segment| (segment.start, segment.id));
        let offset = usize::try_from(page.offset).unwrap_or(usize::MAX);
        let limit = usize::try_from(page.limit).unwrap_or(usize::MAX);
        Ok(segments.into_iter().skip(offset).take(limit).collect())
    }

    async fn count(&self, meeting: &MeetingId) -> Result<u32, DbError> {
        let all = self.list(meeting, Page::ALL).await?;
        Ok(u32::try_from(all.len()).unwrap_or(u32::MAX))
    }
}

#[async_trait]
impl SegmentWriter for FakeSegmentRepo {
    async fn append(&self, segments: &[NewSegment]) -> Result<(), DbError> {
        lock(&self.segments).extend(segments.iter().map(stored));
        Ok(())
    }

    async fn replace(&self, meeting: &MeetingId, segments: &[NewSegment]) -> Result<(), DbError> {
        let mut stored_segments = lock(&self.segments);
        stored_segments.retain(|segment| segment.meeting_id != *meeting);
        stored_segments.extend(segments.iter().map(stored));
        Ok(())
    }

    async fn edit_text(
        &self,
        id: &SegmentId,
        text: &str,
        at: DateTime<Utc>,
    ) -> Result<bool, DbError> {
        Ok(self.update(id, |segment| {
            text.clone_into(&mut segment.text);
            segment.edited_at = Some(at);
        }))
    }

    async fn set_deleted(&self, id: &SegmentId, deleted: bool) -> Result<bool, DbError> {
        Ok(self.update(id, |segment| segment.is_deleted = deleted))
    }

    async fn set_speaker(
        &self,
        id: &SegmentId,
        speaker: Option<&SpeakerId>,
    ) -> Result<bool, DbError> {
        Ok(self.update(id, |segment| segment.speaker_id = speaker.copied()))
    }

    async fn reattribute(&self, changes: &[Reattribution]) -> Result<(), DbError> {
        let mut segments = lock(&self.segments);
        for change in changes {
            let Some(segment) = segments.iter_mut().find(|segment| segment.id == change.id) else {
                continue;
            };
            let head = &change.head;
            segment.speaker_id = head.speaker_id;
            (segment.start, segment.end) = (head.start, head.end);
            segment.text.clone_from(&head.text);
            segment.words.clone_from(&head.words);
            let original = segment.clone();
            let tail = change
                .tail
                .iter()
                .map(|(id, piece)| split_off(&original, *id, piece));
            segments.extend(tail.collect::<Vec<_>>());
        }
        Ok(())
    }
}

#[async_trait]
impl SpeakerRepo for FakeSegmentRepo {
    async fn add_speaker(&self, speaker: &Speaker) -> Result<(), DbError> {
        lock(&self.speakers).push(speaker.clone());
        Ok(())
    }

    async fn speakers(&self, meeting: &MeetingId) -> Result<Vec<Speaker>, DbError> {
        let speakers = lock(&self.speakers);
        let mut listed: Vec<Speaker> = speakers
            .iter()
            .filter(|speaker| speaker.meeting_id == *meeting)
            .cloned()
            .collect();
        listed.sort_by(|a, b| a.label.cmp(&b.label));
        Ok(listed)
    }

    async fn rename_speaker(&self, id: &SpeakerId, label: &str) -> Result<bool, DbError> {
        Ok(self.update_speaker(id, |speaker| label.clone_into(&mut speaker.label)))
    }

    async fn assign_person(
        &self,
        id: &SpeakerId,
        person: Option<&PersonId>,
    ) -> Result<bool, DbError> {
        Ok(self.update_speaker(id, |speaker| speaker.person_id = person.copied()))
    }

    async fn remove_speaker(&self, id: &SpeakerId) -> Result<bool, DbError> {
        Ok(self.remove_speakers_where(|speaker| speaker.id == *id) > 0)
    }

    async fn replace_speakers(
        &self,
        meeting: &MeetingId,
        speakers: &[NewSpeaker],
    ) -> Result<(), DbError> {
        self.remove_speakers_where(|speaker| speaker.meeting_id == *meeting && !speaker.is_me);
        for new in speakers {
            lock(&self.speakers).push(new.speaker.clone());
            if let Some(voice) = &new.voice {
                lock(&self.voices).push((new.speaker.id, voice.clone()));
            }
        }
        Ok(())
    }

    async fn voice(&self, id: &SpeakerId) -> Result<Option<SpeakerVoice>, DbError> {
        let voices = lock(&self.voices);
        Ok(voices
            .iter()
            .find(|(speaker, _)| speaker == id)
            .map(|(_, voice)| voice.clone()))
    }
}
