//! [`OriginalTranscriptService`]: whether a meeting's transcript made on
//! this Mac was kept when the cloud replaced it (CAP-14), and bringing it
//! back. Restoring replaces the cloud transcript in one transaction, each
//! line keeping the speaker the cloud lines gave its time, and forgets the
//! kept copy.

use std::sync::Arc;

use deino_db::DbError;
use deino_domain::{Clock, MeetingId};

use super::{ChannelSpeakers, HeardSpan, KeptLine, OriginalTranscripts, carried_segment};
use crate::segments::{NewSegment, Page, SegmentReader, SegmentWriter, SpeakerRepo};

/// The ports restoring reads and writes through.
#[derive(Clone)]
pub struct OriginalTranscriptDeps {
    /// The kept transcripts.
    pub originals: Arc<dyn OriginalTranscripts>,
    /// The transcript in place.
    pub segments: Arc<dyn SegmentReader>,
    /// Where the restored transcript goes.
    pub writer: Arc<dyn SegmentWriter>,
    /// The meeting's speakers.
    pub speakers: Arc<dyn SpeakerRepo>,
    /// IDs for the restored segments.
    pub clock: Arc<dyn Clock>,
}

/// Kept transcripts: ask, and restore.
pub struct OriginalTranscriptService {
    deps: OriginalTranscriptDeps,
}

impl OriginalTranscriptService {
    /// The service over its ports.
    pub fn new(deps: OriginalTranscriptDeps) -> Self {
        Self { deps }
    }

    /// Whether `meeting` has a kept transcript to restore.
    pub async fn has_original(&self, meeting: &MeetingId) -> Result<bool, DbError> {
        Ok(self.deps.originals.kept(meeting).await?.is_some())
    }

    /// Puts the kept transcript of `meeting` back in place; `false` when
    /// none is kept.
    pub async fn restore(&self, meeting: &MeetingId) -> Result<bool, DbError> {
        let Some(lines) = self.deps.originals.kept(meeting).await? else {
            return Ok(false);
        };
        let rows = self.rows(meeting, lines).await?;
        self.deps.writer.replace(meeting, &rows).await?;
        self.deps.originals.forget(meeting).await?;
        log::info!(
            target: "deino_core::final_transcription",
            "restored the original transcript of meeting {meeting}: {} lines",
            rows.len()
        );
        Ok(true)
    }

    async fn rows(
        &self,
        meeting: &MeetingId,
        lines: Vec<KeptLine>,
    ) -> Result<Vec<NewSegment>, DbError> {
        let current = self.deps.segments.list(meeting, Page::ALL).await?;
        let fallback = ChannelSpeakers::of(&self.deps.speakers.speakers(meeting).await?);
        let clock = self.deps.clock.as_ref();
        Ok(lines
            .into_iter()
            .map(|line| {
                let span = HeardSpan {
                    start: line.start,
                    end: line.end,
                    text: line.text,
                    words: line.words,
                };
                carried_segment(meeting, (line.channel, span), (&current, fallback), clock)
            })
            .collect())
    }
}
