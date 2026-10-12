/**
 * Test helpers for the calendar components: the generated bindings mocked
 * with their real types (a Rust signature change breaks this file, not a
 * test at runtime), and sample rows.
 */
import type { EventCallback } from '@tauri-apps/api/event';
import { addMinutes } from 'date-fns';
import { vi } from 'vitest';

import type * as Generated from '@/lib/ipc/generated/calendar';

/**
 * The mocked `@/lib/ipc/generated/calendar` module: every command a typed
 * `vi.fn`, and `CalendarChanged`'s `listen` recording the UI's handler
 * (read it from `listen.mock.calls`).
 */
export function calendarBindings() {
  const commands = {
    status: vi.fn<typeof Generated.commands.status>(),
    enable: vi.fn<typeof Generated.commands.enable>(),
    disable: vi.fn<typeof Generated.commands.disable>(),
    refresh: vi.fn<typeof Generated.commands.refresh>(),
    openAccessSettings: vi.fn<typeof Generated.commands.openAccessSettings>(),
    listCalendars: vi.fn<typeof Generated.commands.listCalendars>(),
    setCalendarVisible: vi.fn<typeof Generated.commands.setCalendarVisible>(),
    upcoming: vi.fn<typeof Generated.commands.upcoming>(),
    event: vi.fn<typeof Generated.commands.event>(),
    connect: vi.fn<typeof Generated.commands.connect>(),
    disconnect: vi.fn<typeof Generated.commands.disconnect>(),
    meetingEvent: vi.fn<typeof Generated.commands.meetingEvent>(),
    eventSuggestions: vi.fn<typeof Generated.commands.eventSuggestions>(),
    linkEvent: vi.fn<typeof Generated.commands.linkEvent>(),
    unlinkEvent: vi.fn<typeof Generated.commands.unlinkEvent>(),
    relatedMeetings: vi.fn<typeof Generated.commands.relatedMeetings>(),
    recordPlan: vi.fn<typeof Generated.commands.recordPlan>(),
    chooseRecord: vi.fn<typeof Generated.commands.chooseRecord>(),
    join: vi.fn<typeof Generated.commands.join>(),
    reminder: vi.fn<typeof Generated.commands.reminder>(),
    dismissReminder: vi.fn<typeof Generated.commands.dismissReminder>(),
    setOpenNote: vi.fn<typeof Generated.commands.setOpenNote>(),
    lastMeeting: vi.fn<typeof Generated.commands.lastMeeting>(),
    openInCalendar: vi.fn<typeof Generated.commands.openInCalendar>(),
  } satisfies typeof Generated.commands;
  const listen = vi.fn<(handler: EventCallback<Generated.CalendarChanged>) => Promise<() => void>>(
    () => Promise.resolve(vi.fn()),
  );
  const listenReminder = vi.fn<
    (handler: EventCallback<Generated.ReminderShown>) => Promise<() => void>
  >(() => Promise.resolve(vi.fn()));
  return {
    commands,
    events: { calendarChanged: { listen }, reminderShown: { listen: listenReminder } },
  };
}

/** A status: sync on with access by default, and no direct connectors. */
export function status(
  overrides: Partial<Generated.CalendarStatus> = {},
): Generated.CalendarStatus {
  return {
    syncEnabled: true,
    access: 'granted',
    lastSyncedAt: new Date().toISOString(),
    lastError: null,
    connectors: [],
    ...overrides,
  };
}

/** A direct connector: set up but not connected by default. */
export function connector(
  provider: Generated.CalendarProvider,
  overrides: Partial<Generated.ConnectorStatus> = {},
): Generated.ConnectorStatus {
  return {
    provider,
    configured: true,
    connected: false,
    lastSyncedAt: null,
    lastError: null,
    ...overrides,
  };
}

/** The "Work" calendar of an iCloud account. */
export function workCalendar(
  overrides: Partial<Generated.CalendarView> = {},
): Generated.CalendarView {
  return {
    id: 'cal-work' as Generated.CalendarId,
    title: 'Work',
    color: '#34AADC',
    accountId: 'acc-icloud' as Generated.CalendarAccountId,
    accountName: 'iCloud',
    provider: 'apple',
    visible: true,
    ...overrides,
  };
}

/** A 30-minute Zoom meeting `title` starting `inMinutes` from now. */
export function meetingIn(title: string, inMinutes: number): Generated.CalendarEvent {
  const start = addMinutes(new Date(), inMinutes);
  return {
    id: title as Generated.CalendarEventId,
    accountId: 'acc-icloud' as Generated.CalendarAccountId,
    calendarId: 'cal-work' as Generated.CalendarId,
    providerEventId: title,
    icalUid: `${title}@deino.dev`,
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
      { email: 'shreyas@deino.dev', name: null, rsvp: 'accepted', isOrganizer: false },
      { email: 'marcus@brightkite.com', name: 'Marcus', rsvp: 'accepted', isOrganizer: false },
    ],
    myRsvp: 'accepted',
    isExternal: true,
    recordOverride: null,
    createdAt: start.toISOString(),
    updatedAt: start.toISOString(),
  };
}
