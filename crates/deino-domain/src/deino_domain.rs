//! Deino's shared vocabulary, used by every other crate.
//!
//! This crate has no I/O and no Tauri. It holds the types that must mean the
//! same thing everywhere:
//!
//! - [`id`]: newtype identifiers ([`MeetingId`], UUIDv7 IDs), so IDs of
//!   different entities can never be mixed up;
//! - [`error`]: [`AppError`], the only error that crosses IPC, its
//!   [`ErrorCode`] and the [`ErrorClass`] trait that module errors implement;
//! - [`clock`]: the [`Clock`] port, so time-dependent logic is testable.
//!
//! Feature models and services live in `deino-core`; persistence lives in
//! `deino-db` (PARITY_PLAN §4.0).

pub mod clock;
pub mod error;
pub mod id;

#[cfg(any(test, feature = "test-support"))]
pub use clock::FakeClock;
pub use clock::{Clock, SystemClock};
pub use error::{AppError, ErrorClass, ErrorCode};
#[cfg(debug_assertions)]
pub use id::ExampleNoteId;
pub use id::{IdError, MeetingId};
