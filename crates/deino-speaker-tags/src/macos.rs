//! The macOS adapters behind [`crate::ports`]: Accessibility
//! (`AXIsProcessTrusted`, never the prompting variant), running apps
//! (`NSRunningApplication`), window trees (`AXUIElement`) and the System
//! Settings page where Accessibility is granted.

mod dom;
mod reader;
mod workspace;

use std::sync::Arc;

use crate::ports::Platform;

/// This Mac's ports.
pub(crate) fn platform() -> Platform {
    Platform {
        permission: Arc::new(workspace::Trust),
        apps: Arc::new(workspace::Workspace),
        windows: Arc::new(reader::AxWindows::live()),
    }
}

pub(crate) use reader::AxWindows;
pub(crate) use workspace::open_accessibility_settings;
