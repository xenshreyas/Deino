//! The live vocabulary pass (TRN-05): every segment the router saves or
//! shows is corrected with the user's vocabulary and the known names first.
//!
//! Two decorators share one [`Corrector`], built when the recording starts:
//! [`CorrectedStore`] corrects the rows it saves, then keeps each corrected
//! row's original through [`LiveVocabulary::record`] so the correction can
//! be undone; [`CorrectedEvents`] corrects the live text the UI shows, so the
//! screen and the database never disagree. The router itself never knows:
//! the holdback still compares what the engines heard.

use std::collections::HashSet;
use std::sync::{Arc, Mutex, PoisonError};

use deino_core::segments::NewSegment;
use deino_core::vocabulary::{Correction, Corrector, LiveVocabulary};
use deino_domain::SegmentId;
use tokio::runtime::Handle;

use super::{SegmentStore, StoreError};
use crate::events::{HealthNotice, RecordingEvents, RecordingSnapshot, SegmentUpdate};

/// Saves rows corrected, and keeps the originals of those it corrected.
pub struct CorrectedStore {
    inner: Arc<dyn SegmentStore>,
    corrector: Arc<Corrector>,
    vocabulary: Arc<dyn LiveVocabulary>,
    runtime: Handle,
    /// Rows saved corrected so far: a draft among them that is no longer
    /// corrected must forget its original.
    corrected: Mutex<HashSet<SegmentId>>,
}

/// What [`CorrectedStore`] is built from.
pub struct CorrectedStoreParts {
    /// The store rows are saved through.
    pub inner: Arc<dyn SegmentStore>,
    /// The recording's corrector.
    pub corrector: Arc<Corrector>,
    /// Where originals are kept.
    pub vocabulary: Arc<dyn LiveVocabulary>,
    /// The runtime the vocabulary's async writes run on.
    pub runtime: Handle,
}

impl CorrectedStore {
    /// A store correcting rows before `parts.inner` saves them.
    pub fn new(parts: CorrectedStoreParts) -> Self {
        Self {
            inner: parts.inner,
            corrector: parts.corrector,
            vocabulary: parts.vocabulary,
            runtime: parts.runtime,
            corrected: Mutex::new(HashSet::new()),
        }
    }

    /// The rows as saved, and the corrections among them.
    fn correct(&self, segments: &[NewSegment]) -> (Vec<NewSegment>, Vec<Correction>) {
        let mut rows = Vec::with_capacity(segments.len());
        let mut corrections = Vec::new();
        for row in segments {
            match self.corrector.segment(&row.text, &row.words) {
                Some(corrected) => {
                    corrections.push(Correction {
                        segment_id: row.id,
                        original_text: row.text.clone(),
                        original_words: row.words.clone(),
                        corrected_text: corrected.text.clone(),
                        corrected_words: corrected.words.clone(),
                    });
                    rows.push(NewSegment {
                        text: corrected.text,
                        words: corrected.words,
                        ..row.clone()
                    });
                }
                None => rows.push(row.clone()),
            }
        }
        (rows, corrections)
    }

    /// Keeps the originals of `corrections`; forgets those of earlier
    /// corrected rows saved uncorrected this time. A failure only costs the
    /// undo, so it is logged, never raised.
    fn record(&self, saved: &[NewSegment], corrections: &[Correction]) {
        let uncorrected = self.no_longer_corrected(saved, corrections);
        if corrections.is_empty() && uncorrected.is_empty() {
            return;
        }
        let kept = self
            .runtime
            .block_on(self.vocabulary.record(corrections, &uncorrected));
        if let Err(error) = kept {
            log::warn!(target: "recording", "{error}; {} live corrections cannot be undone", corrections.len());
        }
    }

    /// The rows of `saved` that were saved corrected before and are not now;
    /// remembers which are corrected now.
    fn no_longer_corrected(
        &self,
        saved: &[NewSegment],
        corrections: &[Correction],
    ) -> Vec<SegmentId> {
        let now: HashSet<SegmentId> = corrections.iter().map(|c| c.segment_id).collect();
        let mut known = self
            .corrected
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let dropped: Vec<SegmentId> = saved
            .iter()
            .map(|row| row.id)
            .filter(|id| !now.contains(id) && known.contains(id))
            .collect();
        for id in &dropped {
            known.remove(id);
        }
        known.extend(now);
        dropped
    }
}

impl SegmentStore for CorrectedStore {
    fn save(&self, segments: &[NewSegment], removed: &[SegmentId]) -> Result<(), StoreError> {
        let (rows, corrections) = self.correct(segments);
        self.inner.save(&rows, removed)?;
        self.record(&rows, &corrections);
        Ok(())
    }
}

/// Shows live text corrected.
pub struct CorrectedEvents {
    inner: Arc<dyn RecordingEvents>,
    corrector: Arc<Corrector>,
}

impl CorrectedEvents {
    /// Events to `inner`, with segment text corrected by `corrector`.
    pub fn new(inner: Arc<dyn RecordingEvents>, corrector: Arc<Corrector>) -> Self {
        Self { inner, corrector }
    }
}

impl RecordingEvents for CorrectedEvents {
    fn state(&self, state: &RecordingSnapshot) {
        self.inner.state(state);
    }

    fn segment(&self, segment: &SegmentUpdate) {
        match self.corrector.text(&segment.text) {
            Some(text) => self.inner.segment(&SegmentUpdate {
                text,
                ..segment.clone()
            }),
            None => self.inner.segment(segment),
        }
    }

    fn health(&self, notice: &HealthNotice) {
        self.inner.health(notice);
    }
}

#[cfg(test)]
#[path = "vocabulary_tests.rs"]
mod tests;
