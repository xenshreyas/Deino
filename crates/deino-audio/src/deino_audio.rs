//! Deino's crash-safe audio recording (FND-05a).
//!
//! Records the microphone and system audio as two separate, sample-aligned
//! tracks that survive `kill -9` (≤ 1 s lost) and power loss (≤ 5 s lost),
//! without FFmpeg (ADR 0014, `docs/references/spike-native-recording.md`).
//!
//! ```text
//!  AudioSource ─RT─▶ RtSink ═ring═▶ writer thread ──▶ TrackEncoder ──▶ TrackSink
//!  (mock, later    (ring)          TrackTimeline      (AVAssetWriter    (.m4a.part)
//!   tap and mic)                   + Backlog           CMAF, 1 s)           │
//!                                                                  persister: F_FULLFSYNC
//!                                                                  + recording.json, 5 s
//!  launch:  recovery::recover_recording ── truncate_to_playable ── rename ── manifest
//!  stop:    finalize::finalize_recording ── mic.m4a  system.m4a  audio.mp4
//! ```
//!
//! | Module | Role |
//! |---|---|
//! | [`units`] | `Track`, `Frames`, `Millis`, `HostTime`, `Bytes`; the sample index is time |
//! | [`clock`] | The `HostClock` port (monotonic, Core Audio's host time domain) |
//! | [`timeline`] | `TrackTimeline`: gaps become silence, tracks anchor to the epoch |
//! | [`ring`] | The real-time boundary: `RtSink` / `RtSource` over `rtrb` |
//! | [`source`] | The `AudioSource` and `SourceFactory` ports, `MockAudioSource` (`DEINO_MOCK_AUDIO`) |
//! | [`capture`] | The cpal microphone, the process tap, conditioners and the capture supervisor |
//! | [`encoder`] | The `TrackEncoder` port and the `AVAssetWriter` CMAF encoder |
//! | [`sink`] | The `TrackSink` port, `.part` files and the segment ledger |
//! | [`writer`] | The writer thread: ring → timeline → backlog → encoder |
//! | [`persister`] | Every 5 s: full fsync, then the atomic manifest |
//! | [`manifest`] | `recording.json`: typed, versioned, atomically written |
//! | [`fmp4`] | Fragmented MP4 box scanning and `truncate_to_playable` |
//! | [`recovery`] | Launch recovery of unclosed recordings |
//! | [`finalize`] | Flat per-track files and the −6 dB mix |
//! | [`decode`] | A finished track back as mono PCM at any rate (post-meeting stages) |
//! | [`health`] | `HealthEvent`s on a bounded channel; the `DiskSpace` port |
//! | [`recorder`] | `Recorder`: one recording, start to stop |
//!
//! Never decode Deino's AAC files with symphonia 0.5 without trimming the
//! 2112 frames of encoder priming ([`fmp4::AAC_PRIMING`]); it ignores the
//! edit list and every timestamp comes out 44 ms late. Use `ExtAudioFile` or
//! `AVFoundation`, as [`finalize`] does.
//!
//! This crate has no Tauri and no network dependency. Native encoding and
//! finalize are macOS-only; everything else (timeline, manifest, recovery,
//! writer logic) builds and is tested on every platform.

pub mod capture;
pub mod clip;
pub mod clock;
pub mod decode;
pub mod durability;
pub mod encoder;
pub mod finalize;
pub mod fmp4;
pub mod health;
pub mod manifest;
pub mod persister;
pub mod recorder;
pub mod recovery;
pub mod ring;
pub mod sink;
pub mod source;
pub mod timeline;
pub mod units;
pub mod writer;

#[cfg(test)]
mod fmp4_fixture;

#[cfg(any(test, feature = "test-support"))]
pub use clock::FakeHostClock;
pub use clock::{HostClock, SystemHostClock};
pub use decode::{DecodeError, read_mono};
#[cfg(target_os = "macos")]
pub use encoder::CmafEncoderFactory;
#[cfg(any(test, feature = "test-support"))]
pub use encoder::FakeEncoderFactory;
pub use encoder::{EncodeError, EncoderFactory, TrackEncoder};
pub use finalize::{FinalizeError, FinalizedAudio, finalize_recording};
#[cfg(any(test, feature = "test-support"))]
pub use health::FakeDiskSpace;
pub use health::{DiskSpace, HealthEvent, SystemDiskSpace};
pub use manifest::{RecordingManifest, RecordingStatus};
pub use recorder::{
    PlacedAudio, RecordError, Recorder, RecorderDeps, RecordingInputs, RecordingSummary,
};
pub use recovery::{Recovery, RecoveryError, find_unclosed, recording_dir, recover_recording};
pub use ring::{RtSink, TimelineSource};
#[cfg(any(test, feature = "test-support"))]
pub use sink::MemorySink;
pub use sink::TrackSink;
pub use source::{
    AudioSource, MockAudio, MockAudioSource, MockSources, SourceError, SourceFactory, SourceStream,
    TestSignal,
};
pub use timeline::{Placement, TrackTimeline};
pub use units::{Bytes, Frames, HostTime, Millis, STORAGE_RATE, SampleRate, Track};
