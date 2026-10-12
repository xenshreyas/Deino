//! Importing recordings (CAP-11): an audio or video file becomes a meeting
//! that the post-meeting pipeline transcribes like any other.
//!
//! ```text
//! file ─▶ ImportService ─▶ MediaIngest ─▶ <recordings>/<meeting>/audio.m4a (+ video.<ext>)
//!                       ─▶ meetings row (source `import`) ─▶ `processing` ─▶ pipeline
//! ```
//!
//! [`ImportService`] checks the file, converts its audio through the
//! [`MediaIngest`] port (`deino-recording`'s `NativeIngest`: symphonia,
//! then `AVAssetReader`), keeps a video beside it for the player, creates
//! the meeting and moves it to `processing`, which starts the pipeline:
//! `final_transcription` transcribes `audio.m4a`, then diarization, notes
//! and the rest run as after a live recording. Calendar linking is the
//! meeting's own (ORG-03's `link_event`), offered once the meeting exists.
//!
//! Watched folders ([`crate::watch`]) import through the same service.

mod error;
#[cfg(any(test, feature = "test-support"))]
mod fake;
mod model;
mod port;
mod service;

pub use error::ImportError;
#[cfg(any(test, feature = "test-support"))]
pub use fake::FakeIngest;
pub use model::{
    AUDIO_EXTENSIONS, ImportCandidate, ImportRequest, ImportedMeeting, IngestedMedia,
    MAX_IMPORT_BYTES, MediaFacts, MediaKind, VIDEO_EXTENSIONS,
};
pub use port::{FileImporter, MediaIngest, MediaPicker};
pub use service::{ImportDeps, ImportService};
