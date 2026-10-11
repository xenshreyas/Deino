//! The live pipeline: placed track audio in, two lanes of transcription out.
//!
//! ```text
//!  mic writer ── placed 48 kHz ─▶ PlacedReader ─16 kHz─┐
//!                                                      ├─▶ pair by index ─▶ AEC (system = reference)
//!  system writer ─ placed 48 kHz ▶ PlacedReader ─16 kHz─┘        │
//!                                         Me lane: VAD + engine ◀┤ (cleaned mic)
//!                                       Them lane: VAD + engine ◀┘ (system)
//! ```
//!
//! | Module | Role |
//! |---|---|
//! | [`placed`] | A track's placed audio as a gap-free 16 kHz stream |
//! | [`lanes`] | A lane's VAD and transcriber feed |
//! | [`pipeline`] | The pipeline thread: pairing, echo cancellation, keep-up padding |

pub mod lanes;
pub mod pipeline;
pub mod placed;

pub use lanes::LaneInput;
pub use pipeline::{LiveParts, LivePipeline};
pub use placed::PlacedReader;
