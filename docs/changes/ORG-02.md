# ORG-02 · Home, Coming up, notifications, auto-record rules

## Progress log (PARITY_PLAN §7.1)
ORG-02 second half (branch `feat/ORG-02-home-b`; the first half, the rule
engine, Join, reminder bookkeeping and Home, is on main).
- **Meeting ticker** (`deino-core::calendar::ticker`): `MeetingTicker`
  every 15 s over `CalendarService::events_near`, with the pure
  `TickState::plan` (`TickInput` → `TickPlan`). It shows the reminder window
  (`ReminderWindow` port) for the soonest meeting due by `ReminderClock`,
  closes it after `REMINDER_OPEN_FOR` (60 s), skips (never postpones) it
  while Deino's Do Not Disturb (`notifications.manualDndMode`) is on, and
  starts recording an event whose note is open (`OpenNote`, reported by the
  UI) within `START_GRACE` (1 min) of its start when
  `AutoRecordService::starts_on(id, StartTrigger::StartTime { note_open: true })`
  and the recorder is idle (`RecorderBusy`), through `JoinService` (Record
  only). Spawned in `composition/calendar.rs::attach_ticker` on a child of the
  sync scheduler's token, so `CalendarSchedule::stop` ends it.
- **Reminder window**: `features/calendar/reminder.rs` (`ReminderCard` keeps
  the event, `TauriReminderWindow` is a non-activating `NSPanel` at status
  level, CAP-06's pattern; a plain unfocused window elsewhere), page
  `meeting-reminder.html` (`MeetingReminder`, `ReminderCard`): title, time,
  people, **Join & record** (with a call link), **Record only**, dismiss;
  closes itself after 60 s or once a button's Join succeeded. Capability
  `capabilities/meeting-reminder.json` (window `meeting_reminder`,
  `calendar:default`). Event `ReminderShown { eventId }`.
- **Auto-start on the mic** (CAP-06): `features/detection/auto_record.rs`.
  When a detected call's linked event `starts_on(MicInUse)`, the sink skips
  the prompt and calls the new `DetectionService::record_now` (closes the
  prompt and records without the countdown or opening the notes).
- **Menu bar**: `tray/menu.rs` Join/Record call `JoinService::join`
  (`JoinOnly`/`RecordOnly`); its own `join()` helper is gone.
- **Coming up menu**: "Review last meeting" (`LastMeetingService` in
  `calendar::linking`: ORG-03's `RelatedService::related_to` on a key built
  from the event, else the latest earlier meeting with exactly the same
  people, the user left out; new `RelatedBy::People`), "Open in Calendar"
  (`calendar::open_link`: `ical://ekevent/<eventIdentifier>?method=show`
  for macOS Calendar events, the Google or Outlook web day view otherwise;
  built in Rust from the stored event, the `ical` scheme only through this
  command, never `system_open`'s webview path). A right-click on a row opens
  the same menu.
- **Settings → Calendar**: "Auto-record" (the four `calendar.autoRecord`
  switches) and "Meeting reminders" (`calendar.reminders.enabled`, one switch
  per visible calendar writing `quietCalendars`); hidden in onboarding.
- **Event note**: the meeting page reports its linked event
  (`OpenNoteReporter` in the header → `calendar|set_open_note`); NOT-08's
  `MeetingBrief` (above the notepad) and ORG-04's attendees (header) already
  show on it; `HOME_SLOTS.Brief` is filled with `EventBrief`.
- Commands: `reminder`, `dismiss_reminder`, `set_open_note`, `last_meeting`,
  `open_in_calendar`; `AutoRecordService::starts_on`.
- Tests: tick plan table tests, ticker over a synced calendar, `starts_on`,
  `LastMeetingService` over SQLite, `calendar_link`, `record_now`; contract
  tests for `record_plan`, `choose_record`, `join`, `reminder`,
  `dismiss_reminder`, `set_open_note`, `last_meeting`
  (`features/calendar/rules_contract_tests.rs`); Home component test
  (`coming-up-section.test.tsx`: Join, right-click menu, Review last meeting,
  Open in Calendar), `reminder-card.test.tsx`; Playwright
  `e2e/reminders.spec.ts` (reminder page, row menu, settings).

## Follow-ups (PARITY_PLAN §7.2)
- Owner QA: real-Mac Join & record on Zoom and Meet (capture ≤ 2 s); the
  reminder panel's placement and focus behaviour; `ical://ekevent/` opening
  the event in Calendar.
- CAP-10 / NOT-01: recording an event whose scheduled note exists (Home's
  Join, the reminder, the start-time trigger) starts a new meeting linked to
  the event instead of recording into the note. Record into the scheduled
  meeting (a `StartRequest` that reuses a meeting) and drop the duplicate.
- ORG-02 / FND: Deino cannot read macOS Focus (the legacy `is_dnd_active`
  also returns `false`), so `notifications.respectDoNotDisturb` has no effect
  yet; reminders honour Deino's own Do Not Disturb switch only.
- CAP-06: the reminder panel duplicates the detection prompt's panel code
  (`reminder_panel.rs`/`reminder_plain.rs`); extract one floating-panel
  helper for both.
- NOT-08: put the brief's first point in the reminder card
  (`BriefService::for_event`; the window would need `briefs:default`).
- Home: "Open note" creates a new scheduled note each time; reuse the
  event's existing note.

## Architecture (ARCHITECTURE.md)
- Calendar (ORG-02): add `calendar::ticker` (`MeetingTicker`, `TickState`,
  ports `ReminderWindow`, `RecorderBusy`, state `OpenNote`) beside
  `SyncScheduler`; `calendar::open_link` ("Open in Calendar");
  `linking::LastMeetingService`. The app's fourth window: the meeting
  reminder (`meeting_reminder`, `meeting-reminder.html`), a non-activating
  panel like the detection prompt.
- Detection: a call whose event auto-records skips the prompt
  (`DetectionService::record_now`).

## CLAUDE.md
- App windows: `main`, `detection_prompt`, `record_panel`, and
  `meeting_reminder` (ORG-02); each non-main window has its own capability
  file and Vite entry.

## Plan updates
- ORG-02 accept: "The rule matrix is unit tested" ☑ (first half); "Join &
  record starts capture within 2 s" ☑ by the fake-recorder test, real-Mac QA
  pending.
- §7.2 rows closed: ORG-02 second half (reminder window, settings,
  auto-start wiring), ORG-02 tray through `JoinService`, ORG-02 Coming up
  menu and brief slot (real-Mac QA stays open), NOT-08 "drop `EventBrief`
  into the event note's brief slot".
