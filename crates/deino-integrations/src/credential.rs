//! [`Credential`] and [`CredentialVault`]: one Keychain item per connection.
//!
//! A connection's whole credential (access and refresh token, or a pasted
//! token and its username) is one JSON item under
//! `integration.<provider>.<connection id>`, so a rotated refresh token and
//! its access token are replaced in one write: there is never a moment when
//! the Keychain holds a new access token beside a spent refresh token. The
//! user's client secret for a provider is `integration.<provider>.client-secret`.

use std::sync::Arc;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use deino_domain::{ConnectionId, IntegrationProvider};
use deino_secrets::{CredentialKey, CredentialStore, SecretError, SecretValue};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::TokenScheme;

/// A connection's credential.
#[derive(Debug)]
pub enum Credential {
    /// Tokens from an OAuth sign-in.
    OAuth {
        /// The access token, sent as `Bearer`.
        access: SecretValue,
        /// The refresh token, if the provider issued one.
        refresh: Option<SecretValue>,
    },
    /// A pasted token, with the username some providers pair it with.
    Token {
        /// The token or API key.
        token: SecretValue,
        /// The username for HTTP Basic (Jira's email), if any.
        username: Option<String>,
    },
}

impl Credential {
    /// The `Authorization` header value; a pasted token is sent per `scheme`.
    pub fn authorization(&self, scheme: TokenScheme) -> Result<SecretValue, SecretError> {
        let header = Zeroizing::new(match self {
            Self::OAuth { access, .. } => format!("Bearer {}", access.expose()),
            Self::Token { token, username } => match scheme {
                TokenScheme::Bearer => format!("Bearer {}", token.expose()),
                TokenScheme::Bare => token.expose().to_owned(),
                TokenScheme::Basic => {
                    let pair = Zeroizing::new(format!(
                        "{}:{}",
                        username.as_deref().unwrap_or_default(),
                        token.expose()
                    ));
                    format!("Basic {}", STANDARD.encode(pair.as_bytes()))
                }
            },
        });
        SecretValue::parse_credential(&header)
    }

    /// The refresh token, if there is one.
    pub fn refresh_token(&self) -> Option<&SecretValue> {
        match self {
            Self::OAuth { refresh, .. } => refresh.as_ref(),
            Self::Token { .. } => None,
        }
    }

    /// The token to revoke at the provider: the refresh token when there is
    /// one (revoking it ends the whole grant), else the access token.
    pub fn revocable(&self) -> &SecretValue {
        match self {
            Self::OAuth { access, refresh } => refresh.as_ref().unwrap_or(access),
            Self::Token { token, .. } => token,
        }
    }

    fn encode(&self) -> Result<SecretValue, SecretError> {
        let wire = match self {
            Self::OAuth { access, refresh } => Wire::OAuth {
                access: access.expose(),
                refresh: refresh.as_ref().map(SecretValue::expose),
            },
            Self::Token { token, username } => Wire::Token {
                token: token.expose(),
                username: username.as_deref(),
            },
        };
        let json = Zeroizing::new(serde_json::to_string(&wire).unwrap_or_default());
        SecretValue::parse_credential(&json)
    }

    fn decode(value: &SecretValue) -> Option<Self> {
        let wire: Wire<'_> = serde_json::from_str(value.expose()).ok()?;
        let secret = |text: &str| SecretValue::parse_credential(text).ok();
        Some(match wire {
            Wire::OAuth { access, refresh } => Self::OAuth {
                access: secret(access)?,
                refresh: refresh.and_then(secret),
            },
            Wire::Token { token, username } => Self::Token {
                token: secret(token)?,
                username: username.map(str::to_owned),
            },
        })
    }
}

/// The stored JSON. Borrowed, so no unzeroized copy of a token outlives it.
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Wire<'a> {
    #[serde(rename = "oauth")]
    OAuth {
        access: &'a str,
        refresh: Option<&'a str>,
    },
    Token {
        token: &'a str,
        username: Option<&'a str>,
    },
}

/// Connection credentials and client secrets in the Keychain.
#[derive(Clone)]
pub struct CredentialVault {
    store: Arc<dyn CredentialStore>,
}

impl CredentialVault {
    /// A vault over `store` (the Keychain, or the in-memory fake).
    pub fn new(store: Arc<dyn CredentialStore>) -> Self {
        Self { store }
    }

    /// Saves `credential` for the connection, replacing it in one write.
    pub async fn save(
        &self,
        provider: IntegrationProvider,
        connection: ConnectionId,
        credential: &Credential,
    ) -> Result<(), SecretError> {
        let key = connection_key(provider, connection)?;
        self.store.put(&key, credential.encode()?).await
    }

    /// The connection's credential; `None` when it is missing or unreadable.
    pub async fn load(
        &self,
        provider: IntegrationProvider,
        connection: ConnectionId,
    ) -> Result<Option<Credential>, SecretError> {
        let key = connection_key(provider, connection)?;
        let stored = self.store.read(&key).await?;
        Ok(stored.as_ref().and_then(Credential::decode))
    }

    /// Forgets the connection's credential.
    pub async fn forget(
        &self,
        provider: IntegrationProvider,
        connection: ConnectionId,
    ) -> Result<(), SecretError> {
        self.store
            .remove(&connection_key(provider, connection)?)
            .await
    }

    /// The user's client secret for `provider`, if saved.
    pub async fn client_secret(
        &self,
        provider: IntegrationProvider,
    ) -> Result<Option<SecretValue>, SecretError> {
        self.store.read(&secret_key(provider)?).await
    }

    /// Saves (or, with `None`, forgets) the client secret for `provider`.
    pub async fn set_client_secret(
        &self,
        provider: IntegrationProvider,
        secret: Option<SecretValue>,
    ) -> Result<(), SecretError> {
        let key = secret_key(provider)?;
        match secret {
            Some(secret) => self.store.put(&key, secret).await,
            None => self.store.remove(&key).await,
        }
    }
}

fn connection_key(
    provider: IntegrationProvider,
    connection: ConnectionId,
) -> Result<CredentialKey, SecretError> {
    CredentialKey::parse(&format!("integration.{provider}.{connection}"))
}

fn secret_key(provider: IntegrationProvider) -> Result<CredentialKey, SecretError> {
    CredentialKey::parse(&format!("integration.{provider}.client-secret"))
}

#[cfg(test)]
mod credential_tests;
