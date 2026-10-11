//! [`TokenEndpoint`]: exchanging a code, refreshing and revoking (RFC 6749
//! §4.1.3 and §6, RFC 7636 §4.5, RFC 7009).

use std::collections::BTreeMap;
use std::fmt;
use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use deino_net::{NetClient, Request};
use deino_secrets::SecretValue;
use serde::Deserialize;
use url::form_urlencoded;
use zeroize::Zeroizing;

use super::{OAuthError, error_code};
use crate::{ClientAuth, TokenExtras};

/// The largest token answer read: a few tokens and fields.
const MAX_TOKEN_RESPONSE: usize = 64 * 1024;

/// The user's (or Deino's) registration with a provider.
pub struct OAuthClient {
    /// The client ID.
    pub client_id: String,
    /// The client secret, for providers that need one.
    pub client_secret: Option<SecretValue>,
}

impl fmt::Debug for OAuthClient {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OAuthClient")
            .field("client_id", &self.client_id)
            .field("has_secret", &self.client_secret.is_some())
            .finish()
    }
}

/// What a token endpoint issued.
#[derive(Debug)]
pub struct IssuedTokens {
    /// The access token.
    pub access_token: SecretValue,
    /// The refresh token, if one was issued (or rotated).
    pub refresh_token: Option<SecretValue>,
    /// How long the access token lasts, if it expires.
    pub expires_in: Option<Duration>,
    /// The scopes granted, if the provider said (else those requested).
    pub scopes: Option<Vec<String>>,
    /// Other top-level string fields (`api_domain`, `instance_url`).
    pub extras: TokenExtras,
}

/// A provider's token endpoint, as one client.
pub struct TokenEndpoint<'a> {
    /// The client that reaches the provider (scoped to its service).
    pub client: &'a dyn NetClient,
    /// The endpoint URL.
    pub url: &'a str,
    /// Whether, and how, the secret is sent.
    pub auth: ClientAuth,
    /// The registration.
    pub app: &'a OAuthClient,
}

impl TokenEndpoint<'_> {
    /// Exchanges an authorization code (with its PKCE verifier).
    pub async fn exchange(
        &self,
        code: &str,
        redirect_uri: &str,
        verifier: &str,
    ) -> Result<IssuedTokens, OAuthError> {
        let form = [
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("code_verifier", verifier),
        ];
        self.post_for_tokens(&form).await
    }

    /// Refreshes the access token. Providers that rotate refresh tokens
    /// return a new one, which the caller must store before using.
    pub async fn refresh(&self, refresh_token: &SecretValue) -> Result<IssuedTokens, OAuthError> {
        let form = [
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token.expose()),
        ];
        self.post_for_tokens(&form).await
    }

    /// Any grant (`device_code`, ...): posts `form` and parses the tokens.
    pub async fn post_for_tokens(&self, form: &[(&str, &str)]) -> Result<IssuedTokens, OAuthError> {
        let response = self.client.send(self.request(form)?).await?;
        let status = response.status;
        let body = response.bytes(MAX_TOKEN_RESPONSE).await?;
        if !status.is_success() {
            return Err(token_error(&body));
        }
        parse_tokens(&body)
    }

    /// Revokes `token` at an RFC 7009 endpoint `url`.
    pub async fn revoke(&self, url: &str, token: &SecretValue) -> Result<(), OAuthError> {
        let endpoint = TokenEndpoint { url, ..*self };
        let request = endpoint.request(&[("token", token.expose())])?;
        let response = self.client.send(request).await?;
        if response.status.is_success() {
            return Ok(());
        }
        let body = response.bytes(MAX_TOKEN_RESPONSE).await.unwrap_or_default();
        Err(token_error(&body))
    }

    /// A form `POST` with the client's identity per [`ClientAuth`].
    fn request(&self, form: &[(&str, &str)]) -> Result<Request, OAuthError> {
        let secret = self.app.client_secret.as_ref().map(SecretValue::expose);
        let mut body = form_urlencoded::Serializer::new(String::new());
        body.extend_pairs(form);
        let mut request = Request::post(self.url).map_err(|_| OAuthError::BadEndpoint)?;
        match (self.auth, secret) {
            (ClientAuth::SecretInBasicHeader, Some(secret)) => {
                request = request.header("authorization", basic(&self.app.client_id, secret));
            }
            (ClientAuth::SecretInBody, Some(secret)) => {
                body.append_pair("client_id", &self.app.client_id);
                body.append_pair("client_secret", secret);
            }
            _ => {
                body.append_pair("client_id", &self.app.client_id);
            }
        }
        let body = Zeroizing::new(body.finish());
        Ok(request
            .header("content-type", "application/x-www-form-urlencoded")
            .header("accept", "application/json")
            .body(body.as_bytes().to_vec()))
    }
}

/// `Basic base64(id:secret)`, each part form-encoded (RFC 6749 §2.3.1).
fn basic(client_id: &str, secret: &str) -> String {
    let encode = |part: &str| form_urlencoded::byte_serialize(part.as_bytes()).collect::<String>();
    let pair = Zeroizing::new(format!("{}:{}", encode(client_id), encode(secret)));
    format!("Basic {}", STANDARD.encode(pair.as_bytes()))
}

/// The RFC 6749 §5.1 answer.
#[derive(Deserialize)]
struct TokenWire {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<u64>,
    scope: Option<String>,
    #[serde(flatten)]
    rest: BTreeMap<String, serde_json::Value>,
}

/// The RFC 6749 §5.2 error answer.
#[derive(Deserialize)]
struct ErrorWire {
    error: String,
}

fn parse_tokens(body: &[u8]) -> Result<IssuedTokens, OAuthError> {
    let wire: TokenWire = serde_json::from_slice(body).map_err(|_| OAuthError::BadTokenResponse)?;
    let secret = |value: &str| {
        SecretValue::parse_credential(value).map_err(|_| OAuthError::BadTokenResponse)
    };
    let extras = wire
        .rest
        .into_iter()
        .filter(|(name, _)| !matches!(name.as_str(), "id_token" | "token_type"))
        .filter_map(|(name, value)| Some((name, value.as_str()?.to_owned())))
        .collect();
    Ok(IssuedTokens {
        access_token: secret(&wire.access_token)?,
        refresh_token: wire.refresh_token.as_deref().map(secret).transpose()?,
        expires_in: wire.expires_in.map(Duration::from_secs),
        scopes: wire.scope.map(|scope| {
            scope
                .split([' ', ','])
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .collect()
        }),
        extras: TokenExtras::new(extras),
    })
}

fn token_error(body: &[u8]) -> OAuthError {
    match serde_json::from_slice::<ErrorWire>(body) {
        Ok(wire) if wire.error == "invalid_grant" => OAuthError::InvalidGrant,
        Ok(wire) => OAuthError::Provider(error_code(&wire.error)),
        Err(_) => OAuthError::BadTokenResponse,
    }
}

#[cfg(test)]
mod token_tests;
