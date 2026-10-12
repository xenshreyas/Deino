/**
 * Test helpers for Home: a mock of the generated calendar bindings with
 * only what Coming up calls, and one upcoming event.
 */
import type { EventCallback } from '@tauri-apps/api/event';
import { addMinutes } from 'date-fns';
import { vi } from 'vitest';

import type * as Generated from '@/lib/ipc/generated/calendar';

/** The calendar bindings Coming up and its menu use, as mocks. */
export function homeCalendarBindings() {
  const commands = {
    status: vi.fn<typeof Generated.commands.status>(),
    listCalendars: vi.fn<typeof Generated.commands.listCalendars>(),
    upcoming: vi.fn<typeof Generated.commands.upcoming>(),
    recordPlan: vi.fn<typeof Generated.commands.recordPlan>(),
    chooseRecord: vi.fn<typeof Generated.commands.chooseRecord>(),
    join: vi.fn<typeof Generated.commands.join>(),
    lastMeeting: vi.fn<typeof Generated.commands.lastMeeting>(),
    openInCalendar: vi.fn<typeof Generated.commands.openInCalendar>(),
  } satisfies Partial<typeof Generated.commands>;
  const listen = vi.fn<(handler: EventCallback<unknown>) => Promise<() => void>>(() =>
    Promise.resolve(vi.fn()),
  );
  return { commands, events: { calendarChanged: { listen }, reminderShown: { listen } } };
}

/** A 30-minute Zoom call `title` with an outside guest, `inMinutes` from now. */
export function eventIn(title: string, inMinutes: number): Generated.CalendarEvent {
  const start = addMinutes(new Date(), inMinutes);
  return {
    id: title as Generated.CalendarEventId,
    accountId: 'acc-icloud' as Generated.CalendarAccountId,
    calendarId: 'cal-work' as Generated.CalendarId,
    providerEventId: title,
    icalUid: null,
    seriesId: null,
    title,
    description: '',
    startAt: start.toISOString(),
    endAt: addMinutes(start, 30).toISOString(),
    isAllDay: false,
    conferenceUrl: 'https://zoom.us/j/1',
    platform: 'zoom',
    organizerEmail: 'noor@deino.dev',
    attendees: [
      { email: 'maya@deino.dev', name: null, rsvp: 'accepted', isOrganizer: false },
      { email: 'marcus@brightkite.com', name: 'Marcus', rsvp: 'accepted', isOrganizer: false },
    ],
    myRsvp: 'accepted',
    isExternal: true,
    recordOverride: null,
    createdAt: start.toISOString(),
    updatedAt: start.toISOString(),
  };
}

/** Sync on with access, no direct connectors. */
export const SYNCED: Generated.CalendarStatus = {
  syncEnabled: true,
  access: 'granted',
  lastSyncedAt: null,
  lastError: null,
  connectors: [],
};
