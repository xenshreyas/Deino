//! The macOS screen ports: `ScreenCaptureKit` for windows and frames
//! (`windows`, `stream`), `AVAssetWriter` for the H.264 video (`writer`),
//! and Core Graphics for the Screen Recording permission.
//!
//! Nothing here prompts: [`MacPermission`] reads the permission with
//! `CGPreflightScreenCaptureAccess`, and the capture only touches
//! `ScreenCaptureKit` (which would prompt) once that says yes.

mod native;
mod stream;
mod windows;
mod writer;

use std::sync::Arc;

use objc2_app_kit::NSWorkspace;
use objc2_foundation::{NSString, NSURL};

use crate::error::ScreenError;
use crate::ports::{Platform, ScreenPermission};

/// System Settings → Privacy & Security → Screen & System Audio Recording.
const SCREEN_RECORDING_PAGE: &str =
    "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture";

/// The Mac's screen ports.
pub(crate) fn platform() -> Platform {
    Platform {
        permission: Arc::new(MacPermission),
        windows: Arc::new(windows::MacWindows),
        source: Arc::new(stream::MacSource),
        encoders: Arc::new(writer::MacEncoders),
    }
}

/// Opens the Screen Recording page of System Settings.
pub(crate) fn open_settings() -> Result<(), ScreenError> {
    let url = NSURL::URLWithString(&NSString::from_str(SCREEN_RECORDING_PAGE))
        .ok_or(ScreenError::SettingsUnavailable)?;
    if NSWorkspace::sharedWorkspace().openURL(&url) {
        Ok(())
    } else {
        Err(ScreenError::SettingsUnavailable)
    }
}

/// Asks macOS for Screen Recording; true when granted.
pub(crate) fn request_permission() -> bool {
    native::request_screen_capture()
}

/// Screen Recording through `CGPreflightScreenCaptureAccess`, which never
/// shows the prompt.
#[derive(Clone, Copy, Debug)]
pub(crate) struct MacPermission;

impl ScreenPermission for MacPermission {
    fn granted(&self) -> bool {
        native::preflight_screen_capture()
    }
}
