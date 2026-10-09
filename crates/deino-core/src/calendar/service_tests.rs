//! The calendar service: the sync switch and macOS access, what "Coming up"
//! lists, hidden calendars, failures in the status, and the schedule.

use std::sync::Arc;
use std::time::Duration;

use chrono::TimeDelta;
use deino_domain::{CalendarProvider, Clock, FakeClock};

use super::test_events::{clock, hours, meeting, snapshot};
use super::*;
use crate::companies::FakeCompanyRepo;
use crate::people::FakePeopleRepo;
use crate::settings::{FakeSettingsStore, RecordedSettingsEvents, SettingsService};

struct Harness {
    service: Arc<CalendarService>,
    source: Arc<FakeCalendarSource>,
    events: Arc<RecordedCalendarEvents>,
    clock: Arc<FakeClock>,
}

async fn harness() -> Harness {
    let clock = Arc::new(clock());
    let source = Arc::new(FakeCalendarSource::new(CalendarProvider::Apple));
    source.set_snapshot(snapshot(vec![meeting("evt-1", 1), meeting("evt-2", 26)]));
    let events = Arc::new(RecordedCalendarEvents::default());
    let settings = SettingsService::load(
        Arc::new(FakeSettingsStore::default()),
        Arc::new(RecordedSettingsEvents::default()),
    )
    .await
    .unwrap();
    let repo = Arc::new(FakeCalendarRepo::default());
    let service = CalendarService::new(CalendarDeps {
        source: source.clone(),
        store: CalendarStore::over(&repo),
        directory: AttendeeDirectory::new(
            Arc::new(FakePeopleRepo::default()),
            Arc::new(FakeCompanyRepo::default()),
            clock.clone(),
        ),
        settings: Arc::new(settings),
        events: events.clone(),
        clock: clock.clone(),
    });
    Harness {
        service: Arc::new(service),
        source,
        events,
        clock,
    }
}

/// Today and tomorrow.
fn two_days() -> TimeRange {
    TimeRange {
        start: hours(0),
        end: hours(48),
    }
}

async fn upcoming_ids(service: &CalendarService) -> Vec<String> {
    let events = service.upcoming(two_days()).await.unwrap();
    events
        .into_iter()
        .map(|event| event.provider_event_id)
        .collect()
}

#[tokio::test]
async fn enabling_asks_for_access_turns_sync_on_syncs_and_announces() {
    let Harness {
        service,
        source,
        events,
        clock,
    } = harness().await;

    let status = service.enable().await.unwrap();

    assert!(status.sync_enabled);
    assert_eq!(status.access, CalendarAccess::Granted);
    assert_eq!(status.last_synced_at, Some(clock.now()));
    assert_eq!(source.fetch_count(), 1);
    assert_eq!(upcoming_ids(&service).await, ["evt-1", "evt-2"]);
    assert_eq!(events.statuses().last(), Some(&status));
}

#[tokio::test]
async fn denied_access_leaves_sync_off_and_reads_nothing() {
    let Harness {
        service, source, ..
    } = harness().await;
    source.answer_prompt_with(CalendarAccess::Denied);

    let status = service.enable().await.unwrap();

    assert!(!status.sync_enabled);
    assert_eq!(status.access, CalendarAccess::Denied);
    assert_eq!(source.fetch_count(), 0);
    assert_eq!(service.sync_if_enabled().await, None);
}

#[tokio::test]
async fn disabling_empties_coming_up_but_keeps_the_events_meetings_link_to() {
    let Harness { service, .. } = harness().await;
    service.enable().await.unwrap();
    let stored = service.upcoming(two_days()).await.unwrap().remove(0);

    let status = service.disable().await.unwrap();

    assert!(!status.sync_enabled);
    assert!(upcoming_ids(&service).await.is_empty());
    assert_eq!(service.event(stored.id).await.unwrap().id, stored.id);
}

#[tokio::test]
async fn hiding_a_calendar_hides_its_events_until_it_is_shown_again() {
    let Harness {
        service, events, ..
    } = harness().await;
    service.enable().await.unwrap();
    let work = service.calendars().await.unwrap().remove(0);
    let announced = events.statuses().len();

    let hidden = service.set_calendar_visible(work.id, false).await.unwrap();

    assert!(!hidden.visible);
    assert!(upcoming_ids(&service).await.is_empty());
    assert!(!service.calendars().await.unwrap()[0].visible);
    assert_eq!(events.statuses().len(), announced + 1);
    service.set_calendar_visible(work.id, true).await.unwrap();
    assert_eq!(upcoming_ids(&service).await.len(), 2);
}

#[tokio::test]
async fn calendars_list_with_their_account_and_colour() {
    let Harness { service, .. } = harness().await;
    service.enable().await.unwrap();

    let calendars = service.calendars().await.unwrap();

    assert_eq!(calendars.len(), 1);
    let work = &calendars[0];
    assert_eq!(
        (work.title.as_str(), work.account_name.as_str()),
        ("Work", "iCloud")
    );
    assert_eq!(work.provider, CalendarProvider::Apple);
    assert_eq!(
        work.color.as_ref().map(CalendarColor::as_str),
        Some("#34AADC")
    );
}

#[tokio::test]
async fn a_failed_sync_shows_in_the_status_until_the_next_success() {
    let Harness {
        service,
        source,
        clock,
        ..
    } = harness().await;
    service.enable().await.unwrap();
    source.set_failing(true);
    clock.advance(TimeDelta::minutes(2));

    let failed = service.refresh().await;
    source.set_failing(false);
    let recovered = service.refresh().await;

    assert_eq!(
        failed.last_error.as_deref(),
        Some("could not read your calendars")
    );
    assert_eq!(failed.last_synced_at, Some(hours(0)));
    assert_eq!(recovered.last_error, None);
    assert_eq!(recovered.last_synced_at, Some(clock.now()));
}

#[tokio::test]
async fn revoked_access_is_reported_and_nothing_stored_is_lost() {
    let Harness {
        service, source, ..
    } = harness().await;
    service.enable().await.unwrap();
    source.set_access(CalendarAccess::Denied);

    let status = service.refresh().await;

    assert_eq!(status.access, CalendarAccess::Denied);
    assert!(status.last_error.is_some());
    assert_eq!(source.fetch_count(), 1, "no read without access");
    assert_eq!(upcoming_ids(&service).await.len(), 2);
}

#[tokio::test]
async fn upcoming_rejects_backwards_and_overlong_ranges() {
    let Harness { service, .. } = harness().await;
    let backwards = TimeRange {
        start: hours(1),
        end: hours(0),
    };
    let overlong = TimeRange {
        start: hours(0),
        end: hours(0) + TimeDelta::days(MAX_RANGE_DAYS + 1),
    };

    for range in [backwards, overlong] {
        let error = service.upcoming(range).await.unwrap_err();
        assert!(matches!(error, CalendarError::InvalidRange));
    }
}

#[tokio::test]
async fn an_unknown_event_or_calendar_is_not_found() {
    let Harness { service, clock, .. } = harness().await;

    let event = service
        .event(deino_domain::CalendarEventId::generate(clock.as_ref()))
        .await;
    let calendar = service
        .set_calendar_visible(deino_domain::CalendarId::generate(clock.as_ref()), false)
        .await;

    assert!(matches!(event, Err(CalendarError::EventNotFound(_))));
    assert!(matches!(calendar, Err(CalendarError::CalendarNotFound(_))));
}

#[tokio::test]
async fn events_near_now_are_served_from_memory() {
    let Harness { service, .. } = harness().await;
    service.enable().await.unwrap();

    let near: Vec<String> = service
        .events_near(hours(0))
        .into_iter()
        .map(|event| event.provider_event_id)
        .collect();

    assert_eq!(near, ["evt-1"], "evt-2 starts tomorrow");
    service.disable().await.unwrap();
    assert!(service.events_near(hours(0)).is_empty());
}

#[tokio::test(start_paused = true)]
async fn the_scheduler_syncs_at_start_every_interval_and_soon_after_a_nudge() {
    let Harness {
        service, source, ..
    } = harness().await;
    service.enable().await.unwrap();
    let (scheduler, triggers, cancel) = SyncScheduler::new(service.clone());

    let script = async {
        settle().await;
        assert_eq!(
            source.fetch_count(),
            2,
            "enable, then the scheduler's first sync"
        );
        tokio::time::sleep(REFRESH_INTERVAL).await;
        settle().await;
        assert_eq!(source.fetch_count(), 3);
        for _ in 0..5 {
            triggers.send(SyncTrigger::SourceChanged);
        }
        tokio::time::sleep(SETTLE_DELAY).await;
        settle().await;
        assert_eq!(source.fetch_count(), 4, "a burst of nudges is one sync");
        cancel.cancel();
    };
    tokio::join!(scheduler.run(), script);
}

/// Lets the scheduler task run until it waits again.
async fn settle() {
    for _ in 0..10 {
        tokio::task::yield_now().await;
    }
    tokio::time::sleep(Duration::from_millis(1)).await;
}
