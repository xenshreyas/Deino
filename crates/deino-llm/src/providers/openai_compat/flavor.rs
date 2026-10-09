//! What differs between the OpenAI-compatible providers: where they live,
//! whether they need a key, which listed models can chat, and the fields a
//! request adds for them.

use crate::ProviderId;

/// One OpenAI-compatible provider.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flavor {
    /// OpenAI.
    OpenAi,
    /// Groq.
    Groq,
    /// OpenRouter.
    OpenRouter,
    /// A user-configured endpoint (vLLM, LM Studio, a proxy, ...).
    Custom,
    /// Ollama's OpenAI-compatible `/v1` API.
    Ollama,
}

/// Ollama's address when the user set none.
pub(crate) const OLLAMA_DEFAULT_URL: &str = "http://localhost:11434";

impl Flavor {
    /// The provider this flavor serves.
    pub fn provider(self) -> ProviderId {
        match self {
            Self::OpenAi => ProviderId::OpenAi,
            Self::Groq => ProviderId::Groq,
            Self::OpenRouter => ProviderId::OpenRouter,
            Self::Custom => ProviderId::CustomOpenAi,
            Self::Ollama => ProviderId::Ollama,
        }
    }

    /// The API base (ending before `/chat/completions`), given the user's
    /// configured endpoint. `None` when a custom endpoint is not configured.
    pub(super) fn base_url(self, configured: Option<&str>) -> Option<String> {
        let configured = configured
            .map(|url| url.trim().trim_end_matches('/'))
            .filter(|url| !url.is_empty());
        match self {
            Self::OpenAi => Some("https://api.openai.com/v1".to_owned()),
            Self::Groq => Some("https://api.groq.com/openai/v1".to_owned()),
            Self::OpenRouter => Some("https://openrouter.ai/api/v1".to_owned()),
            Self::Custom => configured.map(str::to_owned),
            Self::Ollama => Some(format!("{}/v1", configured.unwrap_or(OLLAMA_DEFAULT_URL))),
        }
    }

    /// Whether the user's endpoint settings matter to this flavor.
    pub(super) fn is_configurable(self) -> bool {
        matches!(self, Self::Custom | Self::Ollama)
    }

    /// `reasoning_effort` to send: local Ollama models answer directly
    /// instead of thinking first (dropped by the compatibility retry when a
    /// model rejects it, as the legacy client did).
    pub(super) fn reasoning_effort(self) -> Option<&'static str> {
        matches!(self, Self::Ollama).then_some("none")
    }

    /// Whether a listed model ID is a chat model worth offering.
    pub(super) fn lists(self, model_id: &str) -> bool {
        let id = model_id.to_ascii_lowercase();
        match self {
            Self::OpenAi => is_openai_chat_model(&id),
            Self::Groq => !["whisper", "embed", "guard", "tool-use"]
                .iter()
                .any(|excluded| id.contains(excluded)),
            Self::OpenRouter | Self::Custom | Self::Ollama => true,
        }
    }

    /// The models offered before a key is set.
    pub(super) fn fallback_models(self) -> &'static [&'static str] {
        match self {
            Self::OpenAi => &["gpt-5", "gpt-5-mini", "gpt-4.1", "gpt-4o"],
            Self::Groq => &["llama-3.3-70b-versatile"],
            Self::OpenRouter | Self::Custom | Self::Ollama => &[],
        }
    }
}

/// OpenAI's model list includes embeddings, speech and image models.
fn is_openai_chat_model(id: &str) -> bool {
    const CHAT_PREFIXES: [&str; 5] = ["gpt-", "o1", "o3", "o4", "chatgpt-"];
    const NOT_CHAT: [&str; 9] = [
        "embedding",
        "tts",
        "whisper",
        "dall-e",
        "babbage",
        "davinci",
        "instruct",
        "realtime",
        "audio",
    ];
    CHAT_PREFIXES.iter().any(|prefix| id.starts_with(prefix))
        && !NOT_CHAT.iter().any(|excluded| id.contains(excluded))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_urls_follow_each_provider_and_the_user_s_endpoint() {
        assert_eq!(
            Flavor::Groq.base_url(None).as_deref(),
            Some("https://api.groq.com/openai/v1")
        );
        assert_eq!(
            Flavor::Ollama.base_url(None).as_deref(),
            Some("http://localhost:11434/v1")
        );
        assert_eq!(
            Flavor::Ollama
                .base_url(Some("http://gpu-box:11434/"))
                .as_deref(),
            Some("http://gpu-box:11434/v1")
        );
        assert_eq!(
            Flavor::Custom
                .base_url(Some(" http://host:8000/v1/ "))
                .as_deref(),
            Some("http://host:8000/v1")
        );
        assert_eq!(Flavor::Custom.base_url(Some("  ")), None);
    }

    #[test]
    fn openai_lists_chat_models_only() {
        let listed: Vec<&str> = [
            "gpt-5",
            "o3-mini",
            "text-embedding-3-small",
            "gpt-4o-audio-preview",
            "dall-e-3",
        ]
        .into_iter()
        .filter(|id| Flavor::OpenAi.lists(id))
        .collect();

        assert_eq!(listed, ["gpt-5", "o3-mini"]);
    }

    #[test]
    fn only_ollama_turns_reasoning_off() {
        assert_eq!(Flavor::Ollama.reasoning_effort(), Some("none"));
        assert_eq!(Flavor::OpenAi.reasoning_effort(), None);
    }
}
