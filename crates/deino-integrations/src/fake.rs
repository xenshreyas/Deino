//! Fakes of this crate's ports, behind `test-support`: the browser, the
//! network policy, the app registrations and time. The scripted provider is
//! in `fake/scripted.rs`.

mod scripted;

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_domain::{Clock, FakeClock, IntegrationProvider, ServiceId};
use deino_net::{NetClient, SwitchError};
use tokio::sync::{Mutex as AsyncMutex, mpsc};
use url::Url;

pub use scripted::ScriptedIntegration;

use crate::{AppRegistration, Browser, Egress, IntegrationError, OAuthApps, OAuthError, Sleeper};

/// A browser that hands each opened URL to the test, which plays the user.
pub struct FakeBrowser {
    opened: mpsc::UnboundedSender<Url>,
    pending: AsyncMutex<mpsc::UnboundedReceiver<Url>>,
    broken: AtomicBool,
}

impl Default for FakeBrowser {
    fn default() -> Self {
        let (opened, pending) = mpsc::unbounded_channel();
        Self {
            opened,
            pending: AsyncMutex::new(pending),
            broken: AtomicBool::new(false),
        }
    }
}

impl FakeBrowser {
    /// The next URL Deino opens, once it does.
    pub async fn next_opened(&self) -> Option<Url> {
        self.pending.lock().await.recv().await
    }

    /// Makes every later `open` fail, like a Mac with no default browser.
    pub fn break_it(&self) {
        self.broken.store(true, Ordering::Relaxed);
    }
}

#[async_trait]
impl Browser for FakeBrowser {
    async fn open(&self, url: &Url) -> Result<(), OAuthError> {
        if self.broken.load(Ordering::Relaxed) {
            return Err(OAuthError::Browser);
        }
        self.opened
            .send(url.clone())
            .map_err(|_| OAuthError::Browser)
    }
}

/// [`Egress`] over one client, with offline mode and switches in memory.
pub struct FakeEgress {
    client: Arc<dyn NetClient>,
    offline: AtomicBool,
    switches: Mutex<BTreeMap<ServiceId, bool>>,
}

impl FakeEgress {
    /// Egress whose every service uses `client` (a `FakeNetClient`, or the
    /// real client pointed at a local mock server).
    pub fn new(client: Arc<dyn NetClient>) -> Self {
        Self {
            client,
            offline: AtomicBool::new(false),
            switches: Mutex::new(BTreeMap::new()),
        }
    }

    /// Turns offline mode on or off.
    pub fn set_offline(&self, offline: bool) {
        self.offline.store(offline, Ordering::Relaxed);
    }

    /// The switches flipped so far.
    pub fn switches(&self) -> BTreeMap<ServiceId, bool> {
        lock(&self.switches).clone()
    }
}

#[async_trait]
impl Egress for FakeEgress {
    fn client(&self, _service: ServiceId) -> Arc<dyn NetClient> {
        Arc::clone(&self.client)
    }

    async fn offline(&self) -> bool {
        self.offline.load(Ordering::Relaxed)
    }

    async fn set_enabled(&self, service: ServiceId, enabled: bool) -> Result<(), SwitchError> {
        lock(&self.switches).insert(service, enabled);
        Ok(())
    }
}

/// Registrations in memory.
#[derive(Debug, Default)]
pub struct FakeOAuthApps {
    apps: Mutex<BTreeMap<IntegrationProvider, AppRegistration>>,
}

#[async_trait]
impl OAuthApps for FakeOAuthApps {
    async fn registration(&self, provider: IntegrationProvider) -> Option<AppRegistration> {
        lock(&self.apps).get(&provider).cloned()
    }

    async fn save(
        &self,
        provider: IntegrationProvider,
        registration: Option<AppRegistration>,
    ) -> Result<(), IntegrationError> {
        let mut apps = lock(&self.apps);
        match registration {
            Some(registration) => apps.insert(provider, registration),
            None => apps.remove(&provider),
        };
        Ok(())
    }
}

/// A [`Sleeper`] that moves a [`FakeClock`] to each deadline at once and
/// records it.
#[derive(Debug)]
pub struct FakeSleeper {
    clock: Arc<FakeClock>,
    waits: Mutex<Vec<DateTime<Utc>>>,
}

impl FakeSleeper {
    /// A sleeper driving `clock`.
    pub fn new(clock: Arc<FakeClock>) -> Self {
        Self {
            clock,
            waits: Mutex::new(Vec::new()),
        }
    }

    /// Every deadline waited for, in order.
    pub fn waits(&self) -> Vec<DateTime<Utc>> {
        lock(&self.waits).clone()
    }
}

#[async_trait]
impl Sleeper for FakeSleeper {
    async fn sleep_until(&self, deadline: DateTime<Utc>) {
        lock(&self.waits).push(deadline);
        let now = self.clock.now();
        if deadline > now {
            self.clock.advance(deadline - now);
        }
    }
}

/// A panic in another test thread cannot leave a map half-updated.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
