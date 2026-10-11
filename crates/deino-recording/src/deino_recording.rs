//! Deino's live recording pipeline (FND-05b, CAP-02).
//!
//! Rust owns a recording from start to stop: it creates the meeting, records
//! the microphone and system audio as separate crash-safe tracks
//! (`deino-audio`), transcribes them in two lanes, Me and Them, each with
//! its own voice detector and engine, removes the call's echo from the
//! microphone, and streams the segments into `transcript_segments` as they
//! are spoken. The UI only watches.
//!
//! ```text
//!   capture (deino-audio) ─▶ track writers ─▶ mic.m4a / system.m4a (crash-safe)
//!                                │ placed audio
//!                                ▼
//!   live pipeline: 16 kHz, pairs, AEC ─▶ Me lane / Them lane (VAD + engine)
//!                                                   │ text
//!                                                   ▼
//!   transcript router: holdback, drafts, batches ─▶ transcript_segments + live events
//! ```
//!
//! | Module | Role |
//! |---|---|
//! | [`service`] | `RecordingService`: start, stop, pause, resume, state; one recording at a time |
//! | [`session`] | `Session`: one recording's parts, started and stopped in order |
//! | [`recovery`] | Launch recovery of meetings left in `recording` |
//! | [`offline`] | Transcribing finished tracks after the meeting (CAP-08) |
//! | [`live`] | The live pipeline thread |
//! | [`engine`] | `LaneEngines` and the streaming and utterance lanes |
//! | [`transcript`] | The router: drafts, holdback verdicts, batched saves, live events |
//! | [`echo`] | `EchoMode`, the `EchoCanceller` port and `SonoraAec` (WebRTC AEC3) |
//! | [`bleed`] | The text holdback (OpenWhispr's, ported) |
//! | [`vad`] | Per-lane voice activity (earshot + hangover) |
//! | [`observer`] | The `StartObserver` port: who hears that a recording started (ORG-03 auto-links its event) |
//! | [`Playhead`] | Where the recorder is on a recording's timeline, for observers that stamp in recording time (CAP-05) |
//! | [`events`] | `RecordingSnapshot`, `SegmentUpdate`, `HealthNotice`, `AudioLevels` and the `RecordingEvents` port |
//! | [`monitor`] | Recording health (CAP-10): the pure `HealthMonitor` and the session's watcher thread |
//! | [`activity`] | What each lane hears and transcribes, for health and auto-stop |
//! | [`lane`] | `Lane`: Me (microphone) and Them (system audio) |
//! | [`backend`] | `DeviceCapture` (the Mac's devices) and `MockCapture` (`DEINO_MOCK_AUDIO`) |
//!
//! No Tauri and no network: the app wires real devices, the engine pool and
//! Tauri events in through the ports.

pub mod activity;
pub mod backend;
pub mod bleed;
pub mod deps;
pub mod echo;
pub mod engine;
mod error;
pub mod events;
pub mod lane;
pub mod live;
pub mod monitor;
pub mod observer;
pub mod offline;
pub mod recovery;
pub mod service;
mod session;
mod state;
#[cfg(any(test, feature = "test-support"))]
pub mod testing;
pub mod transcript;
pub mod vad;

pub use backend::{DeviceCapture, MockCapture};
pub use deps::{CaptureBackend, Preferences, RecordingDeps, RecordingPreferences};
pub use engine::{LaneEngines, LaneRequest, LaneTranscriber};
pub use error::RecordingError;
pub use events::{
    AudioLevels, HealthKind, HealthNotice, RecordingEvents, RecordingPhase, RecordingSnapshot,
    SegmentUpdate, SegmentUpdateKind,
};
pub use lane::Lane;
pub use monitor::{HealthIssue, IssueKind};
pub use observer::{NoStartObserver, StartObserver, StartObservers};
pub use offline::{BatchEngines, NativeDecoder, OfflineTranscriber, TrackDecoder};
pub use recovery::{RecoveryReport, recover_interrupted};
pub use service::{Playhead, RecordingService, StartRequest};
