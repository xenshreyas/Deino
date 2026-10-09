/**
 * Mock of `@/lib/ipc/generated/calendar`: macOS Calendar sync already on,
 * with the fixture meetings' events (`calendar-fixtures.ts`). Switching sync,
 * refreshing and hiding the calendar change state in memory and announce it
 * with `CalendarChanged`, like the Rust service; `upcoming` applies the same
 * rules (overlap, no declined events, no hidden calendars).
 */
import type * as Generated from '@/lib/ipc/generated/calendar';

import { fixtureCalendarEvents, FIXTURE_ACCOUNT, FIXTURE_CALENDAR } from './calendar-fixtures';
import { failure, ok } from './command-result';
import { loadFixtureMeetings } from './fixtures';
import { latency } from './latency';
import { mockEvent } from './mock-event';

type CalendarId = Generated.CalendarId;
type CalendarStatus = Generated.CalendarStatus;

/** `deino_core::calendar::MAX_RANGE_DAYS`. */
const MAX_RANGE_DAYS = 62;
const DAY_MS = 24 * 60 * 60 * 1000;

const calendarEvents = fixtureCalendarEvents(loadFixtureMeetings(), new Date());
const hidden = new Set<CalendarId>();
let status: CalendarStatus = {
  syncEnabled: true,
  access: 'granted',
  lastSyncedAt: new Date().toISOString(),
  lastError: null,
};

export const events = {
  calendarChanged: mockEvent<Generated.CalendarChanged>('plugin:calendar:calendar-changed'),
} satisfies typeof Generated.events;

/** Changes the status and announces it, like `CalendarService::announce`. */
function announce(next: Partial<CalendarStatus>): CalendarStatus {
  status = { ...status, ...next };
  void events.calendarChanged.emit({ status: { ...status } });
  return { ...status };
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

export const commands = {
  async status() {
    await latency();
    return ok({ ...status });
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
    if (!status.syncEnabled) return ok({ ...status });
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
} satisfies typeof Generated.commands;
