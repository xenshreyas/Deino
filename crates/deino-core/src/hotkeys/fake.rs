//! [`FakeRegistrar`]: system hotkeys in memory, with combinations "another
//! app" holds.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Mutex, MutexGuard, PoisonError};

use super::accelerator::Accelerator;
use super::model::HotkeyAction;
use super::ports::{HotkeyRegistrar, RegistrarRefused};

/// The registered hotkeys, and what the system refuses.
#[derive(Debug, Default)]
pub struct FakeRegistrar {
    inner: Mutex<Inner>,
}

#[derive(Debug, Default)]
struct Inner {
    registered: BTreeMap<HotkeyAction, String>,
    held_elsewhere: BTreeSet<String>,
}

impl FakeRegistrar {
    /// Another app holds `accelerator` (canonical text): registering it fails.
    pub fn hold_elsewhere(&self, accelerator: &str) {
        self.inner().held_elsewhere.insert(accelerator.to_owned());
    }

    /// The registered hotkeys, by action, as canonical text.
    pub fn registered(&self) -> BTreeMap<HotkeyAction, String> {
        self.inner().registered.clone()
    }

    fn inner(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl HotkeyRegistrar for FakeRegistrar {
    fn register(
        &self,
        action: HotkeyAction,
        accelerator: &Accelerator,
    ) -> Result<(), RegistrarRefused> {
        let text = accelerator.to_string();
        let mut inner = self.inner();
        let taken = inner.registered.values().any(|held| *held == text);
        if taken || inner.held_elsewhere.contains(&text) {
            return Err(RegistrarRefused);
        }
        inner.registered.insert(action, text);
        Ok(())
    }

    fn unregister(&self, action: HotkeyAction) {
        self.inner().registered.remove(&action);
    }
}
