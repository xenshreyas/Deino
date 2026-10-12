//! The API's error model: every failure is a status and
//! `{"error": {"code", "message"}}`, the shape the CLI's MCP tools use.
//!
//! Service errors arrive as [`AppError`] and keep its [`ErrorCode`] (as
//! snake case); the HTTP layer adds four codes of its own: `unauthorized`,
//! `read_only_key`, `locked` and `forbidden_host`.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use deino_domain::{AppError, ErrorCode};
use serde::Serialize;

/// A failed request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApiError {
    /// The HTTP status.
    pub status: StatusCode,
    /// The machine-readable code.
    pub code: &'static str,
    /// User-safe English; never meeting content or a secret.
    pub message: String,
}

impl ApiError {
    fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
        }
    }

    /// No key, or a wrong, unknown or revoked one.
    pub fn unauthorized() -> Self {
        Self::new(
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            "send a live API key as `Authorization: Bearer deino_…`",
        )
    }

    /// A read-only key asked to change something.
    pub fn read_only() -> Self {
        Self::new(
            StatusCode::FORBIDDEN,
            "read_only_key",
            "this API key can only read; make a read-write key in Settings → Developer",
        )
    }

    /// Deino is locked (PRV-02).
    pub fn locked() -> Self {
        Self::new(
            StatusCode::LOCKED,
            "locked",
            "Deino is locked; unlock it on the Mac and try again",
        )
    }

    /// The `Host` header is not this Mac's loopback address.
    pub fn forbidden_host() -> Self {
        Self::new(
            StatusCode::FORBIDDEN,
            "forbidden_host",
            "the API answers only requests to 127.0.0.1 or localhost",
        )
    }

    /// The request breaks a rule the HTTP layer checks.
    pub fn invalid(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, "invalid", message)
    }

    /// No such route or resource.
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, "not_found", message)
    }
}

impl From<AppError> for ApiError {
    fn from(error: AppError) -> Self {
        let (status, code) = match error.code {
            ErrorCode::NotFound => (StatusCode::NOT_FOUND, "not_found"),
            ErrorCode::Invalid => (StatusCode::BAD_REQUEST, "invalid"),
            ErrorCode::Conflict => (StatusCode::CONFLICT, "conflict"),
            ErrorCode::Unavailable => (StatusCode::SERVICE_UNAVAILABLE, "unavailable"),
            ErrorCode::PermissionDenied => (StatusCode::FORBIDDEN, "permission_denied"),
            ErrorCode::Network => (StatusCode::BAD_GATEWAY, "network"),
            ErrorCode::Llm => (StatusCode::BAD_GATEWAY, "llm"),
            ErrorCode::Cancelled => (StatusCode::CONFLICT, "cancelled"),
            ErrorCode::Internal => (StatusCode::INTERNAL_SERVER_ERROR, "internal"),
        };
        if error.code == ErrorCode::Internal {
            log::warn!(target: "api", "request failed: {}", error.message);
        }
        Self::new(status, code, error.message)
    }
}

/// Converts any module error the services return.
pub(crate) fn app<E: Into<AppError>>(error: E) -> ApiError {
    ApiError::from(error.into())
}

#[derive(Serialize)]
struct Envelope<'a> {
    error: Body<'a>,
}

#[derive(Serialize)]
struct Body<'a> {
    code: &'a str,
    message: &'a str,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let envelope = Envelope {
            error: Body {
                code: self.code,
                message: &self.message,
            },
        };
        (self.status, Json(envelope)).into_response()
    }
}

/// What a handler returns.
pub type ApiResult<T> = Result<T, ApiError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_errors_keep_their_code_and_get_a_status() {
        let error = ApiError::from(AppError::new(
            ErrorCode::NotFound,
            "meeting m was not found",
        ));
        assert_eq!(error.status, StatusCode::NOT_FOUND);
        assert_eq!(error.code, "not_found");
        let conflict = ApiError::from(AppError::new(ErrorCode::Conflict, "taken"));
        assert_eq!(conflict.status, StatusCode::CONFLICT);
    }
}
