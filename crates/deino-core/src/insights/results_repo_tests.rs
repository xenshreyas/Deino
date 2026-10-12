//! The insights tables through [`SqliteInsightRepo`], against the real
//! schema with the fixture meetings seeded.

use deino_db::{Database, test_db};
use deino_domain::{ConditionGroups, FakeClock, Millis};
use deino_testkit::{at, seed_fixtures};

use super::*;
use crate::insights::{
    ApplyWhen, Cardinality, FieldKind, FieldValue, InsightDef, InsightField, InsightRepo,
    InsightValue, MeetingRecords,
};
use crate::testing::query_plan;

const SALES: &str = "meeting-fixture-sales-discovery";
const STANDUP: &str = "meeting-fixture-standup";

async fn seeded() -> (Database, SqliteInsightRepo) {
    let database = test_db().await;
    seed_fixtures(&database).await.unwrap();
    (database.clone(), SqliteInsightRepo::new(database))
}

fn def(name: &str, created: &str) -> InsightDef {
    let clock = FakeClock::new(at(created));
    InsightDef {
        id: InsightId::generate(&clock),
        name: name.into(),
        description: "What was the budget?".into(),
        fields: vec![InsightField {
            name: "Budget".into(),
            kind: FieldKind::Number,
        }],
        cardinality: Cardinality::Fixed { count: 1 },
        apply_when: ApplyWhen::Automatic {
            conditions: ConditionGroups::every_meeting(),
        },
        revision: 1,
        created_at: at(created),
        updated_at: at(created),
    }
}

/// The first transcript segment of `meeting`.
async fn first_segment(database: &Database, meeting: &str) -> SegmentId {
    let id: String = sqlx::query_scalar(
        "SELECT id FROM transcript_segments WHERE meeting_id = ?1 ORDER BY start_ms LIMIT 1",
    )
    .bind(meeting)
    .fetch_one(database.reader())
    .await
    .unwrap();
    id.parse().unwrap()
}

fn budget(value: f64, segment: Option<SegmentId>) -> InsightAnswer {
    InsightAnswer {
        value: InsightValue::Fields {
            fields: vec![crate::insights::FieldEntry {
                name: "Budget".into(),
                value: Some(FieldValue::Number(value)),
            }],
        },
        speaker: Some("Stored name".into()),
        timestamp_ms: Some(Millis(994)),
        segment_id: segment,
    }
}

#[tokio::test]
async fn definitions_round_trip_and_list_oldest_first() {
    let (_, repo) = seeded().await;
    let later = def("Budget", "2026-10-09T09:00:00Z");
    let mut earlier = def("Decisions", "2026-10-08T09:00:00Z");
    earlier.fields = Vec::new();
    earlier.cardinality = Cardinality::Auto;
    earlier.apply_when = ApplyWhen::Manual;

    repo.create(&later).await.unwrap();
    repo.create(&earlier).await.unwrap();

    assert_eq!(repo.list().await.unwrap(), [earlier.clone(), later.clone()]);
    let renamed = InsightDef {
        name: "Budget (USD)".into(),
        revision: 2,
        ..later.clone()
    };
    assert!(repo.update(&renamed).await.unwrap());
    assert_eq!(repo.get(&later.id).await.unwrap(), Some(renamed));
    assert!(repo.delete(&earlier.id).await.unwrap());
    assert!(!repo.delete(&earlier.id).await.unwrap());
}

#[tokio::test]
async fn names_are_unique_ignoring_case() {
    let (_, repo) = seeded().await;
    repo.create(&def("Budget", "2026-10-08T09:00:00Z"))
        .await
        .unwrap();

    let error = repo
        .create(&def("BUDGET", "2026-10-08T09:01:00Z"))
        .await
        .unwrap_err();

    assert!(error.is_unique_violation());
}

#[tokio::test]
async fn a_request_keeps_earlier_answers_until_the_new_run_replaces_them() {
    let (_, repo) = seeded().await;
    let insight = def("Budget", "2026-10-08T09:00:00Z");
    repo.create(&insight).await.unwrap();
    let meeting = MeetingId::new(SALES);
    let answers = RunAnswers {
        insight_id: insight.id,
        meeting_id: meeting.clone(),
        revision: 1,
        answers: vec![budget(40_000.0, None)],
        at: at("2026-10-08T10:00:00Z"),
    };
    repo.save_answers(&answers).await.unwrap();

    repo.request(
        &insight.id,
        std::slice::from_ref(&meeting),
        at("2026-10-09T10:00:00Z"),
    )
    .await
    .unwrap();

    let [run] = repo
        .for_meeting(&meeting)
        .await
        .unwrap()
        .try_into()
        .unwrap();
    assert_eq!(run.status, InsightStatus::Pending);
    assert_eq!(run.answers, answers.answers);
    assert_eq!(run.requested_at, at("2026-10-09T10:00:00Z"));
    assert_eq!(run.generated_at, Some(at("2026-10-08T10:00:00Z")));
}

#[tokio::test]
async fn a_failure_keeps_the_answers_and_a_success_clears_the_error() {
    let (_, repo) = seeded().await;
    let insight = def("Budget", "2026-10-08T09:00:00Z");
    repo.create(&insight).await.unwrap();
    let meeting = MeetingId::new(SALES);

    repo.save_failure(&RunFailure {
        insight_id: insight.id,
        meeting_id: meeting.clone(),
        message: "Add an Anthropic key".into(),
        at: at("2026-10-08T10:00:00Z"),
    })
    .await
    .unwrap();
    let failed = repo.for_meeting(&meeting).await.unwrap();
    repo.save_answers(&RunAnswers {
        insight_id: insight.id,
        meeting_id: meeting.clone(),
        revision: 1,
        answers: Vec::new(),
        at: at("2026-10-08T11:00:00Z"),
    })
    .await
    .unwrap();
    let done = repo.for_meeting(&meeting).await.unwrap();

    assert_eq!(failed[0].status, InsightStatus::Failed);
    assert_eq!(failed[0].error.as_deref(), Some("Add an Anthropic key"));
    assert_eq!(
        (done[0].status, done[0].error.clone()),
        (InsightStatus::Done, None)
    );
    assert_eq!(done[0].revision, Some(1));
}

#[tokio::test]
async fn an_insights_runs_list_newest_meeting_first_and_progress_skips_the_trash() {
    let (database, repo) = seeded().await;
    let insight = def("Budget", "2026-10-08T09:00:00Z");
    repo.create(&insight).await.unwrap();
    let meetings = [MeetingId::new(SALES), MeetingId::new(STANDUP)];
    repo.request(&insight.id, &meetings, at("2026-10-09T10:00:00Z"))
        .await
        .unwrap();
    sqlx::query("UPDATE meetings SET deleted_at = '2026-10-10T00:00:00Z' WHERE id = ?1")
        .bind(SALES)
        .execute(database.writer())
        .await
        .unwrap();

    let rows = repo.for_insight(&insight.id).await.unwrap();
    let progress = repo.progress(&insight.id).await.unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].run.meeting_id, MeetingId::new(STANDUP));
    assert_eq!(rows[0].meeting_title, "Eng standup");
    assert_eq!(
        progress,
        RunProgress {
            pending: 1,
            done: 0,
            failed: 0
        }
    );
}

#[tokio::test]
async fn speaker_names_follow_the_cited_segments_speaker() {
    let (database, repo) = seeded().await;
    let lena = first_segment(&database, SALES).await;
    let unknown: SegmentId = "01920000-0000-7000-8000-00000000abcd".parse().unwrap();

    let names = repo.names(&[lena, unknown]).await.unwrap();

    assert_eq!(names.get(&lena).map(String::as_str), Some("Lena Okafor"));
    assert!(!names.contains_key(&unknown));
    assert!(repo.names(&[]).await.unwrap().is_empty());
}

#[tokio::test]
async fn a_meetings_record_carries_its_title_people_and_organisation() {
    let (_, repo) = seeded().await;

    let record = repo.record(&MeetingId::new(SALES)).await.unwrap().unwrap();
    let past = repo.past_meetings().await.unwrap();

    assert_eq!(
        record.facts.title,
        "Deino × Brightkite Logistics: discovery"
    );
    assert!(
        record
            .facts
            .attendee_domains
            .contains("brightkite-logistics.com")
    );
    assert!(record.attendee_emails.len() >= 4);
    assert!(
        past.iter()
            .any(|meeting| meeting.id == MeetingId::new(SALES))
    );
    assert!(
        past.windows(2)
            .all(|pair| pair[0].started_at >= pair[1].started_at)
    );
    assert_eq!(repo.record(&MeetingId::new("nope")).await.unwrap(), None);
}

#[tokio::test]
async fn a_meetings_runs_are_read_through_its_index() {
    let database = test_db().await;

    let plan = query_plan(&database, include_str!("queries/runs_of_meeting.sql")).await;

    assert_eq!(
        plan.first().map(String::as_str),
        Some("SEARCH insight_results USING INDEX idx_insight_results_meeting (meeting_id=?)")
    );
}
