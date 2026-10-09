//! Notification consent and preferences, stored in the typed settings.
//!
//! Since FND-06 the notification switches are `Settings::notifications`
//! (the legacy `notifications.json` was imported at startup). The
//! [`ConsentManager`] reads and writes that section through the
//! `SettingsService`, so the UI's settings screen and the notification
//! manager always see the same values.

use std::sync::Arc;

use anyhow::{Result, anyhow};
use deino_core::settings::{Settings, SettingsService};
pub use deino_core::settings::{NotificationPreferences, NotificationSettings};
use log::info as log_info;
use tauri::{AppHandle, Manager, Runtime};

/// Reads and changes the notification section of the settings.
pub struct ConsentManager {
    settings: Arc<SettingsService>,
}

impl ConsentManager {
    /// A manager over the app's settings service.
    pub fn new<R: Runtime>(app_handle: &AppHandle<R>) -> Result<Self> {
        let settings = app_handle
            .try_state::<Arc<SettingsService>>()
            .ok_or_else(|| anyhow!("settings are not ready yet"))?;
        Ok(Self {
            settings: Arc::clone(&settings),
        })
    }

    /// The current notification settings.
    pub async fn load_settings(&self) -> Result<NotificationSettings> {
        Ok(self.settings.get().await.notifications)
    }

    /// Replaces the notification settings.
    pub async fn save_settings(&self, notifications: &NotificationSettings) -> Result<()> {
        let notifications = notifications.clone();
        self.update(|settings| settings.notifications = notifications)
            .await
    }

    /// Check if the user has given consent for notifications
    pub async fn has_consent(&self) -> bool {
        self.settings.get().await.notifications.consent_given
    }

    /// Check if system notification permission has been granted
    pub async fn has_system_permission(&self) -> bool {
        self.settings
            .get()
            .await
            .notifications
            .system_permission_granted
    }

    /// Set user consent for notifications
    pub async fn set_consent(&self, consent: bool) -> Result<()> {
        self.update(|settings| settings.notifications.consent_given = consent)
            .await?;
        log_info!("Updated notification consent: {}", consent);
        Ok(())
    }

    /// Set system permission status
    pub async fn set_system_permission(&self, granted: bool) -> Result<()> {
        self.update(|settings| settings.notifications.system_permission_granted = granted)
            .await?;
        log_info!("Updated system notification permission: {}", granted);
        Ok(())
    }

    /// Enable or disable Do Not Disturb mode
    pub async fn set_dnd_mode(&self, enabled: bool) -> Result<()> {
        self.update(|settings| settings.notifications.manual_dnd_mode = enabled)
            .await?;
        log_info!("Set manual DND mode: {}", enabled);
        Ok(())
    }

    async fn update(&self, change: impl FnOnce(&mut Settings) + Send) -> Result<()> {
        self.settings.update(change).await?;
        Ok(())
    }
}
