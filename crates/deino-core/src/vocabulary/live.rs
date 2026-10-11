//! [`LiveVocabulary`]: what the recorder's live pass needs from the
//! vocabulary (TRN-05 "apply to live segments as well").
//!
//! The recorder builds one corrector when a recording starts, corrects every
//! segment it saves and shows with it, and keeps the originals through
//! [`LiveVocabulary::record`] right after each saved batch. The
//! `vocab_fixups` stage corrects again after the meeting, with the meeting's
//! invitees and any entry added since.

use async_trait::async_trait;
use deino_domain::SegmentId;

use super::{Correction, Corrector, VocabError, VocabularyService};

/// The vocabulary as the live recorder uses it.
#[async_trait]
pub trait LiveVocabulary: Send + Sync {
    /// The corrector for a new recording: the user's entries and every
    /// known name.
    async fn corrector(&self) -> Result<Corrector, VocabError>;

    /// Keeps the originals of segments saved corrected; forgets those of
    /// `uncorrected` (drafts the vocabulary no longer changes).
    async fn record(
        &self,
        corrections: &[Correction],
        uncorrected: &[SegmentId],
    ) -> Result<(), VocabError>;
}

#[async_trait]
impl LiveVocabulary for VocabularyService {
    async fn corrector(&self) -> Result<Corrector, VocabError> {
        VocabularyService::corrector(self, None).await
    }

    async fn record(
        &self,
        corrections: &[Correction],
        uncorrected: &[SegmentId],
    ) -> Result<(), VocabError> {
        self.record_live(corrections, uncorrected).await
    }
}

#[cfg(any(test, feature = "test-support"))]
pub use recorded::RecordedLiveVocabulary;

#[cfg(any(test, feature = "test-support"))]
mod recorded {
    use std::sync::Mutex;

    use async_trait::async_trait;
    use deino_domain::SegmentId;

    use super::LiveVocabulary;
    use crate::testing::lock;
    use crate::vocabulary::{Correction, Corrector, VocabError};

    /// A [`LiveVocabulary`] for tests: corrects with a fixed corrector and
    /// records what it is asked to keep and forget, in order.
    #[derive(Debug, Default)]
    pub struct RecordedLiveVocabulary {
        corrector: Corrector,
        kept: Mutex<Vec<Correction>>,
        forgotten: Mutex<Vec<SegmentId>>,
    }

    impl RecordedLiveVocabulary {
        /// A vocabulary correcting with `corrector`.
        pub fn new(corrector: Corrector) -> Self {
            Self {
                corrector,
                ..Self::default()
            }
        }

        /// Every original kept so far, oldest first.
        pub fn kept(&self) -> Vec<Correction> {
            lock(&self.kept).clone()
        }

        /// Every segment whose original was forgotten, oldest first.
        pub fn forgotten(&self) -> Vec<SegmentId> {
            lock(&self.forgotten).clone()
        }
    }

    #[async_trait]
    impl LiveVocabulary for RecordedLiveVocabulary {
        async fn corrector(&self) -> Result<Corrector, VocabError> {
            Ok(self.corrector.clone())
        }

        async fn record(
            &self,
            corrections: &[Correction],
            uncorrected: &[SegmentId],
        ) -> Result<(), VocabError> {
            lock(&self.kept).extend_from_slice(corrections);
            lock(&self.forgotten).extend_from_slice(uncorrected);
            Ok(())
        }
    }
}

/// No live corrections: recordings are saved as heard.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoLiveVocabulary;

#[async_trait]
impl LiveVocabulary for NoLiveVocabulary {
    async fn corrector(&self) -> Result<Corrector, VocabError> {
        Ok(Corrector::default())
    }

    async fn record(&self, _: &[Correction], _: &[SegmentId]) -> Result<(), VocabError> {
        Ok(())
    }
}
