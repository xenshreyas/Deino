//! `SqliteFollowUpRepo` against a migrated database: the upsert that never
//! replaces a dismissed or edited row, dismissal clearing the text, the
//! schema's checks, Home's window and the trash, and ORG-04's participants.

use chrono::TimeDelta;
use deino_domain::{Email, MeetingId};
use deino_testkit::{at, seed};

use super::testing::{NOW, SALES, seeded};
use super::{
    FollowUpRepo, FollowUpStatus, ParticipantReader, QuietReason, Recipients, SqliteFollowUpRepo,
    StoredFollowUp,
};

fn ready(meeting: &str, subject: &str) -> StoredFollowUp {
    StoredFollowUp {
        meeting_id: MeetingId::new(meeting),
        status: FollowUpStatus::Ready,
        quiet_reason: None,
        subject: subject.into(),
        body: "Hi [name],".into(),
        recipients: Recipients {
            to: vec![Email::parse("guest@acme.example").unwrap()],
            ..Recipients::default()
        },
        generated_at: at(NOW),
        edited_at: None,
        dismissed_at: None,
    }
}

async fn repo() -> SqliteFollowUpRepo {
    let database = deino_testkit::test_db().await;
    seed::meeting(&database, "m1", "Kickoff", at(NOW)).await;
    seed::meeting(&database, "m2", "Renewal", at(NOW)).await;
    SqliteFollowUpRepo::new(database)
}

#[tokio::test]
async fn a_new_generation_replaces_a_draft_but_never_an_edit_or_a_dismissal() {
    let repo = repo().await;
    assert!(repo.save(&ready("m1", "First")).await.unwrap());
    assert!(repo.save(&ready("m1", "Second")).await.unwrap());
    assert_eq!(
        repo.find(&MeetingId::new("m1"))
            .await
            .unwrap()
            .unwrap()
            .subject,
        "Second"
    );

    let edited = StoredFollowUp {
        subject: "Mine".into(),
        edited_at: Some(at(NOW)),
        ..ready("m1", "")
    };
    assert!(repo.edit(&edited).await.unwrap());
    assert!(!repo.save(&ready("m1", "Third")).await.unwrap());

    assert!(repo.dismiss(&MeetingId::new("m1"), at(NOW)).await.unwrap());
    assert!(!repo.dismiss(&MeetingId::new("m1"), at(NOW)).await.unwrap());
    assert!(!repo.save(&ready("m1", "Fourth")).await.unwrap());
    let stored = repo.find(&MeetingId::new("m1")).await.unwrap().unwrap();
    assert_eq!(stored.status, FollowUpStatus::Dismissed);
    assert_eq!((stored.subject.as_str(), stored.body.as_str()), ("", ""));
}

#[tokio::test]
async fn a_quiet_row_needs_its_reason_and_carries_no_text() {
    let repo = repo().await;
    let quiet = StoredFollowUp {
        status: FollowUpStatus::Quiet,
        quiet_reason: Some(QuietReason::MovedToChat),
        subject: String::new(),
        body: String::new(),
        ..ready("m1", "")
    };
    let reasonless = StoredFollowUp {
        quiet_reason: None,
        ..quiet.clone()
    };

    assert!(repo.save(&quiet).await.unwrap());
    assert!(repo.save(&reasonless).await.is_err());
    assert!(!repo.edit(&ready("m1", "Edit")).await.unwrap());
}

#[tokio::test]
async fn home_reads_recent_ready_drafts_of_meetings_outside_the_trash() {
    let repo = repo().await;
    repo.save(&ready("m1", "Kickoff follow-up")).await.unwrap();
    repo.save(&ready("m2", "Renewal follow-up")).await.unwrap();
    sqlx::query("UPDATE meetings SET deleted_at = ?1 WHERE id = 'm2'")
        .bind(at(NOW))
        .execute(repo.database.writer())
        .await
        .unwrap();

    let recent = repo
        .ready_since(at(NOW) - TimeDelta::days(3))
        .await
        .unwrap();
    let later = repo
        .ready_since(at(NOW) + TimeDelta::seconds(1))
        .await
        .unwrap();

    assert_eq!(recent.len(), 1);
    assert_eq!(recent[0].meeting_title, "Kickoff");
    assert!(later.is_empty());
}

#[tokio::test]
async fn participants_are_attendees_and_named_speakers_with_their_companies() {
    let repo = SqliteFollowUpRepo::new(seeded().await);

    let people = repo.participants(&MeetingId::new(SALES)).await.unwrap();

    let names: Vec<&str> = people.iter().map(|person| person.name.as_str()).collect();
    assert_eq!(names[0], "Shreyas");
    assert!(people[0].is_me);
    assert_eq!(people.len(), 5, "{names:?}");
    let marcus = people
        .iter()
        .find(|person| person.name == "Marcus Webb")
        .unwrap();
    assert_eq!(marcus.company.as_deref(), Some("Brightkite Logistics"));
}
