//! OAuth 2.0 for a desktop app: authorization code with PKCE over a loopback
//! redirect (RFC 6749, RFC 7636, RFC 8252). The minimal helper ORG-01's
//! calendar connectors need, shaped for INT-00 to adopt for every other
//! integration.
//!
//! - [`OAuthProvider`]: an authorization server ([`GOOGLE`], [`MICROSOFT`]):
//!   its endpoints, read-only scopes and redirect host.
//! - [`OAuthClient`]: a provider with the user's own client ID (PARITY_PLAN
//!   §9: the user registers the app); builds the sign-in page and trades a
//!   [`Grant`] for a [`TokenSet`] through a `deino-net` scoped client.
//! - [`sign_in`]: the whole flow; [`BrowserOpener`] is the port to the
//!   user's browser. [`LoopbackRedirect`] receives the answer on this Mac.
//!
//! Nothing here stores tokens or logs them: callers keep the refresh token
//! in the Keychain and the access token in memory.

mod client;
mod error;
mod flow;
mod loopback;
mod pkce;
mod provider;
mod token;

pub use client::{Grant, OAuthClient};
pub use error::OAuthError;
pub use flow::{BrowserOpener, SIGN_IN_TIMEOUT, SignIn, sign_in};
pub use loopback::LoopbackRedirect;
pub use pkce::{Pkce, random_state};
pub use provider::{GOOGLE, MICROSOFT, OAuthProvider};
pub use token::{EXPIRY_MARGIN, TokenSet};

#[cfg(test)]
mod oauth_tests;
