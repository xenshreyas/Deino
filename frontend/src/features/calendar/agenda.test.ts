import { describe, expect, it } from 'vitest';

import { agendaDays, comingUpRange, dayLabel, eventTime, startsIn } from './agenda';
import type { CalendarEvent, CalendarEventId } from './types';

/** Local noon, so day boundaries never depend on the test machine's zone. */
const NOW = new Date(2026, 9, 9, 12, 0);

/** An event from `start` to `end` (local minutes after midnight of `day`). */
function event(
  title: string,
  [day, startMinutes, endMinutes]: [number, number, number],
  isAllDay = false,
): CalendarEvent {
  const start = new Date(2026, 9, 9 + day, 0, startMinutes);
  const end = new Date(2026, 9, 9 + day, 0, endMinutes);
  return {
    id: title as CalendarEventId,
    accountId: 'account' as CalendarEvent['accountId'],
    calendarId: null,
    providerEventId: title,
    icalUid: null,
    seriesId: null,
    title,
    description: '',
    startAt: start.toISOString(),
    endAt: end.toISOString(),
    isAllDay,
    conferenceUrl: null,
    platform: null,
    organizerEmail: null,
    attendees: [],
    myRsvp: 'accepted',
    isExternal: false,
    recordOverride: null,
    createdAt: start.toISOString(),
    updatedAt: start.toISOString(),
  };
}

describe('comingUpRange', () => {
  it('spans from the start of today to the end of the fourteenth day, all day long', () => {
    const morning = comingUpRange(new Date(2026, 9, 9, 8, 0));
    const evening = comingUpRange(new Date(2026, 9, 9, 23, 0));

    expect(morning).toEqual(evening);
    expect(morning.start).toBe(new Date(2026, 9, 9).toISOString());
    expect(morning.end).toBe(new Date(2026, 9, 24).toISOString());
  });
});

describe('agendaDays', () => {
  it('drops finished events and files a running one under today', () => {
    const finished = event('Standup', [0, 9 * 60, 9 * 60 + 15]);
    const running = event('Workshop', [-1, 9 * 60, 37 * 60]);
    const later = event('Design crit', [0, 15 * 60, 16 * 60]);
    const tomorrow = event('1:1', [1, 10 * 60, 10 * 60 + 30]);

    const days = agendaDays([finished, running, later, tomorrow], NOW);

    expect(days.map((day) => day.label)).toEqual(['Today', 'Tomorrow']);
    expect(days[0]?.events.map((e) => e.title)).toEqual(['Workshop', 'Design crit']);
  });

  it('names later days by weekday and date', () => {
    expect(dayLabel(new Date(2026, 9, 12), NOW)).toBe('Mon, Oct 12');
  });
});

describe('row times', () => {
  it('shows the local start, or All day', () => {
    expect(eventTime(event('Planning', [0, 14 * 60, 15 * 60]))).toBe('14:00');
    expect(eventTime(event('Offsite', [0, 0, 24 * 60], true))).toBe('All day');
  });

  it('counts down in the last hour, says Now while running, and stays quiet otherwise', () => {
    expect(startsIn(event('Soon', [0, 12 * 60 + 12, 13 * 60]), NOW)).toBe('in 12m');
    expect(startsIn(event('Running', [0, 11 * 60 + 50, 12 * 60 + 30]), NOW)).toBe('Now');
    expect(startsIn(event('Later', [0, 15 * 60, 16 * 60]), NOW)).toBeNull();
    expect(startsIn(event('Offsite', [0, 0, 24 * 60], true), NOW)).toBeNull();
  });
});
