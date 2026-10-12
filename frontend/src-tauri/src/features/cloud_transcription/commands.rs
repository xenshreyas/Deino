//! Commands of the `cloud-transcription` plugin: parse input, call one
//! service method, map the error.

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

use std::sync::Arc;

use deino_cloud_transcription::ElevenLabs;
use deino_core::final_transcription::OriginalTranscriptService;
use deino_domain::{AppError, MeetingId, StageId};
use deino_pipeline::{PipelineService, StageStatus};
use deino_secrets::SecretStatus;
use tauri::{Runtime, State};

use crate::error::IntoIpc;

/// The ElevenLabs service, as Tauri state.
type Cloud<'a> = State<'a, Arc<ElevenLabs>>;
/// The kept original transcripts, as Tauri state.
type Originals<'a> = State<'a, Arc<OriginalTranscriptService>>;
/// The post-meeting pipeline, as Tauri state.
type Pipeline<'a> = State<'a, Arc<PipelineService>>;

/// The stage the cloud transcribes in.
const TRANSCRIPT_STAGE: &str = "final_transcription";
/// The first stage after the transcript: speakers, then everything else.
const SPEAKERS_STAGE: &str = "diarize";

/// The plugin's commands, collected here so they can stay private.
pub(super) fn collect<R: Runtime>() -> tauri_specta::Commands<R> {
    tauri_specta::collect_commands![
        key_status,
        save_key,
        check_key,
        forget_key,
        retranscribe,
        has_original,
        restore_original
    ]
}

/// Whether an ElevenLabs key is saved: `missing`, `stored`, or `locked`
/// when macOS refused access. Never the key.
#[tauri::command]
#[specta::specta]
async fn key_status(cloud: Cloud<'_>) -> Result<SecretStatus, AppError> {
    cloud.key_status().await.into_ipc()
}

/// Saves `key` in the Keychain and switches ElevenLabs on in Settings →
/// Privacy → Network. Fails with `invalid` for a blank key.
#[tauri::command]
#[specta::specta]
async fn save_key(cloud: Cloud<'_>, key: String) -> Result<(), AppError> {
    cloud.save_key(&key).await.into_ipc()
}

/// Asks ElevenLabs whether it knows the saved key: `permission_denied` when
/// it refused it, `unavailable` when none is saved, `network` when it could
/// not be asked.
#[tauri::command]
#[specta::specta]
async fn check_key(cloud: Cloud<'_>) -> Result<(), AppError> {
    cloud.check_key().await.into_ipc()
}

/// Forgets the key and switches ElevenLabs off.
#[tauri::command]
#[specta::specta]
async fn forget_key(cloud: Cloud<'_>) -> Result<(), AppError> {
    cloud.forget_key().await.into_ipc()
}

/// Transcribes the meeting again with ElevenLabs, whatever the setting
/// says, then runs every later step on the new transcript; returns the
/// meeting's steps. Fails with `unavailable` when no key is saved.
#[tauri::command]
#[specta::specta]
async fn retranscribe(
    cloud: Cloud<'_>,
    pipeline: Pipeline<'_>,
    meeting_id: MeetingId,
) -> Result<Vec<StageStatus>, AppError> {
    cloud.request(&meeting_id).await.into_ipc()?;
    let stage = StageId::new(TRANSCRIPT_STAGE);
    pipeline.rerun_from(&meeting_id, &stage).await.into_ipc()
}

/// Whether the transcript made on this Mac was kept when ElevenLabs
/// replaced it.
#[tauri::command]
#[specta::specta]
async fn has_original(originals: Originals<'_>, meeting_id: MeetingId) -> Result<bool, AppError> {
    originals.has_original(&meeting_id).await.into_ipc()
}

/// Puts the kept transcript back, then runs the speaker step and every
/// later one on it; `false` when none was kept.
#[tauri::command]
#[specta::specta]
async fn restore_original(
    originals: Originals<'_>,
    pipeline: Pipeline<'_>,
    meeting_id: MeetingId,
) -> Result<bool, AppError> {
    if !originals.restore(&meeting_id).await.into_ipc()? {
        return Ok(false);
    }
    let stage = StageId::new(SPEAKERS_STAGE);
    pipeline.rerun_from(&meeting_id, &stage).await.into_ipc()?;
    Ok(true)
}
