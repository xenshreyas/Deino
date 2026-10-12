//! What a capture follows ([`CaptureTarget`]) and the record panel's
//! picker: [`choices`] lists the screens and windows the user can pick,
//! [`resolve`] finds the target among what macOS shares now.
//!
//! A detected meeting follows its app's window ([`pick_window`]); a pick
//! is one window or one whole display ([`Source`]). Deino's own windows
//! (the record panel, the main window) are never offered.

use deino_domain::BundleId;
use serde::{Deserialize, Serialize};

use crate::target::{MIN_SIDE, WindowInfo, pick_window};

/// Deino's own bundle, whose windows are never captured.
pub const OWN_BUNDLE: &str = "com.xenshreyas.deino";

/// One display macOS shares, as plain data.
#[derive(Clone, Debug, PartialEq)]
pub struct DisplayInfo {
    /// The display ID (`CGDirectDisplayID`).
    pub id: u32,
    /// Its size in points.
    pub width: f64,
    /// Its size in points.
    pub height: f64,
    /// Its backing scale (pixels per point).
    pub scale: f64,
}

/// A window or a whole display.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(tag = "kind", content = "id", rename_all = "camelCase")]
pub enum Source {
    /// One window (`CGWindowID`).
    Window(u32),
    /// One whole display (`CGDirectDisplayID`).
    Display(u32),
}

/// What a capture follows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CaptureTarget {
    /// The detected meeting app's main window.
    MeetingApp(BundleId),
    /// What the user picked in the record panel.
    Picked(Source),
}

/// One entry of the picker.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TargetChoice {
    /// What it captures.
    pub source: Source,
    /// "Entire screen", or the app and the window's title.
    pub label: String,
}

/// The resolved target: what to stream, at what point size and scale,
/// and the window to watch for visibility (`None` for a display).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Resolved {
    /// What to stream.
    pub source: Source,
    /// Its size in points.
    pub size: (f64, f64),
    /// The window whose visibility pauses the capture.
    pub window: Option<u32>,
}

/// The picker's entries: each display, then the ordinary on-screen windows
/// of other apps, largest first.
pub fn choices(windows: &[WindowInfo], displays: &[DisplayInfo]) -> Vec<TargetChoice> {
    let screens = displays.iter().enumerate().map(|(n, display)| TargetChoice {
        source: Source::Display(display.id),
        label: if n == 0 {
            "Entire screen".to_owned()
        } else {
            format!("Screen {}", n + 1)
        },
    });
    let mut pickable: Vec<&WindowInfo> = windows.iter().filter(|w| pickable(w)).collect();
    pickable.sort_by(|a, b| (b.width * b.height).total_cmp(&(a.width * a.height)));
    let windows = pickable.into_iter().map(|window| TargetChoice {
        source: Source::Window(window.id),
        label: window_label(window),
    });
    screens.chain(windows).collect()
}

/// An ordinary, on-screen, meeting-sized window of another app.
fn pickable(window: &WindowInfo) -> bool {
    window.layer == 0
        && window.on_screen
        && window.width >= MIN_SIDE
        && window.height >= MIN_SIDE
        && window.bundle_id.as_ref().is_none_or(|id| id.as_str() != OWN_BUNDLE)
}

/// "Zoom — Design review", or just the app.
fn window_label(window: &WindowInfo) -> String {
    let app = window.app_name.as_deref().unwrap_or("Window");
    match window.title.as_deref().map(str::trim) {
        Some(title) if !title.is_empty() && title != app => format!("{app} — {title}"),
        _ => app.to_owned(),
    }
}

/// `target` among what macOS shares now; `None` when it is gone.
pub fn resolve(
    target: &CaptureTarget,
    windows: &[WindowInfo],
    displays: &[DisplayInfo],
) -> Option<Resolved> {
    let window = |found: &WindowInfo| Resolved {
        source: Source::Window(found.id),
        size: (found.width, found.height),
        window: Some(found.id),
    };
    match target {
        CaptureTarget::MeetingApp(app) => pick_window(windows, app).map(window),
        CaptureTarget::Picked(Source::Window(id)) => windows.iter().find(|w| w.id == *id).map(window),
        CaptureTarget::Picked(Source::Display(id)) => {
            displays.iter().find(|d| d.id == *id).map(|display| Resolved {
                source: Source::Display(display.id),
                size: (display.width, display.height),
                window: None,
            })
        }
    }
}

#[cfg(test)]
#[path = "pick_tests.rs"]
mod tests;
