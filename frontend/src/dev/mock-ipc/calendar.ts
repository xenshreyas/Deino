/**
 * Mock of `@/lib/ipc/generated/calendar`: macOS Calendar sync already on,
 * with the fixture meetings' events (`calendar-fixtures.ts`). Switching sync,
 * refreshing and hiding the calendar change state in memory and announce it
 * with `CalendarChanged`, like the Rust service; `upcoming` applies the same
 * rules (overlap, no declined events, no hidden calendars). Google and
 * Microsoft connect once their client ID is in the mock settings, as if the
 * browser sign-in succeeded; they add no events. A meeting's event link,
 * the picker's suggestions and related meetings (ORG-03) come from
 * `mockEventLinks`.
 */
import type * as Generated from '@/lib/ipc/generated/calendar';
import type { CommandResult } from '@/lib/ipc/result';

import { FIXTURE_ACCOUNT, FIXTURE_CALENDAR } from './calendar-fixtures';
import { failure, ok } from './command-result';
import { mockCalendarEvents as calendarEvents, mockDatabase, mockEventLinks } from './database';
import { rulesCommands } from './calendar-rules';
import { latency } from './latency';
import { mockEvent } from './mock-event';
import { currentSettings } from './settings-store';

type CalendarId = Generated.CalendarId;
type CalendarStatus = Generated.CalendarStatus;

/** `deino_core::calendar::MAX_RANGE_DAYS`. */
const MAX_RANGE_DAYS = 62;
const DAY_MS = 24 * 60 * 60 * 1000;

/** The providers with a direct connector. */
type DirectProvider = Exclude<Generated.CalendarProvider, 'apple'>;
const DIRECT_PROVIDERS: readonly DirectProvider[] = ['google', 'microsoft'];

const hidden = new Set<CalendarId>();
/** When each connected direct connector last synced. */
const connected = new Map<DirectProvider, string>();
let status: Omit<CalendarStatus, 'connectors'> = {
  syncEnabled: true,
  access: 'granted',
  lastSyncedAt: new Date().toISOString(),
  lastError: null,
};

export const events = {
  calendarChanged: mockEvent<Generated.CalendarChanged>('plugin:calendar:calendar-changed'),
} satisfies typeof Generated.events;

/** A direct connector: set up once the mock settings hold its client ID. */
function connectorStatus(provider: DirectProvider): Generated.ConnectorStatus {
  const syncedAt = connected.get(provider) ?? null;
  return {
    provider,
    configured: currentSettings().calendar[provider].clientId !== '',
    connected: syncedAt !== null,
    lastSyncedAt: syncedAt,
    lastError: null,
  };
}

/** The whole status, as `CalendarService::status` builds it. */
function snapshot(): CalendarStatus {
  return { ...status, connectors: DIRECT_PROVIDERS.map(connectorStatus) };
}

/** Changes the status and announces it, like `CalendarService::announce`. */
function announce(next: Partial<CalendarStatus>): CalendarStatus {
  status = { ...status, ...next };
  void events.calendarChanged.emit({ status: snapshot() });
  return snapshot();
}

/** Connects or disconnects `provider`, as if the browser sign-in succeeded. */
async function setConnected(
  provider: Generated.CalendarProvider,
  on: boolean,
): Promise<CommandResult<CalendarStatus>> {
  await latency();
  if (provider === 'apple') {
    return failure({ code: 'invalid', message: 'there is no apple calendar connector' });
  }
  if (on && !connectorStatus(provider).configured) {
    return failure({ code: 'invalid', message: `the ${provider} connector is not set up` });
  }
  if (on) connected.set(provider, new Date().toISOString());
  else connected.delete(provider);
  return ok(announce({}));
}

/** The fixture calendar as Settings lists it. */
function view(): Generated.CalendarView {
  return {
    id: FIXTURE_CALENDAR.id,
    title: FIXTURE_CALENDAR.title,
    color: FIXTURE_CALENDAR.color,
    accountId: FIXTURE_ACCOUNT.id,
    accountName: FIXTURE_ACCOUNT.name,
    provider: 'apple',
    visible: !hidden.has(FIXTURE_CALENDAR.id),
  };
}

/** The `upcoming` rules: overlapping, not declined, not hidden, by start. */
function agenda(range: Generated.TimeRange): Generated.CalendarEvent[] {
  const [start, end] = [Date.parse(range.start), Date.parse(range.end)];
  return calendarEvents
    .filter((event) => Date.parse(event.startAt) < end && Date.parse(event.endAt) > start)
    .filter((event) => event.myRsvp !== 'declined')
    .filter((event) => event.calendarId === null || !hidden.has(event.calendarId))
    .sort((a, b) => a.startAt.localeCompare(b.startAt))
    .map((event) => structuredClone(event));
}

const baseCommands = {
  async status() {
    await latency();
    return ok(snapshot());
  },
  async enable() {
    await latency();
    return ok(
      announce({ syncEnabled: true, access: 'granted', lastSyncedAt: new Date().toISOString() }),
    );
  },
  async disable() {
    await latency();
    return ok(announce({ syncEnabled: false }));
  },
  async refresh() {
    await latency();
    if (!status.syncEnabled) return ok(snapshot());
    return ok(announce({ lastSyncedAt: new Date().toISOString(), lastError: null }));
  },
  async openAccessSettings() {
    await latency();
    return ok(true);
  },
  async listCalendars() {
    await latency();
    return ok([view()]);
  },
  async setCalendarVisible(id, visible) {
    await latency();
    if (id !== FIXTURE_CALENDAR.id) {
      return failure({ code: 'not_found', message: `calendar ${id} was not found` });
    }
    if (visible) hidden.delete(id);
    else hidden.add(id);
    announce({});
    return ok(view());
  },
  async upcoming(range) {
    await latency();
    const span = Date.parse(range.end) - Date.parse(range.start);
    if (!(span > 0 && span <= MAX_RANGE_DAYS * DAY_MS)) {
      return failure({
        code: 'invalid',
        message: `a time range must end after it starts and span at most ${MAX_RANGE_DAYS} days`,
      });
    }
    return ok(status.syncEnabled ? agenda(range) : []);
  },
  async event(id) {
    await latency();
    const event = calendarEvents.find((candidate) => candidate.id === id);
    return event === undefined
      ? failure({ code: 'not_found', message: `calendar event ${id} was not found` })
      : ok(structuredClone(event));
  },
  connect: (provider) => setConnected(provider, true),
  disconnect: (provider) => setConnected(provider, false),
  async meetingEvent(meetingId) {
    await latency();
    const found = mockDatabase.meeting(meetingId);
    return found.status === 'error' ? failure(found.error) : ok(mockEventLinks.linked(meetingId));
  },
  async eventSuggestions(meetingId) {
    await latency();
    const found = mockDatabase.meeting(meetingId);
    if (found.status === 'error') return failure(found.error);
    return ok(status.syncEnabled ? mockEventLinks.suggestions(meetingId) : []);
  },
  async linkEvent(meetingId, eventId) {
    await latency();
    return mockEventLinks.apply(meetingId, eventId, new Date());
  },
  async unlinkEvent(meetingId) {
    await latency();
    return mockEventLinks.apply(meetingId, null, new Date());
  },
  async relatedMeetings(meetingId) {
    await latency();
    const found = mockDatabase.meeting(meetingId);
    return found.status === 'error' ? failure(found.error) : ok(mockEventLinks.related(meetingId));
  },
} satisfies Omit<typeof Generated.commands, 'recordPlan' | 'chooseRecord' | 'join'>;

export const commands = {
  ...baseCommands,
  ...rulesCommands((range) => baseCommands.upcoming(range)),
} satisfies typeof Generated.commands;
