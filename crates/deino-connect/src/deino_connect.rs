//! Deino's connections to the user's online accounts.
//!
//! Everything that signs in to a third-party account and then reads from it
//! lives here, so `deino-core` stays free of a network client (PARITY_PLAN
//! §4.0; the transcription and diarization crates build on the core):
//!
//! - [`oauth`]: OAuth 2.0 with PKCE over a loopback redirect, the minimal
//!   framework ORG-01 needs and INT-00 adopts for every integration;
//! - [`calendar`]: the Google Calendar and Microsoft Graph calendar sources,
//!   `deino_core::calendar::CalendarSource`s next to macOS Calendar
//!   (`deino-eventkit`), with their refresh tokens in the Keychain.
//!
//! Every request goes through a `deino-net` scoped client, so a disabled
//! service or offline mode sends nothing and each request lands in the
//! egress log.

pub mod calendar;
#[cfg(any(test, feature = "test-support"))]
mod fake;
pub mod oauth;

#[cfg(any(test, feature = "test-support"))]
pub use fake::FakeBrowser;
