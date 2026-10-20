//! Speaker tags (CAP-05): the meeting app's names as priors for naming a
//! meeting's speakers.
//!
//! While a meeting records, `deino-speaker-tags` reads who the meeting app
//! (Zoom, Teams, Meet in Chrome) shows as the active speaker and appends
//! [`TagSpan`](deino_domain::TagSpan)s through [`SpeakerTagWriter`]. When the
//! pipeline reaches `identify_speakers` (CAP-04), [`TagPriors`] turns them
//! into a [`TagPlan`] **before** voices are matched:
//!
//! 1. `one_to_one`: when the call window listed exactly one remote person
//!    for the whole call (the roster, [`SpeakerTagReader::roster`]), that
//!    person wins every system-track segment; otherwise [`align`]: a name
//!    wins a system-track segment when its spans cover more than half of it;
//! 2. `resolve`: a name stands for an invited attendee or a known person
//!    with that name, or stays a label;
//! 3. [`seats`]: each name gets one speaker (the person's, the labelled
//!    one, the diarized voice it wins most of, or a new one) and its
//!    segments move there, never off a speaker the user named;
//! 4. `plan`: the result as rows to write ([`crate::speakers::Seating`]).
//!
//! Voice matching then names only what the names left open, so a name from
//! the window always overrides a voiceprint. A meeting without spans gets an
//! empty plan, and identification works exactly as before.
//!
//! Ports: [`SpeakerTagWriter`], [`SpeakerTagReader`] (both implemented by
//! [`SqliteSpeakerTagRepo`]) and [`TagPriors`] ([`SpeakerTagPriors`], or
//! [`NoSpeakerTags`]).

pub mod align;
#[cfg(any(test, feature = "test-support"))]
mod fake;
pub mod one_to_one;
mod plan;
mod priors;
mod repo;
mod resolve;
pub mod seats;

pub use align::{Timed, Win};
#[cfg(any(test, feature = "test-support"))]
pub use fake::FakeSpeakerTagRepo;
pub use plan::TagPlan;
pub use priors::{NoSpeakerTags, SpeakerTagPriors, TagPriorPorts, TagPriors};
pub use repo::{SpeakerTagReader, SpeakerTagWriter, SqliteSpeakerTagRepo};
