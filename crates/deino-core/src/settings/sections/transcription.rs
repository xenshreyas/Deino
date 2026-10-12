//! The transcription section: which local engine and model turn speech into
//! text. Transcription never leaves the Mac, so there is nothing else to set.

use serde::{Deserialize, Serialize};

mod heal;

pub use heal::{InstalledModels, healed_transcription};

/// The Parakeet TDT model used when that engine has no model configured.
pub const DEFAULT_PARAKEET_MODEL: &str = "parakeet-tdt-0.6b-v3-int8";

/// A local speech-to-text engine. New installs use [`Self::ParakeetUnified`];
/// the others stay available under "Advanced".
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type,
)]
#[serde(rename_all = "snake_case")]
pub enum TranscriptionEngine {
    /// NVIDIA Parakeet Unified EN 0.6B over transcribe.cpp (CAP-01): streams
    /// live text while recording. Its `model` is a catalog file ID, or empty
    /// for the quant this Mac's memory suits.
    #[default]
    #[serde(alias = "parakeetUnified")]
    ParakeetUnified,
    /// NVIDIA Parakeet TDT over ONNX Runtime.
    Parakeet,
    /// whisper.cpp. The legacy settings called it `localWhisper`.
    #[serde(alias = "localWhisper")]
    Whisper,
}

impl TranscriptionEngine {
    /// The engine's name as Settings → Transcription shows it.
    pub fn label(self) -> &'static str {
        match self {
            Self::ParakeetUnified => "Parakeet Unified",
            Self::Parakeet => "Parakeet TDT v3",
            Self::Whisper => "Whisper",
        }
    }

    /// The model this engine uses when none is set: Parakeet TDT names its
    /// model; empty lets the others pick (Parakeet Unified by memory size).
    pub fn default_model(self) -> &'static str {
        match self {
            Self::Parakeet => DEFAULT_PARAKEET_MODEL,
            Self::ParakeetUnified | Self::Whisper => "",
        }
    }
}

/// The transcription section of `Settings`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct TranscriptionSettings {
    /// The engine that transcribes recordings.
    pub engine: TranscriptionEngine,
    /// The engine's model; empty means the engine's recommended model.
    /// Parakeet Unified takes a catalog file ID such as
    /// `parakeet-unified-en-0.6b-Q8_0` (the file name without `.gguf`; the
    /// file name itself is accepted too, see `ModelFileId::from_setting`);
    /// Parakeet TDT a model folder such as `parakeet-tdt-0.6b-v3-int8`;
    /// Whisper a model name such as `large-v3-turbo`.
    pub model: String,
    /// "Live transcription" (CAP-08): on, the transcript appears while the
    /// meeting is recorded; off, only audio is recorded (no engine runs) and
    /// the transcript is written after the meeting.
    pub live_transcription: bool,
    /// "High-accuracy re-pass after meeting" (CAP-08): transcribe the whole
    /// recording again with full context and replace the live transcript,
    /// keeping its speakers.
    pub high_accuracy_repass: bool,
}

impl Default for TranscriptionSettings {
    fn default() -> Self {
        Self {
            engine: TranscriptionEngine::ParakeetUnified,
            model: TranscriptionEngine::ParakeetUnified
                .default_model()
                .to_owned(),
            live_transcription: true,
            high_accuracy_repass: false,
        }
    }
}

crate::settings::register_section! {
    /// The local transcription engine.
    transcription: TranscriptionSettings,
    order: 40,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_legacy_whisper_name_still_parses() {
        let engine: TranscriptionEngine = serde_json::from_str(r#""localWhisper""#).unwrap();
        assert_eq!(engine, TranscriptionEngine::Whisper);
        assert_eq!(serde_json::to_value(engine).unwrap(), "whisper");
    }

    #[test]
    fn a_file_without_the_capture_modes_transcribes_live_without_a_re_pass() {
        let settings: TranscriptionSettings =
            serde_json::from_str(r#"{"engine":"parakeet","model":"x"}"#).unwrap();

        assert!(settings.live_transcription);
        assert!(!settings.high_accuracy_repass);
        assert_eq!(settings.engine, TranscriptionEngine::Parakeet);
    }

    #[test]
    fn new_installs_use_parakeet_unified_at_the_recommended_quant() {
        let settings = TranscriptionSettings::default();
        assert_eq!(settings.engine, TranscriptionEngine::ParakeetUnified);
        assert!(settings.model.is_empty());
        let engine: TranscriptionEngine = serde_json::from_str(r#""parakeetUnified""#).unwrap();
        assert_eq!(engine, TranscriptionEngine::ParakeetUnified);
        assert_eq!(serde_json::to_value(engine).unwrap(), "parakeet_unified");
    }
}
