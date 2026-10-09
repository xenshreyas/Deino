//! The whole `diarize` stage on a real database and the real models: the
//! standup fixture seeded into SQLite, its recorded tracks, the ONNX engine.
//! Needs the models, so it is ignored by default:
//!
//! ```bash
//! DEINO_DIARIZE_MODELS=<dir> cargo test -p deino-diarize --release \
//!     --test stage_end_to_end -- --ignored --nocapture
//! ```
#![cfg(target_os = "macos")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::print_stdout,
    reason = "an integration-test crate: helpers fail the test by panicking, like #[test] bodies"
)]

use std::path::PathBuf;
use std::sync::Arc;

use deino_core::diarization::{DiarizeDeps, DiarizeService, ME_LABEL};
use deino_core::meetings::{MeetingAudio, MeetingLifecycleRepo, SqliteMeetingRepo};
use deino_core::models::{FakeByteSource, FakeServer};
use deino_core::people::SqlitePeopleRepo;
use deino_core::segments::{Page, SegmentReader, SpeakerRepo, SqliteSegmentRepo};
use deino_diarize::RuntimeConfig;
use deino_diarize::models::{EMBEDDING_MODEL_ID, ModelStore};
use deino_diarize::track::OnnxTrackDiarizer;
use deino_domain::{Channel, MeetingId, SystemClock};
use deino_testkit::fixture::MeetingKind;
use deino_testkit::{Fixtures, at, seed_fixtures, test_db};

/// The standup's recorded tracks, in the form the meeting row stores them.
fn standup_audio() -> MeetingAudio {
    let fixtures = Fixtures::load().unwrap();
    let audio = fixtures
        .meeting(MeetingKind::Standup)
        .unwrap()
        .audio
        .clone()
        .unwrap();
    let path = |channel| {
        fixtures
            .audio_path(&audio, channel)
            .map(|p: PathBuf| p.display().to_string())
    };
    MeetingAudio {
        audio_path: None,
        audio_mic_path: path(Channel::Mic),
        audio_system_path: path(Channel::System),
        video_path: None,
    }
}

#[tokio::test]
#[ignore = "needs the ONNX models (DEINO_DIARIZE_MODELS)"]
async fn the_standup_is_diarized_into_named_rows() {
    let database = test_db().await;
    seed_fixtures(&database).await.unwrap();
    let meetings = Arc::new(SqliteMeetingRepo::new(database.clone()));
    let meeting = MeetingId::new("meeting-fixture-standup");
    meetings
        .set_audio(&meeting, &standup_audio(), at("2026-10-08T10:00:00Z"))
        .await
        .unwrap();
    let segments = Arc::new(SqliteSegmentRepo::new(database.clone()));
    let models = std::env::var("DEINO_DIARIZE_MODELS").expect("set DEINO_DIARIZE_MODELS");
    let never = Arc::new(FakeByteSource::new(Vec::new(), FakeServer::Stalls));
    let engine = OnnxTrackDiarizer::new(ModelStore::new(models), never, RuntimeConfig::default());
    let service = DiarizeService::new(DiarizeDeps {
        meetings,
        attendees: Arc::new(SqlitePeopleRepo::new(database.clone())),
        segments: segments.clone(),
        writer: segments.clone(),
        speakers: segments.clone(),
        diarizer: Arc::new(engine),
        clock: Arc::new(SystemClock),
    });

    let outcome = service.run(&meeting).await.unwrap();

    println!("{outcome:?}");
    assert_eq!(outcome.channel, Some(Channel::System));
    let speakers = segments.speakers(&meeting).await.unwrap();
    let voiced: Vec<_> = speakers.iter().filter(|speaker| !speaker.is_me).collect();
    assert!(
        (2..=4).contains(&voiced.len()),
        "{} remote speakers",
        voiced.len()
    );
    let voice = segments.voice(&voiced[0].id).await.unwrap().unwrap();
    assert_eq!(
        (voice.centroid.len(), voice.embedding_model.as_str()),
        (256, EMBEDDING_MODEL_ID)
    );
    let transcript = segments.list(&meeting, Page::ALL).await.unwrap();
    assert!(
        transcript
            .iter()
            .all(|segment| segment.speaker_id.is_some())
    );
    assert!(
        speakers
            .iter()
            .any(|speaker| speaker.is_me || speaker.label == ME_LABEL)
    );
}
