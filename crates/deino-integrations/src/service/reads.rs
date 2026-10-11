//! Reading from a provider: where it can write (targets) and context on
//! demand. Nothing read here is stored.

use std::future::Future;
use std::sync::Arc;

use deino_domain::{Capability, ConnectionId};

use super::IntegrationService;
use crate::{
    Connection, ContextItem, ContextQuery, Integration, IntegrationError, ProviderError, Target,
};

impl IntegrationService {
    /// Where the connection can write for `capability`: channels, databases,
    /// projects.
    pub async fn targets(
        &self,
        id: ConnectionId,
        capability: Capability,
    ) -> Result<Vec<Target>, IntegrationError> {
        let connection = self.connection(id).await?;
        let integration = Arc::clone(self.ports.registry.get(connection.provider)?);
        ensure_capability(integration.as_ref(), capability)?;
        self.ensure_online(connection.provider).await?;
        let api = self
            .session(&connection, integration.as_ref(), false)
            .await?;
        let found = match capability {
            Capability::PostMessage => {
                with_capability(integration.messages(), |c| c.message_targets(&api)).await
            }
            Capability::CreatePage => {
                with_capability(integration.pages(), |c| c.page_targets(&api)).await
            }
            Capability::CreateTask => {
                with_capability(integration.tasks(), |c| c.task_targets(&api)).await
            }
            Capability::LogActivity | Capability::ReadContext => Ok(Vec::new()),
        };
        self.observed(&connection, integration.as_ref(), found)
            .await
    }

    /// Context for `query` from the connection, used once and never stored.
    pub async fn read_context(
        &self,
        id: ConnectionId,
        query: &ContextQuery,
    ) -> Result<Vec<ContextItem>, IntegrationError> {
        let connection = self.connection(id).await?;
        let integration = Arc::clone(self.ports.registry.get(connection.provider)?);
        ensure_capability(integration.as_ref(), Capability::ReadContext)?;
        self.ensure_online(connection.provider).await?;
        let api = self
            .session(&connection, integration.as_ref(), false)
            .await?;
        let found = with_capability(integration.context(), |c| c.read_context(&api, query)).await;
        self.observed(&connection, integration.as_ref(), found)
            .await
    }

    /// Records the result of a read on the connection and passes it on.
    async fn observed<T>(
        &self,
        connection: &Connection,
        integration: &dyn Integration,
        found: Result<T, ProviderError>,
    ) -> Result<T, IntegrationError> {
        match found {
            Ok(found) => {
                let now = self.ports.clock.now();
                self.ports
                    .connections
                    .record_success(connection.id, now)
                    .await?;
                Ok(found)
            }
            Err(error) => {
                let error = self.provider_error(integration, error);
                self.note_failure(connection, &error).await;
                Err(error)
            }
        }
    }
}

/// Fails unless `integration` offers `capability`.
pub(super) fn ensure_capability(
    integration: &dyn Integration,
    capability: Capability,
) -> Result<(), IntegrationError> {
    if integration.capabilities().contains(&capability) {
        return Ok(());
    }
    Err(IntegrationError::Unsupported {
        provider: integration.id(),
        capability,
    })
}

/// Calls `call` on a capability the provider offers.
pub(super) async fn with_capability<'a, C: ?Sized + 'a, T, F>(
    capability: Option<&'a C>,
    call: impl FnOnce(&'a C) -> F,
) -> Result<T, ProviderError>
where
    F: Future<Output = Result<T, ProviderError>>,
{
    match capability {
        Some(capability) => call(capability).await,
        None => Err(ProviderError::Rejected {
            reason: "this is not something the provider can do".into(),
        }),
    }
}
