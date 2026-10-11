//! In-flight state: connects waiting for the browser, and token refreshes.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use deino_domain::{ConnectionId, IntegrationProvider};
use tokio::sync::{Mutex as AsyncMutex, OwnedMutexGuard};
use tokio_util::sync::CancellationToken;

use crate::IntegrationError;

/// One browser sign-in per provider at a time, each cancellable.
#[derive(Debug, Default)]
pub(super) struct PendingConnects {
    tokens: Arc<Mutex<BTreeMap<IntegrationProvider, CancellationToken>>>,
}

impl PendingConnects {
    /// Registers a sign-in to `provider`; the guard unregisters it.
    pub(super) fn start(
        &self,
        provider: IntegrationProvider,
    ) -> Result<PendingConnect, IntegrationError> {
        let mut tokens = lock(&self.tokens);
        if tokens.contains_key(&provider) {
            return Err(IntegrationError::AlreadyConnecting(provider));
        }
        let token = CancellationToken::new();
        tokens.insert(provider, token.clone());
        Ok(PendingConnect {
            provider,
            token,
            tokens: Arc::clone(&self.tokens),
        })
    }

    /// Cancels the sign-in to `provider`; false when none is waiting.
    pub(super) fn cancel(&self, provider: IntegrationProvider) -> bool {
        lock(&self.tokens)
            .get(&provider)
            .map(CancellationToken::cancel)
            .is_some()
    }
}

/// A registered sign-in; dropping it unregisters it.
#[derive(Debug)]
pub(super) struct PendingConnect {
    provider: IntegrationProvider,
    token: CancellationToken,
    tokens: Arc<Mutex<BTreeMap<IntegrationProvider, CancellationToken>>>,
}

impl PendingConnect {
    /// Fires when the user cancels.
    pub(super) fn token(&self) -> &CancellationToken {
        &self.token
    }
}

impl Drop for PendingConnect {
    fn drop(&mut self) {
        lock(&self.tokens).remove(&self.provider);
    }
}

/// One refresh at a time per connection. Providers that rotate refresh
/// tokens (Slack, Linear, Todoist, Jira, monday.com) revoke the whole grant
/// when a spent one is reused, so two concurrent refreshes would sign the
/// user out.
#[derive(Debug, Default)]
pub(super) struct RefreshLocks {
    locks: Mutex<BTreeMap<ConnectionId, Arc<AsyncMutex<()>>>>,
}

impl RefreshLocks {
    /// Waits for, then holds, the refresh lock of `connection`.
    pub(super) async fn hold(&self, connection: ConnectionId) -> OwnedMutexGuard<()> {
        let lock_of_connection = Arc::clone(lock(&self.locks).entry(connection).or_default());
        lock_of_connection.lock_owned().await
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_sign_in_per_provider_until_the_first_ends() {
        let pending = PendingConnects::default();

        let first = pending.start(IntegrationProvider::Linear).unwrap();
        let second = pending.start(IntegrationProvider::Linear);
        assert!(matches!(
            second,
            Err(IntegrationError::AlreadyConnecting(_))
        ));

        assert!(pending.cancel(IntegrationProvider::Linear));
        assert!(first.token().is_cancelled());
        drop(first);
        assert!(!pending.cancel(IntegrationProvider::Linear));
        assert!(pending.start(IntegrationProvider::Linear).is_ok());
    }
}
