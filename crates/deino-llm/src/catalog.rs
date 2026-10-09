//! Model lists: what each provider can serve, for Settings → AI and the
//! legacy model pickers.
//!
//! Each provider adapter implements [`ModelCatalog`] next to its
//! [`LlmProvider`](crate::LlmProvider). Without a key, hosted providers list
//! their well-known models so the pickers are never empty; with a key they
//! list the account's live models and report failures as errors.

use async_trait::async_trait;
use serde::Serialize;

use crate::{ApiKey, LlmError};

/// Values typed into the settings form but not saved yet. The form lists a
/// provider's models before the user saves, so these win over the saved
/// key and endpoint when set.
#[derive(Clone, Debug, Default)]
pub struct DraftConnection {
    /// A key the user typed.
    pub api_key: Option<ApiKey>,
    /// A server address the user typed (Ollama and custom endpoints).
    pub endpoint: Option<String>,
}

/// A model a provider can serve.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    /// The provider's model ID, as requests name it.
    pub id: String,
    /// A friendlier name, when the provider has one.
    pub display_name: Option<String>,
    /// The context window in tokens, when the provider says.
    pub context_length: Option<u32>,
    /// The download size in bytes (local models).
    pub size_bytes: Option<u64>,
    /// When the model was last changed (Ollama), as the provider reports it.
    pub modified_at: Option<String>,
    /// Whether the model can serve requests now (local models must be downloaded).
    pub ready: bool,
}

impl ModelInfo {
    /// A ready model known only by its ID.
    pub fn named(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            display_name: None,
            context_length: None,
            size_bytes: None,
            modified_at: None,
            ready: true,
        }
    }

    /// Adds a display name.
    #[must_use]
    pub fn display_name(mut self, name: impl Into<String>) -> Self {
        self.display_name = Some(name.into());
        self
    }
}

/// Lists the models one provider can serve.
#[async_trait]
pub trait ModelCatalog: Send + Sync {
    /// The models, in the provider's order, reached with the `draft` values
    /// where set and the saved ones otherwise.
    async fn models(&self, draft: &DraftConnection) -> Result<Vec<ModelInfo>, LlmError>;
}
