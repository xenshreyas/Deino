//! [`SystemBrowser`]: opens OAuth sign-in pages in the user's default
//! browser, where their accounts are already signed in. The one browser port
//! (`deino_integrations::Browser`) for every connection: calendars (ORG-01)
//! and integrations (INT-00).

use async_trait::async_trait;
use deino_integrations::{Browser, OAuthError};
use url::Url;

/// The system's default browser (macOS: `NSWorkspace`); elsewhere none.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct SystemBrowser;

#[async_trait]
impl Browser for SystemBrowser {
    #[cfg(target_os = "macos")]
    async fn open(&self, url: &Url) -> Result<(), OAuthError> {
        use objc2_app_kit::NSWorkspace;
        use objc2_foundation::{NSString, NSURL};

        let opened = NSURL::URLWithString(&NSString::from_str(url.as_str()))
            .is_some_and(|url| NSWorkspace::sharedWorkspace().openURL(&url));
        if opened {
            Ok(())
        } else {
            Err(OAuthError::Browser)
        }
    }

    #[cfg(not(target_os = "macos"))]
    async fn open(&self, _url: &Url) -> Result<(), OAuthError> {
        Err(OAuthError::Browser)
    }
}
