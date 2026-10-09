//! [`SettingsService`]: the one owner of the current settings.
//!
//! It keeps the settings in memory, so reads never touch the disk, and
//! serialises every change: read, change, validate, save, then announce.
//! A change that fails validation or cannot be saved leaves everything as it
//! was. The UI changes settings with a [`SettingsPatch`]; Rust callers use
//! [`SettingsService::update`] with a typed closure.

use std::sync::Arc;

use tokio::sync::Mutex;

use super::{Settings, SettingsError, SettingsEvents, SettingsPatch, SettingsStore};

/// Reads and changes the user's settings.
pub struct SettingsService {
    store: Arc<dyn SettingsStore>,
    events: Arc<dyn SettingsEvents>,
    current: Mutex<Settings>,
}

impl SettingsService {
    /// Loads the saved settings, or the defaults when there are none yet.
    /// Nothing is written until the first change.
    pub async fn load(
        store: Arc<dyn SettingsStore>,
        events: Arc<dyn SettingsEvents>,
    ) -> Result<Self, SettingsError> {
        let current = store.load().await?.unwrap_or_default();
        Ok(Self {
            store,
            events,
            current: Mutex::new(current),
        })
    }

    /// The current settings.
    pub async fn get(&self) -> Settings {
        self.current.lock().await.clone()
    }

    /// Applies a merge patch from the UI and returns the new settings.
    pub async fn patch(&self, patch: &SettingsPatch) -> Result<Settings, SettingsError> {
        let mut current = self.current.lock().await;
        let next = patch.apply(&current)?;
        self.commit(&mut current, next).await
    }

    /// Changes the settings in Rust and returns the new settings.
    pub async fn update(
        &self,
        change: impl FnOnce(&mut Settings) + Send,
    ) -> Result<Settings, SettingsError> {
        let mut current = self.current.lock().await;
        let mut next = current.clone();
        change(&mut next);
        next.validate()?;
        self.commit(&mut current, next).await
    }

    /// Saves `next`, makes it current and announces it, unless nothing changed.
    async fn commit(
        &self,
        current: &mut Settings,
        next: Settings,
    ) -> Result<Settings, SettingsError> {
        if *current == next {
            return Ok(next);
        }
        self.store.save(&next).await?;
        *current = next.clone();
        self.events.changed(&next);
        log::debug!(target: "settings", "settings saved");
        Ok(next)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::settings::{FakeSettingsStore, RecordedSettingsEvents, Theme};

    struct Harness {
        service: SettingsService,
        store: Arc<FakeSettingsStore>,
        events: Arc<RecordedSettingsEvents>,
    }

    async fn harness(saved: Option<Settings>) -> Harness {
        let store = Arc::new(FakeSettingsStore::with(saved));
        let events = Arc::new(RecordedSettingsEvents::default());
        let service = SettingsService::load(store.clone(), events.clone())
            .await
            .unwrap();
        Harness {
            service,
            store,
            events,
        }
    }

    fn patch(value: serde_json::Value) -> SettingsPatch {
        SettingsPatch::parse(value).unwrap()
    }

    #[tokio::test]
    async fn a_first_launch_starts_from_defaults_without_writing() {
        let Harness { service, store, .. } = harness(None).await;

        assert_eq!(service.get().await, Settings::default());
        assert_eq!(store.saved(), None);
    }

    #[tokio::test]
    async fn a_patch_is_saved_announced_and_returned() {
        let Harness {
            service,
            store,
            events,
        } = harness(None).await;

        let next = service
            .patch(&patch(json!({ "ui": { "theme": "light" } })))
            .await
            .unwrap();

        assert_eq!(next.ui.theme, Theme::Light);
        assert_eq!(store.saved(), Some(next.clone()));
        assert_eq!(events.changes(), std::slice::from_ref(&next));
        assert_eq!(service.get().await, next);
    }

    #[tokio::test]
    async fn a_rejected_patch_changes_nothing() {
        let Harness {
            service,
            store,
            events,
        } = harness(None).await;

        let result = service
            .patch(&patch(json!({ "ui": { "transcriptPaneRatio": 2 } })))
            .await;

        assert!(result.is_err());
        assert_eq!(service.get().await, Settings::default());
        assert_eq!(store.saved(), None);
        assert!(events.changes().is_empty());
    }

    #[tokio::test]
    async fn a_failed_save_keeps_the_old_settings() {
        let Harness { service, store, .. } = harness(None).await;
        store.fail_saves();

        let result = service.update(|s| s.ui.theme = Theme::Dark).await;

        assert!(matches!(result, Err(SettingsError::Write(_))));
        assert_eq!(service.get().await.ui.theme, Theme::System);
    }

    #[tokio::test]
    async fn a_change_to_the_same_value_writes_nothing() {
        let Harness {
            service, events, ..
        } = harness(Some(Settings::default())).await;

        service
            .update(|s| s.ui.theme = Theme::System)
            .await
            .unwrap();

        assert!(events.changes().is_empty());
    }
}
