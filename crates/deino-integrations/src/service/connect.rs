//! Connecting accounts: OAuth in the browser, or a pasted token.

use std::future::Future;
use std::sync::Arc;

use chrono::{DateTime, TimeDelta, Utc};
use deino_domain::{AuthMethod, ConnectionId, ConnectionStatus, IntegrationProvider};
use deino_secrets::SecretValue;

use super::IntegrationService;
use crate::{
    AuthorizationFlow, Connection, Credential, Integration, IntegrationError, IssuedTokens,
    OAuthClient, OAuthError, OAuthSpec, TokenExtras, TokenInput,
};

/// A credential the user just granted, before its account is known.
pub(super) struct Grant {
    pub(super) credential: Credential,
    pub(super) method: AuthMethod,
    pub(super) scopes: Vec<String>,
    pub(super) expires_at: Option<DateTime<Utc>>,
    pub(super) extras: TokenExtras,
}

impl IntegrationService {
    /// Connects `provider` in the browser: OAuth with PKCE through a
    /// loopback redirect. Resolves once the user approves, declines or
    /// cancels ([`IntegrationService::cancel_connect`]), or after 5 minutes.
    pub async fn connect(
        &self,
        provider: IntegrationProvider,
    ) -> Result<Connection, IntegrationError> {
        let integration = Arc::clone(self.ports.registry.get(provider)?);
        let spec = integration
            .descriptor()
            .oauth()
            .ok_or(IntegrationError::MethodUnavailable(provider))?
            .clone();
        let (app, port) = self.oauth_client(provider, &spec).await?;
        self.ensure_online(provider).await?;
        let pending = self.pending.start(provider)?;
        self.enabling(provider, async {
            let client = self.client_for(provider);
            let flow = AuthorizationFlow {
                spec: &spec,
                app: &app,
                client: client.as_ref(),
                browser: self.ports.browser.as_ref(),
                port,
            };
            let tokens = flow
                .run(pending.token())
                .await
                .map_err(|source| oauth_error(provider, source))?
                .ok_or(IntegrationError::Cancelled(provider))?;
            let grant = self.oauth_grant(tokens, &spec);
            self.finish(integration.as_ref(), grant).await
        })
        .await
    }

    /// Cancels a browser sign-in to `provider`; false when none is waiting.
    pub fn cancel_connect(&self, provider: IntegrationProvider) -> bool {
        self.pending.cancel(provider)
    }

    /// Connects `provider` with a pasted token (and username, where needed).
    pub async fn connect_token(
        &self,
        provider: IntegrationProvider,
        input: TokenInput,
    ) -> Result<Connection, IntegrationError> {
        let integration = Arc::clone(self.ports.registry.get(provider)?);
        let spec = integration
            .descriptor()
            .token()
            .ok_or(IntegrationError::MethodUnavailable(provider))?;
        let token = SecretValue::parse_credential(&input.token).map_err(|_| {
            IntegrationError::Invalid(format!("paste the {}", spec.label.to_lowercase()))
        })?;
        let username = input
            .username
            .map(|name| name.trim().to_owned())
            .filter(|name| !name.is_empty());
        if let (Some(label), None) = (spec.username_label, &username) {
            return Err(IntegrationError::Invalid(format!(
                "enter the {}",
                label.to_lowercase()
            )));
        }
        self.ensure_online(provider).await?;
        let grant = Grant {
            credential: Credential::Token { token, username },
            method: AuthMethod::Token,
            scopes: Vec::new(),
            expires_at: None,
            extras: TokenExtras::default(),
        };
        self.enabling(provider, self.finish(integration.as_ref(), grant))
            .await
    }

    /// Identifies the account, stores the credential, then the row.
    pub(super) async fn finish(
        &self,
        integration: &dyn Integration,
        grant: Grant,
    ) -> Result<Connection, IntegrationError> {
        let provider = integration.id();
        let api = self.api_with(integration, &grant.credential, None)?;
        let account = integration
            .identify(&api, &grant.extras)
            .await
            .map_err(|error| self.provider_error(integration, error))?;
        let id = match self.ports.connections.find(provider, &account.id).await? {
            Some(id) => id,
            None => ConnectionId::generate(self.ports.clock.as_ref()),
        };
        self.ports
            .vault
            .save(provider, id, &grant.credential)
            .await?;
        let mut scopes = grant.scopes;
        scopes.sort();
        scopes.dedup();
        let now = self.ports.clock.now();
        let connection = Connection {
            id,
            provider,
            account_id: account.id,
            account_label: account.label,
            auth_method: grant.method,
            status: ConnectionStatus::Connected,
            scopes,
            api_base_url: account.api_base_url.map(String::from),
            token_expires_at: grant.expires_at,
            connected_at: now,
            updated_at: now,
            last_used_at: None,
            last_error: None,
        };
        let id = self.ports.connections.upsert(&connection).await?;
        log::info!(target: "integrations", "connected {provider} as connection {id}");
        self.connection(id).await
    }

    /// The tokens of a sign-in as a [`Grant`].
    fn oauth_grant(&self, tokens: IssuedTokens, spec: &OAuthSpec) -> Grant {
        let requested = || spec.scopes.iter().map(|scope| scope.id.clone()).collect();
        Grant {
            expires_at: self.expiry(tokens.expires_in),
            scopes: tokens.scopes.unwrap_or_else(requested),
            credential: Credential::OAuth {
                access: tokens.access_token,
                refresh: tokens.refresh_token,
            },
            method: AuthMethod::OAuth,
            extras: tokens.extras,
        }
    }

    /// When a token issued now with `lifetime` expires.
    pub(super) fn expiry(&self, lifetime: Option<std::time::Duration>) -> Option<DateTime<Utc>> {
        let lifetime = TimeDelta::from_std(lifetime?).ok()?;
        Some(self.ports.clock.now() + lifetime)
    }

    /// The user's registration for `provider`, with its secret if needed,
    /// and the redirect port it registered.
    pub(super) async fn oauth_client(
        &self,
        provider: IntegrationProvider,
        spec: &OAuthSpec,
    ) -> Result<(OAuthClient, Option<u16>), IntegrationError> {
        let registration = self
            .ports
            .apps
            .registration(provider)
            .await
            .ok_or(IntegrationError::AppNotConfigured(provider))?;
        let client_secret = if spec.client_auth.needs_secret() {
            let secret = self.ports.vault.client_secret(provider).await?;
            Some(secret.ok_or(IntegrationError::AppNotConfigured(provider))?)
        } else {
            None
        };
        let client = OAuthClient {
            client_id: registration.client_id,
            client_secret,
        };
        Ok((client, registration.redirect_port))
    }

    /// Fails with [`IntegrationError::Offline`] while offline mode is on.
    pub(super) async fn ensure_online(
        &self,
        provider: IntegrationProvider,
    ) -> Result<(), IntegrationError> {
        if provider.service().is_some() && self.ports.egress.offline().await {
            return Err(IntegrationError::Offline(provider));
        }
        Ok(())
    }

    /// Runs `work` with `provider`'s service switched on; switches it back
    /// off when `work` fails and nothing else uses it.
    async fn enabling<T>(
        &self,
        provider: IntegrationProvider,
        work: impl Future<Output = Result<T, IntegrationError>>,
    ) -> Result<T, IntegrationError> {
        let Some(service) = provider.service() else {
            return work.await;
        };
        self.ports
            .egress
            .set_enabled(service, true)
            .await
            .map_err(|_| IntegrationError::Settings)?;
        let outcome = work.await;
        if outcome.is_err()
            && let Err(error) = self.release_service(service).await
        {
            log::warn!(target: "integrations", "could not switch {service} back off: {error}");
        }
        outcome
    }
}

/// What a failed sign-in means: a policy refusal keeps its own message.
pub(super) fn oauth_error(provider: IntegrationProvider, error: OAuthError) -> IntegrationError {
    match error {
        OAuthError::Net(error) => IntegrationError::from_net(provider, error),
        source => IntegrationError::OAuth { provider, source },
    }
}
