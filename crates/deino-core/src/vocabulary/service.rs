//! [`VocabularyService`]: the user's entries, the corrector built from them
//! and the automatic names, and the corrections a meeting holds.

use std::sync::Arc;

use deino_domain::{Clock, MeetingId, SegmentId, VocabTermId};

use super::{
    CorrectedSegment, Correction, CorrectionRepo, Corrector, Lexicon, NameSources, TermDraft,
    TermInput, UndoReport, UndoScope, VocabError, VocabRepo, VocabTerm,
};

/// The ports the service works over.
#[derive(Clone)]
pub struct VocabularyPorts {
    /// The user's entries.
    pub terms: Arc<dyn VocabRepo>,
    /// Corrections and their originals.
    pub corrections: Arc<dyn CorrectionRepo>,
    /// People, companies and invitees.
    pub names: NameSources,
    /// Entry IDs and timestamps.
    pub clock: Arc<dyn Clock>,
}

/// The vocabulary feature.
pub struct VocabularyService {
    ports: VocabularyPorts,
}

impl VocabularyService {
    /// The service over its ports.
    pub fn new(ports: VocabularyPorts) -> Self {
        Self { ports }
    }

    /// Every entry, replacements then terms, by spelling.
    pub async fn list(&self) -> Result<Vec<VocabTerm>, VocabError> {
        Ok(self.ports.terms.list().await?)
    }

    /// Adds an entry.
    pub async fn create(&self, input: &TermInput) -> Result<VocabTerm, VocabError> {
        let draft = TermDraft::parse(input)?;
        let now = self.ports.clock.now();
        let term = VocabTerm {
            id: VocabTermId::generate(self.ports.clock.as_ref()),
            kind: draft.kind,
            text: draft.text,
            sounds_like: draft.sounds_like,
            created_at: now,
            updated_at: now,
        };
        self.ports.terms.create(&term).await?;
        Ok(term)
    }

    /// Replaces an entry's kind, spelling and variants.
    pub async fn update(
        &self,
        id: &VocabTermId,
        input: &TermInput,
    ) -> Result<VocabTerm, VocabError> {
        let draft = TermDraft::parse(input)?;
        let current = self.find(id).await?;
        let term = VocabTerm {
            kind: draft.kind,
            text: draft.text,
            sounds_like: draft.sounds_like,
            updated_at: self.ports.clock.now(),
            ..current
        };
        if !self.ports.terms.update(&term).await? {
            return Err(VocabError::NotFound(*id));
        }
        Ok(term)
    }

    /// Deletes an entry. Corrections it already made stay until undone.
    pub async fn delete(&self, id: &VocabTermId) -> Result<(), VocabError> {
        if self.ports.terms.delete(id).await? {
            Ok(())
        } else {
            Err(VocabError::NotFound(*id))
        }
    }

    /// The corrector of the user's entries and every known name, plus
    /// `meeting`'s invitees.
    pub async fn corrector(&self, meeting: Option<&MeetingId>) -> Result<Corrector, VocabError> {
        let terms = self.ports.terms.list().await?;
        let names = self.ports.names.names(meeting).await?;
        Ok(Corrector::new(Lexicon::new(&terms, &names)))
    }

    /// The meeting's corrections that are not undone, in transcript order.
    pub async fn corrections(
        &self,
        meeting: &MeetingId,
    ) -> Result<Vec<CorrectedSegment>, VocabError> {
        Ok(self.ports.corrections.corrections(meeting).await?)
    }

    /// Undoes the corrections in `scope` (see [`CorrectionRepo::undo`]).
    pub async fn undo(&self, scope: UndoScope<'_>) -> Result<UndoReport, VocabError> {
        Ok(self
            .ports
            .corrections
            .undo(scope, self.ports.clock.now())
            .await?)
    }

    /// Keeps the originals of live segments saved corrected (the recorder's
    /// live pass), and forgets those of drafts no longer corrected.
    pub async fn record_live(
        &self,
        corrections: &[Correction],
        uncorrected: &[SegmentId],
    ) -> Result<(), VocabError> {
        let at = self.ports.clock.now();
        Ok(self
            .ports
            .corrections
            .record(corrections, uncorrected, at)
            .await?)
    }

    async fn find(&self, id: &VocabTermId) -> Result<VocabTerm, VocabError> {
        let terms = self.ports.terms.list().await?;
        terms
            .into_iter()
            .find(|term| term.id == *id)
            .ok_or_else(|| VocabError::NotFound(*id))
    }
}

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;
