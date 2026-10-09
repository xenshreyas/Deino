//! The attendee port's contract, run against the SQLite repository and the
//! fake, plus what only the real schema can check.

use deino_db::test_db;
use deino_domain::{AttendeeRole, FakeClock, MeetingId, PersonId, Rsvp};
use deino_testkit::{at, seed};

use super::*;
use crate::meetings::Meeting;
use crate::testing::query_plan;

const CREATED_AT: &str = "2026-10-08T09:00:00Z";

fn meeting(id: &str, created_at: &str) -> Meeting {
    Meeting {
        id: MeetingId::new(id),
        title: "Standup".into(),
        created_at: at(created_at),
        updated_at: at(created_at),
    }
}

fn meetings() -> [Meeting; 2] {
    [
        meeting("meeting-a", CREATED_AT),
        meeting("meeting-b", "2026-10-09T09:00:00Z"),
    ]
}

fn person(name: &str) -> Person {
    let created_at = at(CREATED_AT);
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

fn attendee(meeting: &str, person: &Person, role: AttendeeRole, rsvp: Option<Rsvp>) -> Attendee {
    Attendee {
        meeting_id: MeetingId::new(meeting),
        person_id: person.id,
        role,
        rsvp,
    }
}

async fn sqlite_repo() -> SqlitePeopleRepo {
    let database = test_db().await;
    for meeting in meetings() {
        seed::meeting(
            &database,
            meeting.id.as_str(),
            &meeting.title,
            meeting.created_at,
        )
        .await;
    }
    SqlitePeopleRepo::new(database)
}

/// Ada (invited, then organiser of `meeting-a`; invitee of `meeting-b`)
/// and Grace (spoke in `meeting-a`), as stored by [`attended`].
struct Attendance {
    ada: Person,
    grace: Person,
    accepted: Attendee,
    spoke: Attendee,
}

async fn attended(repo: &(impl PersonRepo + AttendeeRepo)) -> Attendance {
    let (ada, grace) = (person("Ada"), person("Grace"));
    repo.create(&ada).await.unwrap();
    repo.create(&grace).await.unwrap();
    let invited = attendee(
        "meeting-a",
        &ada,
        AttendeeRole::Invitee,
        Some(Rsvp::NeedsAction),
    );
    let accepted = attendee(
        "meeting-a",
        &ada,
        AttendeeRole::Organizer,
        Some(Rsvp::Accepted),
    );
    let spoke = attendee("meeting-a", &grace, AttendeeRole::SpeakerOnly, None);
    let elsewhere = attendee("meeting-b", &ada, AttendeeRole::Invitee, None);
    for row in [&invited, &spoke, &accepted, &elsewhere] {
        repo.upsert_attendee(row).await.unwrap();
    }
    Attendance {
        ada,
        grace,
        accepted,
        spoke,
    }
}

/// Upserts add then update in place; attendees list in insertion order,
/// and a person's meetings list newest first.
async fn attendance_round_trip(repo: &(impl PersonRepo + AttendeeRepo)) {
    let Attendance {
        ada,
        accepted,
        spoke,
        ..
    } = attended(repo).await;

    let listed = repo.attendees(&MeetingId::new("meeting-a")).await.unwrap();
    let meetings = repo.meetings_of(&ada.id).await.unwrap();

    assert_eq!(listed, [accepted, spoke]);
    let ids: Vec<&str> = meetings.iter().map(|meeting| meeting.id.as_str()).collect();
    assert_eq!(ids, ["meeting-b", "meeting-a"]);
}

/// Removing an attendee takes the meeting off their list, once.
async fn removing_an_attendee(repo: &(impl PersonRepo + AttendeeRepo)) {
    let Attendance { grace, .. } = attended(repo).await;
    let meeting = MeetingId::new("meeting-a");

    assert!(repo.remove_attendee(&meeting, &grace.id).await.unwrap());
    assert!(!repo.remove_attendee(&meeting, &grace.id).await.unwrap());
    assert!(repo.meetings_of(&grace.id).await.unwrap().is_empty());
}

#[tokio::test]
async fn sqlite_removing_an_attendee() {
    removing_an_attendee(&sqlite_repo().await).await;
}

#[tokio::test]
async fn fake_removing_an_attendee() {
    removing_an_attendee(&FakePeopleRepo::with_meetings(meetings())).await;
}

#[tokio::test]
async fn sqlite_attendance_round_trip() {
    attendance_round_trip(&sqlite_repo().await).await;
}

#[tokio::test]
async fn fake_attendance_round_trip() {
    attendance_round_trip(&FakePeopleRepo::with_meetings(meetings())).await;
}

#[tokio::test]
async fn every_role_and_rsvp_round_trips_through_the_schema() {
    let repo = sqlite_repo().await;
    for (index, role) in AttendeeRole::ALL.iter().enumerate() {
        let someone = person(role.as_str());
        repo.create(&someone).await.unwrap();
        let rsvp = Rsvp::ALL[index % Rsvp::ALL.len()];
        repo.upsert_attendee(&attendee("meeting-a", &someone, *role, Some(rsvp)))
            .await
            .unwrap();
    }
    let extra = person("extra");
    repo.create(&extra).await.unwrap();
    repo.upsert_attendee(&attendee(
        "meeting-a",
        &extra,
        AttendeeRole::Invitee,
        Some(Rsvp::NeedsAction),
    ))
    .await
    .unwrap();

    let listed = repo.attendees(&MeetingId::new("meeting-a")).await.unwrap();

    let roles: Vec<AttendeeRole> = listed.iter().map(|row| row.role).collect();
    let rsvps: Vec<Option<Rsvp>> = listed.iter().map(|row| row.rsvp).collect();
    assert_eq!(&roles[..3], AttendeeRole::ALL);
    assert_eq!(
        rsvps,
        Rsvp::ALL.iter().copied().map(Some).collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn deleting_a_person_or_meeting_removes_their_attendance() {
    let repo = sqlite_repo().await;
    let ada = person("Ada");
    repo.create(&ada).await.unwrap();
    for meeting in ["meeting-a", "meeting-b"] {
        repo.upsert_attendee(&attendee(meeting, &ada, AttendeeRole::Invitee, None))
            .await
            .unwrap();
    }

    sqlx::query("DELETE FROM meetings WHERE id = 'meeting-a'")
        .execute(repo.database.writer())
        .await
        .unwrap();
    assert_eq!(repo.meetings_of(&ada.id).await.unwrap().len(), 1);
    repo.delete(&ada.id).await.unwrap();

    assert!(
        repo.attendees(&MeetingId::new("meeting-b"))
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn attendance_is_read_through_its_indexes() {
    let repo = sqlite_repo().await;

    let attendees = query_plan(&repo.database, include_str!("queries/attendees.sql")).await;
    let meetings_of = query_plan(&repo.database, include_str!("queries/meetings_of.sql")).await;

    assert_eq!(
        attendees,
        [
            "SEARCH meeting_attendees USING INDEX sqlite_autoindex_meeting_attendees_1 (meeting_id=?)",
            "USE TEMP B-TREE FOR ORDER BY"
        ]
    );
    assert_eq!(
        meetings_of,
        [
            "SEARCH a USING COVERING INDEX idx_meeting_attendees_person_id (person_id=?)",
            "SEARCH m USING INDEX sqlite_autoindex_meetings_1 (id=?)",
            "USE TEMP B-TREE FOR ORDER BY"
        ]
    );
}
