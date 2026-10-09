//! The calendar ports' contract, run against the SQLite repository and the
//! fake, plus what only the real schema can check.

use chrono::TimeDelta;
use deino_db::test_db;
use deino_domain::{
    CalendarAccountId, CalendarEventId, CalendarProvider, ConferencePlatform, FakeClock, MeetingId,
    Rsvp, SeriesId,
};
use deino_testkit::{at, seed};

use super::*;
use crate::meetings::{MeetingLifecycleRepo, MeetingLink, SqliteMeetingRepo};
use crate::testing::query_plan;

const NOW: &str = "2026-10-08T09:00:00Z";

fn account(provider: CalendarProvider, name: &str) -> CalendarAccount {
    CalendarAccount {
        id: CalendarAccountId::generate(&FakeClock::new(at(NOW))),
        provider,
        name: name.into(),
        created_at: at(NOW),
    }
}

fn event(
    account: &CalendarAccount,
    provider_event_id: &str,
    starts_in_hours: i64,
) -> CalendarEvent {
    let start_at = at(NOW) + TimeDelta::hours(starts_in_hours);
    CalendarEvent {
        id: CalendarEventId::generate(&FakeClock::new(at(NOW))),
        account_id: account.id,
        calendar_id: None,
        provider_event_id: provider_event_id.into(),
        ical_uid: None,
        series_id: Some(SeriesId::new("weekly-sync")),
        title: "Weekly sync".into(),
        description: String::new(),
        start_at,
        end_at: start_at + TimeDelta::minutes(30),
        is_all_day: false,
        conference_url: Some("https://meet.example.com/abc".into()),
        platform: Some(ConferencePlatform::GoogleMeet),
        organizer_email: Some("ada@example.com".into()),
        attendees: vec![EventAttendee {
            email: Some("ada@example.com".into()),
            name: Some("Ada".into()),
            rsvp: Some(Rsvp::Accepted),
            is_organizer: true,
        }],
        my_rsvp: Some(Rsvp::Tentative),
        is_external: false,
        record_override: None,
        created_at: at(NOW),
        updated_at: at(NOW),
    }
}

fn provider_ids(events: &[CalendarEvent]) -> Vec<&str> {
    events
        .iter()
        .map(|event| event.provider_event_id.as_str())
        .collect()
}

/// Accounts list by provider and name; removing one removes its events.
async fn accounts_round_trip(repo: &(impl CalendarAccountRepo + CalendarEventRepo)) {
    let google = account(CalendarProvider::Google, "ada@example.com");
    let apple = account(CalendarProvider::Apple, "On My Mac");
    repo.add_account(&google).await.unwrap();
    repo.add_account(&apple).await.unwrap();
    let synced = event(&google, "g-1", 1);
    repo.upsert_event(&synced).await.unwrap();

    let providers: Vec<CalendarProvider> = repo
        .accounts()
        .await
        .unwrap()
        .iter()
        .map(|account| account.provider)
        .collect();
    assert_eq!(
        providers,
        [CalendarProvider::Apple, CalendarProvider::Google]
    );

    assert!(repo.remove_account(&google.id).await.unwrap());
    assert_eq!(repo.event(&synced.id).await.unwrap(), None);
}

/// Syncs three events into a new account; returns the account and the
/// event starting in one hour.
async fn synced(
    repo: &(impl CalendarAccountRepo + CalendarEventRepo),
) -> (CalendarAccount, CalendarEvent) {
    let google = account(CalendarProvider::Google, "ada@example.com");
    repo.add_account(&google).await.unwrap();
    let first = event(&google, "g-1", 1);
    for synced in [
        event(&google, "g-3", 30),
        event(&google, "g-2", 2),
        first.clone(),
    ] {
        repo.upsert_event(&synced).await.unwrap();
    }
    (google, first)
}

/// A re-sync refreshes the stored event in place and keeps the override.
async fn upserts_refresh_in_place(repo: &(impl CalendarAccountRepo + CalendarEventRepo)) {
    let (google, first) = synced(repo).await;
    assert!(
        repo.set_record_override(&first.id, Some(true), at(NOW))
            .await
            .unwrap()
    );

    let mut resynced = event(&google, "g-1", 3);
    resynced.title = "Moved sync".into();
    assert_eq!(repo.upsert_event(&resynced).await.unwrap(), first.id);

    let stored = repo.event(&first.id).await.unwrap().unwrap();
    assert_eq!(
        (stored.title.as_str(), stored.record_override),
        ("Moved sync", Some(true))
    );
    assert_eq!(stored.attendees, first.attendees);
}

/// Events list by start within a window, and delete once.
async fn events_list_by_start_within_a_window(
    repo: &(impl CalendarAccountRepo + CalendarEventRepo),
) {
    let (_, first) = synced(repo).await;

    let window = repo
        .events_between(at(NOW), at(NOW) + TimeDelta::days(1))
        .await
        .unwrap();

    assert_eq!(provider_ids(&window), ["g-1", "g-2"]);
    assert!(repo.delete_event(&first.id).await.unwrap());
    assert!(!repo.delete_event(&first.id).await.unwrap());
}

#[tokio::test]
async fn sqlite_accounts_round_trip() {
    accounts_round_trip(&SqliteCalendarRepo::new(test_db().await)).await;
}

#[tokio::test]
async fn fake_accounts_round_trip() {
    accounts_round_trip(&FakeCalendarRepo::default()).await;
}

#[tokio::test]
async fn sqlite_upserts_refresh_in_place() {
    upserts_refresh_in_place(&SqliteCalendarRepo::new(test_db().await)).await;
}

#[tokio::test]
async fn fake_upserts_refresh_in_place() {
    upserts_refresh_in_place(&FakeCalendarRepo::default()).await;
}

#[tokio::test]
async fn sqlite_events_list_by_start_within_a_window() {
    events_list_by_start_within_a_window(&SqliteCalendarRepo::new(test_db().await)).await;
}

#[tokio::test]
async fn fake_events_list_by_start_within_a_window() {
    events_list_by_start_within_a_window(&FakeCalendarRepo::default()).await;
}

#[tokio::test]
async fn every_platform_provider_and_rsvp_round_trips_through_the_schema() {
    let repo = SqliteCalendarRepo::new(test_db().await);
    for provider in CalendarProvider::ALL {
        repo.add_account(&account(*provider, provider.as_str()))
            .await
            .unwrap();
    }
    let accounts = repo.accounts().await.unwrap();
    let owner = &accounts[0];
    for (index, platform) in ConferencePlatform::ALL.iter().enumerate() {
        let mut row = event(owner, platform.as_str(), i64::try_from(index).unwrap());
        row.platform = Some(*platform);
        row.my_rsvp = Some(Rsvp::ALL[index % Rsvp::ALL.len()]);
        repo.upsert_event(&row).await.unwrap();
    }

    let events = repo
        .events_between(at(NOW), at(NOW) + TimeDelta::days(1))
        .await
        .unwrap();

    let platforms: Vec<_> = events.iter().filter_map(|event| event.platform).collect();
    assert_eq!(platforms, ConferencePlatform::ALL);
}

#[tokio::test]
async fn deleting_an_event_unlinks_its_meeting_but_keeps_the_series() {
    let database = test_db().await;
    let repo = SqliteCalendarRepo::new(database.clone());
    let google = account(CalendarProvider::Google, "ada@example.com");
    repo.add_account(&google).await.unwrap();
    let synced = event(&google, "g-1", 1);
    repo.upsert_event(&synced).await.unwrap();
    seed::meeting(&database, "meeting-a", "Weekly sync", at(NOW)).await;
    let meetings = SqliteMeetingRepo::new(database);
    let id = MeetingId::new("meeting-a");
    let link = MeetingLink {
        calendar_event_id: Some(synced.id),
        series_id: synced.series_id.clone(),
    };
    meetings.link(&id, &link, at(NOW)).await.unwrap();

    repo.delete_event(&synced.id).await.unwrap();

    let state = meetings.state(&id).await.unwrap().unwrap();
    assert_eq!(state.calendar_event_id, None);
    assert_eq!(state.series_id, synced.series_id);
}

#[tokio::test]
async fn the_schema_rejects_an_event_that_ends_before_it_starts() {
    let repo = SqliteCalendarRepo::new(test_db().await);
    let google = account(CalendarProvider::Google, "ada@example.com");
    repo.add_account(&google).await.unwrap();
    let mut backwards = event(&google, "g-1", 1);
    backwards.end_at = backwards.start_at - TimeDelta::minutes(1);

    assert!(repo.upsert_event(&backwards).await.is_err());
}

#[tokio::test]
async fn calendar_lists_are_read_through_their_indexes() {
    let repo = SqliteCalendarRepo::new(test_db().await);

    let accounts = query_plan(&repo.database, include_str!("queries/accounts.sql")).await;
    let events = query_plan(&repo.database, include_str!("queries/events_between.sql")).await;

    assert_eq!(
        accounts,
        ["SCAN calendar_accounts USING INDEX sqlite_autoindex_calendar_accounts_2"]
    );
    assert_eq!(
        events,
        [
            "SEARCH calendar_events USING INDEX idx_calendar_events_start_at (start_at>? AND start_at<?)"
        ]
    );
}
