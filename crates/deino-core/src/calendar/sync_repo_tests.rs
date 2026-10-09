//! The sync port's contract, run against the SQLite repository and the
//! fake: idempotent re-syncs, RSVP changes in place, removals inside the
//! window only, and accounts of other providers untouched.

use chrono::TimeDelta;
use deino_db::test_db;
use deino_domain::{CalendarProvider, Clock, FakeClock, Rsvp};
use deino_testkit::at;

use super::mapping::{BatchContext, to_batch};
use super::test_events::{NOW, clock, me, meeting, snapshot};
use super::*;
use crate::testing::query_plan;

/// Every calendar port at once.
trait Repo: CalendarAccountRepo + CalendarRepo + CalendarEventRepo + CalendarSyncRepo {}
impl<R: CalendarAccountRepo + CalendarRepo + CalendarEventRepo + CalendarSyncRepo> Repo for R {}

/// The batch for `source` as the provider `provider` read it at `clock`.
fn batch_at(source: &SourceSnapshot, provider: CalendarProvider, clock: &FakeClock) -> SyncBatch {
    let context = BatchContext {
        provider,
        window: SyncWindow::around(clock.now()),
        clock,
        profile_emails: &[],
    };
    to_batch(source, &context).0
}

fn batch(source: &SourceSnapshot) -> SyncBatch {
    batch_at(source, CalendarProvider::Apple, &clock())
}

async fn all_events(repo: &impl Repo) -> Vec<CalendarEvent> {
    repo.events_between(
        at(NOW) - TimeDelta::days(400),
        at(NOW) + TimeDelta::days(400),
    )
    .await
    .unwrap()
}

/// The first sync stores everything; the same data again changes nothing.
async fn resyncing_unchanged_data_changes_nothing(repo: &impl Repo) {
    let source = snapshot(vec![meeting("evt-1", 1), meeting("evt-2", 2)]);

    let first = repo.apply(&batch(&source)).await.unwrap();
    let stored = all_events(repo).await;
    let again = repo.apply(&batch(&source)).await.unwrap();

    assert_eq!((first.events_added, first.calendars_changed), (2, 2));
    assert!(!again.changed(), "{again:?}");
    assert_eq!(all_events(repo).await, stored);
    let calendars = repo.calendars().await.unwrap();
    assert_eq!(calendars.len(), 1);
    assert_eq!(stored[0].calendar_id, Some(calendars[0].id));
}

/// An RSVP change updates the event in place and keeps the user's override.
async fn an_rsvp_change_updates_in_place(repo: &impl Repo) {
    repo.apply(&batch(&snapshot(vec![meeting("evt-1", 1)])))
        .await
        .unwrap();
    let stored = all_events(repo).await.remove(0);
    repo.set_record_override(&stored.id, Some(false), at(NOW))
        .await
        .unwrap();
    let mut declined = meeting("evt-1", 1);
    declined.attendees[0] = me(Some(Rsvp::Declined));

    let report = repo.apply(&batch(&snapshot(vec![declined]))).await.unwrap();

    assert_eq!((report.events_added, report.events_updated), (0, 1));
    let updated = repo.event(&stored.id).await.unwrap().unwrap();
    assert_eq!(updated.my_rsvp, Some(Rsvp::Declined));
    assert_eq!(updated.record_override, Some(false));
    assert_eq!(updated.created_at, stored.created_at);
}

/// The provider IDs of every stored event, by start.
async fn stored_ids(repo: &impl Repo) -> Vec<String> {
    let events = all_events(repo).await;
    events
        .into_iter()
        .map(|event| event.provider_event_id)
        .collect()
}

/// Every stored account as (provider, name).
async fn stored_accounts(repo: &impl Repo) -> Vec<(CalendarProvider, String)> {
    let accounts = repo.accounts().await.unwrap();
    accounts.into_iter().map(|a| (a.provider, a.name)).collect()
}

/// Events gone from the provider are removed inside the window only.
async fn only_events_inside_the_window_are_removed(repo: &impl Repo) {
    let last_month = clock();
    last_month.advance(-TimeDelta::days(40));
    let mut old = meeting("old", 1);
    old.start -= TimeDelta::days(40);
    old.end -= TimeDelta::days(40);
    let old = snapshot(vec![old]);
    repo.apply(&batch_at(&old, CalendarProvider::Apple, &last_month))
        .await
        .unwrap();
    let both = snapshot(vec![meeting("evt-1", 1), meeting("evt-2", 2)]);
    repo.apply(&batch(&both)).await.unwrap();

    let one = snapshot(vec![meeting("evt-2", 2)]);
    let report = repo.apply(&batch(&one)).await.unwrap();

    assert_eq!(report.events_removed, 1);
    assert_eq!(
        stored_ids(repo).await,
        ["old", "evt-2"],
        "the old event is outside the window"
    );
}

/// Calendars and accounts the provider lost are removed with their events;
/// other providers' accounts are untouched.
async fn lost_calendars_and_accounts_go_but_other_providers_stay(repo: &impl Repo) {
    let google = snapshot(vec![meeting("g-1", 3)]);
    repo.apply(&batch_at(&google, CalendarProvider::Google, &clock()))
        .await
        .unwrap();
    let mut two = snapshot(vec![meeting("evt-1", 1)]);
    let mut family = two.accounts[0].calendars[0].clone();
    (family.provider_id, family.title) = ("family".into(), "Family".into());
    family.events = vec![meeting("evt-2", 2)];
    two.accounts[0].calendars.push(family);
    repo.apply(&batch(&two)).await.unwrap();
    let mut renamed = snapshot(vec![meeting("evt-1", 1)]);
    renamed.accounts[0].name = "Exchange".into();

    let report = repo.apply(&batch(&renamed)).await.unwrap();

    assert!(report.calendars_changed >= 2, "{report:?}");
    let expected = [
        (CalendarProvider::Apple, "Exchange".to_owned()),
        (CalendarProvider::Google, "iCloud".to_owned()),
    ];
    assert_eq!(
        stored_accounts(repo).await,
        expected,
        "Apple's iCloud went, Google's stayed"
    );
    assert_eq!(stored_ids(repo).await, ["evt-1", "g-1"]);
}

/// A calendar's new title and colour refresh it; its ID stays.
async fn calendar_changes_refresh_in_place(repo: &impl Repo) {
    let mut source = snapshot(vec![meeting("evt-1", 1)]);
    repo.apply(&batch(&source)).await.unwrap();
    let before = repo.calendars().await.unwrap().remove(0);
    source.accounts[0].calendars[0].title = "Work (Acme)".into();
    source.accounts[0].calendars[0].color = CalendarColor::parse("#FF2D55").ok();

    let report = repo.apply(&batch(&source)).await.unwrap();

    let after = repo.calendars().await.unwrap().remove(0);
    assert_eq!(report.calendars_changed, 1);
    assert_eq!((after.id, after.title.as_str()), (before.id, "Work (Acme)"));
    assert_eq!(after.color.unwrap().as_str(), "#FF2D55");
}

macro_rules! contract {
    ($($name:ident),+ $(,)?) => {
        mod sqlite {
            use super::*;
            $(#[tokio::test]
            async fn $name() {
                super::$name(&SqliteCalendarRepo::new(test_db().await)).await;
            })+
        }
        mod fake {
            use super::*;
            $(#[tokio::test]
            async fn $name() {
                super::$name(&FakeCalendarRepo::default()).await;
            })+
        }
    };
}

contract!(
    resyncing_unchanged_data_changes_nothing,
    an_rsvp_change_updates_in_place,
    only_events_inside_the_window_are_removed,
    lost_calendars_and_accounts_go_but_other_providers_stay,
    calendar_changes_refresh_in_place,
);

#[tokio::test]
async fn an_unchanged_resync_commits_no_change_signal() {
    let database = test_db().await;
    let repo = SqliteCalendarRepo::new(database.clone());
    let source = snapshot(vec![meeting("evt-1", 1)]);
    repo.apply(&batch(&source)).await.unwrap();
    let mut signals = database.changes().subscribe();

    repo.apply(&batch(&source)).await.unwrap();
    let mut changed = source.clone();
    changed.accounts[0].calendars[0].events[0].title = "Moved".into();
    repo.apply(&batch(&changed)).await.unwrap();

    let signal = signals.recv().await.unwrap();
    assert_eq!(signal.tables, [deino_db::Table::CalendarEvents]);
    assert!(
        signals.try_recv().is_err(),
        "only the real change was signalled"
    );
}

#[tokio::test]
async fn deleting_a_calendar_deletes_its_events() {
    let database = test_db().await;
    let repo = SqliteCalendarRepo::new(database.clone());
    repo.apply(&batch(&snapshot(vec![meeting("evt-1", 1)])))
        .await
        .unwrap();
    let calendar = repo.calendars().await.unwrap().remove(0);

    sqlx::query("DELETE FROM calendars WHERE id = ?1")
        .bind(calendar.id)
        .execute(database.writer())
        .await
        .unwrap();

    assert!(all_events(&repo).await.is_empty());
}

#[tokio::test]
async fn the_sync_window_scan_uses_the_account_start_index() {
    let repo = SqliteCalendarRepo::new(test_db().await);
    let sql = "SELECT id, provider_event_id FROM calendar_events
        WHERE account_id = ?1 AND start_at >= ?2 AND start_at < ?3";

    let plan = query_plan(&repo.database, sql).await;

    assert_eq!(
        plan,
        [
            "SEARCH calendar_events USING INDEX idx_calendar_events_account_start (account_id=? AND start_at>? AND start_at<?)"
        ]
    );
}
