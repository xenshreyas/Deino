//! [`AxBrowserTabs`]: the page address of each browser window, read through
//! Accessibility (`kAXDocumentAttribute`, Muesli's
//! `BrowserMeetingActivityCollector`).
//!
//! It runs only when the user has granted Accessibility; without it browser
//! calls are still detected, after the sustained hold, just not by URL. Each
//! request has a short messaging timeout, so a hung browser cannot stall
//! the detector. Addresses are never logged.

use cidre::{ax, cf};

use crate::model::Pid;
use crate::ports::BrowserTabs;

/// How long one Accessibility request may take, in seconds.
const AX_TIMEOUT_SECS: f32 = 0.25;

/// At most this many windows are read per browser.
const MAX_WINDOWS: usize = 8;

/// The Accessibility role of a window.
const WINDOW_ROLE: &str = "AXWindow";

/// Browser windows through Accessibility.
pub(super) struct AxBrowserTabs;

impl BrowserTabs for AxBrowserTabs {
    fn available(&self) -> bool {
        ax::is_process_trusted()
    }

    fn window_urls(&self, pid: Pid) -> Vec<String> {
        let app = ax::UiElement::with_app_pid(pid.0);
        if app.set_messaging_timeout_secs(AX_TIMEOUT_SECS).is_err() {
            return Vec::new();
        }
        let Ok(children) = app.children() else {
            return Vec::new();
        };
        children
            .iter()
            .filter(|child| {
                child
                    .role()
                    .is_ok_and(|role| role.to_string() == WINDOW_ROLE)
            })
            .take(MAX_WINDOWS)
            .filter_map(document_url)
            .collect()
    }
}

/// The window's `AXDocument`: the page address in Chromium browsers and Safari.
fn document_url(window: &ax::UiElement) -> Option<String> {
    let value = window.attr_value(ax::attr::document()).ok()?;
    value.try_as_string().map(cf::String::to_string)
}
