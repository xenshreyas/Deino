//! `IdentifyService` over the real SQLite repositories.

use std::sync::Arc;

use deino_db::Database;
use deino_domain::{
    AttendeeRole, CallApp, Channel, DisplayName, FakeClock, Millis, PaletteColor, PersonId, Rsvp,
    SegmentId, SpeakerId, TagSpan,
};
use deino_testkit::{at, seed, test_db};

use super::*;
use crate::people::{Attendee, AttendeeRepo, Person, PersonRepo, SqlitePeopleRepo};
use crate::segments::{
    NewSegment, NewSpeaker, SegmentWriter, Speaker, SpeakerRepo, SpeakerVoice, SqliteSegmentRepo,
};
use crate::speaker_tags::{
    SpeakerTagPriors, SpeakerTagWriter, SqliteSpeakerTagRepo, TagPriorPorts,
};
use crate::speakers::{ParticipantReader, SqliteNamingRepo};
use crate::voiceprints::{
    Quality, SqliteVoiceprintRepo, VOICEPRINT_DIM, VoiceEmbedding, Voiceprint, VoiceprintRepo,
};

const MEETING: &str = "meeting-a";
const MODEL: &str = "wespeaker@test";
const T0: &str = "2026-10-11T09:00:00Z";

/// A fixed policy, so tests need no settings file.
struct Fixed(Option<IdentifyThresholds>);

#[async_trait]
impl IdentifyPolicy for Fixed {
    async fn thresholds(&self) -> Option<IdentifyThresholds> {
        self.0
    }
}

fn clock() -> FakeClock {
    FakeClock::new(at(T0))
}

/// A unit vector with cosine `score` to each listed axis, the rest on the
/// last axis.
fn voice(parts: &[(usize, f32)]) -> Vec<f32> {
    let mut values = vec![0.0; VOICEPRINT_DIM];
    for &(axis, score) in parts {
        values[axis] = score;
    }
    let used: f32 = parts.iter().map(|(_, score)| score * score).sum();
    values[VOICEPRINT_DIM - 1] = (1.0 - used).max(0.0).sqrt();
    values
}

struct World {
    database: Database,
    segments: Arc<SqliteSegmentRepo>,
    people: Arc<SqlitePeopleRepo>,
    naming: Arc<SqliteNamingRepo>,
}

impl World {
    async fn new() -> Self {
        let database = test_db().await;
        seed::meeting(&database, MEETING, "Standup", at(T0)).await;
        Self {
            segments: Arc::new(SqliteSegmentRepo::new(database.clone())),
            people: Arc::new(SqlitePeopleRepo::new(database.clone())),
            naming: Arc::new(SqliteNamingRepo::new(database.clone())),
            database,
        }
    }

    fn service(&self, thresholds: Option<IdentifyThresholds>) -> IdentifyService {
        let ports = IdentifyPorts {
            speakers: self.segments.clone(),
            attendees: self.people.clone(),
            me: self.people.clone(),
            exemplars: Arc::new(SqliteVoiceprintRepo::new(self.database.clone())),
        };
        IdentifyService::new(ports, Arc::new(Fixed(thresholds)), self.naming.clone())
    }

    /// A person, enrolled with one exemplar along `axis` when given.
    async fn person(&self, name: &str, is_me: bool, axis: Option<usize>) -> PersonId {
        let at = at(T0);
        let person = Person {
            id: PersonId::generate(&clock()),
            name: name.into(),
            email: None,
            company_id: None,
            title: None,
            avatar_path: None,
            notes_md: String::new(),
            summary_md: None,
            summary_updated_at: None,
            is_me,
            merged_into: None,
            created_at: at,
            updated_at: at,
        };
        self.people.create(&person).await.unwrap();
        if let Some(axis) = axis {
            self.enrol(person.id, axis).await;
        }
        person.id
    }

    async fn enrol(&self, person: PersonId, axis: usize) {
        let print = Voiceprint {
            id: deino_domain::VoiceprintId::generate(&clock()),
            person_id: person,
            embedding: VoiceEmbedding::new(voice(&[(axis, 1.0)]), MODEL).unwrap(),
            source_meeting_id: None,
            quality: Quality::new(0.8),
            created_at: at(T0),
        };
        let repo = SqliteVoiceprintRepo::new(self.database.clone());
        repo.enrol(&print, 5).await.unwrap();
    }

    async fn invite(&self, person: PersonId) {
        let attendee = Attendee {
            meeting_id: MeetingId::new(MEETING),
            person_id: person,
            role: AttendeeRole::Invitee,
            rsvp: Some(Rsvp::Accepted),
        };
        self.people.upsert_attendee(&attendee).await.unwrap();
    }

    /// "Me" on the microphone plus diarized voices with `speech` ms each.
    async fn speakers(&self, voices: &[(Vec<f32>, u32)]) -> Vec<SpeakerId> {
        let new: Vec<NewSpeaker> = voices
            .iter()
            .enumerate()
            .map(|(index, (centroid, speech))| diarized(index, centroid.clone(), *speech))
            .collect();
        let meeting = MeetingId::new(MEETING);
        self.segments
            .replace_speakers(&meeting, &new)
            .await
            .unwrap();
        let me = speaker("Me", 0, true);
        self.segments.add_speaker(&me).await.unwrap();
        new.iter().map(|new| new.speaker.id).collect()
    }

    async fn names(&self) -> Vec<(String, Option<String>, Option<String>)> {
        let rows = self
            .naming
            .participants(&MeetingId::new(MEETING))
            .await
            .unwrap();
        let mut names: Vec<_> = rows
            .into_iter()
            .map(|row| {
                let person = row.person.map(|person| person.name);
                let suggested = row.suggestion.map(|suggestion| suggestion.person.name);
                (row.speaker.label, person, suggested)
            })
            .collect();
        names.sort();
        names
    }
}

fn speaker(label: &str, index: usize, is_me: bool) -> Speaker {
    Speaker {
        id: SpeakerId::generate(&clock()),
        meeting_id: MeetingId::new(MEETING),
        label: label.into(),
        person_id: None,
        is_me,
        color: PaletteColor::cycle(index),
    }
}

fn diarized(index: usize, centroid: Vec<f32>, speech: u32) -> NewSpeaker {
    NewSpeaker {
        speaker: speaker(&format!("Speaker {}", index + 1), index + 1, false),
        voice: Some(SpeakerVoice {
            centroid,
            embedding_model: MODEL.into(),
            speech: Millis(speech),
        }),
    }
}

fn named(label: &str, person: &str) -> (String, Option<String>, Option<String>) {
    (label.into(), Some(person.into()), None)
}

#[tokio::test]
async fn known_voices_are_named_the_microphone_is_me_and_the_rest_suggested() {
    let world = World::new().await;
    world.person("Shreyas", true, None).await;
    let alex = world.person("Alex", false, Some(0)).await;
    world.person("Maya", false, Some(1)).await;
    world.invite(alex).await;
    world
        .speakers(&[
            (voice(&[(0, 0.86)]), 60_000),
            (voice(&[(1, 0.62)]), 40_000),
            (voice(&[(2, 0.9)]), 30_000),
        ])
        .await;

    let identified = world.service(Some(IdentifyThresholds::default()));
    let outcome = identified.identify(&MeetingId::new(MEETING)).await.unwrap();

    assert_eq!(
        outcome,
        Identified::Done {
            named: 2,
            suggested: 1
        }
    );
    assert_eq!(
        world.names().await,
        [
            named("Me", "Shreyas"),
            named("Speaker 1", "Alex"),
            ("Speaker 2".into(), None, Some("Maya".into())),
            ("Speaker 3".into(), None, None),
        ]
    );
}

#[tokio::test]
async fn a_thin_phantom_voice_is_neither_named_nor_suggested() {
    let world = World::new().await;
    let alex = world.person("Alex", false, Some(0)).await;
    let maya = world.person("Maya", false, None).await;
    // Two invitees and one voice: elimination cannot name it either.
    world.invite(alex).await;
    world.invite(maya).await;
    world.speakers(&[(voice(&[(0, 0.95)]), 2_000)]).await;

    let service = world.service(Some(IdentifyThresholds::default()));
    service.identify(&MeetingId::new(MEETING)).await.unwrap();

    let names = world.names().await;
    assert!(
        names
            .iter()
            .all(|(_, person, suggested)| person.is_none() && suggested.is_none())
    );
}

#[tokio::test]
async fn speakers_the_user_named_are_kept_on_every_run() {
    let world = World::new().await;
    world.person("Alex", false, Some(0)).await;
    let maya = world.person("Maya", false, None).await;
    let ids = world.speakers(&[(voice(&[(0, 0.9)]), 60_000)]).await;
    world
        .segments
        .assign_person(&ids[0], Some(&maya))
        .await
        .unwrap();
    let service = world.service(Some(IdentifyThresholds::default()));

    service.identify(&MeetingId::new(MEETING)).await.unwrap();
    service.identify(&MeetingId::new(MEETING)).await.unwrap();

    assert!(world.names().await.contains(&named("Speaker 1", "Maya")));
}

#[tokio::test]
async fn identification_can_be_switched_off_and_needs_speakers() {
    let world = World::new().await;
    let meeting = MeetingId::new(MEETING);

    let none = world.service(Some(IdentifyThresholds::default()));
    assert_eq!(
        none.identify(&meeting).await.unwrap(),
        Identified::NoSpeakers
    );
    world.speakers(&[(voice(&[(0, 0.9)]), 60_000)]).await;
    let off = world.service(None);
    assert_eq!(off.identify(&meeting).await.unwrap(), Identified::Disabled);
}

/// The world's identifier with speaker tags read from the database.
fn tagged_service(world: &World) -> IdentifyService {
    let priors = SpeakerTagPriors::new(TagPriorPorts {
        tags: Arc::new(SqliteSpeakerTagRepo::new(world.database.clone())),
        segments: world.segments.clone(),
        attendees: world.people.clone(),
        people: world.people.clone(),
        clock: Arc::new(clock()),
    });
    world
        .service(Some(IdentifyThresholds::default()))
        .with_speaker_tags(Arc::new(priors))
}

/// A system-track segment of `speaker`, and the window's name for that time.
async fn tagged(world: &World, speaker: SpeakerId, (start, end): (u32, u32), name: &str) {
    let meeting = MeetingId::new(MEETING);
    let segment = NewSegment {
        id: SegmentId::generate(&clock()),
        meeting_id: meeting.clone(),
        channel: Channel::System,
        speaker_id: Some(speaker),
        start: Millis(start),
        end: Millis(end),
        text: "…".into(),
        words: Vec::new(),
    };
    world.segments.append(&[segment]).await.unwrap();
    let span = TagSpan {
        start: Millis(start),
        end: Millis(end),
        name: DisplayName::parse(name).unwrap(),
        app: CallApp::Zoom,
    };
    let tags = SqliteSpeakerTagRepo::new(world.database.clone());
    tags.append(&meeting, &[span]).await.unwrap();
}

#[tokio::test]
async fn the_meeting_apps_names_override_a_matching_voiceprint() {
    let world = World::new().await;
    let alex = world.person("Alex", false, Some(0)).await;
    let maya = world.person("Maya Patel", false, Some(1)).await;
    world.invite(alex).await;
    world.invite(maya).await;
    let ids = world
        .speakers(&[(voice(&[(0, 0.9)]), 30_000), (voice(&[(2, 0.9)]), 30_000)])
        .await;
    // Speaker 1's voice matches Alex, but the window says Maya spoke.
    tagged(&world, ids[0], (0, 30_000), "Maya Patel").await;
    tagged(&world, ids[1], (30_000, 60_000), "Sam Okafor").await;

    let service = tagged_service(&world);
    service.identify(&MeetingId::new(MEETING)).await.unwrap();
    service.identify(&MeetingId::new(MEETING)).await.unwrap();

    assert_eq!(
        world.names().await,
        [
            ("Me".into(), None, None),
            ("Sam Okafor".into(), None, None),
            named("Speaker 1", "Maya Patel"),
        ]
    );
}

/// A system-track segment of `speaker` that no window named.
async fn said(world: &World, speaker: SpeakerId, (start, end): (u32, u32)) {
    let segment = NewSegment {
        id: SegmentId::generate(&clock()),
        meeting_id: MeetingId::new(MEETING),
        channel: Channel::System,
        speaker_id: Some(speaker),
        start: Millis(start),
        end: Millis(end),
        text: "…".into(),
        words: Vec::new(),
    };
    world.segments.append(&[segment]).await.unwrap();
}

/// The call window listed `name` from `start` to `end`.
async fn listed(world: &World, name: &str, (start, end): (u32, u32)) {
    let span = TagSpan {
        start: Millis(start),
        end: Millis(end),
        name: DisplayName::parse(name).unwrap(),
        app: CallApp::Zoom,
    };
    let tags = SqliteSpeakerTagRepo::new(world.database.clone());
    tags.append_roster(&MeetingId::new(MEETING), &[span])
        .await
        .unwrap();
}

#[tokio::test]
async fn a_one_to_one_call_merges_the_split_remote_voice_into_that_person() {
    let world = World::new().await;
    let bea = world.person("Bea Ruiz", false, None).await;
    world.invite(bea).await;
    // Diarization split Bea's voice in three; the window showed no speaker.
    let ids = world
        .speakers(&[
            (voice(&[(0, 0.9)]), 30_000),
            (voice(&[(1, 0.9)]), 20_000),
            (voice(&[(2, 0.9)]), 10_000),
        ])
        .await;
    said(&world, ids[0], (0, 30_000)).await;
    said(&world, ids[1], (30_000, 50_000)).await;
    said(&world, ids[2], (50_000, 60_000)).await;
    listed(&world, "Bea Ruiz", (0, 600_000)).await;
    // Someone who dialled in for half a minute does not count.
    listed(&world, "Dev Patel", (100_000, 130_000)).await;

    let service = tagged_service(&world);
    service.identify(&MeetingId::new(MEETING)).await.unwrap();
    service.identify(&MeetingId::new(MEETING)).await.unwrap();

    // One speaker is left for the remote track (which of the three keeps
    // its label does not matter), named Bea.
    let names = world.names().await;
    assert_eq!(names.len(), 2, "{names:?}");
    assert_eq!(names[0], ("Me".into(), None, None));
    assert_eq!(names[1].1.as_deref(), Some("Bea Ruiz"));
}

#[tokio::test]
async fn a_group_call_still_votes_by_spans_and_names_the_rest_by_elimination() {
    let world = World::new().await;
    let bea = world.person("Bea Ruiz", false, None).await;
    let cleo = world.person("Cleo Martin", false, None).await;
    world.invite(bea).await;
    world.invite(cleo).await;
    let ids = world
        .speakers(&[(voice(&[(0, 0.9)]), 30_000), (voice(&[(1, 0.9)]), 30_000)])
        .await;
    listed(&world, "Bea Ruiz", (0, 600_000)).await;
    listed(&world, "Cleo Martin", (0, 600_000)).await;
    // Speaker View showed Bea while Speaker 1 talked; Cleo was never shown.
    tagged(&world, ids[0], (0, 30_000), "Bea Ruiz").await;
    said(&world, ids[1], (30_000, 60_000)).await;

    let service = tagged_service(&world);
    service.identify(&MeetingId::new(MEETING)).await.unwrap();

    assert_eq!(
        world.names().await,
        [
            ("Me".into(), None, None),
            named("Speaker 1", "Bea Ruiz"),
            named("Speaker 2", "Cleo Martin"),
        ]
    );
}
