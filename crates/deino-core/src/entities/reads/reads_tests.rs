//! The entity reads against the real schema: what counts as a meeting, the
//! list orders, and the indexes the hot queries use.

use deino_domain::{AttendeeRole, MeetingId};

use super::*;
use crate::entities::testing::{NOW, World};
use crate::people::Person;
use crate::testing::query_plan;

/// Maya at Deino, Owen, Noor, the user, and a duplicate merged into Maya.
async fn people(world: &World) -> [Person; 3] {
    let deino = world.company("Deino", Some("deino.dev")).await;
    let mut maya = world.person("Maya Chen", Some("maya@deino.dev")).await;
    world.employ(&mut maya, &deino).await;
    let owen = world.person("Owen Price", Some("owen@fernhollow.co")).await;
    world.person("Noor Haddad", None).await;
    let me = world.person("Shreyas", Some("shreyas@deino.dev")).await;
    world.make_me(&me.id).await;
    let gone = world.person("M. Chen", None).await;
    world.mark_merged(&gone.id, &maya.id).await;
    [maya, owen, me]
}

/// Two past meetings, one in the trash and one scheduled ahead.
async fn meetings(world: &World) -> [MeetingId; 4] {
    let mut ids = Vec::new();
    for (id, title, at) in [
        ("one", "1:1 Maya", "2026-10-07T10:00:00Z"),
        ("sync", "Fernhollow sync", "2026-10-09T10:00:00Z"),
        ("trashed", "Old", "2026-10-10T10:00:00Z"),
        ("ahead", "Planning", "2026-10-20T10:00:00Z"),
    ] {
        ids.push(world.meeting(id, title, at).await);
    }
    let trash = "UPDATE meetings SET deleted_at = ?1 WHERE id = 'trashed'";
    world.run(trash, &[NOW]).await;
    let schedule = "UPDATE meetings SET status = 'scheduled' WHERE id = 'ahead'";
    world.run(schedule, &[]).await;
    ids.try_into().unwrap()
}

/// Maya met twice (plus once in a trashed meeting and once scheduled
/// ahead), Owen once, Noor never; the user and a merged person are never
/// listed.
async fn world() -> World {
    let world = World::new().await;
    let [maya, owen, me] = people(&world).await;
    let [one, sync, trashed, ahead] = meetings(&world).await;
    for meeting in [&one, &sync, &trashed, &ahead] {
        world.attend(meeting, &maya.id, AttendeeRole::Invitee).await;
        world.attend(meeting, &me.id, AttendeeRole::Organizer).await;
    }
    world.attend(&sync, &owen.id, AttendeeRole::Invitee).await;
    world.summarize(&one, "Agreed to split the queue.").await;
    world
        .run(
            "INSERT INTO meeting_notes (meeting_id, notes_markdown, notes_json, created_at, updated_at)
             VALUES ('sync', '- Owen wants SSO', '{}', ?1, ?1)",
            &[NOW],
        )
        .await;
    world
}

#[tokio::test]
async fn people_are_listed_by_last_met_with_counts_from_past_live_meetings() {
    let world = world().await;
    let repo = SqliteEntityRepo::new(world.database.clone());

    let rows = repo.person_rows(None).await.unwrap();

    let summary: Vec<(&str, u32, Option<&str>)> = rows
        .iter()
        .map(|row| {
            (
                row.name.as_str(),
                row.meeting_count,
                row.company_name.as_deref(),
            )
        })
        .collect();
    assert_eq!(
        summary,
        [
            ("Maya Chen", 2, Some("Deino")),
            ("Owen Price", 1, None),
            ("Noor Haddad", 0, None)
        ]
    );
    assert_eq!(
        rows[0].last_met_at,
        Some(deino_testkit::at("2026-10-09T10:00:00Z"))
    );
    assert_eq!(rows[2].last_met_at, None);
}

#[tokio::test]
async fn a_companys_rows_count_its_people_and_their_meetings() {
    let world = world().await;
    let repo = SqliteEntityRepo::new(world.database.clone());

    let companies = repo.company_rows().await.unwrap();
    let people = repo.person_rows(Some(&companies[0].id)).await.unwrap();

    assert_eq!(companies.len(), 1);
    assert_eq!(
        (companies[0].people_count, companies[0].meeting_count),
        (1, 2)
    );
    let names: Vec<&str> = people.iter().map(|row| row.name.as_str()).collect();
    assert_eq!(names, ["Maya Chen"]);
}

#[tokio::test]
async fn meetings_come_most_recent_first_and_digests_carry_summaries_and_notes() {
    let world = world().await;
    let repo = SqliteEntityRepo::new(world.database.clone());
    let maya = repo.person_rows(None).await.unwrap()[0].id;
    let company = repo.company_rows().await.unwrap()[0].id;

    let meetings = repo.person_meetings(&maya, 10).await.unwrap();
    let limited = repo.company_meetings(&company, 1).await.unwrap();
    let digests = repo.person_digests(&maya, 10).await.unwrap();

    let titles: Vec<&str> = meetings.iter().map(|m| m.title.as_str()).collect();
    assert_eq!(titles, ["Fernhollow sync", "1:1 Maya"]);
    assert_eq!(meetings[0].duration_s, Some(1800));
    assert_eq!(limited.len(), 1);
    assert_eq!(digests[0].notes.as_deref(), Some("- Owen wants SSO"));
    assert_eq!(
        digests[1].summary.as_deref(),
        Some("Agreed to split the queue.")
    );
}

#[tokio::test]
async fn a_persons_meetings_are_read_through_the_attendance_index() {
    let world = World::new().await;

    let plan = query_plan(
        &world.database,
        include_str!("../queries/person_meetings.sql"),
    )
    .await;
    let rows = query_plan(&world.database, include_str!("../queries/person_rows.sql")).await;

    assert!(
        plan[0].contains("idx_meeting_attendees_person_id"),
        "{plan:?}"
    );
    assert!(
        rows.iter()
            .any(|step| step.contains("idx_meeting_attendees_person_id")),
        "{rows:?}"
    );
}
