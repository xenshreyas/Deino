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
    /// No transcript line has the ID.
    #[error("that line is no longer in the transcript")]
    SegmentNotFound,
    /// An edit would leave the line blank.
    #[error("a line cannot be empty; delete it instead")]
    EmptyText,
    /// An edited line or a search is longer than the limit.
    #[error("that is longer than {max} characters")]
    TooLong {
        /// The limit, in characters.
        max: usize,
    },
    /// Find and replace was asked to find nothing.
    #[error("type the text to find")]
    EmptyFind,
}

impl ErrorClass for TranscriptError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::Storage(_) => ErrorCode::Internal,
            Self::SegmentNotFound => ErrorCode::NotFound,
            Self::EmptyText | Self::TooLong { .. } | Self::EmptyFind => ErrorCode::Invalid,
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
    /// (ADR 0033).
    pub async fn transcript(&self, meeting: &MeetingId) -> Result<Transcript, TranscriptError> {
        let segments = self.segments.list(meeting, Page::ALL).await?;
        Ok(Transcript { segments })
    }
}
