//! The IPC error model: what the UI can rely on when a command fails.
//!
//! Every module describes its failures with its own `thiserror` enum
//! (PARITY_PLAN §3A) and implements [`ErrorClass`] to say which [`ErrorCode`]
//! each variant maps to. The command boundary converts those errors into
//! [`AppError`], the only error type that crosses IPC: the UI branches on
//! `code` and may show `message`, while source chains stay in the log.
//!
//! See ADR 0009 for why the code set is small and closed.

use serde::{Deserialize, Serialize};

/// What kind of failure an [`AppError`] reports. The UI branches on it. Stored
/// in the `last_error_code` and `error_code` columns of the integration tables.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type, sqlx::Type,
)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum ErrorCode {
    /// The requested entity does not exist.
    NotFound,
    /// The input was rejected, for example an empty title. The user can fix it.
    Invalid,
    /// The request conflicts with the current state, for example a duplicate name.
    Conflict,
    /// A dependency is not ready, for example a model that is still downloading.
    Unavailable,
    /// The OS or the user denied access (microphone, Keychain, files).
    PermissionDenied,
    /// A network request failed.
    Network,
    /// An LLM provider failed or returned unusable output.
    Llm,
    /// The operation was cancelled.
    Cancelled,
    /// A bug or an unexpected failure; the details are in the log.
    Internal,
}

impl ErrorCode {
    /// Every code in declaration order, exported to TypeScript as `ERROR_CODES`
    /// so the UI can validate errors at runtime.
    pub const ALL: [Self; 9] = [
        Self::NotFound,
        Self::Invalid,
        Self::Conflict,
        Self::Unavailable,
        Self::PermissionDenied,
        Self::Network,
        Self::Llm,
        Self::Cancelled,
        Self::Internal,
    ];
}

/// The only error type that crosses IPC.
///
/// `message` is short, user-safe English that the UI may show as is. It never
/// contains transcript text, notes, prompts or secrets; the command boundary
/// logs the full error chain instead.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type, thiserror::Error)]
#[error("{message}")]
pub struct AppError {
    /// What kind of failure this is.
    pub code: ErrorCode,
    /// A user-safe description of the failure.
    pub message: String,
}

impl AppError {
    /// Creates an error from a code and a user-safe message.
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

/// Classifies a module error for IPC. Every module error implements it.
///
/// The error's `Display` text becomes [`AppError::message`], so a variant's
/// `#[error(..)]` text must be user-safe: IDs and counts, never meeting content.
///
/// ```
/// use deino_domain::{AppError, ErrorClass, ErrorCode};
///
/// #[derive(Debug, thiserror::Error)]
/// enum NoteError {
///     #[error("note {0} was not found")]
///     NotFound(u32),
/// }
///
/// impl ErrorClass for NoteError {
///     fn code(&self) -> ErrorCode {
///         match self {
///             Self::NotFound(_) => ErrorCode::NotFound,
///         }
///     }
/// }
///
/// let error = AppError::from(NoteError::NotFound(7));
/// assert_eq!(error, AppError::new(ErrorCode::NotFound, "note 7 was not found"));
/// ```
pub trait ErrorClass: std::error::Error {
    /// The code the UI sees for this error.
    fn code(&self) -> ErrorCode;
}

impl<E: ErrorClass> From<E> for AppError {
    fn from(error: E) -> Self {
        Self::new(error.code(), error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_codes_serialize_as_snake_case_strings() {
        let json = serde_json::to_string(&ErrorCode::PermissionDenied).unwrap();
        assert_eq!(json, r#""permission_denied""#);
    }

    #[test]
    fn app_error_serializes_to_the_ipc_shape() {
        let error = AppError::new(ErrorCode::NotFound, "meeting m-1 was not found");
        let json = serde_json::to_value(&error).unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "code": "not_found", "message": "meeting m-1 was not found" })
        );
    }

    #[test]
    fn all_lists_each_code_once() {
        let distinct: std::collections::HashSet<_> = ErrorCode::ALL.into_iter().collect();
        assert_eq!(distinct.len(), ErrorCode::ALL.len());
    }
}
