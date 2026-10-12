//! What can go wrong with enhanced notes. Every message is user-safe and
//! says what to do: no transcript, notes or provider bodies, ever.

use deino_db::DbError;
use deino_doc::DocError;
use deino_domain::{ErrorClass, ErrorCode, MeetingId};
use deino_llm::LlmError;

use crate::templates::TemplateError;

/// An enhanced-notes failure.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum EnhanceError {
    /// No meeting has the requested ID.
    #[error("meeting {0} was not found")]
    MeetingNotFound(MeetingId),
    /// There is no transcript and no note to write from.
    #[error("There is nothing to write notes from yet: no transcript and no notes.")]
    NothingToEnhance,
    /// Retention deleted the transcript (PRV-01); the notes it was written
    /// into stay, but writing them again would lose what came from it.
    #[error(
        "The transcript was deleted by your retention settings, so these notes cannot be regenerated."
    )]
    TranscriptDeleted,
    /// The newest version has the user's edits; regenerating needs consent.
    #[error("Regenerating will replace the notes you edited.")]
    WouldReplaceEdits,
    /// Notes for this meeting are being written already.
    #[error("Notes for this meeting are already being written.")]
    AlreadyRunning,
    /// The version asked for does not exist.
    #[error("version {version} of the notes was not found")]
    VersionNotFound {
        /// The missing version.
        version: u32,
    },
    /// Only the newest version can be edited.
    #[error("Only the newest version of the notes can be edited.")]
    NotLatest,
    /// The edited document breaks the editor's schema.
    #[error("the notes are not a valid document")]
    InvalidDoc(#[source] DocError),
    /// The model wrote nothing usable.
    #[error("The AI returned empty notes. Try again, or pick another model in Settings → AI.")]
    EmptyNotes,
    /// The LLM call failed; the message says what to do about it.
    #[error("{}", llm_message(.0))]
    Llm(#[from] LlmError),
    /// The meeting's template could not be read.
    #[error("could not read the meeting's template")]
    Template(#[source] TemplateError),
    /// The database failed; the source carries the detail.
    #[error("could not load or save the notes")]
    Storage(#[from] DbError),
}

impl EnhanceError {
    /// Whether trying again later may succeed (a rate limit, the network,
    /// a busy database), as the pipeline stage's retries need to know.
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Llm(error) => error.is_retryable(),
            Self::Storage(_) | Self::AlreadyRunning | Self::EmptyNotes => true,
            _ => false,
        }
    }
}

impl From<TemplateError> for EnhanceError {
    fn from(error: TemplateError) -> Self {
        Self::Template(error)
    }
}

impl ErrorClass for EnhanceError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::MeetingNotFound(_) | Self::VersionNotFound { .. } => ErrorCode::NotFound,
            Self::NothingToEnhance | Self::TranscriptDeleted | Self::InvalidDoc(_) => {
                ErrorCode::Invalid
            }
            Self::WouldReplaceEdits | Self::AlreadyRunning | Self::NotLatest => ErrorCode::Conflict,
            Self::EmptyNotes => ErrorCode::Llm,
            Self::Llm(error) => error.code(),
            Self::Template(error) => error.code(),
            Self::Storage(_) => ErrorCode::Internal,
        }
    }
}

/// A sentence the user can act on for each way a provider call fails.
fn llm_message(error: &LlmError) -> String {
    match error {
        LlmError::NoKey(_) | LlmError::Unavailable { .. } => {
            "Add an Anthropic key or pick a local model in Settings → AI.".to_owned()
        }
        LlmError::Auth(provider) => {
            format!("{provider} rejected the API key. Check it in Settings → AI.")
        }
        LlmError::RateLimited { provider, .. } | LlmError::Overloaded { provider, .. } => {
            format!("{provider} is busy. Deino will try again shortly.")
        }
        LlmError::Network { provider, .. } => {
            format!("Could not reach {provider}. Check your connection or offline mode.")
        }
        LlmError::ContextTooLong(_) => {
            "The meeting is too long for the chosen model. Pick a larger one in Settings → AI."
                .to_owned()
        }
        LlmError::InvalidOutput { .. } | LlmError::Malformed { .. } | LlmError::Truncated => {
            "The AI's answer could not be read. Try again, or pick another model in Settings → AI."
                .to_owned()
        }
        LlmError::Cancelled => "Writing the notes was cancelled.".to_owned(),
        other => format!("The AI could not write the notes: {other}."),
    }
}

#[cfg(test)]
mod tests {
    use deino_llm::ProviderId;

    use super::*;

    #[test]
    fn a_missing_model_tells_the_user_where_to_fix_it() {
        let error = EnhanceError::Llm(LlmError::Unavailable {
            provider: ProviderId::BuiltIn,
            reason: "no model downloaded".into(),
        });

        assert_eq!(
            error.to_string(),
            "Add an Anthropic key or pick a local model in Settings → AI."
        );
        assert!(!error.is_retryable());
    }

    #[test]
    fn a_rate_limit_is_retried_and_edits_need_consent() {
        let limited = EnhanceError::Llm(LlmError::RateLimited {
            provider: ProviderId::Anthropic,
            retry_after: None,
        });

        assert!(limited.is_retryable());
        assert_eq!(EnhanceError::WouldReplaceEdits.code(), ErrorCode::Conflict);
    }
}
