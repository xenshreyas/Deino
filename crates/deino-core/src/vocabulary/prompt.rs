//! [`PromptVocabulary`]: the enhanced notes' vocabulary input (NOT-02's
//! [`Vocabulary`] port): the user's terms, spelled as given, so the notes
//! spell them the same way. Names reach the prompt already, as attendees and
//! speakers; replacement rules are corrections, not words to know.

use std::sync::Arc;

use async_trait::async_trait;
use deino_db::DbError;
use deino_domain::{MeetingId, VocabKind};

use super::VocabRepo;
use crate::enhance::Vocabulary;

/// The user's terms, for every meeting's notes.
#[derive(Clone)]
pub struct PromptVocabulary {
    terms: Arc<dyn VocabRepo>,
}

impl PromptVocabulary {
    /// The terms kept in `terms`.
    pub fn new(terms: Arc<dyn VocabRepo>) -> Self {
        Self { terms }
    }
}

#[async_trait]
impl Vocabulary for PromptVocabulary {
    async fn terms(&self, _meeting: &MeetingId) -> Result<Vec<String>, DbError> {
        let terms = self.terms.list().await?;
        Ok(terms
            .into_iter()
            .filter(|term| term.kind == VocabKind::Term)
            .map(|term| term.text)
            .collect())
    }
}
