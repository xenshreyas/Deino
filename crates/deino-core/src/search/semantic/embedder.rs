//! Text to vectors: the [`Embedder`] port and its adapter over the LLM
//! gateway's local `embed` route (llama-helper running
//! `nomic-embed-text-v1.5`, PARITY_PLAN §4.4, §8 D3: always on the Mac).

use std::sync::Arc;

use async_trait::async_trait;
use deino_llm::{Gateway, LOCAL_EMBEDDING_MODEL};

use crate::models::ModelService;
use crate::search::SearchError;

/// nomic-embed-text-v1.5 is trained with task prefixes: stored chunks are
/// embedded as documents, searches as queries (model card, "Usage").
pub const DOCUMENT_PREFIX: &str = "search_document: ";
/// The prefix of a search query (see [`DOCUMENT_PREFIX`]).
pub const QUERY_PREFIX: &str = "search_query: ";
/// Texts per call to the sidecar: enough to amortise a call, short enough
/// that a search query waiting behind it is not held up for long.
pub const EMBED_BATCH: usize = 16;

/// Which side of a search a text is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmbedRole {
    /// Indexed content.
    Document,
    /// A search query.
    Query,
}

/// Turns text into unit-length vectors with the local embedding model.
///
/// Implementations are cancel-safe: dropping a call's future (a search that
/// stopped waiting, an app that is quitting) leaves the embedder usable for
/// the next call.
#[async_trait]
pub trait Embedder: Send + Sync {
    /// The model's name, stored with every vector it makes.
    fn model(&self) -> &str;

    /// Whether the model is downloaded and can embed now.
    async fn ready(&self) -> bool;

    /// One vector per text, in order.
    async fn embed(&self, texts: &[String], role: EmbedRole) -> Result<Vec<Vec<f32>>, SearchError>;
}

/// [`Embedder`] over [`Gateway::embed`], with the model file managed by the
/// embedding [`ModelService`] (Settings → AI → Search index).
pub struct GatewayEmbedder {
    gateway: Arc<Gateway>,
    models: Arc<ModelService>,
}

impl GatewayEmbedder {
    /// An embedder over the gateway and the embedding model's files.
    pub fn new(gateway: Arc<Gateway>, models: Arc<ModelService>) -> Self {
        Self { gateway, models }
    }
}

#[async_trait]
impl Embedder for GatewayEmbedder {
    fn model(&self) -> &str {
        LOCAL_EMBEDDING_MODEL
    }

    async fn ready(&self) -> bool {
        match self.models.recommended() {
            Ok(file) => self.models.ready_path(&file).await.is_ok(),
            Err(_) => false,
        }
    }

    async fn embed(&self, texts: &[String], role: EmbedRole) -> Result<Vec<Vec<f32>>, SearchError> {
        let mut vectors = Vec::with_capacity(texts.len());
        for batch in texts.chunks(EMBED_BATCH) {
            let inputs = batch.iter().map(|text| prefixed(text, role)).collect();
            let embedded = self
                .gateway
                .embed(inputs)
                .await
                .map_err(|error| SearchError::Embedding(error.to_string()))?;
            if embedded.len() != batch.len() {
                return Err(SearchError::Embedding(format!(
                    "expected {} vectors, got {}",
                    batch.len(),
                    embedded.len()
                )));
            }
            vectors.extend(embedded);
        }
        Ok(vectors)
    }
}

/// The [`Embedder`] of an app without an LLM gateway: never ready, so search
/// stays keyword-only.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoEmbedder;

#[async_trait]
impl Embedder for NoEmbedder {
    fn model(&self) -> &'static str {
        LOCAL_EMBEDDING_MODEL
    }

    async fn ready(&self) -> bool {
        false
    }

    async fn embed(
        &self,
        _texts: &[String],
        _role: EmbedRole,
    ) -> Result<Vec<Vec<f32>>, SearchError> {
        Err(SearchError::ModelMissing)
    }
}

/// `text` with the model's task prefix for `role`.
pub fn prefixed(text: &str, role: EmbedRole) -> String {
    let prefix = match role {
        EmbedRole::Document => DOCUMENT_PREFIX,
        EmbedRole::Query => QUERY_PREFIX,
    };
    format!("{prefix}{text}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn documents_and_queries_get_the_nomic_task_prefixes() {
        assert_eq!(
            prefixed("Q4 roadmap", EmbedRole::Document),
            "search_document: Q4 roadmap"
        );
        assert_eq!(
            prefixed("when is the beta?", EmbedRole::Query),
            "search_query: when is the beta?"
        );
    }
}
