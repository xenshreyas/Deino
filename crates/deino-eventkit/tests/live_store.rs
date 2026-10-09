//! Reads this Mac's real calendars (manual QA, never CI):
//!
//!   cargo test -p deino-eventkit --test live_store -- --ignored --nocapture
//!
//! It never asks for access: grant it first (System Settings → Privacy &
//! Security → Calendars, for the app running the test). It prints counts,
//! never titles, notes or attendees.

#![cfg(target_os = "macos")]

use deino_core::calendar::{CalendarAccess, CalendarSource, SyncWindow};
use deino_domain::{Clock, SystemClock};
use deino_eventkit::EventKitSource;

#[tokio::test]
#[ignore = "reads this Mac's real calendars; run by hand with Calendars access granted"]
async fn reads_this_macs_accounts_calendars_and_events() {
    let source = EventKitSource::new();
    let access = source.access().await;
    println!("Calendars access: {access:?}");
    if access != CalendarAccess::Granted {
        return;
    }

    let snapshot = source
        .fetch(SyncWindow::around(SystemClock.now()))
        .await
        .unwrap();

    for account in &snapshot.accounts {
        let events: usize = account.calendars.iter().map(|c| c.events.len()).sum();
        let recurring = account
            .calendars
            .iter()
            .flat_map(|c| &c.events)
            .filter(|e| e.series_key.is_some())
            .count();
        println!(
            "account: {} calendars, {events} events ({recurring} recurring occurrences)",
            account.calendars.len()
        );
    }
    assert!(!snapshot.accounts.is_empty(), "a Mac always has a calendar account");
}
