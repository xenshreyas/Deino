//! [`AuthorizationFlow`]: one sign-in, from the browser to the tokens.

use std::time::Duration;

use deino_net::NetClient;
use tokio_util::sync::CancellationToken;
use url::Url;

use super::{
    IssuedTokens, LoopbackListener, OAuthClient, OAuthError, Pkce, TokenEndpoint, random_token,
};
use crate::{Browser, OAuthSpec};

/// How long the user has to finish in the browser.
pub const CALLBACK_TIMEOUT: Duration = Duration::from_mins(5);

/// Random bytes in `state`: 128 bits, unguessable by another page.
const STATE_BYTES: usize = 16;

/// What goes into the authorization URL.
#[derive(Clone, Copy, Debug)]
pub struct AuthorizationRequest<'a> {
    /// The provider's endpoints and scopes.
    pub spec: &'a OAuthSpec,
    /// The client ID.
    pub client_id: &'a str,
    /// Where the browser comes back to.
    pub redirect_uri: &'a Url,
    /// The value that ties the redirect to this sign-in.
    pub state: &'a str,
    /// The PKCE S256 challenge.
    pub challenge: &'a str,
}

/// The URL the browser opens to approve access (RFC 6749 §4.1.1, RFC 7636 §4.3).
pub fn authorization_url(request: &AuthorizationRequest<'_>) -> Result<Url, OAuthError> {
    let spec = request.spec;
    let mut url = Url::parse(&spec.authorize_url).map_err(|_| OAuthError::BadEndpoint)?;
    let separator = spec.scope_separator.to_string();
    let scopes: Vec<&str> = spec.scopes.iter().map(|scope| scope.id.as_str()).collect();
    url.query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("client_id", request.client_id)
        .append_pair("redirect_uri", request.redirect_uri.as_str())
        .append_pair(spec.scope_param, &scopes.join(&separator))
        .append_pair("state", request.state)
        .append_pair("code_challenge", request.challenge)
        .append_pair("code_challenge_method", "S256")
        .extend_pairs(spec.extra_params.iter().copied());
    Ok(url)
}

/// One sign-in (see [`AuthorizationFlow::run`]).
pub struct AuthorizationFlow<'a> {
    /// The provider's OAuth rules.
    pub spec: &'a OAuthSpec,
    /// The registration to sign in with.
    pub app: &'a OAuthClient,
    /// The client that reaches the token endpoint.
    pub client: &'a dyn NetClient,
    /// Opens the authorization URL.
    pub browser: &'a dyn Browser,
    /// The redirect port the user registered, replacing the provider's default.
    pub port: Option<u16>,
}

impl AuthorizationFlow<'_> {
    /// Binds the loopback listener, opens the browser, waits for the
    /// redirect (at most [`CALLBACK_TIMEOUT`]) and exchanges the code.
    /// `Ok(None)` when `cancel` fires first.
    pub async fn run(
        &self,
        cancel: &CancellationToken,
    ) -> Result<Option<IssuedTokens>, OAuthError> {
        let listener = LoopbackListener::bind(self.spec.redirect, self.port).await?;
        let redirect_uri = listener.redirect_uri().clone();
        let pkce = Pkce::generate()?;
        let state = random_token(STATE_BYTES)?;
        let url = authorization_url(&AuthorizationRequest {
            spec: self.spec,
            client_id: &self.app.client_id,
            redirect_uri: &redirect_uri,
            state: &state,
            challenge: pkce.challenge(),
        })?;
        self.browser.open(&url).await?;
        let Some(code) = listener.wait(&state, cancel, CALLBACK_TIMEOUT).await? else {
            return Ok(None);
        };
        let endpoint = TokenEndpoint {
            client: self.client,
            url: &self.spec.token_url,
            auth: self.spec.client_auth,
            app: self.app,
        };
        let tokens = endpoint
            .exchange(&code, redirect_uri.as_str(), pkce.verifier())
            .await?;
        Ok(Some(tokens))
    }
}

#[cfg(test)]
mod flow_tests;
