//! NOT-08's acceptance checks over the seeded fixture library and the mock
//! LLM: a brief for an upcoming event with someone the user has met, none
//! for a new contact; idempotent preparation, the 06:00 and hour-before
//! runs on an injected clock, redaction and citations that resolve.

use std::sync::Arc;

use chrono::TimeDelta;
use deino_domain::MeetingId;
use deino_testkit::at;

use super::testing::{
    MARCUS, NOW, SECRET, STRANGER, WordRedactor, answering, briefs, later, scheduler, seeded,
    upcoming,
};
use super::{BriefCitation, BriefFeedback, BriefRun};
use crate::calendar::{CalendarEventRepo, SqliteCalendarRepo};
use crate::meetings::{MeetingRepo, SqliteMeetingRepo};
use crate::settings::{BriefAudience, BriefSettings};

/// The sales discovery fixture, where the user met Marcus Webb.
const DISCOVERY: &str = "meeting-fixture-sales-discovery";

#[tokio::test]
async fn a_brief_appears_for_an_upcoming_event_with_people_the_user_has_met() {
    let database = seeded().await;
    let event = upcoming(&database, MARCUS, TimeDelta::hours(3)).await;
    let briefs = briefs(&database, None);

    let run = briefs.service.prepare(&event).await.unwrap();

    assert_eq!(run, BriefRun::Written);
    let brief = briefs.service.for_event(&event.id).await.unwrap().unwrap();
    // The bullet citing an invented `m9` is dropped.
    assert_eq!(brief.bullets.len(), 2);
    assert!(brief.bullets[0].text.contains("DispatchPro"));
    assert!(brief.read_at.is_none());
    let prompt = briefs.llm.requests()[0].messages[0].text();
    assert!(prompt.contains("[m1] Meeting: Deino × Brightkite Logistics: discovery"));
    assert!(prompt.contains("Wednesday 30 September 2026"));
    assert!(prompt.contains("[e1] Invitation"));
}

#[tokio::test]
async fn every_citation_resolves_to_what_the_prompt_showed() {
    let database = seeded().await;
    let event = upcoming(&database, MARCUS, TimeDelta::hours(3)).await;
    let briefs = briefs(&database, None);
    briefs.service.prepare(&event).await.unwrap();

    let brief = briefs.service.for_event(&event.id).await.unwrap().unwrap();

    let BriefCitation::Meeting {
        meeting_id, title, ..
    } = &brief.bullets[0].citations[0]
    else {
        panic!("the first bullet cites a meeting: {:?}", brief.bullets[0]);
    };
    assert_eq!(meeting_id, &MeetingId::new(DISCOVERY));
    let stored = SqliteMeetingRepo::new(database.clone())
        .find(meeting_id)
        .await
        .unwrap();
    assert_eq!(&stored.unwrap().title, title);
    assert_eq!(brief.bullets[1].citations[0], BriefCitation::Event);
    assert_eq!(brief.bullets[1].citations.len(), 2);
}

#[tokio::test]
async fn no_brief_and_no_model_call_for_a_new_contact() {
    let database = seeded().await;
    let event = upcoming(&database, STRANGER, TimeDelta::hours(3)).await;
    let briefs = briefs(&database, None);

    let run = briefs.service.prepare(&event).await.unwrap();

    assert_eq!(run, BriefRun::Insufficient);
    assert!(briefs.llm.requests().is_empty());
    assert_eq!(briefs.service.for_event(&event.id).await.unwrap(), None);
}

#[tokio::test]
async fn the_models_insufficient_answer_or_one_cited_bullet_means_no_brief() {
    let database = seeded().await;
    let event = upcoming(&database, MARCUS, TimeDelta::hours(3)).await;
    let insufficient = answering(&database, None, r#"{"insufficient": true, "bullets": []}"#);
    let one = r#"{"insufficient": false, "bullets": [{"text": "Licence ends in January.", "sources": ["m1"]}]}"#;

    assert_eq!(
        insufficient.service.prepare(&event).await.unwrap(),
        BriefRun::Insufficient
    );
    assert_eq!(
        insufficient.service.for_event(&event.id).await.unwrap(),
        None
    );
    let single = answering(&database, None, one);
    // The inputs are unchanged, so only another event gets a new answer.
    let other = upcoming(&database, MARCUS, TimeDelta::hours(5)).await;
    assert_eq!(
        single.service.prepare(&other).await.unwrap(),
        BriefRun::Insufficient
    );
    assert_eq!(single.llm.requests().len(), 1);
}

#[tokio::test]
async fn preparing_again_with_unchanged_inputs_does_nothing() {
    let database = seeded().await;
    let mut event = upcoming(&database, MARCUS, TimeDelta::hours(3)).await;
    let briefs = briefs(&database, None);
    briefs.service.prepare(&event).await.unwrap();
    briefs.service.mark_read(&event.id).await.unwrap();

    assert_eq!(
        briefs.service.prepare(&event).await.unwrap(),
        BriefRun::Unchanged
    );
    assert_eq!(briefs.llm.requests().len(), 1);
    assert!(
        briefs
            .service
            .for_event(&event.id)
            .await
            .unwrap()
            .unwrap()
            .read_at
            .is_some()
    );

    event.description = "Agenda: pilot scope, pricing and security review.".to_owned();
    SqliteCalendarRepo::new(database.clone())
        .upsert_event(&event)
        .await
        .unwrap();
    assert_eq!(
        briefs.service.prepare(&event).await.unwrap(),
        BriefRun::Written
    );
    assert_eq!(briefs.llm.requests().len(), 2);
    let fresh = briefs.service.for_event(&event.id).await.unwrap().unwrap();
    assert!(fresh.read_at.is_none(), "a new brief is unread again");
}

#[tokio::test]
async fn secrets_are_redacted_before_the_model_sees_them() {
    let database = seeded().await;
    let event = upcoming(&database, MARCUS, TimeDelta::hours(3)).await;
    let redactor = Arc::new(WordRedactor::default());
    let briefs = briefs(&database, Some(redactor.clone()));

    briefs.service.prepare(&event).await.unwrap();

    let prompt = briefs.llm.requests()[0].messages[0].text();
    assert!(!prompt.contains(SECRET));
    assert!(prompt.contains("[redacted]"));
    assert!(
        redactor
            .seen
            .lock()
            .unwrap()
            .iter()
            .any(|text| text.contains(SECRET))
    );
}

#[tokio::test]
async fn feedback_and_reading_are_stored_and_found_from_the_meeting_note() {
    let database = seeded().await;
    let event = upcoming(&database, MARCUS, TimeDelta::hours(3)).await;
    let briefs = briefs(&database, None);
    briefs.service.prepare(&event).await.unwrap();
    let note = MeetingId::new("meeting-upcoming-note");
    deino_testkit::seed::meeting(&database, note.as_str(), "Brightkite follow-up", at(NOW)).await;
    sqlx::query("UPDATE meetings SET calendar_event_id = ?1 WHERE id = ?2")
        .bind(event.id)
        .bind(&note)
        .execute(database.writer())
        .await
        .unwrap();

    briefs
        .service
        .give_feedback(&event.id, BriefFeedback::NotUseful)
        .await
        .unwrap();
    briefs.service.mark_read(&event.id).await.unwrap();

    let brief = briefs.service.for_meeting(&note).await.unwrap().unwrap();
    assert_eq!(brief.feedback, Some(BriefFeedback::NotUseful));
    assert_eq!(brief.read_at, Some(at(NOW)));
    assert_eq!(
        briefs.service.prepare(&event).await.unwrap(),
        BriefRun::Unchanged
    );
}

#[tokio::test]
async fn the_job_runs_at_six_and_an_hour_before_and_survives_a_restart() {
    let database = seeded().await;
    // 10:00 Pacific; the tests' "now" is 06:30.
    let event = upcoming(&database, MARCUS, TimeDelta::minutes(210)).await;
    let briefs = briefs(&database, None);
    let clock = Arc::new(deino_domain::FakeClock::new(later(-60)));
    let job = scheduler(&database, &briefs, clock.clone(), BriefSettings::default());

    assert!(
        job.tick().await.unwrap().is_empty(),
        "05:30: nothing is due"
    );
    clock.advance(TimeDelta::minutes(60));
    assert_eq!(
        job.tick().await.unwrap(),
        vec![event.id],
        "06:30: the morning run"
    );
    assert!(
        job.tick().await.unwrap().is_empty(),
        "the morning run ran once"
    );
    clock.advance(TimeDelta::minutes(151));
    assert_eq!(
        job.tick().await.unwrap(),
        vec![event.id],
        "09:01: the hour before"
    );
    assert_eq!(briefs.llm.requests().len(), 1, "the inputs did not change");

    let restarted = scheduler(&database, &briefs, clock.clone(), BriefSettings::default());
    assert_eq!(restarted.tick().await.unwrap(), vec![event.id]);
    assert_eq!(
        briefs.llm.requests().len(),
        1,
        "a restart writes nothing new"
    );
    clock.advance(TimeDelta::minutes(60));
    assert!(
        restarted.tick().await.unwrap().is_empty(),
        "the meeting has started"
    );
}

#[tokio::test]
async fn the_job_skips_internal_meetings_unless_groups_are_on_and_when_off() {
    let database = seeded().await;
    let mut event = upcoming(&database, MARCUS, TimeDelta::minutes(30)).await;
    event.is_external = false;
    SqliteCalendarRepo::new(database.clone())
        .upsert_event(&event)
        .await
        .unwrap();
    let briefs = briefs(&database, None);
    let clock = Arc::new(deino_domain::FakeClock::new(later(0)));
    let off = BriefSettings {
        enabled: false,
        audience: BriefAudience::Groups,
    };
    let groups = BriefSettings {
        enabled: true,
        audience: BriefAudience::Groups,
    };

    let external_only = scheduler(&database, &briefs, clock.clone(), BriefSettings::default());
    assert!(external_only.tick().await.unwrap().is_empty());
    assert!(
        scheduler(&database, &briefs, clock.clone(), off)
            .tick()
            .await
            .unwrap()
            .is_empty()
    );
    let prepared = scheduler(&database, &briefs, clock, groups)
        .tick()
        .await
        .unwrap();
    assert_eq!(prepared, vec![event.id]);
}
