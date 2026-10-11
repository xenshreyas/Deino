use std::sync::Arc;

use deino_domain::{Channel, FakeClock, MeetingId, Millis, PaletteColor, PersonId, SegmentId};
use deino_testkit::at;

use super::*;
use crate::segments::{
    FakeSegmentRepo, NewSegment, NewSpeaker, SegmentWriter, Speaker, SpeakerRepo, SpeakerVoice,
};
use crate::voiceprints::{ExemplarReader, FakeVoiceprintRepo, VOICEPRINT_DIM};

const MODEL: &str = "wespeaker@test";

fn clock() -> Arc<FakeClock> {
    Arc::new(FakeClock::new(at("2026-10-11T09:00:00Z")))
}

fn meeting() -> MeetingId {
    MeetingId::new("meeting-a")
}

struct Setup {
    service: VoiceprintService,
    prints: Arc<FakeVoiceprintRepo>,
    segments: Arc<FakeSegmentRepo>,
    clock: Arc<FakeClock>,
}

fn setup() -> Setup {
    let (prints, segments, clock) = (
        Arc::new(FakeVoiceprintRepo::default()),
        Arc::new(FakeSegmentRepo::default()),
        clock(),
    );
    let service = VoiceprintService::new(VoiceprintDeps {
        repo: prints.clone(),
        speakers: segments.clone(),
        segments: segments.clone(),
        clock: clock.clone(),
    });
    Setup {
        service,
        prints,
        segments,
        clock,
    }
}

/// A diarized speaker with a 256-d voice who spoke for `speech` ms in one
/// stretch.
async fn voiced_speaker(setup: &Setup, speech: u32) -> SpeakerId {
    let speaker = Speaker {
        id: SpeakerId::generate(setup.clock.as_ref()),
        meeting_id: meeting(),
        label: "Speaker 1".into(),
        person_id: None,
        is_me: false,
        color: PaletteColor::cycle(1),
    };
    let voice = SpeakerVoice {
        centroid: vec![0.0625; VOICEPRINT_DIM],
        embedding_model: MODEL.into(),
        speech: Millis(speech),
    };
    let new = NewSpeaker {
        speaker: speaker.clone(),
        voice: Some(voice),
    };
    setup
        .segments
        .replace_speakers(&meeting(), &[new])
        .await
        .unwrap();
    let said = NewSegment {
        id: SegmentId::generate(setup.clock.as_ref()),
        meeting_id: meeting(),
        channel: Channel::System,
        speaker_id: Some(speaker.id),
        start: Millis(0),
        end: Millis(speech),
        text: "hello there".into(),
        words: Vec::new(),
    };
    setup.segments.append(&[said]).await.unwrap();
    speaker.id
}

#[tokio::test]
async fn a_confirmed_voice_is_enrolled_with_its_quality() {
    let setup = setup();
    let speaker = voiced_speaker(&setup, 30_000).await;
    let person = PersonId::generate(setup.clock.as_ref());

    let enrolment = setup
        .service
        .enrol(&meeting(), speaker, person)
        .await
        .unwrap();

    assert!(matches!(enrolment, Enrolment::Enrolled { evicted: 0, .. }));
    let stored = setup.prints.stored();
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].person_id, person);
    assert_eq!(stored[0].source_meeting_id, Some(meeting()));
    assert!(
        (stored[0].quality.get() - 10.0 / 60.0).abs() < 1e-9,
        "one 10 s span"
    );
    assert_eq!(setup.prints.exemplars(MODEL).await.unwrap().len(), 1);
}

#[tokio::test]
async fn a_phantom_voice_with_little_clean_speech_is_not_enrolled() {
    let setup = setup();
    let speaker = voiced_speaker(&setup, 2_000).await;
    let person = PersonId::generate(setup.clock.as_ref());

    let enrolment = setup
        .service
        .enrol(&meeting(), speaker, person)
        .await
        .unwrap();

    assert_eq!(enrolment, Enrolment::Skipped(EnrolSkip::TooLittleSpeech));
    assert!(setup.prints.stored().is_empty());
}

#[tokio::test]
async fn a_speaker_without_a_diarized_voice_is_not_enrolled() {
    let setup = setup();
    let me = Speaker {
        id: SpeakerId::generate(setup.clock.as_ref()),
        meeting_id: meeting(),
        label: "Me".into(),
        person_id: None,
        is_me: true,
        color: PaletteColor::cycle(0),
    };
    setup.segments.add_speaker(&me).await.unwrap();
    let person = PersonId::generate(setup.clock.as_ref());

    let enrolment = setup
        .service
        .enrol(&meeting(), me.id, person)
        .await
        .unwrap();

    assert_eq!(enrolment, Enrolment::Skipped(EnrolSkip::NoVoice));
}

#[tokio::test]
async fn profiles_can_be_listed_and_deleted() {
    let setup = setup();
    let speaker = voiced_speaker(&setup, 30_000).await;
    let person = PersonId::generate(setup.clock.as_ref());
    setup
        .service
        .enrol(&meeting(), speaker, person)
        .await
        .unwrap();

    let profiles = setup.service.profiles().await.unwrap();
    assert_eq!(profiles.len(), 1);
    let id = profiles[0].voiceprints[0].id;

    setup.service.delete(&id).await.unwrap();
    assert!(matches!(
        setup.service.delete(&id).await,
        Err(VoiceprintError::NotFound(_))
    ));
    assert_eq!(setup.service.forget(&person).await.unwrap(), 0);
}
