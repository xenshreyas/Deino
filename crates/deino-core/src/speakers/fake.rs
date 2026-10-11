//! [`ParticipantReader`] over the in-memory segment store, for tests of
//! code that only reads speakers (the notes input, the pipeline's fakes).

use std::collections::HashMap;

use async_trait::async_trait;
use deino_db::DbError;
use deino_domain::{MeetingId, Millis, SpeakerId};

use super::{ParticipantReader, ParticipantRow};
use crate::segments::{FakeSegmentRepo, Page, SegmentReader, SpeakerRepo};

/// The fake knows speakers and segments but no people or suggestions: rows
/// carry no person name and no suggestion. Ordered like the SQL (most
/// speech first, then by label).
#[async_trait]
impl ParticipantReader for FakeSegmentRepo {
    async fn participants(&self, meeting: &MeetingId) -> Result<Vec<ParticipantRow>, DbError> {
        let mut speech: HashMap<SpeakerId, u32> = HashMap::new();
        for segment in self.list(meeting, Page::ALL).await? {
            if let Some(speaker) = segment.speaker_id {
                let length = segment.end.get().saturating_sub(segment.start.get());
                *speech.entry(speaker).or_default() += length;
            }
        }
        let mut rows: Vec<ParticipantRow> = self
            .speakers(meeting)
            .await?
            .into_iter()
            .map(|speaker| ParticipantRow {
                speech: Millis(speech.get(&speaker.id).copied().unwrap_or_default()),
                speaker,
                person: None,
                suggestion: None,
            })
            .collect();
        rows.sort_by(|a, b| {
            b.speech
                .cmp(&a.speech)
                .then(a.speaker.label.cmp(&b.speaker.label))
        });
        Ok(rows)
    }
}
