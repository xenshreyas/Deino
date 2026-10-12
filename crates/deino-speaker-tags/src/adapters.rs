//! The per-app adapters, over two shared scanners:
//!
//! - [`described`]: native and WebKit apps describe each participant's tile
//!   or roster row in one accessibility description, "Alex Chen, Unmuted,
//!   Speaking" (Zoom, Teams);
//! - [`classed`]: Chromium exposes the page's CSS classes, so Meet's tiles
//!   are read by the classes of their name label and voice indicator.
//!
//! Each adapter is the app's vocabulary for its scanner, plus how it tells a
//! call window from the app's other windows. The vocabularies were taken
//! from dumps of each app's call window (`fixtures/`); when an app changes
//! its UI, capture a new dump (`fixtures/README.md`) and update the tables.

mod classed;
mod described;
mod meet;
mod teams;
mod zoom;

pub use meet::MeetAdapter;
pub use teams::TeamsAdapter;
pub use zoom::ZoomAdapter;
