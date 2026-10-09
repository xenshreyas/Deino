//! [`OAuthClient`]: one provider with the user's own client ID: the sign-in
//! page's address, and token requests (code exchange and refresh).

use std::fmt;

use chrono::{DateTime, Utc};
use deino_net::{NetClient, NetError, Request};
use url::Url;
use url::form_urlencoded::Serializer;

use super::token::{TokenErrorResponse, TokenResponse};
use super::{OAuthError, OAuthProvider, Pkce, TokenSet};

/// What is traded for tokens.
#[derive(Clone, Copy)]
pub enum Grant<'a> {
    /// A sign-in's code, with the redirect it came to and the PKCE verifier.
    Code {
        /// The code from the redirect.
        code: &'a str,
        /// The redirect URI the sign-in page was given.
        redirect_uri: &'a str,
        /// The verifier behind the challenge the page was given.
        verifier: &'a str,
    },
    /// A saved refresh token.
    Refresh(&'a str),
}

/// A provider and the user's registration with it. Never printed with its
/// secret.
#[derive(Clone)]
pub struct OAuthClient {
    provider: &'static OAuthProvider,
    client_id: String,
    client_secret: Option<String>,
}

impl OAuthClient {
    /// A client of `provider` registered as `client_id` (and, for Google's
    /// desktop clients, `client_secret`).
    pub fn new(
        provider: &'static OAuthProvider,
        client_id: String,
        client_secret: Option<String>,
    ) -> Self {
        Self {
            provider,
            client_id,
            client_secret,
        }
    }

    /// The provider.
    pub fn provider(&self) -> &'static OAuthProvider {
        self.provider
    }

    /// The sign-in page for a PKCE pair, a loopback redirect and a state.
    pub fn authorize_url(
        &self,
        redirect_uri: &str,
        pkce: &Pkce,
        state: &str,
    ) -> Result<Url, OAuthError> {
        let mut url = Url::parse(self.provider.authorize_url)
            .map_err(|_| OAuthError::InvalidResponse(self.provider.name))?;
        url.query_pairs_mut()
            .append_pair("response_type", "code")
            .append_pair("client_id", &self.client_id)
            .append_pair("redirect_uri", redirect_uri)
            .append_pair("scope", &self.provider.scopes.join(" "))
            .append_pair("code_challenge", pkce.challenge())
            .append_pair("code_challenge_method", "S256")
            .append_pair("state", state)
            .extend_pairs(self.provider.authorize_params.iter().copied());
        Ok(url)
    }

    /// Trades `grant` for tokens at the provider's token endpoint.
    pub async fn exchange(
        &self,
        http: &dyn NetClient,
        grant: Grant<'_>,
        now: DateTime<Utc>,
    ) -> Result<TokenSet, OAuthError> {
        let request = Request::post(self.provider.token_url)?
            .header("content-type", "application/x-www-form-urlencoded")
            .header("accept", "application/json")
            .body(self.form(grant));
        let response = http.send(request).await?;
        // RFC 6749 §5.2: a refused grant is a 400 (401 for a bad client).
        if matches!(response.status.as_u16(), 400 | 401) {
            return Err(self.refusal(response, grant).await);
        }
        let answer: TokenResponse =
            response
                .error_for_status()?
                .json()
                .await
                .map_err(|error| match error {
                    NetError::Decode(_) => OAuthError::InvalidResponse(self.provider.name),
                    other => OAuthError::Net(other),
                })?;
        Ok(answer.into_tokens(now))
    }

    /// The form body of a token request.
    fn form(&self, grant: Grant<'_>) -> String {
        let mut form = Serializer::new(String::new());
        form.append_pair("client_id", &self.client_id);
        if let (true, Some(secret)) = (self.provider.sends_client_secret, &self.client_secret) {
            form.append_pair("client_secret", secret);
        }
        match grant {
            Grant::Code {
                code,
                redirect_uri,
                verifier,
            } => form
                .append_pair("grant_type", "authorization_code")
                .append_pair("code", code)
                .append_pair("redirect_uri", redirect_uri)
                .append_pair("code_verifier", verifier),
            Grant::Refresh(token) => form
                .append_pair("grant_type", "refresh_token")
                .append_pair("refresh_token", token),
        };
        form.finish()
    }

    /// The error a 400 or 401 answer means: a refused refresh is a revoked
    /// connection, a refused code is a denial.
    async fn refusal(&self, response: deino_net::Response, grant: Grant<'_>) -> OAuthError {
        let reason = match response.json::<TokenErrorResponse>().await {
            Ok(answer) => answer.error,
            Err(_) => "invalid_request".to_owned(),
        };
        match grant {
            Grant::Refresh(_) if reason == "invalid_grant" => {
                OAuthError::Revoked(self.provider.name)
            }
            _ => OAuthError::Denied {
                provider: self.provider.name,
                reason,
            },
        }
    }
}

impl fmt::Debug for OAuthClient {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OAuthClient")
            .field("provider", &self.provider.name)
            .finish_non_exhaustive()
    }
}
