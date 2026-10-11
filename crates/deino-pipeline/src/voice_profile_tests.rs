//! CAP-04 acceptance on a real database, through the real `diarize` and
//! `identify_speakers` stages and the user's rename: name a voice once in
//! the one-on-one fixture, and the standup fixture names it by itself.
//! Unknown voices stay unnamed, and a phantom cluster contaminated with
//! that voice is rejected by the margin and mutual best.
//!
//! The engine is scripted (`FakeTrackDiarizer`) with 256-d centroids whose
//! similarities mirror the spike's (`docs/references/spike-diarization.md`):
//! the same voice across meetings 0.92, a contaminated phantom 0.81. The
//! real models run in `crates/deino-diarize/tests/voice_profiles.rs`.

use std::sync::Arc;

use crate::stages::{DiarizeStage, IdentifySpeakersStage};
use crate::{PipelineStage, RecordedProgress, StageCtx, StageOutcome, StagePorts};
use async_trait::async_trait;
use deino_core::diarization::{
    FakeTrackDiarizer, IdentifyPolicy, IdentifyPorts, IdentifyService, TrackDiarization, Voice,
    VoiceTurn,
};
use deino_core::meetings::{MeetingAudio, MeetingLifecycleRepo, SqliteMeetingRepo};
use deino_core::people::{MeRepo, SqlitePeopleRepo};
use deino_core::segments::SqliteSegmentRepo;
use deino_core::settings::IdentifyThresholds;
use deino_core::speakers::{
    Participant, Rename, RenameScope, SpeakerName, SpeakerPorts, SpeakerService, SqliteNamingRepo,
};
use deino_core::voiceprints::{SqliteVoiceprintRepo, VoiceprintDeps, VoiceprintService};
use deino_db::Database;
use deino_domain::{Channel, FakeClock, MeetingId, Millis, PersonId};
use deino_testkit::fixture::{FixtureMeeting, MeetingKind, fixture_uuid};
use deino_testkit::{Fixtures, at, seed_fixtures, test_db};
use tokio_util::sync::CancellationToken;

const NOW: &str = "2026-10-11T12:00:00Z";
const DIM: usize = 256;

/// The spike's thresholds, as Settings ships them.
struct Defaults;

#[async_trait]
impl IdentifyPolicy for Defaults {
    async fn thresholds(&self) -> Option<IdentifyThresholds> {
        Some(IdentifyThresholds::default())
    }
}

/// A unit vector with these components on these axes.
fn unit(parts: &[(usize, f32)]) -> Vec<f32> {
    let mut values = vec![0.0; DIM];
    for &(axis, value) in parts {
        values[axis] = value;
    }
    let norm = values.iter().map(|v| v * v).sum::<f32>().sqrt();
    values.iter().map(|v| v / norm).collect()
}

fn clock() -> Arc<FakeClock> {
    Arc::new(FakeClock::new(at(NOW)))
}

fn person(email: &str) -> PersonId {
    fixture_uuid(&format!("person/{email}")).parse().unwrap()
}

/// The seeded fixtures, the user marked as "Me", both recorded meetings
/// given a system track.
async fn world() -> (Database, Fixtures) {
    let database = test_db().await;
    seed_fixtures(&database).await.unwrap();
    let people = SqlitePeopleRepo::new(database.clone());
    people
        .make_me(&person("shreyas@deino.dev"), at(NOW))
        .await
        .unwrap();
    let meetings = SqliteMeetingRepo::new(database.clone());
    for id in ["meeting-fixture-one-on-one", "meeting-fixture-standup"] {
        let audio = MeetingAudio {
            audio_system_path: Some(format!("/rec/{id}/system.m4a")),
            ..MeetingAudio::default()
        };
        meetings
            .set_audio(&MeetingId::new(id), &audio, at(NOW))
            .await
            .unwrap();
    }
    (database, Fixtures::load().unwrap())
}

/// What the scripted engine hears on a fixture's system track: one turn per
/// system segment, voiced by `voice_of` its fixture speaker's attendee.
fn heard(fixture: &FixtureMeeting, voices: Vec<(Option<&str>, Vec<f32>)>) -> TrackDiarization {
    let index_of = |label: &str| {
        let attendee = fixture.speaker(label).and_then(|s| s.attendee.as_ref());
        let email = attendee.map(deino_domain::Email::as_str);
        voices.iter().position(|(owner, _)| *owner == email)
    };
    let turns: Vec<VoiceTurn> = fixture
        .segments
        .iter()
        .filter(|segment| segment.channel == Channel::System)
        .filter_map(|segment| {
            Some(VoiceTurn {
                start: segment.start_ms,
                end: segment.end_ms,
                voice: index_of(&segment.speaker)?,
            })
        })
        .collect();
    let speech = |voice: usize| {
        let spoken = turns.iter().filter(|turn| turn.voice == voice);
        Millis(
            spoken
                .map(|turn| turn.end.get() - turn.start.get())
                .sum::<u32>()
                .max(5_000),
        )
    };
    let voices = voices
        .into_iter()
        .enumerate()
        .map(|(index, (_, centroid))| Voice {
            centroid,
            speech: speech(index),
        })
        .collect();
    TrackDiarization { turns, voices }
}

/// The stages' ports over the database, the scripted engine and the real
/// identifier.
fn ports(database: &Database, engine: TrackDiarization) -> StagePorts {
    let segments = Arc::new(SqliteSegmentRepo::new(database.clone()));
    let people = Arc::new(SqlitePeopleRepo::new(database.clone()));
    let prints = Arc::new(SqliteVoiceprintRepo::new(database.clone()));
    let naming = Arc::new(SqliteNamingRepo::new(database.clone()));
    let identify_ports = IdentifyPorts {
        speakers: segments.clone(),
        attendees: people.clone(),
        me: people.clone(),
        exemplars: prints.clone(),
    };
    StagePorts {
        segments: segments.clone(),
        segment_writer: segments.clone(),
        speakers: segments,
        meetings: Arc::new(SqliteMeetingRepo::new(database.clone())),
        attendees: people,
        diarizer: Arc::new(FakeTrackDiarizer::returning(engine)),
        voiceprints: prints,
        identifier: Arc::new(IdentifyService::new(
            identify_ports,
            Arc::new(Defaults),
            naming.clone(),
        )),
        participants: naming,
        clock: clock(),
        ..StagePorts::fakes()
    }
}

/// Runs `diarize` and `identify_speakers` for `meeting` over the database.
async fn process(database: &Database, meeting: &str, engine: TrackDiarization) {
    let ports = ports(database, engine);
    let stages: [Arc<dyn PipelineStage>; 2] = [
        Arc::new(DiarizeStage::new(&ports)),
        Arc::new(IdentifySpeakersStage::new(&ports)),
    ];
    let ctx = StageCtx {
        attempt: 1,
        cancel: CancellationToken::new(),
        clock: clock(),
        events: Arc::new(RecordedProgress::default()),
    };
    for stage in stages {
        let outcome = stage.run(&ctx, MeetingId::new(meeting)).await.unwrap();
        assert_eq!(outcome, StageOutcome::Done, "{}", stage.id());
    }
}

fn speakers(database: &Database) -> SpeakerService {
    let segments = Arc::new(SqliteSegmentRepo::new(database.clone()));
    let people = Arc::new(SqlitePeopleRepo::new(database.clone()));
    let naming = Arc::new(SqliteNamingRepo::new(database.clone()));
    let voiceprints = VoiceprintService::new(VoiceprintDeps {
        repo: Arc::new(SqliteVoiceprintRepo::new(database.clone())),
        speakers: segments.clone(),
        segments: segments.clone(),
        clock: clock(),
    });
    SpeakerService::new(SpeakerPorts {
        participants: naming.clone(),
        naming,
        speakers: segments,
        people: people.clone(),
        attendees: people,
        voiceprints: Arc::new(voiceprints),
        clock: clock(),
    })
}

fn name(participant: &Participant) -> Option<&str> {
    participant
        .person
        .as_ref()
        .map(|person| person.name.as_str())
}

const MAYA: &str = "maya@deino.dev";

/// Processes the one-on-one, then names its remote voice Maya for all
/// segments, as the user would; whether her voice was enrolled.
async fn name_maya_in_the_one_on_one(database: &Database, fixtures: &Fixtures) -> bool {
    let maya = MAYA;
    let one_on_one = fixtures.meeting(MeetingKind::OneOnOne).unwrap();
    let first_voice = unit(&[(0, 0.95), (10, 0.31)]);
    process(
        database,
        one_on_one.id.as_str(),
        heard(one_on_one, vec![(Some(maya), first_voice)]),
    )
    .await;
    let service = speakers(database);
    let first = service.participants(&one_on_one.id).await.unwrap();
    let remote = first.iter().find(|p| !p.is_me).unwrap();

    let renamed = service
        .rename(&Rename {
            meeting_id: one_on_one.id.clone(),
            speaker_id: remote.speaker_id,
            name: SpeakerName::Person {
                person_id: person(maya),
            },
            scope: RenameScope::All,
        })
        .await
        .unwrap();
    renamed.enrolled
}

/// The standup's remote voices: Maya again, three strangers, and a phantom
/// half Maya, half Tom (0.81 against Maya's voiceprint).
fn standup_voices() -> Vec<(Option<&'static str>, Vec<f32>)> {
    vec![
        (Some(MAYA), unit(&[(0, 0.97), (11, 0.243)])),
        (Some("diego@deino.dev"), unit(&[(1, 1.0)])),
        (Some("priya@deino.dev"), unit(&[(2, 1.0)])),
        (Some("tom@deino.dev"), unit(&[(3, 1.0)])),
        (None, unit(&[(0, 0.853), (3, 0.522)])),
    ]
}

#[tokio::test]
async fn a_voice_named_once_is_named_in_the_next_meeting() {
    let (database, fixtures) = world().await;
    let enrolled = name_maya_in_the_one_on_one(&database, &fixtures).await;
    assert!(enrolled, "a confirmed rename enrols the voice");

    let standup = fixtures.meeting(MeetingKind::Standup).unwrap();
    process(
        &database,
        standup.id.as_str(),
        heard(standup, standup_voices()),
    )
    .await;

    let service = speakers(&database);
    let panel = service.participants(&standup.id).await.unwrap();
    let named: Vec<Option<&str>> = panel.iter().filter(|p| !p.is_me).map(name).collect();
    assert_eq!(
        named.iter().filter(|n| **n == Some("Maya Chen")).count(),
        1,
        "{named:?}"
    );
    assert_eq!(
        named.iter().filter(|n| n.is_none()).count(),
        4,
        "unknown voices and the phantom"
    );
    assert!(
        panel.iter().all(|p| p.suggestion.is_none()),
        "Maya is taken; nobody else is known"
    );
    let me = panel.iter().find(|p| p.is_me).unwrap();
    assert_eq!(name(me), Some("Shreyas"));
}
