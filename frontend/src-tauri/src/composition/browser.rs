//! [`SystemBrowser`]: opens OAuth sign-in pages in the user's default
//! browser, where their accounts are already signed in.

use deino_connect::oauth::BrowserOpener;

/// The system's default browser (macOS: `NSWorkspace`); elsewhere none.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct SystemBrowser;

impl BrowserOpener for SystemBrowser {
    #[cfg(target_os = "macos")]
    fn open(&self, url: &str) -> bool {
        use objc2_app_kit::NSWorkspace;
        use objc2_foundation::{NSString, NSURL};

        NSURL::URLWithString(&NSString::from_str(url))
            .is_some_and(|url| NSWorkspace::sharedWorkspace().openURL(&url))
    }

    #[cfg(not(target_os = "macos"))]
    fn open(&self, _url: &str) -> bool {
        false
    }
}
