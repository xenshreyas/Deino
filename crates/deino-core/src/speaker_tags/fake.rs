//! An in-memory store for the speaker-tag ports.

use std::sync::Mutex;

use async_trait::async_trait;
use deino_db::DbError;
use deino_domain::{MeetingId, TagSpan};

use super::{SpeakerTagReader, SpeakerTagWriter};
use crate::testing::lock;

/// Keeps spans per meeting and honours both ports: appends accumulate and
/// reads come back in time order.
#[derive(Debug, Default)]
pub struct FakeSpeakerTagRepo {
    spans: Mutex<Vec<(MeetingId, TagSpan)>>,
    roster: Mutex<Vec<(MeetingId, TagSpan)>>,
}

/// `meeting`'s entries of `stored`, in time order.
fn of(stored: &Mutex<Vec<(MeetingId, TagSpan)>>, meeting: &MeetingId) -> Vec<TagSpan> {
    let mut spans: Vec<TagSpan> = lock(stored)
        .iter()
        .filter(|(owner, _)| owner == meeting)
        .map(|(_, span)| span.clone())
        .collect();
    spans.sort_by_key(|span| span.start);
    spans
}

#[async_trait]
impl SpeakerTagWriter for FakeSpeakerTagRepo {
    async fn append(&self, meeting: &MeetingId, spans: &[TagSpan]) -> Result<(), DbError> {
        let mut stored = lock(&self.spans);
        stored.extend(spans.iter().map(|span| (meeting.clone(), span.clone())));
        Ok(())
    }

    async fn append_roster(&self, meeting: &MeetingId, spans: &[TagSpan]) -> Result<(), DbError> {
        let mut stored = lock(&self.roster);
        stored.extend(spans.iter().map(|span| (meeting.clone(), span.clone())));
        Ok(())
    }
}

#[async_trait]
impl SpeakerTagReader for FakeSpeakerTagRepo {
    async fn spans(&self, meeting: &MeetingId) -> Result<Vec<TagSpan>, DbError> {
        Ok(of(&self.spans, meeting))
    }

    async fn roster(&self, meeting: &MeetingId) -> Result<Vec<TagSpan>, DbError> {
        Ok(of(&self.roster, meeting))
    }
}
