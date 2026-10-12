//! Accessibility trust, running apps, and the System Settings page.

use objc2_app_kit::{NSRunningApplication, NSWorkspace};
use objc2_foundation::{NSString, NSURL};

use crate::ports::{AccessibilityPermission, Pid, RunningApps};

/// System Settings → Privacy & Security → Accessibility.
const ACCESSIBILITY_PAGE: &str =
    "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility";

/// Accessibility trust through `AXIsProcessTrusted`, which never prompts.
pub(super) struct Trust;

impl AccessibilityPermission for Trust {
    fn granted(&self) -> bool {
        cidre::ax::is_process_trusted()
    }
}

/// Running apps through `NSRunningApplication`.
pub(super) struct Workspace;

impl RunningApps for Workspace {
    fn pids(&self, bundle_ids: &[&str]) -> Vec<Pid> {
        let mut pids = Vec::new();
        for bundle_id in bundle_ids {
            let id = NSString::from_str(bundle_id);
            let apps = NSRunningApplication::runningApplicationsWithBundleIdentifier(&id);
            pids.extend(
                apps.iter()
                    .filter(|app| !app.isTerminated())
                    .map(|app| Pid(app.processIdentifier())),
            );
        }
        pids
    }
}

/// Opens the Accessibility page of System Settings, where the user adds
/// Deino. Changes nothing itself; false when macOS would not open it.
pub(crate) fn open_accessibility_settings() -> bool {
    let Some(url) = NSURL::URLWithString(&NSString::from_str(ACCESSIBILITY_PAGE)) else {
        return false;
    };
    NSWorkspace::sharedWorkspace().openURL(&url)
}
