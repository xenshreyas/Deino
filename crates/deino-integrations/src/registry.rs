//! The provider registry: **adding a provider is one line in
//! [`integrations`]** (plus its module).

use std::collections::BTreeMap;
use std::sync::Arc;

use deino_domain::IntegrationProvider;

use crate::{Integration, IntegrationError, SandboxIntegration};

/// Every provider this build can connect, by ID.
#[derive(Clone, Default)]
pub struct IntegrationRegistry {
    providers: BTreeMap<IntegrationProvider, Arc<dyn Integration>>,
}

impl IntegrationRegistry {
    /// A registry of `providers`; a later entry with the same ID replaces
    /// an earlier one.
    pub fn new(providers: impl IntoIterator<Item = Arc<dyn Integration>>) -> Self {
        Self {
            providers: providers
                .into_iter()
                .map(|provider| (provider.id(), provider))
                .collect(),
        }
    }

    /// The provider `id`.
    pub fn get(&self, id: IntegrationProvider) -> Result<&Arc<dyn Integration>, IntegrationError> {
        self.providers
            .get(&id)
            .ok_or(IntegrationError::UnknownProvider(id))
    }

    /// Every provider, in `IntegrationProvider` order.
    pub fn iter(&self) -> impl Iterator<Item = &Arc<dyn Integration>> {
        self.providers.values()
    }
}

/// What the app's build includes.
#[derive(Clone, Copy, Debug, Default)]
pub struct RegistryOptions {
    /// Include the [`SandboxIntegration`] (debug builds only).
    pub sandbox: bool,
}

/// The one list of providers. Provider cards (INT-01..07) add their line
/// here, for example `Arc::new(slack::SlackIntegration::new())`.
pub fn integrations(options: RegistryOptions) -> IntegrationRegistry {
    let providers: Vec<Arc<dyn Integration>> = Vec::new();
    let sandbox = options
        .sandbox
        .then(|| Arc::new(SandboxIntegration::new()) as Arc<dyn Integration>);
    IntegrationRegistry::new(providers.into_iter().chain(sandbox))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_builds_list_no_sandbox_and_debug_builds_do() {
        let release = integrations(RegistryOptions { sandbox: false });
        let debug = integrations(RegistryOptions { sandbox: true });

        assert!(release.get(IntegrationProvider::Sandbox).is_err());
        assert_eq!(
            debug.get(IntegrationProvider::Sandbox).unwrap().id(),
            IntegrationProvider::Sandbox
        );
        assert!(matches!(
            debug.get(IntegrationProvider::Slack),
            Err(IntegrationError::UnknownProvider(
                IntegrationProvider::Slack
            ))
        ));
    }
}
