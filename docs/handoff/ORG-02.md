# ORG-02 handoff: second half

Branch `feat/ORG-02-home`. The first half shipped: the rule engine, Join,
reminder bookkeeping and Home. This is what is left, in order.

## Shipped (for orientation)

- `crates/deino-core/src/calendar/rules.rs` + `rules/`: `decide` (pure),
  `starts_recording`, `SeriesChoiceRepo` (`calendar_series_choices`),
  `AutoRecordService` (`plan`, `decision`, `choose`). Table tests in
  `rules/policy_tests.rs`, service tests on the real schema.
- `calendar/notify.rs`: `ReminderClock::due(events, now, settings)` and
  `wants_reminder`. `calendar/join.rs`: `JoinService` with the
  `EventRecorder`, `CallOpener`, `EventLookup` ports; the 2 s capture test
  is `join/join_tests.rs` (paused tokio time, 5 s browser, 0.3 s recorder).
- Settings: `calendar.autoRecord` (`AutoRecordSettings`, all audiences off
  by default) and `calendar.reminders` (`enabled`, `quietCalendars`).
- App: `record_plan`, `choose_record`, `join` (`features/calendar/rules_commands.rs`),
  adapters in `join_adapters.rs`, wiring in `composition/calendar.rs::attach_rules`.
- UI: `features/home/` (Home while idle), `features/calendar/plan.ts`,
  mock in `dev/mock-ipc/calendar-rules.ts`.

## 1. Rebase on CAP-10 (do first)

- `LiveRecorder::record` calls `RecordingService::start(title)` today and
  relies on `LinkOnStart`. After CAP-10, build
  `StartRequest { title, link: Some(MeetingLink { calendar_event_id, series_id }) }`.
- Make the tray's `meeting_action` (`tray/menu.rs`, "join"/"record") call
  `Arc<JoinService>::join` with `JoinOnly`/`RecordOnly`, deleting its own
  `join()` helper, so the menu bar, Home and the reminder share one path.

## 2. Reminder window

- A ticker (owned task like `SyncScheduler`, cancelled at exit) every 15 s:
  `ReminderClock::due(&calendar.events_near(now), now, &settings.calendar.reminders)`.
- For each due event open a small always-on-top, non-activating webview
  (reuse CAP-06's detection prompt window pattern in `features/detection`)
  at a route like `/reminder?event=<id>` showing title, time, people and
  **Join & record** / **Record only** / dismiss; buttons call
  `calendar|join` with `joinAndRecord` / `recordOnly`. Close after 60 s or
  when the event starts. Respect `notifications.respectDoNotDisturb`.
- Settings → Calendar: a "Remind me a minute before meetings" switch and a
  per-calendar bell toggle writing `calendar.reminders.quietCalendars`.

## 3. Auto-record switches and auto-start

- Settings → Calendar → "Auto-record": four switches over
  `calendar.autoRecord` (internal, external, organised by others, not accepted).
- Mic trigger: where CAP-06 decides to prompt/auto-start
  (`features/detection/recorder.rs`), look up the linked event
  (`SyncedCalendar`), get `AutoRecordService::decision`, and if
  `starts_recording(decision, StartTrigger::MicInUse)` start without the
  countdown.
- Start-time trigger: the ticker above, for an event whose scheduled note
  is the open page (the UI reports the open meeting/event), uses
  `StartTrigger::StartTime { note_open: true }`.

## 4. Event note and menu

- Mount `HOME_SLOTS.Brief` (NOT-08) above the notepad of a `scheduled`
  meeting linked to an event, plus its attendees (via
  `features/meeting/slots.ts`).
- Coming up menu: "Open in Calendar" (needs a provider web link or
  allowing `ical://` in `system_open`) and "Review last meeting" (add an
  event-keyed `RelatedService` query by series, then attendees).
- Right-click on a row should open the same menu as ⋯.

## 5. Tests and QA still owed

- Contract tests for `record_plan`, `choose_record`, `join` (JSON shape),
  a Home component test, and a Playwright smoke (`e2e/`) clicking Join.
- Real-Mac QA (§7.2): Join & record on Zoom and Meet, capture ≤ 2 s.
