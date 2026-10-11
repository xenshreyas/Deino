//! The configured transcription engine, typed (`Settings::transcription`).
//!
//! Replaces the inherited string bridge (`"parakeetUnified"`, `"parakeet"`,
//! `"localWhisper"`): callers match on [`TranscriptionEngine`].

use deino_core::settings::{TranscriptionEngine, TranscriptionSettings};
use tauri::{AppHandle, Runtime};

/// The engine and model chosen in Settings.
pub async fn transcription<R: Runtime>(app: &AppHandle<R>) -> TranscriptionSettings {
    crate::features::settings::current(app).await.transcription
}

/// The model configured for `engine`: `Some` only when `engine` is the
/// selected one and names a model.
pub async fn model_for<R: Runtime>(
    app: &AppHandle<R>,
    engine: TranscriptionEngine,
) -> Option<String> {
    let settings = transcription(app).await;
    let model = settings.model.trim();
    (settings.engine == engine && !model.is_empty()).then(|| model.to_owned())
}
