//! OAuth 2.0 for a desktop app with no server (RFC 8252): authorization code
//! with PKCE (RFC 7636) through a loopback redirect, token refresh, RFC 7009
//! revocation, and a device authorization (RFC 8628) helper.
//!
//! - [`Pkce`]: a fresh verifier and its S256 challenge per sign-in.
//! - [`authorization_url`]: the URL the browser opens.
//! - [`LoopbackListener`]: a one-shot HTTP listener on `127.0.0.1` (and
//!   `[::1]` for `localhost`) that waits for `?code=…&state=…`, checks the
//!   state and answers the browser with a page that says what happened.
//! - [`TokenEndpoint`]: code exchange, refresh and revocation, with or
//!   without a client secret ([`ClientAuth`](crate::ClientAuth)).
//! - [`AuthorizationFlow`]: the whole sign-in, from opening the browser to
//!   [`IssuedTokens`].
//! - [`device`]: start a device authorization and poll for its tokens.
//!
//! Nothing here stores anything; the service puts [`IssuedTokens`] in the
//! Keychain. Tokens are [`SecretValue`](deino_secrets::SecretValue)s from
//! the moment they are parsed.

pub mod device;
mod flow;
mod loopback;
mod pkce;
mod token;

use deino_domain::{ErrorClass, ErrorCode};
use deino_net::NetError;

pub use flow::{AuthorizationFlow, AuthorizationRequest, CALLBACK_TIMEOUT, authorization_url};
pub use loopback::LoopbackListener;
pub use pkce::{Pkce, random_token};
pub use token::{IssuedTokens, OAuthClient, TokenEndpoint};

/// Why a sign-in or a refresh failed. Messages are user-safe: a provider's
/// error code (`invalid_scope`) at most, never a token or a full URL.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum OAuthError {
    /// The OS had no randomness to give (practically never).
    #[error("could not create a secure sign-in request")]
    Random,
    /// The fixed redirect port is taken by another app.
    #[error("port {0} on this Mac is in use; close the app using it, or change the port")]
    PortInUse(u16),
    /// The loopback listener could not be opened or read.
    #[error("could not listen for the browser on this Mac")]
    Listen(#[source] std::io::Error),
    /// A configured endpoint is not a valid URL (a bug or a bad setting).
    #[error("the provider's sign-in address is not valid")]
    BadEndpoint,
    /// The browser could not be opened.
    #[error("the browser could not be opened")]
    Browser,
    /// The browser did not come back in time.
    #[error("the browser did not come back within 5 minutes")]
    TimedOut,
    /// The user declined in the browser.
    #[error("access was not granted")]
    AccessDenied,
    /// The provider answered with an OAuth error code.
    #[error("the provider answered `{0}`")]
    Provider(String),
    /// The code or refresh token is no longer valid (`invalid_grant`).
    #[error("the provider no longer accepts this sign-in")]
    InvalidGrant,
    /// The token answer is not what RFC 6749 describes.
    #[error("the provider's answer could not be read")]
    BadTokenResponse,
    /// The device code expired before the user approved it.
    #[error("the code expired before it was approved")]
    Expired,
    /// The request failed (policy, transport).
    #[error(transparent)]
    Net(#[from] NetError),
}

impl ErrorClass for OAuthError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::Random | Self::Listen(_) | Self::BadEndpoint => ErrorCode::Internal,
            Self::PortInUse(_) | Self::Browser => ErrorCode::Unavailable,
            Self::TimedOut | Self::Expired => ErrorCode::Cancelled,
            Self::AccessDenied | Self::InvalidGrant => ErrorCode::PermissionDenied,
            Self::Provider(_) => ErrorCode::Invalid,
            Self::BadTokenResponse => ErrorCode::Network,
            Self::Net(error) => error.code(),
        }
    }
}

/// Keeps only the characters RFC 6749 allows in an error code, so a hostile
/// answer cannot put arbitrary text in a message.
fn error_code(raw: &str) -> String {
    raw.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .take(64)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_error_codes_are_reduced_to_safe_characters() {
        assert_eq!(error_code("invalid_scope"), "invalid_scope");
        assert_eq!(error_code("bad<script>\nx"), "badscriptx");
    }
}
