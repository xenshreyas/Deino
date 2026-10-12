//! The transcription section's cloud option (CAP-14): a service that
//! transcribes each meeting again after it ends, on the user's own key.
//! Live transcription stays on the Mac either way.

use serde::{Deserialize, Serialize};

/// The cloud service that re-transcribes meetings after they end. Off
/// ([`Self::None`]) unless the user turns it on; the key lives in the
/// Keychain, never here.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type,
)]
#[serde(rename_all = "snake_case")]
pub enum CloudTranscriptionProvider {
    /// Transcripts are made on this Mac only.
    #[default]
    None,
    /// ElevenLabs Scribe, over the user's ElevenLabs API key.
    #[serde(rename = "elevenlabs")]
    ElevenLabs,
}

/// The ElevenLabs speech-to-text model (`model_id`).
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type,
)]
pub enum CloudTranscriptionModel {
    /// Scribe v2, ElevenLabs' current batch model.
    #[default]
    #[serde(rename = "scribe_v2")]
    ScribeV2,
    /// Scribe v1, the first generation (deprecated by ElevenLabs).
    #[serde(rename = "scribe_v1")]
    ScribeV1,
}

impl CloudTranscriptionModel {
    /// The `model_id` ElevenLabs knows the model by.
    pub const fn id(self) -> &'static str {
        match self {
            Self::ScribeV2 => "scribe_v2",
            Self::ScribeV1 => "scribe_v1",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_match_what_elevenlabs_and_the_ui_expect() {
        for model in [
            CloudTranscriptionModel::ScribeV2,
            CloudTranscriptionModel::ScribeV1,
        ] {
            assert_eq!(serde_json::to_value(model).unwrap(), model.id());
        }
        let provider = serde_json::to_value(CloudTranscriptionProvider::ElevenLabs).unwrap();
        assert_eq!(provider, "elevenlabs");
        assert_eq!(
            serde_json::to_value(CloudTranscriptionProvider::None).unwrap(),
            "none"
        );
    }
}
