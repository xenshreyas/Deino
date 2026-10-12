//! The operating system's permissions: the app's [`PermissionGate`] adapter
//! behind `deino_core::permissions::PermissionService` (FND-11).
//!
//! The microphone, system audio and Accessibility go through macOS
//! (`permissions/macos.rs`); elsewhere they are not needed
//! (`permissions/other.rs`). Notifications go through the notification
//! plugin the app already sends them with, on every platform.

#[cfg(target_os = "macos")]
mod macos;
#[cfg(not(target_os = "macos"))]
mod other;

use async_trait::async_trait;
use deino_core::permissions::{PermissionGate, PermissionKind, PermissionState};
#[cfg(target_os = "macos")]
use macos as platform;
#[cfg(not(target_os = "macos"))]
use other as platform;
use tauri::plugin::PermissionState as PluginState;
use tauri::{AppHandle, Runtime};
use tauri_plugin_notification::NotificationExt;

/// The real permissions of this app.
pub(crate) struct SystemPermissions<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> SystemPermissions<R> {
    /// The permissions of the app behind `app`.
    pub(crate) const fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }

    fn notifications(&self) -> PermissionState {
        self.app
            .notification()
            .permission_state()
            .map_or(PermissionState::Unknown, from_plugin)
    }

    fn request_notifications(&self) -> PermissionState {
        self.app
            .notification()
            .request_permission()
            .map_or(PermissionState::Unknown, from_plugin)
    }
}

#[async_trait]
impl<R: Runtime> PermissionGate for SystemPermissions<R> {
    fn state(&self, kind: PermissionKind) -> PermissionState {
        match kind {
            PermissionKind::Microphone => platform::microphone(),
            PermissionKind::SystemAudio => platform::system_audio(),
            PermissionKind::Notifications => self.notifications(),
            PermissionKind::Accessibility => platform::accessibility(),
        }
    }

    async fn request(&self, kind: PermissionKind) -> PermissionState {
        match kind {
            PermissionKind::Microphone => platform::request_microphone().await,
            PermissionKind::SystemAudio => platform::request_system_audio().await,
            PermissionKind::Notifications => self.request_notifications(),
            PermissionKind::Accessibility => platform::request_accessibility(),
        }
    }

    fn open_settings(&self, kind: PermissionKind) -> bool {
        platform::open_settings(kind)
    }
}

/// The notification plugin's answer in Deino's terms.
const fn from_plugin(state: PluginState) -> PermissionState {
    match state {
        PluginState::Granted => PermissionState::Granted,
        PluginState::Denied => PermissionState::Denied,
        PluginState::Prompt | PluginState::PromptWithRationale => PermissionState::NotDetermined,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_plugin_states_map_onto_deinos() {
        assert_eq!(from_plugin(PluginState::Granted), PermissionState::Granted);
        assert_eq!(from_plugin(PluginState::Denied), PermissionState::Denied);
        assert_eq!(
            from_plugin(PluginState::Prompt),
            PermissionState::NotDetermined
        );
    }
}
