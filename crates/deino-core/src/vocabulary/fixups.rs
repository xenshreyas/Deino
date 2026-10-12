//! [`FixupService`]: the `vocab_fixups` stage's rules (order 40).
//!
//! It corrects every segment of a meeting with the user's entries, every
//! known name and the meeting's invitees, and stores the corrected segments
//! with their originals in one transaction. Segments the user edited are
//! never touched, nor are corrections the user undid. Running it again finds
//! nothing left to correct, so the stage is idempotent.

use std::sync::Arc;

use deino_domain::{Clock, MeetingId};

use super::{Correction, CorrectionRepo, Corrector, VocabError, VocabularyService};
use crate::segments::{Page, Segment, SegmentReader};

/// What one run did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FixupOutcome {
    /// Segments read.
    pub segments: usize,
    /// Segments corrected.
    pub corrected: u32,
    /// True when the vocabulary had nothing to correct with.
    pub empty_vocabulary: bool,
}

/// The ports the stage's rules run over.
#[derive(Clone)]
pub struct FixupPorts {
    /// The vocabulary (its corrector).
    pub vocabulary: Arc<VocabularyService>,
    /// The meeting's transcript.
    pub segments: Arc<dyn SegmentReader>,
    /// Where corrections are applied.
    pub corrections: Arc<dyn CorrectionRepo>,
    /// When corrections were made.
    pub clock: Arc<dyn Clock>,
}

/// Corrects a finished meeting's transcript.
pub struct FixupService {
    ports: FixupPorts,
}

impl FixupService {
    /// The service over its ports.
    pub fn new(ports: FixupPorts) -> Self {
        Self { ports }
    }

    /// Corrects `meeting`'s transcript.
    pub async fn run(&self, meeting: &MeetingId) -> Result<FixupOutcome, VocabError> {
        let corrector = self.ports.vocabulary.corrector(Some(meeting)).await?;
        if corrector.is_empty() {
            return Ok(FixupOutcome {
                empty_vocabulary: true,
                ..FixupOutcome::default()
            });
        }
        let segments = self.ports.segments.list(meeting, Page::ALL).await?;
        let corrections = corrections(&corrector, &segments);
        let corrected = self
            .ports
            .corrections
            .apply(&corrections, self.ports.clock.now())
            .await?;
        Ok(FixupOutcome {
            segments: segments.len(),
            corrected,
            empty_vocabulary: false,
        })
    }
}

/// The corrections `corrector` makes to the segments the user never edited.
pub fn corrections(corrector: &Corrector, segments: &[Segment]) -> Vec<Correction> {
    segments
        .iter()
        .filter(|segment| segment.edited_at.is_none())
        .filter_map(|segment| {
            let corrected = corrector.segment(&segment.text, &segment.words)?;
            Some(Correction {
                segment_id: segment.id,
                original_text: segment.text.clone(),
                original_words: segment.words.clone(),
                corrected_text: corrected.text,
                corrected_words: corrected.words,
            })
        })
        .collect()
}

#[cfg(test)]
#[path = "fixups_tests.rs"]
mod tests;
