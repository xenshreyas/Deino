//! Cloud transcription after the meeting (CAP-14): ElevenLabs Scribe on the
//! user's own key, behind `deino_core::final_transcription`'s
//! `CloudTranscriber` and `CloudChoice` ports.
//!
//! | Module | What |
//! |---|---|
//! | `api` | [`ScribeApi`]: `POST /v1/speech-to-text` and `GET /v1/user` (the key check), errors mapped |
//! | `multipart` | The `multipart/form-data` upload body |
//! | `wire` | The documented response shapes |
//! | `utterances` | Scribe's words grouped into utterances, with their voices |
//! | `transcriber` | [`ElevenLabsTranscriber`]: one track per request |
//! | `service` | [`ElevenLabs`]: the key (save, check, forget), the choice per meeting |
//!
//! Requests go through `deino-net`'s scoped client for
//! `ServiceId::ElevenLabs`; the key lives in the Keychain. Nothing here logs
//! the key, audio or what was said.

mod api;
mod multipart;
mod service;
mod transcriber;
mod utterances;
mod wire;

pub use api::{API_HOST, ScribeApi, ScribeOptions};
pub use service::{CloudSettings, ElevenLabs, ElevenLabsParts, KeyError};
pub use transcriber::{ElevenLabsTranscriber, MAX_UPLOAD_BYTES};
pub use utterances::utterances;
pub use wire::{ScribeTranscript, ScribeWord, ScribeWordKind};
