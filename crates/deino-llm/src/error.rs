//! What can go wrong in an LLM call, and how callers should react.
//!
//! Messages are user-safe: provider names, status codes and the provider's
//! own short error text, never prompt or answer content. [`LlmError::kind`]
//! is what `llm_calls.error_kind` stores; [`LlmError::retry_after`] and
//! [`LlmError::is_retryable`] drive retries (FND-08 pipeline stages).

use std::time::Duration;

use deino_domain::{ErrorClass, ErrorCode};
use serde::Serialize;

use crate::{CredentialError, HttpError, ProviderId, RouteStoreError};

/// An LLM call failed.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum LlmError {
    /// The provider needs an API key and none is set.
    #[error("no API key is set for {0}")]
    NoKey(ProviderId),
    /// The provider rejected the API key.
    #[error("{0} rejected the API key")]
    Auth(ProviderId),
    /// Too many requests; wait `retry_after` when the provider says how long.
    #[error("{provider} is rate limiting requests")]
    RateLimited {
        /// The provider.
        provider: ProviderId,
        /// How long the provider asked to wait.
        retry_after: Option<Duration>,
    },
    /// The provider is temporarily overloaded.
    #[error("{provider} is overloaded")]
    Overloaded {
        /// The provider.
        provider: ProviderId,
        /// How long the provider asked to wait.
        retry_after: Option<Duration>,
    },
    /// The prompt does not fit the model's context window.
    #[error("the request is too long for the {0} model")]
    ContextTooLong(ProviderId),
    /// The provider refused the request as invalid (4xx).
    #[error("{provider} rejected the request ({status}): {message}")]
    Rejected {
        /// The provider.
        provider: ProviderId,
        /// The HTTP status.
        status: u16,
        /// The provider's short explanation.
        message: String,
    },
    /// The provider failed (5xx or an error event mid-stream).
    #[error("{provider} failed ({status}): {message}")]
    Provider {
        /// The provider.
        provider: ProviderId,
        /// The HTTP status, or 0 for an error inside a stream.
        status: u16,
        /// The provider's short explanation.
        message: String,
    },
    /// The provider could not be reached.
    #[error("could not reach {provider}")]
    Network {
        /// The provider.
        provider: ProviderId,
        /// What the transport saw.
        #[source]
        source: HttpError,
    },
    /// The provider answered in a shape Deino cannot read.
    #[error("{provider} sent a response Deino could not read: {detail}")]
    Malformed {
        /// The provider.
        provider: ProviderId,
        /// What was wrong, without the content.
        detail: String,
    },
    /// The answer stopped before the provider said it was complete.
    #[error("the answer ended before it was complete")]
    Truncated,
    /// The provider cannot serve requests right now (a model to download, a
    /// server that is not running).
    #[error("{provider} is not available: {reason}")]
    Unavailable {
        /// The provider.
        provider: ProviderId,
        /// What the user can do about it.
        reason: String,
    },
    /// The provider cannot do what was asked.
    #[error("{provider} does not support {feature}")]
    Unsupported {
        /// The provider.
        provider: ProviderId,
        /// The missing capability, for example "tool calling".
        feature: &'static str,
    },
    /// Structured output did not match the schema, even after the repair retry.
    #[error("the model's answer did not match the expected format ({detail})")]
    InvalidOutput {
        /// What failed to parse (error class and position only).
        detail: String,
    },
    /// The tool loop used every step without a final answer.
    #[error("the model kept calling tools for {steps} steps without answering")]
    ToolLoopExceeded {
        /// The step limit.
        steps: usize,
    },
    /// The credential store could not be read.
    #[error("could not read the key for {provider}: {source}")]
    Credentials {
        /// The provider.
        provider: ProviderId,
        /// The store's failure.
        #[source]
        source: CredentialError,
    },
    /// The route settings could not be read or saved.
    #[error("could not load or save the AI route settings")]
    Routes(#[from] RouteStoreError),
    /// The caller stopped waiting.
    #[error("the request was cancelled")]
    Cancelled,
}

/// The kind of an [`LlmError`], as `llm_calls.error_kind` stores it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, specta::Type, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum LlmErrorKind {
    /// [`LlmError::NoKey`].
    NoKey,
    /// [`LlmError::Auth`].
    Auth,
    /// [`LlmError::RateLimited`].
    RateLimited,
    /// [`LlmError::Overloaded`].
    Overloaded,
    /// [`LlmError::ContextTooLong`].
    ContextTooLong,
    /// [`LlmError::Rejected`].
    Rejected,
    /// [`LlmError::Provider`].
    Provider,
    /// [`LlmError::Network`].
    Network,
    /// [`LlmError::Malformed`].
    Malformed,
    /// [`LlmError::Truncated`].
    Truncated,
    /// [`LlmError::Unavailable`].
    Unavailable,
    /// [`LlmError::Unsupported`].
    Unsupported,
    /// [`LlmError::InvalidOutput`].
    InvalidOutput,
    /// [`LlmError::ToolLoopExceeded`].
    ToolLoopExceeded,
    /// [`LlmError::Credentials`].
    Credentials,
    /// [`LlmError::Routes`].
    Routes,
    /// [`LlmError::Cancelled`].
    Cancelled,
}

impl LlmErrorKind {
    /// Every kind, in declaration order (the migration's CHECK list).
    pub const ALL: &'static [Self] = &[
        Self::NoKey,
        Self::Auth,
        Self::RateLimited,
        Self::Overloaded,
        Self::ContextTooLong,
        Self::Rejected,
        Self::Provider,
        Self::Network,
        Self::Malformed,
        Self::Truncated,
        Self::Unavailable,
        Self::Unsupported,
        Self::InvalidOutput,
        Self::ToolLoopExceeded,
        Self::Credentials,
        Self::Routes,
        Self::Cancelled,
    ];
}

impl LlmError {
    /// The error's kind, for usage records and metrics.
    pub fn kind(&self) -> LlmErrorKind {
        match self {
            Self::NoKey(_) => LlmErrorKind::NoKey,
            Self::Auth(_) => LlmErrorKind::Auth,
            Self::RateLimited { .. } => LlmErrorKind::RateLimited,
            Self::Overloaded { .. } => LlmErrorKind::Overloaded,
            Self::ContextTooLong(_) => LlmErrorKind::ContextTooLong,
            Self::Rejected { .. } => LlmErrorKind::Rejected,
            Self::Provider { .. } => LlmErrorKind::Provider,
            Self::Network { .. } => LlmErrorKind::Network,
            Self::Malformed { .. } => LlmErrorKind::Malformed,
            Self::Truncated => LlmErrorKind::Truncated,
            Self::Unavailable { .. } => LlmErrorKind::Unavailable,
            Self::Unsupported { .. } => LlmErrorKind::Unsupported,
            Self::InvalidOutput { .. } => LlmErrorKind::InvalidOutput,
            Self::ToolLoopExceeded { .. } => LlmErrorKind::ToolLoopExceeded,
            Self::Credentials { .. } => LlmErrorKind::Credentials,
            Self::Routes(_) => LlmErrorKind::Routes,
            Self::Cancelled => LlmErrorKind::Cancelled,
        }
    }

    /// How long the provider asked callers to wait before retrying.
    pub fn retry_after(&self) -> Option<Duration> {
        match self {
            Self::RateLimited { retry_after, .. } | Self::Overloaded { retry_after, .. } => {
                *retry_after
            }
            _ => None,
        }
    }

    /// Whether the same request may succeed later without any change.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::RateLimited { .. }
                | Self::Overloaded { .. }
                | Self::Provider { .. }
                | Self::Network { .. }
                | Self::Truncated
        )
    }
}

impl ErrorClass for LlmError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::NoKey(_) | Self::Unavailable { .. } => ErrorCode::Unavailable,
            Self::Auth(_) => ErrorCode::PermissionDenied,
            Self::Network { .. } => ErrorCode::Network,
            Self::Unsupported { .. } => ErrorCode::Invalid,
            Self::Credentials { .. } | Self::Routes(_) => ErrorCode::Internal,
            Self::Cancelled => ErrorCode::Cancelled,
            Self::RateLimited { .. }
            | Self::Overloaded { .. }
            | Self::ContextTooLong(_)
            | Self::Rejected { .. }
            | Self::Provider { .. }
            | Self::Malformed { .. }
            | Self::Truncated
            | Self::InvalidOutput { .. }
            | Self::ToolLoopExceeded { .. } => ErrorCode::Llm,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_limits_carry_their_retry_after_and_are_retryable() {
        let error = LlmError::RateLimited {
            provider: ProviderId::Anthropic,
            retry_after: Some(Duration::from_secs(20)),
        };

        assert_eq!(error.retry_after(), Some(Duration::from_secs(20)));
        assert!(error.is_retryable());
        assert_eq!(error.code(), ErrorCode::Llm);
    }

    #[test]
    fn a_missing_key_is_not_retryable_and_tells_the_ui_what_is_unavailable() {
        let error = LlmError::NoKey(ProviderId::OpenAi);

        assert!(!error.is_retryable());
        assert_eq!(error.retry_after(), None);
        assert_eq!(error.code(), ErrorCode::Unavailable);
        assert_eq!(error.to_string(), "no API key is set for OpenAI");
    }

    #[test]
    fn kinds_serialize_as_snake_case() {
        let kind = LlmError::ToolLoopExceeded { steps: 8 }.kind();

        assert_eq!(serde_json::to_value(kind).unwrap(), "tool_loop_exceeded");
    }
}
