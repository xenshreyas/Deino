//! Embedding one meeting: chunk it, embed what changed, store it.
//!
//! Re-indexing is cheap when little changed: a chunk whose text the same
//! model embedded before keeps its vector, so editing one note section
//! embeds one chunk. The meeting's rows are replaced in one transaction and
//! the in-memory index follows.

use std::collections::HashMap;
use std::sync::Arc;

use deino_domain::{Clock, MeetingId};

use super::chunk::{self, Chunk, ChunkKind};
use super::{
    EmbedRole, Embedder, EmbeddingStore, MeetingSourceReader, MeetingSources, Replacement,
    SharedVectors,
};
use crate::search::SearchError;

/// What indexing a meeting did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IndexedMeeting {
    /// Chunks the meeting has now.
    pub chunks: u32,
    /// Chunks that had to be embedded (the rest were unchanged).
    pub embedded: u32,
}

/// What a [`SemanticIndexer`] works with.
pub struct IndexerParts {
    /// What meetings say.
    pub sources: Arc<dyn MeetingSourceReader>,
    /// Where chunks are stored.
    pub store: Arc<dyn EmbeddingStore>,
    /// The embedding model.
    pub embedder: Arc<dyn Embedder>,
    /// The in-memory index to keep in step.
    pub vectors: Arc<SharedVectors>,
    /// Stamps stored rows.
    pub clock: Arc<dyn Clock>,
}

/// Embeds meetings into the semantic index.
pub struct SemanticIndexer {
    parts: IndexerParts,
}

impl SemanticIndexer {
    /// An indexer over its parts.
    pub fn new(parts: IndexerParts) -> Self {
        Self { parts }
    }

    /// Whether the embedding model is downloaded.
    pub async fn ready(&self) -> bool {
        self.parts.embedder.ready().await
    }

    /// (Re)embeds `meeting`. Fails with [`SearchError::ModelMissing`] before
    /// touching anything when the model is not downloaded.
    pub async fn index_meeting(&self, meeting: &MeetingId) -> Result<IndexedMeeting, SearchError> {
        if !self.ready().await {
            return Err(SearchError::ModelMissing);
        }
        let chunks = chunks_of(meeting, self.parts.sources.sources(meeting).await?);
        let (vectors, embedded) = self.vectors_for(meeting, &chunks).await?;
        let replacement = Replacement {
            meeting,
            model: self.parts.embedder.model(),
            chunks: &chunks,
            vectors: &vectors,
            at: self.parts.clock.now(),
        };
        let stored = self.parts.store.replace(replacement).await?;
        self.parts.vectors.replace_meeting(meeting, stored).await;
        Ok(IndexedMeeting {
            chunks: count(chunks.len()),
            embedded,
        })
    }

    /// One vector per chunk: reused when this model embedded the same text
    /// before, embedded now otherwise. Also returns how many were embedded.
    async fn vectors_for(
        &self,
        meeting: &MeetingId,
        chunks: &[Chunk],
    ) -> Result<(Vec<Vec<f32>>, u32), SearchError> {
        let model = self.parts.embedder.model();
        let mut known: HashMap<String, Vec<f32>> = self
            .parts
            .store
            .existing(meeting)
            .await?
            .into_iter()
            .filter(|existing| existing.model == model)
            .map(|existing| (existing.text, existing.vector))
            .collect();
        let mut missing: Vec<String> = Vec::new();
        for chunk in chunks {
            if !known.contains_key(&chunk.text) && !missing.contains(&chunk.text) {
                missing.push(chunk.text.clone());
            }
        }
        let fresh = self
            .parts
            .embedder
            .embed(&missing, EmbedRole::Document)
            .await?;
        let embedded = count(missing.len());
        known.extend(missing.into_iter().zip(fresh));
        let vectors = chunks
            .iter()
            .map(|chunk| known.get(&chunk.text).cloned())
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| SearchError::Embedding("a chunk got no vector".to_owned()))?;
        Ok((vectors, embedded))
    }
}

/// Every chunk of a meeting: transcript windows, then note and summary
/// sections.
pub fn chunks_of(meeting: &MeetingId, sources: MeetingSources) -> Vec<Chunk> {
    let mut chunks = chunk::transcript_chunks(&sources.transcript);
    for (kind, markdown) in [
        (ChunkKind::Note, sources.notes),
        (ChunkKind::Summary, sources.summary),
    ] {
        if let Some(markdown) = markdown {
            chunks.extend(chunk::section_chunks(kind, meeting, &markdown));
        }
    }
    chunks
}

fn count(length: usize) -> u32 {
    u32::try_from(length).unwrap_or(u32::MAX)
}

#[cfg(test)]
#[path = "indexer_tests.rs"]
mod tests;
