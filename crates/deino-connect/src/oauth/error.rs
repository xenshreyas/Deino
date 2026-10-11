//! What can go wrong while connecting an account.
//!
//! Messages are user-safe: they name the provider and the step, never a
//! code, a token or a URL.

use deino_domain::{ErrorClass, ErrorCode};
use deino_net::NetError;

/// An OAuth failure.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum OAuthError {
    /// The provider's client ID (or Google's client secret) is not set.
    #[error("set up {0} first: paste its client ID in Settings → Calendar")]
    NotConfigured(&'static str),
    /// No browser could be opened for the sign-in page.
    #[error("could not open a browser to sign in")]
    BrowserUnavailable,
    /// The user declined, or the provider refused the request.
    #[error("{provider} did not allow access ({reason})")]
    Denied {
        /// The provider.
        provider: &'static str,
        /// The provider's error code, for example `access_denied`.
        reason: String,
    },
    /// The redirect carried another sign-in's state (a stale or forged tab).
    #[error("the sign-in answer did not match this request; try again")]
    StateMismatch,
    /// The user did not finish signing in in time.
    #[error("signing in took too long; try again")]
    TimedOut,
    /// The provider answered with something that is not a token.
    #[error("{0} sent an answer Deino could not read")]
    InvalidResponse(&'static str),
    /// The provider no longer accepts the saved sign-in (revoked or expired).
    #[error("{0} needs you to connect again")]
    Revoked(&'static str),
    /// The loopback redirect could not listen or read.
    #[error("could not receive the sign-in answer on this Mac")]
    Loopback(#[source] std::io::Error),
    /// The operating system could not provide random bytes.
    #[error("could not create a secure sign-in request")]
    Random,
    /// The request to the provider failed.
    #[error(transparent)]
    Net(#[from] NetError),
}

impl ErrorClass for OAuthError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::NotConfigured(_) => ErrorCode::Invalid,
            Self::Denied { .. } | Self::Revoked(_) => ErrorCode::PermissionDenied,
            Self::StateMismatch | Self::TimedOut => ErrorCode::Cancelled,
            Self::BrowserUnavailable | Self::Loopback(_) | Self::Random => ErrorCode::Unavailable,
            Self::InvalidResponse(_) | Self::Net(_) => ErrorCode::Network,
        }
    }
}
