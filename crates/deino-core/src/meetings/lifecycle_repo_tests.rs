//! [`MeetingLifecycleRepo`] against the real schema.

use chrono::{DateTime, Utc};
use deino_db::test_db;
use deino_domain::{
    CalendarEventId, FakeClock, MeetingId, MeetingSource, MeetingStatus, Seconds, SeriesId,
    TemplateId,
};
use deino_testkit::at;

use super::*;
use crate::testing::query_plan;

const CREATED_AT: &str = "2026-10-08T09:00:00Z";
const LATER: &str = "2026-10-08T10:00:00Z";

fn new_meeting(id: &str, status: MeetingStatus, created_at: &str) -> NewMeeting {
    NewMeeting {
        id: MeetingId::new(id),
        title: MeetingTitle::parse("Standup").unwrap(),
        status,
        source: MeetingSource::Live,
        created_at: at(created_at),
        started_at: Some(at(created_at)),
        folder_path: Some("/Users/me/Movies/Deino/standup".into()),
    }
}

async fn repo_with(meetings: &[NewMeeting]) -> SqliteMeetingRepo {
    let repo = SqliteMeetingRepo::new(test_db().await);
    for meeting in meetings {
        repo.create(meeting).await.unwrap();
    }
    repo
}

async fn state(repo: &SqliteMeetingRepo, id: &str) -> MeetingState {
    repo.state(&MeetingId::new(id)).await.unwrap().unwrap()
}

fn change(from: MeetingStatus, to: MeetingStatus) -> StatusChange {
    StatusChange {
        from,
        to,
        at: at(LATER),
    }
}

#[tokio::test]
async fn a_created_meeting_is_listed_and_has_its_initial_state() {
    let repo = repo_with(&[new_meeting(
        "meeting-a",
        MeetingStatus::Recording,
        CREATED_AT,
    )])
    .await;

    let meeting = repo
        .find(&MeetingId::new("meeting-a"))
        .await
        .unwrap()
        .unwrap();
    let state = state(&repo, "meeting-a").await;

    assert_eq!(meeting.title, "Standup");
    assert_eq!(meeting.updated_at, at(CREATED_AT));
    assert_eq!(state.status, MeetingStatus::Recording);
    assert_eq!(state.source, MeetingSource::Live);
    assert_eq!(state.started_at, Some(at(CREATED_AT)));
    assert_eq!(
        (state.ended_at, state.duration, state.deleted_at),
        (None, None, None)
    );
}

#[tokio::test]
async fn creating_a_taken_id_fails() {
    let repo = repo_with(&[new_meeting("meeting-a", MeetingStatus::Ready, CREATED_AT)]).await;

    let duplicate = repo
        .create(&new_meeting("meeting-a", MeetingStatus::Ready, LATER))
        .await;

    assert!(duplicate.is_err());
}

#[tokio::test]
async fn every_status_and_source_round_trips_through_the_schema() {
    let repo = repo_with(&[]).await;
    for (index, status) in MeetingStatus::ALL.iter().enumerate() {
        let source = MeetingSource::ALL[index % MeetingSource::ALL.len()];
        let mut meeting = new_meeting(&format!("meeting-{index}"), *status, CREATED_AT);
        meeting.source = source;
        repo.create(&meeting).await.unwrap();

        let stored = state(&repo, meeting.id.as_str()).await;

        assert_eq!((stored.status, stored.source), (*status, source));
    }
}

#[tokio::test]
async fn set_status_is_a_compare_and_set() {
    let repo = repo_with(&[new_meeting(
        "meeting-a",
        MeetingStatus::Recording,
        CREATED_AT,
    )])
    .await;
    let id = MeetingId::new("meeting-a");

    let moved = repo.set_status(
        &id,
        change(MeetingStatus::Recording, MeetingStatus::Processing),
    );
    assert!(moved.await.unwrap());
    let stale = repo.set_status(&id, change(MeetingStatus::Recording, MeetingStatus::Failed));
    assert!(!stale.await.unwrap(), "the meeting is no longer recording");

    assert_eq!(
        state(&repo, "meeting-a").await.status,
        MeetingStatus::Processing
    );
    let meeting = repo.find(&id).await.unwrap().unwrap();
    assert_eq!(meeting.updated_at, at(LATER));
}

#[tokio::test]
async fn list_by_status_finds_live_meetings_newest_first_through_its_index() {
    let repo = repo_with(&[
        new_meeting("meeting-a", MeetingStatus::Recording, CREATED_AT),
        new_meeting("meeting-b", MeetingStatus::Ready, CREATED_AT),
        new_meeting("meeting-c", MeetingStatus::Recording, LATER),
    ])
    .await;

    let recording = repo.list_by_status(MeetingStatus::Recording).await.unwrap();
    let plan = query_plan(&repo.database, include_str!("queries/list_by_status.sql")).await;

    let ids: Vec<&str> = recording
        .iter()
        .map(|meeting| meeting.id.as_str())
        .collect();
    assert_eq!(ids, ["meeting-c", "meeting-a"]);
    assert_eq!(
        plan,
        ["SEARCH meetings USING INDEX idx_meetings_status_created_at (status=?)"]
    );
}

#[tokio::test]
async fn timing_audio_and_link_are_stored() {
    let repo = repo_with(&[new_meeting(
        "meeting-a",
        MeetingStatus::Recording,
        CREATED_AT,
    )])
    .await;
    let id = MeetingId::new("meeting-a");
    let timing = MeetingTiming {
        started_at: Some(at(CREATED_AT)),
        ended_at: Some(at(LATER)),
        duration: Some(Seconds(3_540)),
    };
    let audio = MeetingAudio {
        audio_path: Some("/m/audio.mp4".into()),
        audio_mic_path: Some("/m/mic.m4a".into()),
        audio_system_path: Some("/m/system.m4a".into()),
        video_path: None,
    };
    let link = MeetingLink {
        calendar_event_id: None,
        series_id: Some(SeriesId::new("series-1")),
    };
    assert!(repo.set_timing(&id, &timing, at(LATER)).await.unwrap());
    assert!(repo.set_audio(&id, &audio, at(LATER)).await.unwrap());
    assert!(repo.link(&id, &link, at(LATER)).await.unwrap());

    let state = state(&repo, "meeting-a").await;
    assert_eq!(
        (state.started_at, state.ended_at, state.duration),
        (timing.started_at, timing.ended_at, timing.duration)
    );
    assert_eq!(state.audio, audio);
    assert_eq!(state.series_id, link.series_id);
}

#[tokio::test]
async fn a_template_is_stored_only_when_it_exists() {
    let repo = repo_with(&[new_meeting("meeting-a", MeetingStatus::Ready, CREATED_AT)]).await;
    let id = MeetingId::new("meeting-a");
    // `meetings.template_id` references `templates` (NOT-04).
    sqlx::query(
        "INSERT INTO templates (id, name, created_at, updated_at)
         VALUES ('general', 'General', '2026-10-09T08:00:00Z', '2026-10-09T08:00:00Z')",
    )
    .execute(repo.database.writer())
    .await
    .unwrap();
    let template = TemplateId::new("general");
    let missing = TemplateId::new("standard_meeting");

    let dangling = repo.set_template(&id, Some(&missing), at(LATER)).await;
    let stored = repo.set_template(&id, Some(&template), at(LATER)).await;

    assert!(dangling.is_err());
    assert!(stored.unwrap());
    assert_eq!(state(&repo, "meeting-a").await.template_id, Some(template));
}

#[tokio::test]
async fn the_schema_rejects_an_end_before_the_start_and_links_to_missing_events() {
    let repo = repo_with(&[new_meeting("meeting-a", MeetingStatus::Recording, LATER)]).await;
    let id = MeetingId::new("meeting-a");
    let backwards = MeetingTiming {
        started_at: Some(at(LATER)),
        ended_at: Some(at(CREATED_AT)),
        duration: None,
    };
    let clock = FakeClock::new(at(LATER));
    let dangling = MeetingLink {
        calendar_event_id: Some(CalendarEventId::generate(&clock)),
        series_id: None,
    };

    assert!(repo.set_timing(&id, &backwards, at(LATER)).await.is_err());
    assert!(repo.link(&id, &dangling, at(LATER)).await.is_err());
}

#[tokio::test]
async fn writes_to_a_missing_meeting_report_false() {
    let repo = repo_with(&[]).await;
    let id = MeetingId::new("meeting-x");
    let now: DateTime<Utc> = at(LATER);

    assert!(
        !repo
            .set_timing(&id, &MeetingTiming::default(), now)
            .await
            .unwrap()
    );
    assert!(
        !repo
            .set_audio(&id, &MeetingAudio::default(), now)
            .await
            .unwrap()
    );
    assert!(!repo.link(&id, &MeetingLink::default(), now).await.unwrap());
    assert!(!repo.set_template(&id, None, now).await.unwrap());
    assert_eq!(repo.state(&id).await.unwrap(), None);
}
