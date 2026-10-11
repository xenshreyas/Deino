//! `SpeakerService` over the real SQLite repositories: the panel's rows,
//! the name picker, renaming for one segment or all, merging, suggestions,
//! and what a rename moves along (action items, the voice profile).

use std::sync::Arc;

use deino_db::Database;
use deino_domain::{
    ActionItemId, ActionItemOrigin, ActionItemStatus, AttendeeRole, Channel, FakeClock, Millis,
    PaletteColor, PersonId, Rsvp, SegmentId,
};
use deino_testkit::{at, seed, test_db};

use super::*;
use crate::action_items::{ActionItem, ActionItemRepo, SqliteActionItemRepo};
use crate::companies::SqliteCompanyRepo;
use crate::entities::EntityDirectory;
use crate::people::{Attendee, AttendeeRepo, Person, PersonRepo, SqlitePeopleRepo};
use crate::segments::{
    NewSegment, NewSpeaker, Page, SegmentReader, SegmentWriter, Speaker, SpeakerRepo, SpeakerVoice,
    SqliteSegmentRepo,
};
use crate::speakers::{Identification, IdentificationWriter, SqliteNamingRepo, SuggestedName};
use crate::voiceprints::{
    ExemplarReader, SqliteVoiceprintRepo, VOICEPRINT_DIM, VoiceprintDeps, VoiceprintService,
};

const MEETING: &str = "meeting-a";
const MODEL: &str = "wespeaker@test";
const T0: &str = "2026-10-11T09:00:00Z";

fn meeting() -> MeetingId {
    MeetingId::new(MEETING)
}

fn clock() -> Arc<FakeClock> {
    Arc::new(FakeClock::new(at(T0)))
}

struct World {
    database: Database,
    service: SpeakerService,
    segments: Arc<SqliteSegmentRepo>,
    people: Arc<SqlitePeopleRepo>,
    naming: Arc<SqliteNamingRepo>,
    voiceprints: Arc<SqliteVoiceprintRepo>,
}

impl World {
    async fn new() -> Self {
        let database = test_db().await;
        seed::meeting(&database, MEETING, "Standup", at(T0)).await;
        let segments = Arc::new(SqliteSegmentRepo::new(database.clone()));
        let people = Arc::new(SqlitePeopleRepo::new(database.clone()));
        let naming = Arc::new(SqliteNamingRepo::new(database.clone()));
        let voiceprints = Arc::new(SqliteVoiceprintRepo::new(database.clone()));
        let enrolment = VoiceprintService::new(VoiceprintDeps {
            repo: voiceprints.clone(),
            speakers: segments.clone(),
            segments: segments.clone(),
            clock: clock(),
        });
        let service = SpeakerService::new(SpeakerPorts {
            participants: naming.clone(),
            naming: naming.clone(),
            speakers: segments.clone(),
            people: people.clone(),
            attendees: people.clone(),
            voiceprints: Arc::new(enrolment),
            directory: Arc::new(EntityDirectory::new(
                people.clone(),
                Arc::new(SqliteCompanyRepo::new(database.clone())),
                clock(),
            )),
            clock: clock(),
        });
        Self {
            database,
            service,
            segments,
            people,
            naming,
            voiceprints,
        }
    }

    async fn person(&self, name: &str, email: Option<&str>) -> PersonId {
        let at = at(T0);
        let person = Person {
            id: PersonId::generate(clock().as_ref()),
            name: name.into(),
            email: email.map(|email| deino_domain::Email::parse(email).unwrap()),
            company_id: None,
            title: None,
            avatar_path: None,
            notes_md: String::new(),
            summary_md: None,
            summary_updated_at: None,
            is_me: false,
            merged_into: None,
            created_at: at,
            updated_at: at,
        };
        self.people.create(&person).await.unwrap();
        person.id
    }

    async fn invite(&self, person: PersonId, rsvp: Rsvp) {
        let attendee = Attendee {
            meeting_id: meeting(),
            person_id: person,
            role: AttendeeRole::Invitee,
            rsvp: Some(rsvp),
        };
        self.people.upsert_attendee(&attendee).await.unwrap();
    }

    /// Diarized speakers with voices, each saying one segment: `(label,
    /// start, length)` in ms. Replaces the meeting's diarized speakers.
    async fn speakers(&self, specs: &[(&str, u32, u32)]) -> Vec<(SpeakerId, SegmentId)> {
        let new: Vec<NewSpeaker> = specs
            .iter()
            .enumerate()
            .map(|(index, (label, _, length))| voiced(label, index, *length))
            .collect();
        self.segments
            .replace_speakers(&meeting(), &new)
            .await
            .unwrap();
        let mut said = Vec::new();
        for (new, (_, start, length)) in new.iter().zip(specs) {
            let segment = self.said(new.speaker.id, *start, *length).await;
            said.push((new.speaker.id, segment));
        }
        said
    }

    /// One diarized speaker saying one segment.
    async fn speaker(&self, label: &str, start: u32, length: u32) -> (SpeakerId, SegmentId) {
        self.speakers(&[(label, start, length)]).await[0]
    }

    async fn said(&self, speaker: SpeakerId, start: u32, length: u32) -> SegmentId {
        let segment = NewSegment {
            id: SegmentId::generate(clock().as_ref()),
            meeting_id: meeting(),
            channel: Channel::System,
            speaker_id: Some(speaker),
            start: Millis(start),
            end: Millis(start + length),
            text: "something".into(),
            words: Vec::new(),
        };
        self.segments
            .append(std::slice::from_ref(&segment))
            .await
            .unwrap();
        segment.id
    }

    async fn item_for(&self, person: PersonId) -> ActionItemId {
        let at = at(T0);
        let item = ActionItem {
            id: ActionItemId::generate(clock().as_ref()),
            meeting_id: Some(meeting()),
            title: "Send the deck".into(),
            description: String::new(),
            assignee_person_id: Some(person),
            due_at: None,
            status: ActionItemStatus::Open,
            priority: None,
            source_segment_id: None,
            completed_at: None,
            created_by: ActionItemOrigin::Ai,
            created_at: at,
            updated_at: at,
        };
        SqliteActionItemRepo::new(self.database.clone())
            .create(&item)
            .await
            .unwrap();
        item.id
    }

    async fn assignee(&self, item: ActionItemId) -> Option<PersonId> {
        let repo = SqliteActionItemRepo::new(self.database.clone());
        repo.find(&item).await.unwrap().unwrap().assignee_person_id
    }

    async fn rename(&self, speaker: SpeakerId, name: SpeakerName, scope: RenameScope) -> Renamed {
        let rename = Rename {
            meeting_id: meeting(),
            speaker_id: speaker,
            name,
            scope,
        };
        self.service.rename(&rename).await.unwrap()
    }

    /// Leaves "Is this …?" suggestions, as `identify_speakers` would.
    async fn suggest(&self, pairs: &[(SpeakerId, PersonId)]) {
        let suggested = pairs
            .iter()
            .map(|&(speaker_id, person_id)| SuggestedName {
                speaker_id,
                person_id,
                score: 0.64,
            })
            .collect();
        let decided = Identification {
            named: Vec::new(),
            suggested,
        };
        self.naming.apply(&meeting(), &decided).await.unwrap();
    }

    async fn panel(&self) -> Vec<Participant> {
        self.service.participants(&meeting()).await.unwrap()
    }
}

fn voiced(label: &str, index: usize, speech: u32) -> NewSpeaker {
    NewSpeaker {
        speaker: Speaker {
            id: SpeakerId::generate(clock().as_ref()),
            meeting_id: meeting(),
            label: label.into(),
            person_id: None,
            is_me: false,
            color: PaletteColor::cycle(index + 1),
        },
        voice: Some(SpeakerVoice {
            centroid: vec![0.0625; VOICEPRINT_DIM],
            embedding_model: MODEL.into(),
            speech: Millis(speech),
        }),
    }
}

fn person(person_id: PersonId) -> SpeakerName {
    SpeakerName::Person { person_id }
}

#[tokio::test]
async fn the_panel_lists_speakers_by_speaking_time_with_their_share() {
    let world = World::new().await;
    let said = world
        .speakers(&[("Speaker 1", 0, 10_000), ("Speaker 2", 10_000, 30_000)])
        .await;
    let (quiet, talker) = (said[0].0, said[1].0);
    let alex = world.person("Alex", None).await;
    world
        .segments
        .assign_person(&talker, Some(&alex))
        .await
        .unwrap();

    let panel = world.panel().await;

    assert_eq!(panel.len(), 2);
    assert_eq!(panel[0].speaker_id, talker);
    assert_eq!(
        panel[0].person.as_ref().map(|p| p.name.as_str()),
        Some("Alex")
    );
    assert_eq!((panel[0].share_percent, panel[1].share_percent), (75, 25));
    assert_eq!(panel[1].speaker_id, quiet);
    assert_eq!(panel[1].speech, Millis(10_000));
}

#[tokio::test]
async fn renaming_all_segments_names_the_speaker_moves_its_items_and_enrols_the_voice() {
    let world = World::new().await;
    let (speaker, _) = world.speaker("Speaker 1", 0, 30_000).await;
    let (maya, alex) = (
        world.person("Maya", None).await,
        world.person("Alex", None).await,
    );
    world
        .segments
        .assign_person(&speaker, Some(&maya))
        .await
        .unwrap();
    let item = world.item_for(maya).await;

    let renamed = world.rename(speaker, person(alex), RenameScope::All).await;

    assert_eq!(renamed.speaker_id, speaker);
    assert!(renamed.enrolled);
    assert_eq!(
        world.panel().await[0].person.as_ref().map(|p| p.id),
        Some(alex)
    );
    assert_eq!(
        world.assignee(item).await,
        Some(alex),
        "Maya's item follows the voice"
    );
    let exemplars = world.voiceprints.exemplars(MODEL).await.unwrap();
    assert_eq!(exemplars.len(), 1);
    assert_eq!(exemplars[0].person_id, alex);
}

#[tokio::test]
async fn items_stay_with_a_person_who_still_speaks_in_the_meeting() {
    let world = World::new().await;
    let said = world
        .speakers(&[("Speaker 1", 0, 20_000), ("Speaker 2", 20_000, 20_000)])
        .await;
    let (first, second) = (said[0].0, said[1].0);
    let (maya, alex) = (
        world.person("Maya", None).await,
        world.person("Alex", None).await,
    );
    for speaker in [first, second] {
        world
            .segments
            .assign_person(&speaker, Some(&maya))
            .await
            .unwrap();
    }
    let item = world.item_for(maya).await;

    world.rename(first, person(alex), RenameScope::All).await;

    assert_eq!(world.assignee(item).await, Some(maya));
}

#[tokio::test]
async fn renaming_one_segment_moves_it_to_the_persons_speaker() {
    let world = World::new().await;
    let (speaker, segment) = world.speaker("Speaker 1", 0, 20_000).await;
    world.said(speaker, 30_000, 5_000).await;

    let renamed = world
        .rename(
            speaker,
            SpeakerName::New {
                name: "  Priya  ".into(),
            },
            RenameScope::Segment {
                segment_id: segment,
            },
        )
        .await;

    assert_eq!(renamed.person.name, "Priya");
    assert!(!renamed.enrolled);
    assert_ne!(renamed.speaker_id, speaker);
    let transcript = world.segments.list(&meeting(), Page::ALL).await.unwrap();
    assert_eq!(transcript[0].speaker_id, Some(renamed.speaker_id));
    assert_eq!(transcript[1].speaker_id, Some(speaker));
    let again = world
        .rename(
            speaker,
            person(renamed.person.id),
            RenameScope::Segment {
                segment_id: segment,
            },
        )
        .await;
    assert_eq!(
        again.speaker_id, renamed.speaker_id,
        "the person's speaker is reused"
    );
    assert!(world.voiceprints.exemplars(MODEL).await.unwrap().is_empty());
}

#[tokio::test]
async fn renaming_all_to_a_person_with_a_speaker_merges_the_two() {
    let world = World::new().await;
    let said = world
        .speakers(&[("Speaker 1", 0, 20_000), ("Speaker 2", 20_000, 10_000)])
        .await;
    let (named, fragment) = (said[0].0, said[1].0);
    let alex = world.person("Alex", None).await;
    world
        .segments
        .assign_person(&named, Some(&alex))
        .await
        .unwrap();

    let renamed = world.rename(fragment, person(alex), RenameScope::All).await;

    assert_eq!(renamed.speaker_id, named);
    let panel = world.panel().await;
    assert_eq!(panel.len(), 1);
    assert_eq!(panel[0].speech, Millis(30_000));
}

#[tokio::test]
async fn bad_renames_are_refused_before_anything_changes() {
    let world = World::new().await;
    let (speaker, _) = world.speaker("Speaker 1", 0, 20_000).await;
    let rename = |name: SpeakerName, scope: RenameScope| Rename {
        meeting_id: meeting(),
        speaker_id: speaker,
        name,
        scope,
    };
    let blank = rename(SpeakerName::New { name: " ".into() }, RenameScope::All);
    let nobody = rename(
        person(PersonId::generate(clock().as_ref())),
        RenameScope::All,
    );
    let missing = SegmentId::generate(clock().as_ref());
    let elsewhere = rename(
        SpeakerName::New {
            name: "Priya".into(),
        },
        RenameScope::Segment {
            segment_id: missing,
        },
    );

    assert!(matches!(
        world.service.rename(&blank).await,
        Err(SpeakerError::InvalidName)
    ));
    assert!(matches!(
        world.service.rename(&nobody).await,
        Err(SpeakerError::PersonNotFound(_))
    ));
    assert!(matches!(
        world.service.rename(&elsewhere).await,
        Err(SpeakerError::SegmentNotFound(_))
    ));
    assert!(world.panel().await[0].person.is_none());
}

#[tokio::test]
async fn merging_folds_one_speaker_into_another() {
    let world = World::new().await;
    let said = world
        .speakers(&[("Speaker 1", 0, 20_000), ("Speaker 2", 20_000, 5_000)])
        .await;
    let (keep, fold) = (said[0].0, said[1].0);
    let maya = world.person("Maya", None).await;
    world
        .segments
        .assign_person(&fold, Some(&maya))
        .await
        .unwrap();

    world.service.merge(&meeting(), fold, keep).await.unwrap();

    let panel = world.panel().await;
    assert_eq!(panel.len(), 1);
    assert_eq!(panel[0].speaker_id, keep);
    assert_eq!(
        panel[0].person.as_ref().map(|p| p.id),
        Some(maya),
        "the name survives"
    );
    assert!(matches!(
        world.service.merge(&meeting(), keep, keep).await,
        Err(SpeakerError::SameSpeaker)
    ));
}

#[tokio::test]
async fn the_users_voice_cannot_be_merged_away() {
    let world = World::new().await;
    let (other, _) = world.speaker("Speaker 1", 0, 20_000).await;
    let me = Speaker {
        id: SpeakerId::generate(clock().as_ref()),
        meeting_id: meeting(),
        label: "Me".into(),
        person_id: None,
        is_me: true,
        color: PaletteColor::cycle(0),
    };
    world.segments.add_speaker(&me).await.unwrap();

    let merged = world.service.merge(&meeting(), me.id, other).await;

    assert!(matches!(merged, Err(SpeakerError::MergingMe)));
    world.service.merge(&meeting(), other, me.id).await.unwrap();
}

#[tokio::test]
async fn a_suggestion_can_be_confirmed_or_dismissed() {
    let world = World::new().await;
    let said = world
        .speakers(&[("Speaker 1", 0, 20_000), ("Speaker 2", 20_000, 20_000)])
        .await;
    let (yes, no) = (said[0].0, said[1].0);
    let (alex, maya) = (
        world.person("Alex", None).await,
        world.person("Maya", None).await,
    );
    world.suggest(&[(yes, alex), (no, maya)]).await;
    let suggested = world.panel().await;
    assert!(suggested.iter().all(|row| row.suggestion.is_some()));

    let confirmed = world
        .service
        .confirm_suggestion(&meeting(), yes)
        .await
        .unwrap();
    world.service.dismiss_suggestion(no).await.unwrap();

    assert_eq!(confirmed.person.id, alex);
    assert!(
        confirmed.enrolled,
        "a confirmed suggestion is a confirmed rename"
    );
    assert!(
        world
            .panel()
            .await
            .iter()
            .all(|row| row.suggestion.is_none())
    );
    assert!(matches!(
        world.service.dismiss_suggestion(no).await,
        Err(SpeakerError::NoSuggestion(_))
    ));
}

#[tokio::test]
async fn the_picker_offers_attendees_first_and_who_declined_like_anyone() {
    let world = World::new().await;
    world.person("Anna Lee", Some("anna@example.com")).await;
    let alex = world.person("Alex Kim", None).await;
    let declined = world.person("Al Green", None).await;
    world.person("Bob", None).await;
    world.invite(alex, Rsvp::Accepted).await;
    world.invite(declined, Rsvp::Declined).await;

    let found = world.service.candidates(&meeting(), " a").await.unwrap();

    let names: Vec<(&str, bool)> = found
        .iter()
        .map(|candidate| (candidate.person.name.as_str(), candidate.is_attendee))
        .collect();
    assert_eq!(
        names,
        [("Alex Kim", true), ("Al Green", false), ("Anna Lee", false)]
    );
}
