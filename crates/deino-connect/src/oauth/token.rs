//! Tokens: what a token endpoint answers, and when an access token expires.

use std::fmt;

use chrono::{DateTime, TimeDelta, Utc};
use serde::Deserialize;

/// An access token is refreshed this long before it expires, so a request
/// never leaves with one that dies in flight.
pub const EXPIRY_MARGIN: TimeDelta = TimeDelta::seconds(60);

/// How long an access token lives when the provider does not say.
const DEFAULT_LIFETIME: TimeDelta = TimeDelta::hours(1);

/// What a sign-in or a refresh produced.
#[derive(Clone, PartialEq, Eq)]
pub struct TokenSet {
    /// The short-lived token that authorizes API requests.
    pub access_token: String,
    /// The long-lived token that gets new access tokens; providers may send
    /// a new one on every refresh (Microsoft) or only on sign-in (Google).
    pub refresh_token: Option<String>,
    /// When `access_token` stops working.
    pub expires_at: DateTime<Utc>,
}

impl TokenSet {
    /// True when the access token is about to expire at `now`.
    pub fn is_stale(&self, now: DateTime<Utc>) -> bool {
        now + EXPIRY_MARGIN >= self.expires_at
    }
}

impl fmt::Debug for TokenSet {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TokenSet")
            .field("expires_at", &self.expires_at)
            .finish_non_exhaustive()
    }
}

/// A token endpoint's JSON answer (RFC 6749 §5.1).
#[derive(Deserialize)]
pub(crate) struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
}

impl TokenResponse {
    /// The token set this answer gives at `now`.
    pub(crate) fn into_tokens(self, now: DateTime<Utc>) -> TokenSet {
        let lifetime = self.expires_in.map_or(DEFAULT_LIFETIME, TimeDelta::seconds);
        TokenSet {
            access_token: self.access_token,
            refresh_token: self.refresh_token,
            expires_at: now + lifetime,
        }
    }
}

/// A token endpoint's JSON error (RFC 6749 §5.2).
#[derive(Deserialize)]
pub(crate) struct TokenErrorResponse {
    pub(crate) error: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-10-09T09:00:00Z")
            .unwrap()
            .to_utc()
    }

    #[test]
    fn a_response_expires_after_its_lifetime_and_turns_stale_a_minute_early() {
        let response: TokenResponse =
            serde_json::from_str(r#"{"access_token":"a","expires_in":3600,"token_type":"Bearer"}"#)
                .unwrap();

        let tokens = response.into_tokens(now());

        assert_eq!(tokens.expires_at, now() + TimeDelta::hours(1));
        assert!(!tokens.is_stale(now() + TimeDelta::minutes(58)));
        assert!(tokens.is_stale(now() + TimeDelta::minutes(59)));
        assert_eq!(tokens.refresh_token, None);
    }

    #[test]
    fn tokens_never_print() {
        let tokens = TokenSet {
            access_token: "secret-access".into(),
            refresh_token: Some("secret-refresh".into()),
            expires_at: now(),
        };
        assert!(!format!("{tokens:?}").contains("secret"));
    }
}
