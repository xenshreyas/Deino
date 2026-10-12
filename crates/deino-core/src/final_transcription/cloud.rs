//! The cloud transcription port (CAP-14): one finished track sent to a
//! speech-to-text service the user turned on with their own key, timed
//! utterances back, each with the voice the service heard it in.
//!
//! `deino-cloud-transcription` implements [`CloudTranscriber`] for
//! ElevenLabs Scribe and [`CloudChoice`] over the settings and the Keychain;
//! the `final_transcription` stage asks the choice once per run.

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use deino_domain::{ErrorClass, ErrorCode, MeetingId};
use tokio_util::sync::CancellationToken;

use super::{EngineError, HeardSpan};

/// What every cloud failure's status line starts with; the meeting page
/// offers Retry on a transcript step skipped with it.
pub const CLOUD_FAILED: &str = "ElevenLabs transcription failed";

/// One finished track to send.
#[derive(Clone, Debug)]
pub struct CloudRequest {
    /// The finalized file (`mic.m4a`, `system.m4a` or `audio.mp4`).
    pub audio: PathBuf,
    /// Tell the voices on the track apart: system audio and the mixed file
    /// may hold several people; the microphone is the user.
    pub diarize: bool,
    /// Fires when the stage is cancelled.
    pub cancel: CancellationToken,
}

/// One utterance the service heard.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CloudSpan {
    /// When, and what was said.
    pub span: HeardSpan,
    /// The service's name for the voice (`speaker_0`), when it diarized.
    pub voice: Option<String>,
}

/// Why a cloud transcription failed. The messages are user-safe: never a
/// key, a path or what was said.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CloudError {
    /// No key is saved, or macOS refused to hand it over.
    #[error("Save your ElevenLabs API key in Settings → Advanced → ElevenLabs first.")]
    MissingKey,
    /// The service refused the key (401).
    #[error("ElevenLabs refused the API key. Check it and save it again.")]
    KeyRefused,
    /// The key may not use speech to text, or the plan does not include it (403).
    #[error("This ElevenLabs API key is not allowed to use speech to text.")]
    NotPermitted,
    /// The account has no credits left (402).
    #[error("Your ElevenLabs account is out of credits.")]
    NoCredits,
    /// The track is larger or longer than the service accepts.
    #[error("The recording is too large for ElevenLabs.")]
    TooLarge,
    /// The service rejected the request (400, 422): a retired model, say.
    #[error("ElevenLabs could not transcribe this recording.")]
    Rejected,
    /// Too many requests (429); retried with backoff.
    #[error("ElevenLabs is busy right now.")]
    RateLimited,
    /// The service failed (5xx); retried with backoff.
    #[error("ElevenLabs is not available right now.")]
    Unavailable,
    /// No answer: connection, TLS or timeout; retried with backoff.
    #[error("ElevenLabs could not be reached.")]
    Network(#[source] EngineError),
    /// Offline mode is on, or the service is switched off.
    #[error("ElevenLabs is switched off in Settings → Privacy → Network.")]
    Blocked,
    /// The answer is not what the API documents.
    #[error("ElevenLabs answered in a way Deino cannot read.")]
    Unreadable,
    /// The track could not be read from disk.
    #[error("The recording could not be read.")]
    Audio(#[source] std::io::Error),
    /// The run was cancelled.
    #[error("Transcription was cancelled.")]
    Cancelled,
}

impl CloudError {
    /// Whether the pipeline should try again: rate limits, server failures
    /// and the network pass on their own.
    pub const fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::RateLimited | Self::Unavailable | Self::Network(_)
        )
    }

    /// The meeting's status line once the stage gives up on the cloud and
    /// keeps the transcript made on this Mac. Always starts with
    /// [`CLOUD_FAILED`].
    pub const fn status(&self) -> &'static str {
        match self {
            Self::MissingKey => "ElevenLabs transcription failed: no API key is saved.",
            Self::KeyRefused => "ElevenLabs transcription failed: the API key was refused.",
            Self::NotPermitted => {
                "ElevenLabs transcription failed: the API key may not use speech to text."
            }
            Self::NoCredits => "ElevenLabs transcription failed: the account is out of credits.",
            Self::TooLarge => "ElevenLabs transcription failed: the recording is too large.",
            Self::Rejected => "ElevenLabs transcription failed: the recording was rejected.",
            Self::RateLimited => "ElevenLabs transcription failed: the service is busy.",
            Self::Unavailable => "ElevenLabs transcription failed: the service is unavailable.",
            Self::Network(_) => {
                "ElevenLabs transcription failed: the service could not be reached."
            }
            Self::Blocked => "ElevenLabs transcription failed: it is switched off in Network.",
            Self::Unreadable => "ElevenLabs transcription failed: the answer could not be read.",
            Self::Audio(_) => "ElevenLabs transcription failed: the recording could not be read.",
            Self::Cancelled => "ElevenLabs transcription failed: it was cancelled.",
        }
    }
}

impl ErrorClass for CloudError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::MissingKey | Self::NoCredits => ErrorCode::Unavailable,
            Self::KeyRefused | Self::NotPermitted | Self::Blocked => ErrorCode::PermissionDenied,
            Self::TooLarge | Self::Rejected => ErrorCode::Invalid,
            Self::RateLimited | Self::Unavailable | Self::Network(_) => ErrorCode::Network,
            Self::Unreadable | Self::Audio(_) => ErrorCode::Internal,
            Self::Cancelled => ErrorCode::Cancelled,
        }
    }
}

/// A cloud speech-to-text service.
#[async_trait]
pub trait CloudTranscriber: Send + Sync {
    /// The utterances of one track, in time order.
    async fn transcribe(&self, request: CloudRequest) -> Result<Vec<CloudSpan>, CloudError>;
}

/// The cloud service for one run of the stage.
#[derive(Clone)]
pub struct CloudPass {
    /// The service.
    pub transcriber: Arc<dyn CloudTranscriber>,
    /// The user asked for it on this meeting ("Re-transcribe with
    /// ElevenLabs"): it replaces an edited transcript too (the original is
    /// kept), and a live transcript even when the setting is off.
    pub requested: bool,
}

impl std::fmt::Debug for CloudPass {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CloudPass")
            .field("requested", &self.requested)
            .finish_non_exhaustive()
    }
}

/// Which cloud service, if any, transcribes a meeting after it ends.
#[async_trait]
pub trait CloudChoice: Send + Sync {
    /// The service for `meeting`: the one turned on in Settings with its
    /// key saved, or the one the user asked for on this meeting.
    async fn for_meeting(&self, meeting: &MeetingId) -> Option<CloudPass>;

    /// The stage is done with `meeting` (written, kept or given up): a
    /// request made on it is spent.
    fn finished(&self, meeting: &MeetingId);
}

/// No cloud service: every transcript is made on this Mac.
#[derive(Clone, Copy, Debug, Default)]
pub struct LocalOnly;

#[async_trait]
impl CloudChoice for LocalOnly {
    async fn for_meeting(&self, _meeting: &MeetingId) -> Option<CloudPass> {
        None
    }

    fn finished(&self, _meeting: &MeetingId) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_status_line_names_the_failure_the_same_way() {
        let errors = [
            CloudError::MissingKey,
            CloudError::KeyRefused,
            CloudError::NotPermitted,
            CloudError::NoCredits,
            CloudError::TooLarge,
            CloudError::Rejected,
            CloudError::RateLimited,
            CloudError::Unavailable,
            CloudError::Network("timeout".into()),
            CloudError::Blocked,
            CloudError::Unreadable,
            CloudError::Audio(std::io::Error::other("gone")),
            CloudError::Cancelled,
        ];
        for error in &errors {
            assert!(error.status().starts_with(CLOUD_FAILED), "{error:?}");
        }
        let retried: Vec<bool> = errors.iter().map(CloudError::is_retryable).collect();
        assert_eq!(retried.iter().filter(|retry| **retry).count(), 3);
        assert_eq!(CloudError::KeyRefused.code(), ErrorCode::PermissionDenied);
    }
}
