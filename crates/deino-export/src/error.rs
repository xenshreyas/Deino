//! [`RenderError`]: why a renderer could not produce a file.

use deino_domain::{ErrorClass, ErrorCode};

/// A renderer failed. Text formats cannot fail; DOCX and PDF can, when their
/// writer or the typesetter rejects the document. Messages never quote
/// meeting content, so they are safe to show and to log.
#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    /// The DOCX writer failed to pack the document.
    #[error("the Word document could not be written")]
    Docx(#[source] Box<dyn std::error::Error + Send + Sync>),
    /// The typesetter reported errors in the PDF template (a bug); the count
    /// is logged, never the diagnostics, which can quote the meeting.
    #[error("the PDF could not be typeset ({0} errors)")]
    Typeset(usize),
    /// The PDF writer failed.
    #[error("the PDF could not be written ({0} errors)")]
    Pdf(usize),
    /// The meeting could not be serialized.
    #[error("the meeting could not be written as JSON")]
    Json(#[source] serde_json::Error),
}

impl ErrorClass for RenderError {
    fn code(&self) -> ErrorCode {
        ErrorCode::Internal
    }
}
