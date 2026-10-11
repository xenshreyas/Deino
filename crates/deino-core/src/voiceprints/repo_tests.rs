//! The voiceprint ports' contract, run against the SQLite repository and the
//! fake, plus what only the real schema can check.

use chrono::{DateTime, Duration, Utc};
use deino_db::{Database, test_db};
use deino_domain::{FakeClock, MeetingId, PersonId, VoiceprintId};
use deino_testkit::{at, seed};

use super::*;
use crate::people::{Person, PersonRepo, SqlitePeopleRepo};
use crate::testing::query_plan;

const MEETING: &str = "meeting-a";
const MODEL: &str = "wespeaker@test";
const T0: &str = "2026-10-11T09:00:00Z";

fn person(name: &str) -> Person {
    let created_at = at(T0);
    Person {
        id: PersonId::generate(&FakeClock::new(created_at)),
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
        created_at,
        updated_at: created_at,
    }
}

/// A 256-d unit vector pointing along `axis`.
fn axis(axis: usize) -> Vec<f32> {
    let mut values = vec![0.0; VOICEPRINT_DIM];
    values[axis] = 1.0;
    values
}

fn print(person: PersonId, quality: f64, minutes: i64) -> Voiceprint {
    let created_at: DateTime<Utc> = at(T0) + Duration::minutes(minutes);
    Voiceprint {
        id: VoiceprintId::generate(&FakeClock::new(created_at)),
        person_id: person,
        embedding: VoiceEmbedding::new(axis(0), MODEL).unwrap(),
        source_meeting_id: Some(MeetingId::new(MEETING)),
        quality: Quality::new(quality),
        created_at,
    }
}

/// A database with the meeting and two people, Alex and Maya.
async fn database() -> (Database, Person, Person) {
    let database = test_db().await;
    seed::meeting(&database, MEETING, "Standup", at(T0)).await;
    let people = SqlitePeopleRepo::new(database.clone());
    let (alex, maya) = (person("Alex"), person("Maya"));
    people.create(&alex).await.unwrap();
    people.create(&maya).await.unwrap();
    (database, alex, maya)
}

/// A sixth exemplar evicts the weakest; the oldest goes among equals.
async fn enrolment_keeps_the_best_five(
    repo: &(impl VoiceprintRepo + ExemplarReader),
    person: PersonId,
) {
    let five: Vec<Voiceprint> = [(0, 0.5), (1, 0.9), (2, 0.2), (3, 0.7), (4, 0.2)]
        .into_iter()
        .map(|(minute, quality)| print(person, quality, minute))
        .collect();
    for exemplar in &five {
        assert!(repo.enrol(exemplar, 5).await.unwrap().is_empty());
    }

    let evicted = repo.enrol(&print(person, 0.6, 5), 5).await.unwrap();

    assert_eq!(
        evicted,
        vec![five[2].id],
        "the weakest, oldest exemplar goes"
    );
    assert_eq!(repo.exemplars(MODEL).await.unwrap().len(), 5);
}

/// The weakest new exemplar is itself evicted, leaving the five better ones.
async fn a_weak_newcomer_is_turned_away(repo: &impl VoiceprintRepo, person: PersonId) {
    for minute in 0..5 {
        repo.enrol(&print(person, 0.8, minute), 5).await.unwrap();
    }
    let weak = print(person, 0.1, 9);

    assert_eq!(repo.enrol(&weak, 5).await.unwrap(), vec![weak.id]);
}

/// Exemplars of another model never reach matching.
async fn exemplars_are_per_model(repo: &(impl VoiceprintRepo + ExemplarReader), person: PersonId) {
    let mut other = print(person, 0.5, 0);
    other.embedding = VoiceEmbedding::new(axis(1), "campplus@test").unwrap();
    repo.enrol(&other, 5).await.unwrap();
    repo.enrol(&print(person, 0.5, 1), 5).await.unwrap();

    let exemplars = repo.exemplars(MODEL).await.unwrap();

    assert_eq!(exemplars.len(), 1);
    assert_eq!(exemplars[0].embedding, axis(0));
    assert!(!exemplars[0].is_me);
}

/// Deleting one exemplar or a whole profile.
async fn exemplars_can_be_deleted(repo: &(impl VoiceprintRepo + ExemplarReader), person: PersonId) {
    let (one, two) = (print(person, 0.5, 0), print(person, 0.4, 1));
    repo.enrol(&one, 5).await.unwrap();
    repo.enrol(&two, 5).await.unwrap();

    assert!(repo.delete(&one.id).await.unwrap());
    assert!(!repo.delete(&one.id).await.unwrap());
    assert_eq!(repo.forget(&person).await.unwrap(), 1);
    assert!(repo.exemplars(MODEL).await.unwrap().is_empty());
}

#[tokio::test]
async fn the_sqlite_repository_honours_the_contract() {
    for check in 0..4 {
        let (database, alex, _) = database().await;
        let repo = SqliteVoiceprintRepo::new(database);
        match check {
            0 => enrolment_keeps_the_best_five(&repo, alex.id).await,
            1 => a_weak_newcomer_is_turned_away(&repo, alex.id).await,
            2 => exemplars_are_per_model(&repo, alex.id).await,
            _ => exemplars_can_be_deleted(&repo, alex.id).await,
        }
    }
}

#[tokio::test]
async fn the_fake_honours_the_contract() {
    let alex = person("Alex").id;
    enrolment_keeps_the_best_five(&FakeVoiceprintRepo::default(), alex).await;
    a_weak_newcomer_is_turned_away(&FakeVoiceprintRepo::default(), alex).await;
    exemplars_are_per_model(&FakeVoiceprintRepo::default(), alex).await;
    exemplars_can_be_deleted(&FakeVoiceprintRepo::default(), alex).await;
}

#[tokio::test]
async fn profiles_list_people_by_name_with_their_meeting() {
    let (database, alex, maya) = database().await;
    let repo = SqliteVoiceprintRepo::new(database);
    repo.enrol(&print(maya.id, 0.4, 0), 5).await.unwrap();
    repo.enrol(&print(alex.id, 0.3, 1), 5).await.unwrap();
    repo.enrol(&print(alex.id, 0.9, 2), 5).await.unwrap();

    let rows = repo.profile_rows().await.unwrap();

    let names: Vec<&str> = rows.iter().map(|row| row.name.as_str()).collect();
    assert_eq!(names, ["Alex", "Alex", "Maya"]);
    assert!((rows[0].voiceprint.quality - 0.9).abs() < 1e-6);
    assert_eq!(
        rows[0].voiceprint.source_meeting_title.as_deref(),
        Some("Standup")
    );
}

#[tokio::test]
async fn deleting_the_person_or_the_meeting_is_handled_by_the_schema() {
    let (database, alex, _) = database().await;
    let repo = SqliteVoiceprintRepo::new(database.clone());
    repo.enrol(&print(alex.id, 0.5, 0), 5).await.unwrap();
    sqlx::query("DELETE FROM meetings")
        .execute(database.writer())
        .await
        .unwrap();

    let rows = repo.profile_rows().await.unwrap();
    assert_eq!(rows[0].voiceprint.source_meeting_id, None);

    SqlitePeopleRepo::new(database)
        .delete(&alex.id)
        .await
        .unwrap();
    assert!(repo.exemplars(MODEL).await.unwrap().is_empty());
}

#[tokio::test]
async fn the_schema_rejects_a_vector_that_is_not_256_values() {
    let (database, alex, _) = database().await;
    let short: Vec<u8> = vec![0; 12];
    let result = sqlx::query(
        "INSERT INTO voiceprints (id, person_id, embedding, embedding_model, quality, created_at)
         VALUES ('00000000-0000-7000-8000-000000000001', ?1, ?2, 'm', 0.5, 't')",
    )
    .bind(alex.id)
    .bind(short)
    .execute(database.writer())
    .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn eviction_reads_a_persons_exemplars_through_the_index() {
    let (database, ..) = database().await;
    let plan = query_plan(&database, include_str!("queries/surplus.sql")).await;
    assert!(
        plan.iter()
            .any(|step| step.contains("idx_voiceprints_person_id")),
        "{plan:?}"
    );
}
