//! What Settings → Speaker tags shows for each app: Enabled, Needs
//! permission, or Unsupported.
//!
//! - **Unsupported**: not on macOS, or the last look at the app's call
//!   window found a layout its adapter does not know (the app's UI changed);
//!   the next recognised window makes it Enabled again.
//! - **Needs permission**: Accessibility is not granted.
//! - **Enabled**: names are read whenever Deino records a call in the app.

use std::sync::Mutex;

use deino_domain::CallApp;
use serde::Serialize;

/// One app's speaker-tag state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum TagState {
    /// Names are read while recording a call in the app.
    Enabled,
    /// Accessibility must be granted in System Settings first.
    NeedsPermission,
    /// Not on this platform, or not this version of the app.
    Unsupported,
}

/// One row of Settings → Speaker tags.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AppTagStatus {
    /// The app.
    pub app: CallApp,
    /// Its state.
    pub state: TagState,
}

/// Whether each app's call window was recognised the last time it was read.
#[derive(Debug, Default)]
pub struct LayoutHealth {
    unrecognised: Mutex<Vec<CallApp>>,
}

impl LayoutHealth {
    /// Records whether `app`'s call window was recognised just now.
    pub fn record(&self, app: CallApp, recognised: bool) {
        let mut unrecognised = self
            .unrecognised
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        unrecognised.retain(|known| *known != app);
        if !recognised {
            unrecognised.push(app);
        }
    }

    /// Whether `app`'s call window was recognised (or never read).
    pub fn recognised(&self, app: CallApp) -> bool {
        let unrecognised = self
            .unrecognised
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        !unrecognised.contains(&app)
    }
}

/// Every app's row, in [`CallApp::ALL`] order.
pub fn statuses(supported: bool, granted: bool, health: &LayoutHealth) -> Vec<AppTagStatus> {
    CallApp::ALL
        .iter()
        .map(|&app| {
            let state = if !supported || !health.recognised(app) {
                TagState::Unsupported
            } else if granted {
                TagState::Enabled
            } else {
                TagState::NeedsPermission
            };
            AppTagStatus { app, state }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn states(supported: bool, granted: bool, health: &LayoutHealth) -> Vec<TagState> {
        statuses(supported, granted, health)
            .into_iter()
            .map(|status| status.state)
            .collect()
    }

    #[test]
    fn each_app_is_enabled_needs_permission_or_unsupported() {
        let health = LayoutHealth::default();

        assert_eq!(states(true, true, &health), [TagState::Enabled; 3]);
        assert_eq!(states(true, false, &health), [TagState::NeedsPermission; 3]);
        assert_eq!(states(false, true, &health), [TagState::Unsupported; 3]);
    }

    #[test]
    fn an_unrecognised_window_marks_only_its_app_until_one_is_recognised() {
        let health = LayoutHealth::default();

        health.record(CallApp::Teams, false);
        assert_eq!(
            states(true, true, &health),
            [TagState::Enabled, TagState::Unsupported, TagState::Enabled]
        );
        health.record(CallApp::Teams, true);
        assert_eq!(states(true, true, &health), [TagState::Enabled; 3]);
    }

    #[test]
    fn rows_serialize_in_camel_case() {
        let row = AppTagStatus {
            app: CallApp::Meet,
            state: TagState::NeedsPermission,
        };

        let json = serde_json::to_string(&row).unwrap();

        assert_eq!(json, r#"{"app":"meet","state":"needsPermission"}"#);
    }
}
