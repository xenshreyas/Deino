//! ElevenLabs' answers, as the API reference documents them
//! (`https://elevenlabs.io/docs/api-reference/speech-to-text/convert`,
//! checked 2026-10-12). Unknown fields are ignored, so additions to the API
//! do not break a transcript.

use serde::Deserialize;

/// The 200 answer of `POST /v1/speech-to-text` for one channel.
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct ScribeTranscript {
    /// The detected language (`en`; Deino always asks for English).
    #[serde(default)]
    pub language_code: String,
    /// Every word, space and audio event, in order.
    #[serde(default)]
    pub words: Vec<ScribeWord>,
}

/// One item of [`ScribeTranscript::words`].
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct ScribeWord {
    /// The word, the space, or the event (`(laughter)`).
    pub text: String,
    /// `word`, `spacing` or `audio_event`.
    #[serde(rename = "type")]
    pub kind: ScribeWordKind,
    /// Start, in seconds from the start of the file.
    #[serde(default)]
    pub start: Option<f64>,
    /// End, in seconds.
    #[serde(default)]
    pub end: Option<f64>,
    /// The voice (`speaker_0`), when the request asked to diarize.
    #[serde(default)]
    pub speaker_id: Option<String>,
}

/// What a [`ScribeWord`] is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScribeWordKind {
    /// A spoken word, punctuation attached.
    Word,
    /// The space between words.
    Spacing,
    /// A sound that is not speech; Deino asks for none and skips any.
    AudioEvent,
    /// A kind added after 2026-10-12; skipped.
    #[serde(other)]
    Other,
}

/// An error answer: `{"detail": {"type", "code", "message", ...}}`. Older
/// answers carry `status` instead of `code`, or a list (422 validation).
#[derive(Clone, Debug, Default, Deserialize)]
pub(crate) struct ErrorBody {
    #[serde(default)]
    pub(crate) detail: Option<serde_json::Value>,
}

impl ErrorBody {
    /// The specific cause (`detail.code`, else the legacy `detail.status`).
    pub(crate) fn code(&self) -> Option<&str> {
        let detail = self.detail.as_ref()?;
        detail
            .get("code")
            .or_else(|| detail.get("status"))
            .and_then(serde_json::Value::as_str)
    }
}
