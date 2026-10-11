//! Related meetings: the series and title-and-people rules, "Previous in
//! series", and the SQLite lookup with its indexes.

use std::collections::BTreeSet;

use deino_db::test_db;
use deino_domain::{AttendeeRole, MeetingId, PersonId, SeriesId};
use deino_testkit::{at, seed};

use super::related::relate;
use super::test_people::{add_person, person_id as person};
use super::*;
use crate::people::{Attendee, AttendeeRepo, SqlitePeopleRepo};
use crate::testing::query_plan;

fn people(ids: &[u128]) -> BTreeSet<PersonId> {
    ids.iter().copied().map(person).collect()
}

/// The meeting being looked at: "Weekly sync" on Oct 9 with the user (1)
/// and Ada (2), in the weekly series.
fn key() -> RelationKey {
    RelationKey {
        meeting: MeetingId::new("meeting-now"),
        title: "Weekly sync".into(),
        series_id: Some(SeriesId::new("weekly")),
        happened_at: at("2026-10-09T10:00:00Z"),
        attendees: people(&[1, 2]),
    }
}

fn candidate(id: &str, title: &str, day: u32, attendees: &[u128]) -> RelatedCandidate {
    RelatedCandidate {
        id: MeetingId::new(id),
        title: title.into(),
        series_id: None,
        happened_at: at(&format!("2026-10-{day:02}T10:00:00Z")),
        attendees: people(attendees),
    }
}

fn in_series(mut candidate: RelatedCandidate) -> RelatedCandidate {
    candidate.series_id = Some(SeriesId::new("weekly"));
    candidate
}

#[test]
fn the_series_or_the_same_title_and_people_relate_meetings() {
    let candidates = vec![
        in_series(candidate("series-other-title", "Renamed sync", 7, &[9])),
        candidate("same-title-and-people", "Weekly sync", 6, &[2]),
        candidate("same-title-other-people", "Weekly sync", 5, &[3]),
        candidate("same-people-other-title", "Pricing review", 4, &[1, 2]),
    ];

    let related = relate(&key(), candidates, Some(&person(1)));

    let previous = related.previous.unwrap();
    assert_eq!(previous.id.as_str(), "series-other-title");
    assert_eq!(previous.by, RelatedBy::Series);
    let others: Vec<_> = related
        .others
        .iter()
        .map(|m| (m.id.as_str(), m.by))
        .collect();
    assert_eq!(
        others,
        [("same-title-and-people", RelatedBy::TitleAndPeople)]
    );
}

#[test]
fn previous_is_the_latest_meeting_before_this_one_and_later_ones_are_only_listed() {
    let candidates = vec![
        in_series(candidate("next-week", "Weekly sync", 16, &[2])),
        in_series(candidate("last-week", "Weekly sync", 2, &[2])),
        in_series(candidate("two-weeks-ago", "Weekly sync", 1, &[2])),
    ];

    let related = relate(&key(), candidates, None);

    assert_eq!(related.previous.unwrap().id.as_str(), "last-week");
    let others: Vec<_> = related.others.iter().map(|m| m.id.as_str()).collect();
    assert_eq!(others, ["next-week", "two-weeks-ago"]);
}

#[test]
fn a_title_alone_with_only_the_user_relates_nothing() {
    let mut alone = key();
    alone.series_id = None;
    alone.attendees = people(&[1]);
    let candidates = vec![candidate("also-alone", "Weekly sync", 2, &[1])];

    let related = relate(&alone, candidates, Some(&person(1)));

    assert_eq!(related, RelatedMeetings::default());
}

#[test]
fn the_list_is_capped() {
    let candidates = (10..30)
        .rev()
        .map(|day| in_series(candidate(&format!("day-{day}"), "Weekly sync", day, &[2])))
        .collect();

    let related = relate(&key(), candidates, None);

    assert_eq!(related.others.len(), RELATED_LIMIT);
    assert_eq!(related.previous, None, "every candidate is later");
}

async fn attend(repo: &SqlitePeopleRepo, meeting: &str, person: PersonId) {
    let row = Attendee {
        meeting_id: MeetingId::new(meeting),
        person_id: person,
        role: AttendeeRole::Invitee,
        rsvp: None,
    };
    repo.upsert_attendee(&row).await.unwrap();
}

/// Two "Weekly sync" meetings with Ada and a "Pricing review"; the repository.
async fn seeded_repo() -> SqliteRelatedRepo {
    let database = test_db().await;
    seed::meeting(
        &database,
        "meeting-a",
        "Weekly sync",
        at("2026-10-02T10:00:00Z"),
    )
    .await;
    seed::meeting(
        &database,
        "meeting-b",
        "Weekly sync",
        at("2026-10-09T10:00:00Z"),
    )
    .await;
    seed::meeting(
        &database,
        "meeting-c",
        "Pricing review",
        at("2026-10-05T10:00:00Z"),
    )
    .await;
    let people_repo = SqlitePeopleRepo::new(database.clone());
    add_person(&people_repo, 2, "Ada", None).await;
    attend(&people_repo, "meeting-a", person(2)).await;
    attend(&people_repo, "meeting-b", person(2)).await;
    SqliteRelatedRepo::new(database)
}

#[tokio::test]
async fn the_sqlite_lookup_reads_titles_series_and_attendees() {
    let repo = seeded_repo().await;

    let key = repo
        .key(&MeetingId::new("meeting-b"))
        .await
        .unwrap()
        .unwrap();
    let candidates = repo.candidates(&key).await.unwrap();

    assert_eq!(key.attendees, people(&[2]));
    assert_eq!(key.happened_at, at("2026-10-09T10:00:00Z"));
    let ids: Vec<_> = candidates.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(ids, ["meeting-a"]);
    assert_eq!(candidates[0].attendees, people(&[2]));
    assert!(
        repo.key(&MeetingId::new("missing"))
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn candidates_are_searched_by_index() {
    let database = test_db().await;

    let plan = query_plan(&database, include_str!("queries/related_candidates.sql")).await;

    let scans_meetings = |step: &str| {
        step == "SCAN m" || step.starts_with("SCAN m ") || step.starts_with("SCAN meetings")
    };
    assert!(
        !plan.iter().any(|step| scans_meetings(step)),
        "no full scan of meetings: {plan:?}"
    );
    assert!(
        plan.iter().any(|step| step.starts_with("SEARCH m USING")),
        "{plan:?}"
    );
    assert!(
        plan.iter().any(|step| step.contains("idx_meetings_title")),
        "{plan:?}"
    );
    assert!(
        plan.iter()
            .any(|step| step.contains("idx_meetings_series_id")),
        "{plan:?}"
    );
}
