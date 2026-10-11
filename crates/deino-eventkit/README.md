# deino-eventkit

## Role

macOS Calendar through EventKit: Deino's default calendar source (ORG-01,
[ADR 0027](../../docs/adr/0027-calendar-sync-through-eventkit.md)). Every
account the user added to the Mac (iCloud, Google, Exchange, CalDAV,
subscriptions) comes through one Calendars permission, with no OAuth and no
network.

## Owns

- `EventKitSource`: `deino_core::calendar::CalendarSource` for
  `CalendarProvider::Apple` (access state, the macOS 14 full-access prompt,
  a snapshot of every account, calendar and event in a window), and `watch`,
  which nudges the sync scheduler on `EKEventStoreChangedNotification` and
  `NSWorkspaceDidWakeNotification`.
- `worker.rs`: the one thread that owns the `EKEventStore`; nothing touches
  EventKit on the main thread or on async workers.
- `convert.rs`: every raw EventKit code (authorization, status,
  availability, participant answer and type) and occurrence IDs, pure and
  tested on every platform.

## Does Not Own

- What an event means (its join link, who is external, what is hidden, its
  series): `deino_core::calendar` (`mapping.rs`, `filter.rs`), the same for
  every provider.
- Storage, scheduling and the UI: `deino-core`, the app's `calendar` plugin.

## Invariants

- No Tauri, no network client, no database.
- EventKit objects never leave the worker thread; replies are plain values.
- Off macOS the source reports `CalendarAccess::Unsupported` and every read
  fails with `SourceError::Unsupported`.
- Logs carry counts and errors, never titles, notes or attendees.
