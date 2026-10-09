//! `DiarizeService` over fakes: the plan, the engine call, the speaker rows
//! and the segment attributions.

use std::sync::Arc;

use deino_domain::{
    AttendeeRole, Channel, ErrorClass, ErrorCode, FakeClock, MeetingId, MeetingSource,
    MeetingStatus, Millis, PersonId, Rsvp, SegmentId,
};
use deino_testkit::at;

use super::*;
use crate::diarization::{FAKE_EMBEDDING_MODEL, FakeTrackDiarizer, SpeakerHint, Voice, VoiceTurn};
use crate::meetings::{FakeMeetingRepo, MeetingAudio, MeetingTitle, NewMeeting};
use crate::people::{Attendee, FakePeopleRepo};
use crate::segments::{FakeSegmentRepo, NewSegment, WordTiming};

const MEETING: &str = "meeting-a";
const NOW: &str = "2026-10-08T10:00:00Z";
const SPLIT: [&str; 7] = ["we", "ship", "friday", "sounds", "good", "to", "me"];

struct Harness {
    service: DiarizeService,
    segments: Arc<FakeSegmentRepo>,
    diarizer: Arc<FakeTrackDiarizer>,
}

fn meeting() -> MeetingId {
    MeetingId::new(MEETING)
}

fn clock() -> FakeClock {
    FakeClock::new(at(NOW))
}

/// Words every 500 ms, each 400 ms long.
fn words(texts: &[&str], from: u32) -> Vec<WordTiming> {
    let word = |(text, start): (&&str, u32)| WordTiming {
        text: (*text).into(),
        start: Millis(start),
        end: Millis(start + 400),
    };
    texts.iter().zip((from..).step_by(500)).map(word).collect()
}

fn segment(channel: Channel, words: Vec<WordTiming>) -> NewSegment {
    NewSegment {
        id: SegmentId::generate(&clock()),
        meeting_id: meeting(),
        channel,
        speaker_id: None,
        start: words[0].start,
        end: words[words.len() - 1].end,
        text: words
            .iter()
            .map(|w| w.text.as_str())
            .collect::<Vec<_>>()
            .join(" "),
        words,
    }
}

/// A call: the user, then one remote segment that changes voice, then more
/// of the second voice (over 3 s of remote speech in all).
fn call() -> Vec<NewSegment> {
    vec![
        segment(Channel::Mic, words(&["morning", "all"], 0)),
        segment(Channel::System, words(&SPLIT, 1_000)),
        segment(Channel::System, words(&["right", "next", "item"], 5_000)),
    ]
}

/// Two remote voices: voice 0 until 2.45 s, voice 1 after.
fn two_voices() -> TrackDiarization {
    let turn = |start, end, voice| VoiceTurn {
        start: Millis(start),
        end: Millis(end),
        voice,
    };
    let voice = |centroid: [f32; 2], speech| Voice {
        centroid: centroid.to_vec(),
        speech: Millis(speech),
    };
    TrackDiarization {
        turns: vec![turn(0, 2_450, 0), turn(2_450, 9_000, 1)],
        voices: vec![voice([1.0, 0.0], 2_450), voice([0.0, 1.0], 6_550)],
    }
}

async fn recorded_meeting() -> Arc<FakeMeetingRepo> {
    let meetings = Arc::new(FakeMeetingRepo::default());
    let new = NewMeeting {
        id: meeting(),
        title: MeetingTitle::parse("Standup").unwrap(),
        status: MeetingStatus::Processing,
        source: MeetingSource::Live,
        created_at: at(NOW),
        started_at: None,
        folder_path: None,
    };
    meetings.create(&new).await.unwrap();
    let audio = MeetingAudio {
        audio_path: Some("/rec/audio.mp4".into()),
        audio_mic_path: Some("/rec/mic.m4a".into()),
        audio_system_path: Some("/rec/system.m4a".into()),
        video_path: None,
    };
    meetings
        .set_audio(&meeting(), &audio, at(NOW))
        .await
        .unwrap();
    meetings
}

/// `count` invitees who all accepted (the first organised it).
async fn invitees(count: usize) -> Arc<FakePeopleRepo> {
    let people = Arc::new(FakePeopleRepo::default());
    for index in 0..count {
        let role = if index == 0 {
            AttendeeRole::Organizer
        } else {
            AttendeeRole::Invitee
        };
        let person_id = PersonId::generate(&clock());
        let attendee = Attendee {
            meeting_id: meeting(),
            person_id,
            role,
            rsvp: Some(Rsvp::Accepted),
        };
        people.upsert_attendee(&attendee).await.unwrap();
    }
    people
}

async fn harness(
    segments: &[NewSegment],
    attendees: usize,
    result: Option<TrackDiarization>,
) -> Harness {
    let store = Arc::new(FakeSegmentRepo::default());
    store.append(segments).await.unwrap();
    let diarizer =
        Arc::new(result.map_or_else(FakeTrackDiarizer::default, FakeTrackDiarizer::returning));
    let deps = DiarizeDeps {
        meetings: recorded_meeting().await,
        attendees: invitees(attendees).await,
        segments: store.clone(),
        writer: store.clone(),
        speakers: store.clone(),
        diarizer: diarizer.clone(),
        clock: Arc::new(clock()),
    };
    Harness {
        service: DiarizeService::new(deps),
        segments: store,
        diarizer,
    }
}

#[tokio::test]
async fn a_call_diarizes_the_system_track_without_the_user() {
    let h = harness(&call(), 3, Some(two_voices())).await;

    let outcome = h.service.run(&meeting()).await.unwrap();

    let expected = DiarizationOutcome {
        channel: Some(Channel::System),
        speakers: 2,
        segments: 3,
        splits: 1,
    };
    assert_eq!(outcome, expected);
    let request = &h.diarizer.requests()[0];
    assert_eq!(request.audio.to_str(), Some("/rec/system.m4a"));
    assert_eq!(request.speakers, SpeakerHint::Exactly(2));
    let speakers = h.segments.speakers(&meeting()).await.unwrap();
    let labels: Vec<&str> = speakers.iter().map(|s| s.label.as_str()).collect();
    assert_eq!(labels, ["Me", "Speaker 1", "Speaker 2"]);
    let voice = h.segments.voice(&speakers[1].id).await.unwrap().unwrap();
    assert_eq!(voice.centroid, vec![1.0, 0.0]);
    assert_eq!(voice.embedding_model, FAKE_EMBEDDING_MODEL);
}

#[tokio::test]
async fn segments_are_attributed_and_split_at_speaker_changes() {
    let segments = call();
    let h = harness(&segments, 3, Some(two_voices())).await;

    h.service.run(&meeting()).await.unwrap();

    let speakers = h.segments.speakers(&meeting()).await.unwrap();
    let transcript = h.segments.list(&meeting(), Page::ALL).await.unwrap();
    let said: Vec<(&str, Option<SpeakerId>)> = transcript
        .iter()
        .map(|s| (s.text.as_str(), s.speaker_id))
        .collect();
    let (me, first, second) = (
        Some(speakers[0].id),
        Some(speakers[1].id),
        Some(speakers[2].id),
    );
    let expected = [
        ("morning all", me),
        ("we ship friday", first),
        ("sounds good to me", second),
        ("right next item", second),
    ];
    assert_eq!(said, expected);
    assert_eq!(
        transcript[1].id, segments[1].id,
        "the first piece keeps the segment's ID"
    );
}

#[tokio::test]
async fn running_the_stage_again_replaces_the_previous_result() {
    let h = harness(&call(), 3, Some(two_voices())).await;

    h.service.run(&meeting()).await.unwrap();
    let again = h.service.run(&meeting()).await.unwrap();

    assert_eq!(
        again.splits, 0,
        "the pieces of the first run are already one voice each"
    );
    assert_eq!(h.segments.speakers(&meeting()).await.unwrap().len(), 3);
    assert_eq!(
        h.segments.list(&meeting(), Page::ALL).await.unwrap().len(),
        4
    );
}

#[tokio::test]
async fn an_in_person_meeting_diarizes_the_shared_mic_and_assumes_no_me() {
    let h = harness(
        &[segment(Channel::Mic, words(&SPLIT, 1_000))],
        3,
        Some(two_voices()),
    )
    .await;

    let outcome = h.service.run(&meeting()).await.unwrap();

    assert_eq!(outcome.channel, Some(Channel::Mic));
    let request = &h.diarizer.requests()[0];
    assert_eq!(request.audio.to_str(), Some("/rec/mic.m4a"));
    assert_eq!(request.speakers, SpeakerHint::Exactly(3));
    let speakers = h.segments.speakers(&meeting()).await.unwrap();
    assert!(speakers.iter().all(|speaker| !speaker.is_me));
}

#[tokio::test]
async fn a_solo_memo_only_labels_me() {
    let h = harness(
        &[segment(Channel::Mic, words(&["note", "to", "self"], 0))],
        0,
        None,
    )
    .await;

    let outcome = h.service.run(&meeting()).await.unwrap();

    assert_eq!(outcome.channel, None);
    assert!(h.diarizer.requests().is_empty());
    let transcript = h.segments.list(&meeting(), Page::ALL).await.unwrap();
    let speakers = h.segments.speakers(&meeting()).await.unwrap();
    assert_eq!(transcript[0].speaker_id, Some(speakers[0].id));
    assert_eq!(speakers[0].label, ME_LABEL);
}

#[test]
fn new_labels_skip_the_users_label() {
    assert_eq!(speaker_labels(2, &[]), ["Speaker 1", "Speaker 2"]);
    let taken = ["Speaker 1".to_owned()];
    assert_eq!(speaker_labels(2, &taken), ["Speaker 2", "Speaker 3"]);
}

#[tokio::test]
async fn engine_failures_and_unknown_meetings_are_typed() {
    let h = harness(&call(), 3, None).await;

    let missing_models = h.service.run(&meeting()).await.unwrap_err();
    assert_eq!(missing_models.code(), ErrorCode::Unavailable);
    let unknown = h
        .service
        .run(&MeetingId::new("meeting-x"))
        .await
        .unwrap_err();
    assert!(matches!(unknown, DiarizationError::NotFound(_)));
    assert!(
        h.segments.speakers(&meeting()).await.unwrap().is_empty(),
        "nothing written on failure"
    );
}
