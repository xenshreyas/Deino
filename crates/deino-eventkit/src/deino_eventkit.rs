//! macOS Calendar through `EventKit`: Deino's default calendar source
//! (ORG-01).
//!
//! macOS Calendar already holds every account the user added to the Mac
//! (iCloud, Google, Exchange, `CalDAV`, subscriptions), so reading it needs no
//! OAuth and no network: one Calendars permission, asked for only when the
//! user turns calendar sync on (PARITY_PLAN §3C).
//!
//! [`EventKitSource`] implements `deino_core::calendar::CalendarSource`. All
//! `EventKit` work runs on one dedicated thread (`worker.rs`) that owns the
//! event store; `read.rs` turns events into the core's plain values, with
//! every raw code interpreted in `convert.rs` (pure, tested everywhere). The
//! store's change notification and the Mac's wake notification nudge the
//! sync scheduler (`observe.rs`). Off macOS the source reports
//! `Unsupported`.

#[cfg_attr(
    not(target_os = "macos"),
    allow(
        dead_code,
        reason = "the conversions are tested on every platform, but only macOS reads EventKit"
    )
)]
mod convert;
#[cfg(target_os = "macos")]
mod observe;
#[cfg(target_os = "macos")]
mod privacy;
#[cfg(target_os = "macos")]
mod read;
mod source;
#[cfg(target_os = "macos")]
mod store;
#[cfg(target_os = "macos")]
mod worker;

pub use source::{EventKitSource, FETCH_TIMEOUT};
