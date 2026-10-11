/**
 * The fixture meetings' calendar events as the `calendar` mock serves them:
 * each meeting's own event, exactly as `deino_testkit`'s seeder stores it
 * (same IDs, account and "Work" calendar), plus a copy of it in the coming
 * days so "Coming up" has something to show. The first copy is under way and
 * the second starts in 25 minutes, so "Now" and the countdown show too.
 */
import { addDays, addMinutes, setHours, setMinutes, startOfDay } from 'date-fns';

import type * as Generated from '@/lib/ipc/generated/calendar';

import { fixtureUuid } from './fixture-ids';
import type { FixtureMeeting } from './fixture-schema';

type CalendarEvent = Generated.CalendarEvent;

/** `deino_testkit::fixture::USER_EMAIL`: the fixtures' "me". */
const USER_EMAIL = 'shreyas@deino.dev';
/** The fixture account, calendar and colour (`deino_testkit::fixture::ids`). */
export const FIXTURE_ACCOUNT = {
  id: fixtureUuid('calendar-account') as Generated.CalendarAccountId,
  name: 'Deino fixtures',
};
export const FIXTURE_CALENDAR = {
  id: fixtureUuid('calendar/work') as Generated.CalendarId,
  title: 'Work',
  color: '#34AADC',
};

const PLATFORMS: Record<string, Generated.ConferencePlatform | null> = {
  google_meet: 'google_meet',
  zoom: 'zoom',
  teams: 'teams',
  in_person: null,
};

/** One fixture meeting's event, starting at `start`, under `key`. */
function eventOf(meeting: FixtureMeeting, start: Date, key: string): CalendarEvent {
  const fixture = meeting.calendarEvent;
  const duration = Date.parse(fixture.endAt) - Date.parse(fixture.startAt);
  const me = meeting.attendees.find((attendee) => attendee.email === USER_EMAIL);
  return {
    id: fixtureUuid(key) as Generated.CalendarEventId,
    accountId: FIXTURE_ACCOUNT.id,
    calendarId: FIXTURE_CALENDAR.id,
    providerEventId:
      key === `event/${meeting.id}` ? fixture.id : `${fixture.id}/${start.getTime()}`,
    icalUid: fixture.icalUid,
    seriesId: fixture.recurrence === 'none' ? null : (fixture.icalUid as Generated.SeriesId),
    title: fixture.title,
    description: fixture.description,
    startAt: start.toISOString(),
    endAt: new Date(start.getTime() + duration).toISOString(),
    isAllDay: false,
    conferenceUrl: fixture.conferenceUrl,
    platform: PLATFORMS[fixture.platform] ?? null,
    organizerEmail: fixture.organizerEmail,
    attendees: meeting.attendees.map((attendee) => ({
      email: attendee.email,
      name: attendee.name,
      rsvp: attendee.rsvp,
      isOrganizer: attendee.role === 'organizer',
    })),
    myRsvp: me?.rsvp ?? null,
    isExternal: meeting.attendees.some((attendee) => !attendee.email.endsWith('@deino.dev')),
    recordOverride: null,
    createdAt: meeting.startedAt,
    updatedAt: meeting.startedAt,
  };
}

/** When the `index`th copy starts: one under way, one soon, then two a day. */
function upcomingStart(meeting: FixtureMeeting, index: number, now: Date): Date {
  if (index === 0) return addMinutes(now, -10);
  if (index === 1) return addMinutes(now, 25);
  const original = new Date(meeting.calendarEvent.startAt);
  const day = addDays(startOfDay(now), Math.floor(index / 2));
  return setMinutes(setHours(day, original.getHours()), original.getMinutes());
}

/** Every fixture event, then its copy in the coming days, relative to `now`. */
export function fixtureCalendarEvents(
  meetings: readonly FixtureMeeting[],
  now: Date,
): CalendarEvent[] {
  const recorded = meetings.map((meeting) =>
    eventOf(meeting, new Date(meeting.calendarEvent.startAt), `event/${meeting.id}`),
  );
  const coming = meetings.map((meeting, index) =>
    eventOf(meeting, upcomingStart(meeting, index, now), `upcoming/${meeting.id}`),
  );
  return [...recorded, ...coming];
}
