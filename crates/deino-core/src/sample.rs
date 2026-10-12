//! The sample meeting (FND-11): a short, realistic standup bundled into the
//! app, so a new user can see enhanced notes, a transcript with speakers and
//! action items before recording anything. Onboarding's last step offers it.
//!
//! - [`SampleMeeting`]: the bundled resource (`sample/sample-meeting.json`,
//!   compiled in with `include_str!` so every build has it), typed and
//!   checked. Its transcript is cut from the standup fixture.
//! - [`SampleImporter`]: writes it through the meeting, speaker, segment,
//!   enhanced-notes and action-item repositories as an ordinary meeting with
//!   the fixed ID [`SAMPLE_MEETING_ID`], which marks it as the sample.
//!   Importing again returns the meeting already there instead of a copy.
//!
//! The sample has no audio and creates no people, companies or calendar
//! events, so deleting it like any other meeting leaves nothing behind
//! (the app's meeting delete also removes the sample's action items, which
//! a real meeting's items would outlive).

mod error;
mod importer;
mod model;
#[cfg(test)]
mod sample_tests;

pub use error::SampleError;
pub use importer::{SAMPLE_MEETING_ID, SampleImporter, SamplePorts};
pub use model::{SampleActionItem, SampleMeeting, SampleSegment, SampleSpeaker};
