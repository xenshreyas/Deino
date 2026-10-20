//! The developer section: diagnostics the owner turns on to help fix Deino
//! on their own Mac. Everything here is off by default.

use serde::{Deserialize, Serialize};

/// The developer section of `Settings`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct DeveloperSettings {
    /// When speaker tags (CAP-05) cannot read the meeting app's call
    /// window, save one Accessibility dump of that app's windows (with
    /// participants' names) to Deino's log folder, once per meeting, so the
    /// app's adapter can be fixed. Off by default.
    pub capture_call_windows: bool,
}

crate::settings::register_section! {
    /// Diagnostics for fixing Deino on this Mac.
    developer: DeveloperSettings,
    order: 250,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn call_windows_are_not_captured_unless_the_file_says_so() {
        let missing: DeveloperSettings = serde_json::from_str("{}").unwrap();
        assert_eq!(missing, DeveloperSettings::default());
        assert!(!missing.capture_call_windows);

        let on: DeveloperSettings =
            serde_json::from_str(r#"{ "captureCallWindows": true }"#).unwrap();
        assert!(on.capture_call_windows);
    }
}
