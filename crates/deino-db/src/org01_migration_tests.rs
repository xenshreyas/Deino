//! ORG-01 migration round trip: events stored before calendars existed keep
//! every column, belong to no calendar and are not all-day; a calendar
//! takes its events with it when it goes.

use super::migration_tests::migrator_before;
use super::*;

/// The version of `20261009155147_org01_calendars.sql`.
const ORG01_VERSION: i64 = 20_261_009_155_147;

const ACCOUNT: &str = "01926b2c-0000-7000-8000-000000000001";
const EVENT: &str = "01926b2c-0000-7000-8000-000000000002";
const CALENDAR: &str = "01926b2c-0000-7000-8000-000000000003";

/// A database just before ORG-01 holding one synced event.
async fn pre_org01_database() -> Database {
    let database = Database::open(&DbLocation::Memory, &migrator_before(ORG01_VERSION))
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO calendar_accounts (id, provider, name, created_at)
         VALUES (?1, 'apple', 'iCloud', '2026-10-01T09:00:00+00:00')",
    )
    .bind(ACCOUNT)
    .execute(database.writer())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO calendar_events (id, account_id, provider_event_id, title, start_at, end_at, created_at, updated_at)
         VALUES (?1, ?2, 'evt-1', 'Standup', '2026-10-01T09:00:00+00:00', '2026-10-01T09:15:00+00:00',
                 '2026-10-01T08:00:00+00:00', '2026-10-01T08:00:00+00:00')",
    )
    .bind(EVENT)
    .bind(ACCOUNT)
    .execute(database.writer())
    .await
    .unwrap();
    MIGRATOR.run(database.writer()).await.unwrap();
    database
}

#[tokio::test]
async fn existing_events_keep_their_columns_and_belong_to_no_calendar() {
    let database = pre_org01_database().await;

    let row: (String, Option<String>, i64) =
        sqlx::query_as("SELECT title, calendar_id, is_all_day FROM calendar_events WHERE id = ?1")
            .bind(EVENT)
            .fetch_one(database.reader())
            .await
            .unwrap();

    assert_eq!(row, ("Standup".to_owned(), None, 0));
}

#[tokio::test]
async fn a_calendar_takes_its_events_and_checks_its_colour() {
    let database = pre_org01_database().await;
    let insert_calendar = |color: &'static str| {
        sqlx::query(
            "INSERT INTO calendars (id, account_id, provider_calendar_id, title, color, created_at, updated_at)
             VALUES (?1, ?2, 'work', 'Work', ?3, '2026-10-01T08:00:00+00:00', '2026-10-01T08:00:00+00:00')",
        )
        .bind(CALENDAR)
        .bind(ACCOUNT)
        .bind(color)
    };
    assert!(
        insert_calendar("blue")
            .execute(database.writer())
            .await
            .is_err()
    );
    insert_calendar("#34AADC")
        .execute(database.writer())
        .await
        .unwrap();
    sqlx::query("UPDATE calendar_events SET calendar_id = ?1")
        .bind(CALENDAR)
        .execute(database.writer())
        .await
        .unwrap();

    sqlx::query("DELETE FROM calendars WHERE id = ?1")
        .bind(CALENDAR)
        .execute(database.writer())
        .await
        .unwrap();

    let left: (i64,) = sqlx::query_as("SELECT count(*) FROM calendar_events")
        .fetch_one(database.reader())
        .await
        .unwrap();
    assert_eq!(left, (0,));
}

#[test]
fn calendars_changes_reach_the_ui() {
    assert_eq!(Table::from_name("calendars"), Some(Table::Calendars));
}
