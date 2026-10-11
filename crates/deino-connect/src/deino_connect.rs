//! Deino's connections to the user's online calendar accounts.
//!
//! The calendar sources that sign in to a third-party account and then read
//! from it live here, so `deino-core` stays free of a network client
//! (PARITY_PLAN §4.0; the transcription and diarization crates build on the
//! core):
//!
//! - [`calendar`]: the Google Calendar and Microsoft Graph calendar sources,
//!   `deino_core::calendar::CalendarSource`s next to macOS Calendar
//!   (`deino-eventkit`), with their refresh tokens in the Keychain.
//!
//! OAuth itself (PKCE, the loopback redirect, the token endpoint, the
//! browser port) is the integrations framework's: `deino_integrations::oauth`
//! (INT-00, ADR 0031), the one OAuth layer in Deino. Every request goes
//! through a `deino-net` scoped client, so a disabled service or offline
//! mode sends nothing and each request lands in the egress log.

pub mod calendar;
