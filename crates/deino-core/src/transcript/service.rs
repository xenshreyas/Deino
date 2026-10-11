//! The transcript service: one read that the transcript view renders.

use std::sync::Arc;

use deino_db::DbError;
use deino_domain::{ErrorClass, ErrorCode, MeetingId};

use super::Transcript;
use crate::segments::{Page, SegmentReader};

/// A transcript failure.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum TranscriptError {
    /// The database failed; the source carries the detail.
    #[error("could not load the transcript")]
    Storage(#[from] DbError),
}

impl ErrorClass for TranscriptError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::Storage(_) => ErrorCode::Internal,
        }
    }
}

/// Reads meeting transcripts for the UI.
pub struct TranscriptService {
    segments: Arc<dyn SegmentReader>,
}

impl TranscriptService {
    /// Creates the service over the segment port.
    pub fn new(segments: Arc<dyn SegmentReader>) -> Self {
        Self { segments }
    }

    /// The saved transcript of `meeting`, in spoken order, deleted segments
    /// left out. A meeting without a transcript (or an unknown one) has no
    /// segments. The whole meeting is read at once: a two-hour meeting is a
    /// few thousand rows, which the view virtualises and searches in memory
    /// (ADR 0030).
    pub async fn transcript(&self, meeting: &MeetingId) -> Result<Transcript, TranscriptError> {
        let segments = self.segments.list(meeting, Page::ALL).await?;
        Ok(Transcript { segments })
    }
}
