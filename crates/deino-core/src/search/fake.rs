//! Fakes of the search ports, for tests here and in other crates.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use async_trait::async_trait;
use deino_domain::MeetingId;

use super::semantic::{EmbedRole, Embedder, SemanticSearch};
use super::{Hit, SearchError};
use crate::testing::lock;

/// Dimensions of [`FakeEmbedder`]'s vectors.
pub const FAKE_DIMS: usize = 64;

/// A deterministic bag-of-words embedder: each lower-case word adds one to
/// a hashed dimension, so texts sharing words are similar. Tracks how many
/// texts it embedded, and can pretend the model is missing.
#[derive(Debug)]
pub struct FakeEmbedder {
    ready: AtomicBool,
    embedded: AtomicUsize,
}

impl Default for FakeEmbedder {
    fn default() -> Self {
        Self {
            ready: AtomicBool::new(true),
            embedded: AtomicUsize::new(0),
        }
    }
}

impl FakeEmbedder {
    /// Whether the "model" is downloaded.
    pub fn set_ready(&self, ready: bool) {
        self.ready.store(ready, Ordering::SeqCst);
    }

    /// Document texts embedded so far.
    pub fn embedded(&self) -> usize {
        self.embedded.load(Ordering::SeqCst)
    }

    /// The vector of `text`.
    pub fn vector(text: &str) -> Vec<f32> {
        let mut vector = vec![0.0; FAKE_DIMS];
        let words = text
            .split(|c: char| !c.is_alphanumeric())
            .filter(|word| word.len() > 2)
            .map(str::to_lowercase);
        for word in words {
            let mut hasher = DefaultHasher::new();
            word.hash(&mut hasher);
            let slot = usize::try_from(hasher.finish() % FAKE_DIMS as u64).unwrap_or(0);
            if let Some(value) = vector.get_mut(slot) {
                *value += 1.0;
            }
        }
        vector
    }
}

#[async_trait]
impl Embedder for FakeEmbedder {
    fn model(&self) -> &'static str {
        "fake-embedder"
    }

    async fn ready(&self) -> bool {
        self.ready.load(Ordering::SeqCst)
    }

    async fn embed(&self, texts: &[String], role: EmbedRole) -> Result<Vec<Vec<f32>>, SearchError> {
        if !self.ready.load(Ordering::SeqCst) {
            return Err(SearchError::ModelMissing);
        }
        if role == EmbedRole::Document {
            self.embedded.fetch_add(texts.len(), Ordering::SeqCst);
        }
        Ok(texts.iter().map(|text| Self::vector(text)).collect())
    }
}

/// Semantic search that answers with scripted hits (or `None`), and records
/// the scopes it was asked for.
#[derive(Debug, Default)]
pub struct FakeSemanticSearch {
    hits: Mutex<Option<Vec<Hit>>>,
    scopes: Mutex<Vec<Option<Vec<MeetingId>>>>,
}

impl FakeSemanticSearch {
    /// Answers every search with `hits`.
    pub fn answering(hits: Vec<Hit>) -> Self {
        Self {
            hits: Mutex::new(Some(hits)),
            ..Self::default()
        }
    }

    /// The scopes searched, in order.
    pub fn scopes(&self) -> Vec<Option<Vec<MeetingId>>> {
        lock(&self.scopes).clone()
    }
}

#[async_trait]
impl SemanticSearch for FakeSemanticSearch {
    async fn search(
        &self,
        _query: &str,
        scope: Option<&[MeetingId]>,
        k: usize,
    ) -> Result<Option<Vec<Hit>>, SearchError> {
        lock(&self.scopes).push(scope.map(<[MeetingId]>::to_vec));
        Ok(lock(&self.hits)
            .clone()
            .map(|hits| hits.into_iter().take(k).collect()))
    }
}
