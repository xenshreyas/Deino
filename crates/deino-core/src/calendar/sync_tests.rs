//! One sync of one source: the window it reads, what it stores, and what it
//! never does with an empty or failed read.

use std::sync::Arc;

use deino_domain::{CalendarProvider, Email};
use deino_testkit::at;

use super::test_events::{NOW, clock, meeting, snapshot};
use super::*;
use crate::companies::FakeCompanyRepo;
use crate::people::{FakePeopleRepo, PersonRepo};

struct Harness {
    sync: CalendarSync,
    source: FakeCalendarSource,
    repo: Arc<FakeCalendarRepo>,
    people: Arc<FakePeopleRepo>,
}

fn harness() -> Harness {
    let repo = Arc::new(FakeCalendarRepo::default());
    let people = Arc::new(FakePeopleRepo::default());
    let clock = Arc::new(clock());
    let directory = AttendeeDirectory::over(
        people.clone(),
        Arc::new(FakeCompanyRepo::default()),
        clock.clone(),
    );
    Harness {
        sync: CalendarSync::new(repo.clone(), directory, clock),
        source: FakeCalendarSource::new(CalendarProvider::Apple),
        repo,
        people,
    }
}

#[tokio::test]
async fn a_sync_reads_the_window_around_now_and_stores_events_and_people() {
    let Harness {
        sync,
        source,
        repo,
        people,
    } = harness();
    source.set_snapshot(snapshot(vec![meeting("evt-1", 1)]));

    let report = sync.run(&source, &[]).await.unwrap();

    assert_eq!(source.windows(), [SyncWindow::around(at(NOW))]);
    assert_eq!(report.events_added, 1);
    assert_eq!(repo.calendars().await.unwrap().len(), 1);
    let marcus = Email::parse("marcus@brightkite.com").unwrap();
    assert!(people.find_by_email(&marcus).await.unwrap().is_some());
}

#[tokio::test]
async fn a_store_that_reports_no_accounts_deletes_nothing() {
    let Harness {
        sync, source, repo, ..
    } = harness();
    source.set_snapshot(snapshot(vec![meeting("evt-1", 1)]));
    sync.run(&source, &[]).await.unwrap();
    source.set_snapshot(SourceSnapshot::default());

    let report = sync.run(&source, &[]).await.unwrap();

    assert!(!report.changed());
    assert_eq!(repo.accounts().await.unwrap().len(), 1);
}

#[tokio::test]
async fn a_failed_read_stores_nothing_and_says_why() {
    let Harness {
        sync, source, repo, ..
    } = harness();
    source.set_failing(true);

    let error = sync.run(&source, &[]).await.unwrap_err();

    assert!(matches!(
        error,
        CalendarError::Source(SourceError::Failed(_))
    ));
    assert_eq!(error.to_string(), "could not read your calendars");
    assert!(repo.accounts().await.unwrap().is_empty());
}
