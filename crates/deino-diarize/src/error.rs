//! What can go wrong while diarizing.

use deino_domain::{ErrorClass, ErrorCode};

/// Why diarization failed.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum DiarizeError {
    /// ONNX Runtime failed to load or run a model.
    #[error("the speaker model failed")]
    Onnx(#[from] ort::Error),
    /// A model returned a tensor of an unexpected shape.
    #[error("the speaker model returned {actual} values where {expected} were expected")]
    OutputShape {
        /// Values the pipeline needs.
        expected: usize,
        /// Values the model returned.
        actual: usize,
    },
    /// The feature extractor rejected the audio.
    #[error("speaker features could not be computed")]
    Features(#[from] knf_rs::FbankError),
    /// The audio source could not be read.
    #[error("the audio could not be read")]
    Audio(#[source] Box<dyn std::error::Error + Send + Sync>),
    /// The caller cancelled the run.
    #[error("diarization was cancelled")]
    Cancelled,
}

impl ErrorClass for DiarizeError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::Cancelled => ErrorCode::Cancelled,
            _ => ErrorCode::Internal,
        }
    }
}
