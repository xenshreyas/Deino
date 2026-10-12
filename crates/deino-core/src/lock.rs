//! The app lock (PRV-02): Deino's window locks behind Touch ID or the Mac's
//! password after a stretch without use, and when the Mac wakes.
//!
//! [`LockService`] is a small state machine over [`LockState`], driven by the
//! `Clock` port: the UI reports activity, the app reports wake and screen
//! lock, and [`LockService::unlock`] asks the [`Authenticator`] port
//! (`LocalAuthentication` on macOS, a fake in tests). Reads are synchronous,
//! so the IPC guard can ask [`LockService::is_locked`] on every command.
//!
//! The lock protects what the window shows; recording is never paused by
//! it, and the database stays open (encryption at rest is the
//! [`crate::privacy`] module's job).

mod authenticator;
mod service;
#[cfg(test)]
mod service_tests;

#[cfg(any(test, feature = "test-support"))]
pub use authenticator::FakeAuthenticator;
pub use authenticator::{AuthError, Authenticator};
pub use service::{LockService, LockState};
