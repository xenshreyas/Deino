//! What can go wrong with an outgoing request.
//!
//! Messages are user-safe: they may name a service or a host, never a path,
//! a query, a header or a body. Transport detail stays in the source chain,
//! with any URL already stripped from it.

use std::error::Error;

use deino_domain::{ErrorClass, ErrorCode, ServiceId};

use crate::Status;

/// A transport error carried as a source, for the log only.
pub(crate) type BoxError = Box<dyn Error + Send + Sync>;

/// A request failure.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum NetError {
    /// The URL is not an absolute `http` or `https` URL.
    #[error("the address is not a valid http or https URL")]
    InvalidUrl,
    /// Offline mode is on, so nothing may leave this Mac.
    #[error("offline mode is on, so {0} cannot be reached")]
    Offline(ServiceId),
    /// The user switched the service off.
    #[error("{0} is switched off in Settings → Privacy → Network")]
    ServiceDisabled(ServiceId),
    /// The host is not one the service may reach.
    #[error("{host} is not a host of {service}")]
    HostNotAllowed {
        /// The service the request was made for.
        service: ServiceId,
        /// The host it tried to reach.
        host: String,
    },
    /// The service has no offline download exemption.
    #[error("{0} does not download model files")]
    NotADownloadService(ServiceId),
    /// No connection could be made (server down, DNS, TLS).
    #[error("could not connect")]
    Connect(#[source] BoxError),
    /// The request did not finish within its timeout.
    #[error("the request timed out")]
    Timeout(#[source] BoxError),
    /// The connection broke while sending or receiving.
    #[error("the connection failed")]
    Transport(#[source] BoxError),
    /// The server answered with a status the caller does not accept.
    #[error("the server answered {0}")]
    Status(Status),
    /// A redirect had no usable `Location`, or there were too many.
    #[error("the server redirected badly")]
    Redirect,
    /// The response body is larger than the caller accepts.
    #[error("the response is larger than {limit} bytes")]
    TooLarge {
        /// The limit, in bytes.
        limit: usize,
    },
    /// The request body could not be encoded as JSON (a bug).
    #[error("could not encode the request")]
    Encode(#[source] serde_json::Error),
    /// The response is not the JSON the caller expects.
    #[error("the response is not the expected JSON")]
    Decode(#[source] serde_json::Error),
}

impl NetError {
    /// True when policy stopped the request before anything was sent.
    pub const fn is_blocked(&self) -> bool {
        matches!(
            self,
            Self::Offline(_) | Self::ServiceDisabled(_) | Self::HostNotAllowed { .. }
        )
    }
}

impl ErrorClass for NetError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::Offline(_) | Self::ServiceDisabled(_) | Self::HostNotAllowed { .. } => {
                ErrorCode::PermissionDenied
            }
            Self::InvalidUrl => ErrorCode::Invalid,
            Self::NotADownloadService(_) | Self::Encode(_) => ErrorCode::Internal,
            _ => ErrorCode::Network,
        }
    }
}

#[cfg(test)]
mod tests {
    use deino_domain::AppError;

    use super::*;

    #[test]
    fn a_switched_off_service_tells_the_user_where_to_switch_it_on() {
        let error = AppError::from(NetError::ServiceDisabled(ServiceId::Slack));

        assert_eq!(error.code, ErrorCode::PermissionDenied);
        assert_eq!(
            error.message,
            "slack is switched off in Settings → Privacy → Network"
        );
    }

    #[test]
    fn policy_refusals_are_blocked_and_network_failures_are_not() {
        assert!(NetError::Offline(ServiceId::LlmOpenAi).is_blocked());
        assert!(!NetError::Status(Status::NOT_FOUND).is_blocked());
        assert_eq!(
            NetError::Status(Status::NOT_FOUND).code(),
            ErrorCode::Network
        );
    }
}
