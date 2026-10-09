//! Tests of the fixture schema, the repository's fixtures, the seeder and the
//! fixture audio.

use std::collections::HashSet;

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
fn fixtures_keep_the_microphone_and_system_tracks_apart() {
    let found = problem(|m| {
        m["speakers"][1]["channel"] = json!("mixed");
    });
    assert_eq!(found, Problem::MixedChannel("Speaker 2".into()));
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
        count(&database, "SELECT COUNT(*) FROM transcript_segments").await,
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
    let segments = count(&database, "SELECT COUNT(*) FROM transcript_segments").await;
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
async fn seeded_segments_keep_their_id_channel_speaker_and_timing() {
    let database = test_db().await;
    seed_fixtures(&database).await.unwrap();
    let id = fixture_uuid("segment/meeting-fixture-one-on-one/0");

    let (channel, label, start, end): (String, String, i64, i64) = sqlx::query_as(
        "SELECT s.channel, sp.label, s.start_ms, s.end_ms
         FROM transcript_segments AS s JOIN speakers AS sp ON sp.id = s.speaker_id
         WHERE s.id = ?1",
    )
    .bind(&id)
    .fetch_one(database.reader())
    .await
    .unwrap();

    let fixtures = Fixtures::load().unwrap();
    let first = &fixtures.meeting(MeetingKind::OneOnOne).unwrap().segments[0];
    assert_eq!(
        (channel.as_str(), label.as_str()),
        (first.channel.as_str(), first.speaker.as_str())
    );
    assert_eq!(
        (start, end),
        (i64::from(first.start_ms.0), i64::from(first.end_ms.0))
    );
}

#[tokio::test]
async fn seeding_links_people_companies_attendees_and_the_calendar_event() {
    let database = test_db().await;
    seed_fixtures(&database).await.unwrap();
    seed_fixtures(&database).await.unwrap();

    let fixtures = Fixtures::load().unwrap();
    let emails: HashSet<&str> = fixtures
        .meetings()
        .iter()
        .flat_map(|m| m.attendees.iter().map(|a| a.email.as_str()))
        .collect();
    let attendees: usize = fixtures.meetings().iter().map(|m| m.attendees.len()).sum();
    assert_eq!(
        count(&database, "SELECT COUNT(*) FROM people").await,
        emails.len()
    );
    assert_eq!(
        count(&database, "SELECT COUNT(*) FROM meeting_attendees").await,
        attendees
    );
    let unlinked =
        "SELECT COUNT(*) FROM meetings WHERE calendar_event_id IS NULL OR status <> 'ready'";
    assert_eq!(count(&database, unlinked).await, 0);
    let domains: HashSet<&str> = fixtures
        .meetings()
        .iter()
        .flat_map(|m| m.companies.iter().map(|c| c.domain.as_str()))
        .collect();
    let independent = emails
        .iter()
        .filter(|email| !domains.contains(email.split_once('@').unwrap().1));
    let without_company = "SELECT COUNT(*) FROM people WHERE company_id IS NULL";
    assert_eq!(count(&database, without_company).await, independent.count());
}

#[tokio::test]
async fn legacy_code_reads_the_seeded_transcript_through_the_compatibility_view() {
    let database = test_db().await;
    seed_fixtures(&database).await.unwrap();

    let segments = count(&database, "SELECT COUNT(*) FROM transcript_segments").await;
    let legacy = count(&database, "SELECT COUNT(*) FROM transcripts").await;

    assert_eq!(legacy, segments);
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
        let mic = PcmTrack::decode(&fixtures.audio_path(audio, Channel::Mic).unwrap()).unwrap();
        let system =
            PcmTrack::decode(&fixtures.audio_path(audio, Channel::System).unwrap()).unwrap();
        assert!(mic.duration() >= audio.clip_end_ms && mic.duration() <= Millis(60_000));
        let spoken = meeting
            .segments
            .iter()
            .filter(|s| s.end_ms <= audio.clip_end_ms);
        for segment in spoken {
            let (heard, silent) = match segment.channel {
                Channel::Mic => (&mic, &system),
                Channel::System | Channel::Mixed => (&system, &mic),
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
