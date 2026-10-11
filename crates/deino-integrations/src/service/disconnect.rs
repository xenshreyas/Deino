//! Disconnecting: revoke at the provider (best effort), forget, delete.

use deino_domain::{AuthMethod, ConnectionId, IntegrationProvider};

use super::IntegrationService;
use crate::{Connection, Credential, IntegrationError, OAuthSpec, TokenEndpoint};

impl IntegrationService {
    /// Disconnects: revokes at the provider when it can (best effort),
    /// forgets the credential, deletes the row (its deliveries stay in the
    /// log) and switches the service off if nothing else uses it.
    pub async fn disconnect(&self, id: ConnectionId) -> Result<(), IntegrationError> {
        let connection = self.connection(id).await?;
        self.revoke_quietly(&connection).await;
        self.ports.vault.forget(connection.provider, id).await?;
        self.ports.connections.delete(id).await?;
        if let Some(service) = connection.provider.service() {
            self.release_service(service).await?;
        }
        log::info!(target: "integrations", "disconnected {} connection {id}", connection.provider);
        Ok(())
    }

    /// Revokes the credential at the provider, logging (never failing) when
    /// it cannot: offline, or the provider is down. Disconnecting goes on.
    async fn revoke_quietly(&self, connection: &Connection) {
        let provider = connection.provider;
        let Ok(integration) = self.ports.registry.get(provider) else {
            return;
        };
        let Ok(Some(credential)) = self.ports.vault.load(provider, connection.id).await else {
            return;
        };
        if let Some(spec) = integration
            .descriptor()
            .oauth()
            .filter(|_| connection.auth_method == AuthMethod::OAuth)
        {
            self.revoke_token(provider, spec, &credential).await;
        }
        let Ok(api) = self.api_with(integration.as_ref(), &credential, None) else {
            return;
        };
        if let Err(error) = integration.revoke(&api).await {
            log::info!(target: "integrations", "{provider} did not confirm the revocation: {error}");
        }
    }

    /// RFC 7009 revocation, when the provider has an endpoint.
    async fn revoke_token(
        &self,
        provider: IntegrationProvider,
        spec: &OAuthSpec,
        credential: &Credential,
    ) {
        let Some(url) = spec.revoke_url.as_deref() else {
            return;
        };
        let Ok((app, _)) = self.oauth_client(provider, spec).await else {
            return;
        };
        let client = self.client_for(provider);
        let endpoint = TokenEndpoint {
            client: client.as_ref(),
            url: &spec.token_url,
            auth: spec.client_auth,
            app: &app,
        };
        if let Err(error) = endpoint.revoke(url, credential.revocable()).await {
            log::info!(target: "integrations", "{provider} did not confirm the revocation: {error}");
        }
    }
}
