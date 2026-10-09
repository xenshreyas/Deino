//! The System Settings page where Calendars access is granted.

use objc2_app_kit::NSWorkspace;
use objc2_foundation::{NSString, NSURL};

/// System Settings → Privacy & Security → Calendars.
const CALENDARS_PRIVACY_PAGE: &str =
    "x-apple.systempreferences:com.apple.preference.security?Privacy_Calendars";

/// Opens the Calendars privacy page; false when macOS would not open it.
pub(crate) fn open_calendars_page() -> bool {
    let Some(url) = NSURL::URLWithString(&NSString::from_str(CALENDARS_PRIVACY_PAGE)) else {
        return false;
    };
    NSWorkspace::sharedWorkspace().openURL(&url)
}
