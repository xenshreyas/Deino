use chrono::DateTime;
use deino_domain::{
    AttendeeRole, CallApp, FakeClock, Millis, PaletteColor, PersonId, SegmentId, SpeakerId, TagSpan,
};

use super::*;
use crate::people::{Attendee, FakePeopleRepo, Person};
use crate::segments::{FakeSegmentRepo, NewSegment, SegmentWriter, SpeakerRepo};
use crate::speaker_tags::{FakeSpeakerTagRepo, SpeakerTagWriter};

const MEETING: &str = "meeting";

fn clock() -> FakeClock {
    FakeClock::new(DateTime::UNIX_EPOCH)
}

struct Rig {
    tags: Arc<FakeSpeakerTagRepo>,
    segments: Arc<FakeSegmentRepo>,
    people: Arc<FakePeopleRepo>,
}

impl Rig {
    fn new() -> Self {
        Self {
            tags: Arc::default(),
            segments: Arc::default(),
            people: Arc::default(),
        }
    }

    fn priors(&self) -> SpeakerTagPriors {
        SpeakerTagPriors::new(TagPriorPorts {
            tags: self.tags.clone(),
            segments: self.segments.clone(),
            attendees: self.people.clone(),
            people: self.people.clone(),
            me: self.people.clone(),
            clock: Arc::new(clock()),
        })
    }

    async fn speaker(&self, label: &str) -> Speaker {
        let speaker = Speaker {
            id: SpeakerId::generate(&clock()),
            meeting_id: MeetingId::new(MEETING),
            label: label.into(),
            person_id: None,
            is_me: false,
            color: PaletteColor::cycle(0),
        };
        self.segments.add_speaker(&speaker).await.unwrap();
        speaker
    }

    async fn say(
        &self,
        speaker: &Speaker,
        (start, end): (u32, u32),
        channel: Channel,
    ) -> SegmentId {
        let id = SegmentId::generate(&clock());
        let segment = NewSegment {
            id,
            meeting_id: MeetingId::new(MEETING),
            channel,
            speaker_id: Some(speaker.id),
            start: Millis(start),
            end: Millis(end),
            text: "…".into(),
            words: Vec::new(),
        };
        self.segments.append(&[segment]).await.unwrap();
        id
    }

    async fn tag(&self, (start, end): (u32, u32), name: &str) {
        let span = TagSpan {
            start: Millis(start),
            end: Millis(end),
            name: DisplayName::parse(name).unwrap(),
            app: CallApp::Meet,
        };
        let meeting = MeetingId::new(MEETING);
        self.tags.append(&meeting, &[span]).await.unwrap();
    }

    async fn invite(&self, name: &str) -> PersonId {
        let person = person(name);
        self.people.create(&person).await.unwrap();
        let attendee = Attendee {
            meeting_id: MeetingId::new(MEETING),
            person_id: person.id,
            role: AttendeeRole::Invitee,
            rsvp: None,
        };
        self.people.upsert_attendee(&attendee).await.unwrap();
        person.id
    }

    async fn plan(&self) -> TagPlan {
        let meeting = MeetingId::new(MEETING);
        let speakers = self.segments.speakers(&meeting).await.unwrap();
        self.priors().plan(&meeting, &speakers).await.unwrap()
    }
}

fn person(name: &str) -> Person {
    let at = DateTime::UNIX_EPOCH;
    Person {
        id: PersonId::generate(&clock()),
        name: name.into(),
        email: None,
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
    }
}

#[tokio::test]
async fn a_meeting_without_speaker_tags_has_an_empty_plan() {
    let rig = Rig::new();
    let them = rig.speaker("Them").await;
    rig.say(&them, (0, 5_000), Channel::System).await;

    assert_eq!(rig.plan().await, TagPlan::default());
}

#[tokio::test]
async fn names_seat_voices_people_and_labels_and_move_stray_segments() {
    let rig = Rig::new();
    let alex = rig.invite("Alex Chen").await;
    let (one, two) = (
        rig.speaker("Speaker 1").await,
        rig.speaker("Speaker 2").await,
    );
    rig.say(&one, (0, 4_000), Channel::System).await;
    let stray = rig.say(&one, (4_000, 5_000), Channel::System).await;
    rig.say(&two, (5_000, 9_000), Channel::System).await;
    rig.tag((0, 4_000), "Alex").await;
    rig.tag((4_000, 9_000), "Bea Ruiz").await;

    let plan = rig.plan().await;

    assert_eq!(plan.named, [(one.id, alex)]);
    assert_eq!(
        plan.seating.labelled,
        [(two.id, DisplayName::parse("Bea Ruiz").unwrap())]
    );
    let moved: Vec<(SegmentId, SpeakerId)> = plan
        .seating
        .moved
        .iter()
        .map(|m| (m.segment, m.to))
        .collect();
    assert_eq!(moved, [(stray, two.id)]);
    assert!(plan.seating.created.is_empty() && plan.seating.emptied.is_empty());
    assert_eq!(plan.claimed, [two.id]);
}

#[tokio::test]
async fn microphone_and_deleted_segments_are_not_the_apps_to_name() {
    let rig = Rig::new();
    let me = rig.speaker("Me").await;
    let them = rig.speaker("Them").await;
    rig.say(&me, (0, 3_000), Channel::Mic).await;
    let deleted = rig.say(&them, (3_000, 6_000), Channel::System).await;
    rig.segments.set_deleted(&deleted, true).await.unwrap();
    rig.tag((0, 6_000), "Alex").await;

    assert_eq!(rig.plan().await, TagPlan::default());
}

#[tokio::test]
async fn a_name_no_voice_holds_gets_a_new_speaker_and_empties_a_stray_one() {
    let rig = Rig::new();
    let them = rig.speaker("Them").await;
    let stray = rig.speaker("Speaker 2").await;
    rig.say(&them, (0, 6_000), Channel::System).await;
    rig.say(&stray, (6_000, 8_000), Channel::System).await;
    let lone = rig.say(&stray, (8_000, 9_000), Channel::System).await;
    rig.tag((0, 8_000), "Alex").await;
    rig.tag((8_000, 9_000), "Cleo").await;

    let plan = rig.plan().await;

    let [created] = plan.seating.created.as_slice() else {
        panic!("one new speaker: {plan:?}");
    };
    assert_eq!(created.label, "Cleo");
    assert_eq!(created.person_id, None);
    assert_eq!(plan.seating.moved.len(), 2);
    assert!(
        plan.seating
            .moved
            .iter()
            .any(|change| change.segment == lone && change.to == created.id)
    );
    assert_eq!(plan.seating.emptied, [stray.id]);
    let overlay = plan.overlay(vec![them.clone(), stray]);
    let labels: Vec<&str> = overlay.iter().map(|s| s.label.as_str()).collect();
    assert_eq!(labels, ["Alex", "Cleo"]);
}
