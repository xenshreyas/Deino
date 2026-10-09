//! The options every call to a loaded model uses, and what it reports back.

use transcribe_cpp::{
    Capabilities, CommitPolicy, DeviceType, Model, ParakeetBufferedStreamOptions, RunOptions,
    StreamExtension, StreamOptions, TimestampKind,
};

use crate::protocol::LoadedInfo;
use crate::window::StreamWindow;

/// Token timestamps where the model has them; English pinned for multilingual
/// models (Deino is English-only), left unset for English-only ones.
pub(super) fn run_options(capabilities: &Capabilities) -> RunOptions {
    RunOptions {
        timestamps: timestamps(capabilities.max_timestamp_kind),
        language: english_hint(&capabilities.languages),
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
