//! The sign-in flow: PKCE, the loopback redirect, the user's browser and
//! the code exchange, in one call.

use std::time::Duration;

use deino_domain::Clock;
use deino_net::NetClient;

use super::client::Grant;
use super::loopback::LoopbackRedirect;
use super::pkce::random_state;
use super::{OAuthClient, OAuthError, Pkce, TokenSet};

/// How long the user has to sign in and approve in the browser.
pub const SIGN_IN_TIMEOUT: Duration = Duration::from_mins(5);

/// Opens a page in the user's browser (the app opens the system default).
pub trait BrowserOpener: Send + Sync {
    /// Opens `url`; false when no browser could be opened.
    fn open(&self, url: &str) -> bool;
}

/// What a sign-in needs.
#[derive(Clone, Copy)]
pub struct SignIn<'a> {
    /// The provider and the user's client registration.
    pub client: &'a OAuthClient,
    /// Reaches the provider's token endpoint (a scoped client).
    pub http: &'a dyn NetClient,
    /// Shows the sign-in page.
    pub browser: &'a dyn BrowserOpener,
    /// Stamps when the tokens expire.
    pub clock: &'a dyn Clock,
}

/// Signs the user in: opens the provider's page in their browser, waits up
/// to [`SIGN_IN_TIMEOUT`] for the loopback redirect, and trades its code
/// (with the PKCE verifier) for tokens.
pub async fn sign_in(sign_in: SignIn<'_>) -> Result<TokenSet, OAuthError> {
    let pkce = Pkce::generate()?;
    let state = random_state()?;
    let redirect = LoopbackRedirect::bind(sign_in.client.provider().redirect_host).await?;
    let redirect_uri = redirect.redirect_uri().to_owned();
    let page = sign_in.client.authorize_url(&redirect_uri, &pkce, &state)?;
    if !sign_in.browser.open(page.as_str()) {
        return Err(OAuthError::BrowserUnavailable);
    }
    let code = redirect.receive(&state, SIGN_IN_TIMEOUT).await?;
    let grant = Grant::Code {
        code: &code,
        redirect_uri: &redirect_uri,
        verifier: pkce.verifier(),
    };
    let tokens = sign_in
        .client
        .exchange(sign_in.http, grant, sign_in.clock.now())
        .await?;
    log::info!(target: "oauth", "signed in to {}", sign_in.client.provider().name);
    Ok(tokens)
}
