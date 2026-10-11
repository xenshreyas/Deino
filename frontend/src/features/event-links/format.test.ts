import { describe, expect, test } from 'vitest';

import { dayLabel, eventTitle, eventWhen, guestNames } from './format';
import { calendarEvent } from './test-event-links';

/** Local times, so the labels do not depend on the machine's zone. */
const NOW = new Date(2026, 9, 9, 10, 0);
const local = (day: number, hours: number, minutes = 0) =>
  new Date(2026, 9, day, hours, minutes).toISOString();

describe('eventWhen', () => {
  test('names today, yesterday and tomorrow, else the date', () => {
    expect(eventWhen(calendarEvent('Sync', local(9, 10)), NOW)).toBe('Today 10:00–10:30');
    expect(eventWhen(calendarEvent('Sync', local(8, 16)), NOW)).toBe('Yesterday 16:00–16:30');
    expect(eventWhen(calendarEvent('Sync', local(10, 9, 15)), NOW)).toBe('Tomorrow 09:15–09:45');
    expect(eventWhen(calendarEvent('Sync', local(2, 14)), NOW)).toBe('Oct 2, 14:00–14:30');
  });
});

describe('dayLabel', () => {
  test('is relative near today, the date otherwise, with the year only when it differs', () => {
    expect(dayLabel(local(9, 8), NOW)).toBe('Today');
    expect(dayLabel(local(2, 8), NOW)).toBe('Oct 2');
    expect(dayLabel(new Date(2025, 11, 31, 9).toISOString(), NOW)).toBe('Dec 31, 2025');
    expect(dayLabel('not a date', NOW)).toBe('');
  });
});

describe('guestNames', () => {
  test('lists the organiser first and leaves out who declined', () => {
    expect(guestNames(calendarEvent('Sync', local(9, 10)))).toEqual(['Ada', 'Bob']);
  });

  test('falls back to the address when a guest has no name', () => {
    const event = calendarEvent('Sync', local(9, 10), {
      attendees: [{ email: 'dan@brightkite.com', name: null, rsvp: null, isOrganizer: false }],
    });
    expect(guestNames(event)).toEqual(['dan@brightkite.com']);
  });
});

describe('eventTitle', () => {
  test('calls a blank title an untitled event', () => {
    expect(eventTitle(calendarEvent('  ', local(9, 10)))).toBe('Untitled event');
  });
});
