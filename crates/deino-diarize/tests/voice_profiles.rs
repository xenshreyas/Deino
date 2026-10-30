//! CAP-04 on the real models and the fixtures' recorded audio: diarize the
//! one-on-one, name its remote voice Maya for all segments (which enrols
//! her voiceprint), then diarize and identify the standup, where Maya
//! speaks again. Needs the two ONNX files, so it is ignored by default:
//!
//! ```bash
//! DEINO_DIARIZE_SEGMENTATION=<pyannote seg-3.0 .onnx> \
//! DEINO_DIARIZE_EMBEDDING=<WeSpeaker ResNet34-LM .onnx> \
//!     cargo test -p deino-diarize --release --test voice_profiles -- --ignored --nocapture
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

use async_trait::async_trait;
use deino_core::companies::SqliteCompanyRepo;
use deino_core::diarization::{
    DiarizeDeps, DiarizeService, IdentifyPolicy, IdentifyPorts, IdentifyService, SpeakerIdentifier,
    TrackDiarization, TrackDiarizer, TrackError, TrackRequest,
};
use deino_core::entities::EntityDirectory;
use deino_core::meetings::{MeetingAudio, MeetingLifecycleRepo, SqliteMeetingRepo};
use deino_core::people::{MeRepo, SqlitePeopleRepo};
use deino_core::segments::SqliteSegmentRepo;
use deino_core::settings::IdentifyThresholds;
use deino_core::speakers::{
    Rename, RenameScope, SpeakerName, SpeakerPorts, SpeakerService, SqliteNamingRepo,
};
use deino_core::voiceprints::{SqliteVoiceprintRepo, VoiceprintDeps, VoiceprintService};
use deino_db::Database;
use deino_diarize::models::EMBEDDING_MODEL_ID;
use deino_diarize::track::diarize_file;
use deino_diarize::{ModelPaths, RuntimeConfig};
use deino_domain::{Channel, MeetingId, PersonId, SystemClock};
use deino_testkit::fixture::{MeetingKind, fixture_uuid};
use deino_testkit::{Fixtures, at, seed_fixtures, test_db};

/// The engine over model files named by the environment, without the store.
struct LocalModels(ModelPaths);

#[async_trait]
impl TrackDiarizer for LocalModels {
    fn embedding_model(&self) -> &str {
        EMBEDDING_MODEL_ID
    }

    async fn diarize(&self, request: TrackRequest) -> Result<TrackDiarization, TrackError> {
        diarize_file(&self.0, RuntimeConfig::default(), &request)
    }
}

struct Defaults;

#[async_trait]
impl IdentifyPolicy for Defaults {
    async fn thresholds(&self) -> Option<IdentifyThresholds> {
        Some(IdentifyThresholds::default())
    }
}

fn person(email: &str) -> PersonId {
    fixture_uuid(&format!("person/{email}")).parse().unwrap()
}

fn models() -> ModelPaths {
    let path = |name: &str| PathBuf::from(std::env::var(name).expect(name));
    ModelPaths {
        segmentation: path("DEINO_DIARIZE_SEGMENTATION"),
        embedding: path("DEINO_DIARIZE_EMBEDDING"),
    }
}

/// Points the meeting at its recorded fixture tracks.
async fn record(database: &Database, fixtures: &Fixtures, kind: MeetingKind) -> MeetingId {
    let meeting = fixtures.meeting(kind).unwrap();
    let audio = meeting.audio.clone().unwrap();
    let path = |channel| {
        let file: PathBuf = fixtures.audio_path(&audio, channel).unwrap();
        Some(file.display().to_string())
    };
    let tracks = MeetingAudio {
        audio_mic_path: path(Channel::Mic),
        audio_system_path: path(Channel::System),
        ..MeetingAudio::default()
    };
    let repo = SqliteMeetingRepo::new(database.clone());
    repo.set_audio(&meeting.id, &tracks, at("2026-10-11T12:00:00Z"))
        .await
        .unwrap();
    meeting.id.clone()
}

/// `diarize` then `identify_speakers` for `meeting`.
async fn process(database: &Database, meeting: &MeetingId) {
    let segments = Arc::new(SqliteSegmentRepo::new(database.clone()));
    let people = Arc::new(SqlitePeopleRepo::new(database.clone()));
    let prints = Arc::new(SqliteVoiceprintRepo::new(database.clone()));
    let diarize = DiarizeService::new(DiarizeDeps {
        aligner: std::sync::Arc::new(deino_core::diarization::NoWordAligner),
        meetings: Arc::new(SqliteMeetingRepo::new(database.clone())),
        attendees: people.clone(),
        segments: segments.clone(),
        writer: segments.clone(),
        speakers: segments.clone(),
        diarizer: Arc::new(LocalModels(models())),
        exemplars: prints.clone(),
        clock: Arc::new(SystemClock),
    });
    println!("{:?}", diarize.run(meeting).await.unwrap());
    let ports = IdentifyPorts {
        speakers: segments,
        attendees: people,
        me: Arc::new(SqlitePeopleRepo::new(database.clone())),
        exemplars: prints,
    };
    let naming = Arc::new(SqliteNamingRepo::new(database.clone()));
    let identify = IdentifyService::new(ports, Arc::new(Defaults), naming);
    println!("{:?}", identify.identify(meeting).await.unwrap());
}

fn naming(database: &Database) -> SpeakerService {
    let segments = Arc::new(SqliteSegmentRepo::new(database.clone()));
    let people = Arc::new(SqlitePeopleRepo::new(database.clone()));
    let naming = Arc::new(SqliteNamingRepo::new(database.clone()));
    let voiceprints = VoiceprintService::new(VoiceprintDeps {
        repo: Arc::new(SqliteVoiceprintRepo::new(database.clone())),
        speakers: segments.clone(),
        segments: segments.clone(),
        clock: Arc::new(SystemClock),
    });
    SpeakerService::new(SpeakerPorts {
        participants: naming.clone(),
        naming,
        speakers: segments,
        people: people.clone(),
        attendees: people.clone(),
        voiceprints: Arc::new(voiceprints),
        directory: Arc::new(EntityDirectory::new(
            people.clone(),
            Arc::new(SqliteCompanyRepo::new(database.clone())),
            Arc::new(SystemClock),
        )),
        clock: Arc::new(SystemClock),
    })
}

#[tokio::test]
#[ignore = "needs the ONNX models (DEINO_DIARIZE_SEGMENTATION, DEINO_DIARIZE_EMBEDDING)"]
async fn maya_named_in_the_one_on_one_is_recognised_in_the_standup() {
    let database = test_db().await;
    seed_fixtures(&database).await.unwrap();
    let me = person("shreyas@deino.dev");
    SqlitePeopleRepo::new(database.clone())
        .make_me(&me, at("2026-10-11T12:00:00Z"))
        .await
        .unwrap();
    let fixtures = Fixtures::load().unwrap();
    let service = naming(&database);

    let one_on_one = record(&database, &fixtures, MeetingKind::OneOnOne).await;
    process(&database, &one_on_one).await;
    let first = service.participants(&one_on_one).await.unwrap();
    let remote = first.iter().find(|p| !p.is_me).expect("a remote voice");
    let maya = person("maya@deino.dev");
    let rename = Rename {
        meeting_id: one_on_one.clone(),
        speaker_id: remote.speaker_id,
        name: SpeakerName::Person { person_id: maya },
        scope: RenameScope::All,
    };
    assert!(service.rename(&rename).await.unwrap().enrolled);

    let standup = record(&database, &fixtures, MeetingKind::Standup).await;
    process(&database, &standup).await;

    let panel = service.participants(&standup).await.unwrap();
    for participant in &panel {
        println!("{participant:?}");
    }
    let named_maya = panel
        .iter()
        .filter(|p| p.person.as_ref().is_some_and(|person| person.id == maya))
        .count();
    assert_eq!(named_maya, 1, "Maya's voice is recognised in the standup");
}
