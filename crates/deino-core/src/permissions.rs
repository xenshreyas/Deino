//! The macOS permissions Deino asks for during onboarding (FND-11, §3C):
//! microphone, system audio, notifications and Accessibility. Calendar
//! access belongs to [`crate::calendar`].
//!
//! - [`PermissionKind`], [`PermissionState`], [`PermissionStatus`]: what the
//!   UI shows for each permission.
//! - [`PermissionGate`]: the port onto the operating system (the app's
//!   `permissions` adapter on macOS, a stub elsewhere).
//! - [`PermissionService`]: reads every status, asks for one and opens its
//!   System Settings page. macOS has no way to read some permissions without
//!   asking (system audio capture), so the service remembers the answer to
//!   the last request for those.

mod error;
#[cfg(any(test, feature = "test-support"))]
mod fake;
mod model;
mod service;
#[cfg(test)]
mod service_tests;

pub use error::PermissionError;
#[cfg(any(test, feature = "test-support"))]
pub use fake::FakePermissionGate;
pub use model::{PermissionKind, PermissionState, PermissionStatus};
pub use service::{PermissionGate, PermissionService};
