/**
 * Test helpers for the event-links components: the `calendar` bindings
 * mocked with their real types, and sample events.
 */
import { vi } from 'vitest';

import type * as Generated from '@/lib/ipc/generated/calendar';

/** The mocked `@/lib/ipc/generated/calendar` module: every command a typed `vi.fn`. */
export function linkBindings() {
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
  } satisfies typeof Generated.commands;
  return { commands, events: {} };
}

/** A 30-minute event `title` starting at `startAt`, with Ada organising and Bob invited. */
export function calendarEvent(
  title: string,
  startAt: string,
  overrides: Partial<Generated.CalendarEvent> = {},
): Generated.CalendarEvent {
  const end = new Date(Date.parse(startAt) + 30 * 60_000).toISOString();
  return {
    id: `event-${title}` as Generated.CalendarEventId,
    accountId: 'acc-icloud' as Generated.CalendarAccountId,
    calendarId: null,
    providerEventId: title,
    icalUid: null,
    seriesId: null,
    title,
    description: '',
    startAt,
    endAt: end,
    isAllDay: false,
    conferenceUrl: null,
    platform: null,
    organizerEmail: 'ada@acme.com',
    attendees: [
      { email: 'bob@acme.com', name: 'Bob', rsvp: 'accepted', isOrganizer: false },
      { email: 'ada@acme.com', name: 'Ada', rsvp: 'accepted', isOrganizer: true },
      { email: 'carol@acme.com', name: 'Carol', rsvp: 'declined', isOrganizer: false },
    ],
    myRsvp: 'accepted',
    isExternal: true,
    recordOverride: null,
    createdAt: startAt,
    updatedAt: startAt,
    ...overrides,
  };
}
