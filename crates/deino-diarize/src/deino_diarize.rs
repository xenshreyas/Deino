//! Deino's local speaker diarization (CAP-03): who spoke when, on the Mac.
//!
//! A port of anarlog's `pyannote-local`, `embedding` and `voiceprint` crates
//! (MIT), the engine the diarization spike chose
//! (`docs/references/spike-diarization.md`): pyannote segmentation-3.0 on the
//! CPU, WeSpeaker ResNet34-LM embeddings on Core ML, centroid-linkage
//! clustering bounded by the calendar's speaker count.
//!
//! ```text
//!  16 kHz track ─▶ 10 s windows ─▶ Segmenter (batched) ─▶ local speakers
//!                                  filterbank (knf-rs) ─▶ EmbeddingExtractor (batched)
//!                ─▶ clustering (anchors, bounds) ─▶ known voiceprints ─▶ majority vote
//!                ─▶ Diarization { turns, speakers (256-d unit centroids) } ─▶ prune
//! ```
//!
//! | Module | Role |
//! |---|---|
//! | [`pipeline`] | [`Diarizer`]: models, [`DiarizeRequest`], [`Diarization`] |
//! | [`config`] / [`policy`] | Tuning knobs; per-meeting parameters by length |
//! | [`segmentation`], [`embedding`] | The two ONNX models |
//! | [`clustering`] | Centroid-linkage clustering under speaker bounds |
//! | [`prune`] | Phantom-speaker pruning and collapse detection (OpenWhispr rules) |
//! | [`online`] | Live online clustering for CAP-02 |
//! | [`metrics`] | Diarization error rate for the benchmark |
//! | [`models`] | Pinned model files, the verified store, the downloader port |
//! | [`track`] | The `deino-core` `TrackDiarizer` adapter (decode, run, prune) |
//! | [`vbx`] | Community-1's clustering: AHC, PLDA, VBx, constrained assignment |
//! | [`align`] | The `deino-core` `WordAligner` adapter: wav2vec2 CTC word times |
//!
//! No `tauri`, no `deino-net`: the store downloads through `deino-core`'s
//! `ByteSource` port. The voiceprint matching rules and enrolment spans moved
//! to `deino-core::voiceprints` (CAP-04), where `identify_speakers` uses
//! them; the known-speaker merge here imports them from there.

pub mod align;
pub mod clustering;
pub mod config;
pub mod embedding;
pub mod error;
pub mod metrics;
pub mod models;
pub mod online;
mod onnx;
pub mod pipeline;
pub mod policy;
pub mod prune;
pub mod python;
pub mod segmentation;
pub mod track;
pub mod units;
pub mod vbx;
pub mod vector;

pub use config::{DiarizationConfig, RuntimeConfig, SpeakerBounds};
pub use error::DiarizeError;
pub use pipeline::{
    AudioSource, Diarization, DiarizeRequest, DiarizedSpeaker, Diarizer, KnownSpeaker, ModelPaths,
    SpeakerIdentity, SpeakerTurn,
};
pub use units::SAMPLE_RATE;
pub use vector::Embedding;
