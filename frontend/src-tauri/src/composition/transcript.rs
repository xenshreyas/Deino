//! Wiring for the transcript stages: `final_transcription` (CAP-08) over
//! `deino-recording`'s `OfflineTranscriber` (the finished tracks decoded
//! with `ExtAudioFile`, the batch engine chosen in Settings), its re-pass
//! setting, and `vocab_fixups` (TRN-05) over the vocabulary's fixups; and
//! the user's transcript edits (TRN-02, ADR 0040), re-indexed through the
//! `ReindexQueue` that `search::attach_gateway_and_search` manages.

use std::sync::Arc;

use async_trait::async_trait;
use deino_core::final_transcription::{HeardSpan, TrackError, TrackRequest, TrackTranscriber};
use deino_core::search::ReindexQueue;
use deino_core::segments::SqliteSegmentRepo;
use deino_core::settings::SettingsService;
use deino_core::transcript::{TranscriptEditDeps, TranscriptEditService, TranscriptIndexer};
use deino_core::vocabulary::FixupService;
use deino_db::Database;
use deino_domain::Clock;
use deino_recording::{NativeDecoder, OfflineTranscriber};
use tauri::{AppHandle, Manager};

use super::attach::Phase;
use super::pipeline::TranscriptStages;
use crate::stt::{SttBatchEngines, SttRuntime};

/// The transcript stages' services, once the settings and the vocabulary
/// are managed.
pub(super) fn stages(app: &AppHandle) -> Option<TranscriptStages> {
    let settings = Arc::clone(&*app.try_state::<Arc<SettingsService>>()?);
    let fixups = Arc::clone(&*app.try_state::<Arc<FixupService>>()?);
    Some(TranscriptStages {
        transcriber: track_transcriber(app, &settings),
        repass: settings,
        vocab_fixer: fixups,
    })
}

/// Finished tracks through the STT runtime, or a stand-in that reports
/// transcription as unavailable when the runtime could not start.
fn track_transcriber(
    app: &AppHandle,
    settings: &Arc<SettingsService>,
) -> Arc<dyn TrackTranscriber> {
    let Some(stt) = app.try_state::<Arc<SttRuntime>>() else {
        log::warn!(target: "pipeline", "speech-to-text is not ready; recordings cannot be transcribed after the meeting");
        return Arc::new(NoTranscription);
    };
    let engines = SttBatchEngines::new(
        app.clone(),
        Arc::clone(&stt),
        Arc::clone(settings),
        tauri::async_runtime::handle().inner().clone(),
    );
    Arc::new(OfflineTranscriber::new(
        Arc::new(engines),
        Arc::new(NativeDecoder),
    ))
}

/// Transcription without an STT runtime: always unavailable.
struct NoTranscription;

#[async_trait]
impl TrackTranscriber for NoTranscription {
    async fn transcribe(&self, _request: TrackRequest) -> Result<Vec<HeardSpan>, TrackError> {
        Err(TrackError::Unavailable(
            "speech-to-text could not start".into(),
        ))
    }
}

super::attach::attach!(
    "transcript-edits",
    30,
    Phase::Background(|cx| {
        attach_transcript_edits(cx.app, cx.database, cx.clock);
    })
);

/// Manages `Arc<TranscriptEditService>`; logs and skips it when search (and
/// so the re-index queue) was not wired.
pub(super) fn attach_transcript_edits(
    app: &AppHandle,
    database: &Database,
    clock: &Arc<dyn Clock>,
) {
    let Some(queue) = app.try_state::<Arc<ReindexQueue>>() else {
        log::error!(target: "startup", "the re-index queue is missing; transcript edits are unavailable");
        return;
    };
    let indexer: Arc<dyn TranscriptIndexer> = queue.inner().clone();
    let segments = Arc::new(SqliteSegmentRepo::new(database.clone()));
    let service = TranscriptEditService::new(TranscriptEditDeps {
        editor: segments.clone(),
        writer: segments,
        indexer,
        clock: Arc::clone(clock),
    });
    app.manage(Arc::new(service));
}
