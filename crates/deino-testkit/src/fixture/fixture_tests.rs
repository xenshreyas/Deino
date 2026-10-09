//! Tests of the fixture schema, the repository's fixtures, the seeder and the
//! fixture audio.

use deino_db::test_db;
use serde_json::{Value, json};

use super::*;
use crate::at;

/// The smallest meeting that passes every rule: the user and Maya on a call.
fn minimal() -> Value {
    json!({
        "id": "meeting-fixture-minimal",
        "title": "Sync",
        "kind": "one_on_one",
        "startedAt": "2026-10-07T16:01:00Z",
        "durationMs": 10_000,
        "calendarEvent": {
            "id": "evt-minimal", "icalUid": "minimal@google.com", "title": "Sync",
            "description": "", "startAt": "2026-10-07T16:00:00Z", "endAt": "2026-10-07T16:30:00Z",
            "platform": "google_meet", "conferenceUrl": "https://meet.google.com/abc-defg-hij",
            "location": null, "organizerEmail": "shreyas@deino.dev", "recurrence": "weekly"
        },
        "companies": [{ "name": "Deino", "domain": "deino.dev" }],
        "attendees": [
            { "name": "Shreyas", "email": "shreyas@deino.dev", "title": "CTO", "role": "organizer", "rsvp": "accepted" },
            { "name": "Maya Chen", "email": "maya@deino.dev", "title": null, "role": "invitee", "rsvp": "accepted" }
        ],
        "speakers": [
            { "label": "Speaker 1", "attendee": "shreyas@deino.dev", "channel": "mic" },
            { "label": "Speaker 2", "attendee": "maya@deino.dev", "channel": "system" }
        ],
        "segments": [
            segment("Speaker 1", "mic", 0, "Hey Maya, ship it Friday?"),
            segment("Speaker 2", "system", 5_000, "Yes, I will ship it Friday.")
        ],
        "userNotes": null,
        "groundTruth": {
            "decisions": ["Ship on Friday."],
            "actionItems": [{
                "title": "Ship it", "assignee": "maya@deino.dev", "due": "2026-10-09",
                "evidence": "I will ship it Friday"
            }],
            "facts": []
        },
        "audio": null
    })
}

/// A segment whose words split its four-second slot evenly.
fn segment(speaker: &str, channel: &str, start: u64, text: &str) -> Value {
    let words: Vec<&str> = text.split_whitespace().collect();
    let step = 4_000 / words.len() as u64;
    let timed: Vec<Value> = (0..)
        .zip(&words)
        .map(|(i, word)| json!({ "text": word, "startMs": start + i * step, "endMs": start + (i + 1) * step }))
        .collect();
    json!({ "speaker": speaker, "channel": channel, "startMs": start, "endMs": start + 4_000,
            "text": text, "words": timed })
}

fn parse(value: Value) -> FixtureMeeting {
    serde_json::from_value(value).unwrap()
}

fn problem(edit: impl FnOnce(&mut Value)) -> Problem {
    let mut value = minimal();
    edit(&mut value);
    validate(&parse(value)).unwrap_err()
}

#[test]
fn the_minimal_meeting_is_valid() {
    assert_eq!(validate(&parse(minimal())), Ok(()));
}

#[test]
fn unknown_fields_and_values_fail_to_parse() {
    let mut extra = minimal();
    extra["segments"][0]["confidence"] = json!(0.9);
    let mut channel = minimal();
    channel["segments"][0]["channel"] = json!("speaker");
    let mut email = minimal();
    email["attendees"][1]["email"] = json!("Maya@Deino.dev");

    for invalid in [extra, channel, email] {
        assert!(serde_json::from_value::<FixtureMeeting>(invalid).is_err());
    }
}

#[test]
fn a_segment_by_an_unknown_speaker_or_on_the_wrong_channel_is_rejected() {
    let wrong_channel = problem(|m| m["segments"][1]["channel"] = json!("mic"));
    let unknown = problem(|m| m["segments"][1]["speaker"] = json!("Speaker 9"));
    assert_eq!(wrong_channel, Problem::SegmentSpeaker(1));
    // Speaker 2 now never speaks, which the label check reports first.
    assert_eq!(unknown, Problem::SpeakerOrder("Speaker 2".into()));
}

#[test]
fn the_user_is_never_heard_on_system_audio() {
    let found = problem(|m| {
        m["speakers"][0]["channel"] = json!("system");
        m["segments"][0]["channel"] = json!("system");
    });
    assert_eq!(found, Problem::UserOnSystem);
}

#[test]
fn overlapping_or_overlong_segments_are_rejected() {
    let overlap = problem(|m| {
        m["segments"][1] = segment("Speaker 2", "system", 3_000, "Yes, I will ship it Friday.");
    });
    let overlong = problem(|m| m["durationMs"] = json!(8_000));
    assert_eq!(overlap, Problem::SegmentTiming(1));
    assert_eq!(overlong, Problem::SegmentTiming(1));
}

#[test]
fn words_must_spell_the_segment_text() {
    let found = problem(|m| m["segments"][0]["text"] = json!("Hey Maya, ship it Monday?"));
    assert_eq!(found, Problem::Words(0));
}

#[test]
fn evidence_must_be_quoted_from_the_transcript() {
    let found = problem(|m| {
        m["groundTruth"]["actionItems"][0]["evidence"] = json!("I will ship it Monday");
    });
    assert_eq!(found, Problem::Evidence(0));
}

#[test]
fn owners_must_be_invited() {
    let found =
        problem(|m| m["groundTruth"]["actionItems"][0]["assignee"] = json!("ruth@deino.dev"));
    assert_eq!(found, Problem::Assignee(0));
}

#[test]
fn speakers_are_labelled_in_order_of_first_appearance() {
    let found = problem(|m| {
        m["segments"][0]["speaker"] = json!("Speaker 2");
        m["segments"][0]["channel"] = json!("system");
        m["segments"][1]["speaker"] = json!("Speaker 1");
        m["segments"][1]["channel"] = json!("mic");
    });
    assert_eq!(found, Problem::SpeakerOrder("Speaker 1".into()));
}

#[test]
fn the_repository_fixtures_cover_every_meeting_kind_once() {
    let fixtures = Fixtures::load().unwrap();

    let kinds = [
        MeetingKind::OneOnOne,
        MeetingKind::Standup,
        MeetingKind::SalesDiscovery,
        MeetingKind::Interview,
        MeetingKind::BoardUpdate,
        MeetingKind::CustomerSupport,
        MeetingKind::Brainstorm,
        MeetingKind::Planning,
    ];
    assert_eq!(fixtures.meetings().len(), kinds.len());
    for kind in kinds {
        assert!(fixtures.meeting(kind).is_some(), "no {kind:?} fixture");
    }
}

#[test]
fn every_repository_fixture_has_ground_truth_and_the_long_one_is_long() {
    let fixtures = Fixtures::load().unwrap();

    for meeting in fixtures.meetings() {
        let truth = &meeting.ground_truth;
        assert!(!truth.decisions.is_empty(), "{}", meeting.id);
        assert!(!truth.action_items.is_empty(), "{}", meeting.id);
    }
    let planning = fixtures.meeting(MeetingKind::Planning).unwrap();
    assert!(planning.duration >= Millis(80 * 60 * 1000));
}

#[tokio::test]
async fn seeding_writes_every_fixture_meeting_and_segment_in_one_commit() {
    let database = test_db().await;
    let mut commits = database.changes().subscribe();
    let fixtures = Fixtures::load().unwrap();

    let report = seed_fixtures(&database).await.unwrap();

    let segments: usize = fixtures.meetings().iter().map(|m| m.segments.len()).sum();
    assert_eq!(report.meetings as usize, fixtures.meetings().len());
    assert_eq!(report.segments as usize, segments);
    assert_eq!(
        count(&database, "SELECT COUNT(*) FROM transcripts").await,
        segments
    );
    assert_eq!(commits.recv().await.unwrap().seq, 1);
    assert!(commits.try_recv().is_err(), "a seed is a single commit");
}

#[tokio::test]
async fn reseeding_restores_fixtures_and_keeps_the_users_meetings() {
    let database = test_db().await;
    crate::seed::meeting(
        &database,
        "meeting-mine",
        "Mine",
        at("2026-10-01T09:00:00Z"),
    )
    .await;
    seed_fixtures(&database).await.unwrap();
    sqlx::query("UPDATE meetings SET title = 'Renamed' WHERE id LIKE 'meeting-fixture-%'")
        .execute(database.writer())
        .await
        .unwrap();

    seed_fixtures(&database).await.unwrap();

    let renamed = count(
        &database,
        "SELECT COUNT(*) FROM meetings WHERE title = 'Renamed'",
    )
    .await;
    let segments = count(&database, "SELECT COUNT(*) FROM transcripts").await;
    let first = Fixtures::load()
        .unwrap()
        .meetings()
        .iter()
        .map(|m| m.segments.len())
        .sum::<usize>();
    assert_eq!(renamed, 0);
    assert_eq!(
        segments, first,
        "old transcript rows are replaced, not duplicated"
    );
    assert_eq!(
        count(
            &database,
            "SELECT COUNT(*) FROM meetings WHERE id = 'meeting-mine'"
        )
        .await,
        1
    );
}

#[tokio::test]
async fn seeded_transcripts_keep_channel_and_timing() {
    let database = test_db().await;
    seed_fixtures(&database).await.unwrap();

    let (speaker, start, end): (String, f64, f64) = sqlx::query_as(
        "SELECT speaker, audio_start_time, audio_end_time FROM transcripts
         WHERE id = 'meeting-fixture-one-on-one-segment-0000'",
    )
    .fetch_one(database.reader())
    .await
    .unwrap();

    let fixtures = Fixtures::load().unwrap();
    let first = &fixtures.meeting(MeetingKind::OneOnOne).unwrap().segments[0];
    assert_eq!(speaker, first.channel.as_str());
    assert!((start - first.start_ms.as_secs_f64()).abs() < 1e-9);
    assert!((end - first.end_ms.as_secs_f64()).abs() < 1e-9);
}

#[test]
fn fixture_audio_is_spoken_on_the_right_track_at_the_right_time() {
    let fixtures = Fixtures::load().unwrap();
    let with_audio: Vec<_> = fixtures
        .meetings()
        .iter()
        .filter(|m| m.audio.is_some())
        .collect();
    assert_eq!(with_audio.len(), 2);

    for meeting in with_audio {
        let audio = meeting.audio.as_ref().unwrap();
        let mic = PcmTrack::decode(&fixtures.audio_path(audio, Channel::Mic)).unwrap();
        let system = PcmTrack::decode(&fixtures.audio_path(audio, Channel::System)).unwrap();
        assert!(mic.duration() >= audio.clip_end_ms && mic.duration() <= Millis(60_000));
        let spoken = meeting
            .segments
            .iter()
            .filter(|s| s.end_ms <= audio.clip_end_ms);
        for segment in spoken {
            let (heard, silent) = match segment.channel {
                Channel::Mic => (&mic, &system),
                Channel::System => (&system, &mic),
            };
            let (from, to) = (segment.start_ms, segment.end_ms);
            assert!(
                heard.rms(from, to) > 0.01,
                "{} is silent at {from:?}",
                meeting.id
            );
            assert!(
                silent.rms(from, to) < 0.001,
                "{} leaks at {from:?}",
                meeting.id
            );
        }
    }
}

async fn count(database: &deino_db::Database, sql: &str) -> usize {
    let count: i64 = sqlx::query_scalar(sql)
        .fetch_one(database.reader())
        .await
        .unwrap();
    usize::try_from(count).unwrap()
}
