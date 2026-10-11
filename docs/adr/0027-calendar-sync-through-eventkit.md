# 0027. Sync calendars through EventKit, with optional Google and Microsoft connectors

- **Status:** Accepted
- **Date:** 2026-10-09
- **Card:** ORG-01 (`docs/references/granola.md` §1, §6.4;
  `docs/references/circleback.md` §3.1)

## Context

Granola and Circleback start from the calendar: Home lists what is coming
up, a meeting takes its title and attendees from its event, and detection
links a call to the event it belongs to (CAP-06). Both require a Google or
Microsoft sign-in. Deino has no server and sends nothing to its own
infrastructure (§8 D6), so every connection must run on the Mac with the
user's own app registration, which Google and Microsoft only grant to the
owner (§9). Most Macs already hold the user's calendars: macOS Calendar
syncs iCloud, Google, Exchange and CalDAV accounts and exposes them through
EventKit behind one privacy permission.

FND-07 left `calendar_accounts` and `calendar_events` without calendars, an
all-day flag or a source, and every outbound request must go through
`deino-net`'s scoped clients (ADR 0019).

## Decision

- **EventKit is the default source.** `deino-eventkit` reads every account
  the Mac knows through `objc2-event-kit` (`=0.3.2`, Zlib/Apache-2.0/MIT,
  the objc2 release already in the tree), with no OAuth and no network.
  Calendars access is requested only when the user turns sync on (Settings
  explains the prompt first, and links to System Settings when access is
  off). One worker thread owns the `EKEventStore`; nothing touches EventKit
  on the main thread.
- **A `CalendarSource` port in `deino-core`** (`provider`, `access`,
  `request_access`, `fetch(window)` returning a `SourceSnapshot` of plain
  accounts, calendars and events, `disconnect`). Every rule lives in the
  core, the same for every provider: the conference-link parser (19
  services, Safe Links / Google / Proofpoint redirects unwrapped), the
  filters (declined, out of office, Focus, working location, duplicates by
  iCal UID), internal vs external through NOT-05's `InternalDomains`, the
  user's RSVP, the series key, and people and companies from attendees
  (public mail domains and invitations over 25 people skipped).
- **A pure sync over a 30-days-back, 14-days-ahead window.**
  `CalendarSyncRepo::apply` upserts accounts, calendars and events in one
  transaction and removes, for that provider only, what the snapshot no
  longer has. Upserts write only when a field changed
  (`ON CONFLICT DO UPDATE ... WHERE ... IS NOT ...`), so an unchanged sync
  emits no `DbChanged`. `record_override` is the user's and is never
  written by a sync.
- **A `calendars` table** (per account, `#RRGGBB` colour) and
  `calendar_events.calendar_id` / `is_all_day`, in migration
  `20261011090000_org01_calendars.sql`. Hidden calendars are a setting
  (`Settings::calendar.hidden_calendars`): they keep syncing, they are only
  not shown.
- **Refresh** on `EKEventStoreChangedNotification`, on wake
  (`NSWorkspaceDidWakeNotification`), every 2 minutes, and after a 1 s
  settle so a burst of changes syncs once (`SyncScheduler`).
- **Optional Google Calendar and Microsoft Graph connectors** in a new crate,
  `deino-connect`: OAuth 2.0 for native apps (RFC 8252) with PKCE S256
  (RFC 7636) over a loopback redirect on `127.0.0.1`, in the user's default
  browser. The client ID is a setting; Google's desktop client secret and
  each refresh token live in the Keychain; access tokens stay in memory.
  Requests go through `Net::scoped(GoogleCalendar | MicrosoftGraph)`, and
  connecting switches that service on in the network settings (disconnecting
  switches it off). Read-only scopes only. `deino_connect::oauth` is
  provider-neutral so INT-00 can adopt it for Gmail, Drive and the other
  integrations.
- **IPC:** the `calendar` plugin (`status`, `enable`, `disable`, `refresh`,
  `open_access_settings`, `list_calendars`, `set_calendar_visible`,
  `upcoming(range)`, `event(id)`, `connect(provider)`,
  `disconnect(provider)`) and the typed `CalendarChanged { status }` event.
  CAP-06's `CalendarLookup` is backed by `CalendarService::events_near`, an
  in-memory list refreshed by every sync, so the detector thread never waits
  on the database.

## Alternatives

- **Google and Microsoft APIs only** (Granola, Circleback). Every user would
  need their own app registration before seeing a single event, and iCloud
  and CalDAV calendars would be missing. Kept as the optional path for an
  account that is not on the Mac.
- **Putting the connectors in `deino-core`.** The core would depend on
  `deino-net`, and through it every crate that depends on the core
  (`deino-diarize`) would reach the network; `deino-connect` keeps the core
  network-free.
- **Reading the Calendar database files directly** (`~/Library/Calendars`).
  Undocumented, breaks across macOS releases, and bypasses the privacy
  permission.
- **Storing the whole token set in the Keychain.** `deino-secrets` caps a
  secret at 4 096 characters, which Microsoft access tokens exceed; only the
  long-lived refresh token needs to survive a restart.

## Consequences

- Calendar sync works on first use with one macOS prompt and no setup; the
  connectors need the owner's app registrations (`docs/setup/google-calendar.md`,
  `docs/setup/microsoft-graph.md`).
- A new provider is an adapter of `CalendarSource` (plus, for a web API, a
  `CalendarApi` in `deino-connect` and a `ServiceId`); the rules, storage and
  UI do not change.
- One account per connector for now; each connector removes only its own
  provider's rows, so the sources never prune each other.
- Off macOS, EventKit reports `Unsupported`; the connectors still work.
- Revisit if Apple deprecates EventKit's full-access API, or if users need
  several Google or Microsoft accounts at once.
