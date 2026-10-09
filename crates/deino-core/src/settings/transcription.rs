//! The transcription section: which local engine and model turn speech into
//! text. Transcription never leaves the Mac, so there is nothing else to set.

use serde::{Deserialize, Serialize};

/// The Parakeet model used when nothing else is configured.
pub const DEFAULT_PARAKEET_MODEL: &str = "parakeet-tdt-0.6b-v3-int8";

/// A local speech-to-text engine.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type,
)]
#[serde(rename_all = "snake_case")]
pub enum TranscriptionEngine {
    /// NVIDIA Parakeet over ONNX Runtime.
    #[default]
    Parakeet,
    /// whisper.cpp. The legacy settings called it `localWhisper`.
    #[serde(alias = "localWhisper")]
    Whisper,
}

/// The transcription section of [`super::Settings`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct TranscriptionSettings {
    /// The engine that transcribes recordings.
    pub engine: TranscriptionEngine,
    /// The engine's model, for example `parakeet-tdt-0.6b-v3-int8` or `large-v3-turbo`.
    pub model: String,
}

impl Default for TranscriptionSettings {
    fn default() -> Self {
        Self {
            engine: TranscriptionEngine::Parakeet,
            model: DEFAULT_PARAKEET_MODEL.to_owned(),
        }
    }
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
}
