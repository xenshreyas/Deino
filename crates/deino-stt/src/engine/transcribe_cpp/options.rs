//! The options every call to a loaded model uses, and what it reports back.

use transcribe_cpp::{
    Capabilities, CommitPolicy, DeviceType, Model, ParakeetBufferedStreamOptions, RunExtension,
    RunOptions, StreamExtension, StreamOptions, TimestampKind, WhisperRunOptions,
};

use crate::protocol::LoadedInfo;
use crate::window::StreamWindow;

/// The seed of Whisper's sampling when it falls back to a higher
/// temperature. Fixed, so the same audio always gives the same transcript:
/// a re-transcription repeats, and a clip transcribed during the meeting
/// is the clip the pass after it would transcribe. Unset, transcribe-cpp
/// draws a new seed on every run.
pub const WHISPER_SEED: u32 = 7;

/// Token timestamps where the model has them; English pinned for multilingual
/// models (Deino is English-only), left unset for English-only ones; Whisper
/// sampling seeded with [`WHISPER_SEED`]. `arch` is the model family.
pub(super) fn run_options(capabilities: &Capabilities, arch: &str) -> RunOptions {
    RunOptions {
        timestamps: timestamps(capabilities.max_timestamp_kind),
        language: english_hint(&capabilities.languages),
        family: (arch == "whisper").then(|| {
            RunExtension::Whisper(WhisperRunOptions {
                seed: Some(WHISPER_SEED),
                ..WhisperRunOptions::default()
            })
        }),
        ..RunOptions::default()
    }
}

fn timestamps(finest: TimestampKind) -> TimestampKind {
    match finest {
        TimestampKind::Token => TimestampKind::Token,
        _ => TimestampKind::Auto,
    }
}

fn english_hint(languages: &[String]) -> Option<String> {
    let multilingual = languages.len() > 1;
    let english = languages.iter().any(|language| language == "en");
    (multilingual && english).then(|| "en".to_string())
}

pub(super) fn stream_options(window: Option<StreamWindow>) -> StreamOptions {
    let millis = |ms: u32| i32::try_from(ms).ok();
    StreamOptions {
        commit_policy: CommitPolicy::Auto,
        stable_prefix_agreement_n: 0,
        family: window.map(|window| {
            StreamExtension::ParakeetBuffered(ParakeetBufferedStreamOptions {
                left_ms: millis(window.left_ms()),
                chunk_ms: millis(window.chunk_ms()),
                right_ms: millis(window.right_ms()),
            })
        }),
    }
}

pub(super) fn loaded_info(model: &Model, capabilities: &Capabilities) -> LoadedInfo {
    let backend = model.backend();
    let on_gpu = model.device().map_or_else(
        |_| !backend.is_empty() && !backend.eq_ignore_ascii_case("cpu"),
        |device| matches!(device.device_type, DeviceType::Gpu | DeviceType::Igpu),
    );
    LoadedInfo {
        arch: model.arch(),
        backend,
        on_gpu,
        supports_streaming: capabilities.supports_streaming,
        max_audio_ms: capabilities.max_audio_ms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multilingual_models_are_pinned_to_english() {
        assert_eq!(
            english_hint(&["en".into(), "de".into()]).as_deref(),
            Some("en")
        );
        assert_eq!(english_hint(&["en".into()]), None);
    }

    #[test]
    fn token_timestamps_are_asked_for_only_where_the_model_has_them() {
        assert_eq!(timestamps(TimestampKind::Token), TimestampKind::Token);
        assert_eq!(timestamps(TimestampKind::Segment), TimestampKind::Auto);
    }

    #[test]
    fn the_live_window_becomes_a_parakeet_buffered_extension() {
        let options = stream_options(Some(StreamWindow::LIVE));

        assert_eq!(
            options.family,
            Some(StreamExtension::ParakeetBuffered(
                ParakeetBufferedStreamOptions {
                    left_ms: Some(5600),
                    chunk_ms: Some(560),
                    right_ms: Some(320),
                }
            ))
        );
    }
}
