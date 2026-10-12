import { describe, expect, it } from 'vitest';

import type { CalendarEvent, RecordDecision } from '@/features/calendar';

import { flipChoice, inviteesMailto, isRecurring } from './event-actions';

function event(overrides: Partial<CalendarEvent>): CalendarEvent {
  const base = {
    title: 'Q4 roadmap & pricing',
    seriesId: null,
    attendees: [
      { email: 'ada@example.com', name: 'Ada', rsvp: 'accepted', isOrganizer: true },
      { email: null, name: 'Room 4', rsvp: null, isOrganizer: false },
      { email: 'noor+eng@deino.dev', name: null, rsvp: null, isOrganizer: false },
    ],
  } as const satisfies Partial<CalendarEvent>;
  // Only the fields under test matter here.
  return { ...base, ...overrides } as CalendarEvent;
}

describe('inviteesMailto', () => {
  it('addresses every invitee with an email and encodes the subject', () => {
    expect(inviteesMailto(event({}))).toBe(
      'mailto:ada%40example.com,noor%2Beng%40deino.dev?subject=Q4%20roadmap%20%26%20pricing',
    );
  });

  it('is null when nobody has an address', () => {
    expect(inviteesMailto(event({ attendees: [] }))).toBeNull();
  });
});

describe('the record toggle', () => {
  const records: RecordDecision = { record: true, reason: 'rules' };

  it('flips the current decision for the chosen scope', () => {
    expect(flipChoice(records, 'allOccurrences')).toEqual({
      record: false,
      scope: 'allOccurrences',
    });
    expect(flipChoice({ record: false, reason: 'audienceOff' }, 'thisEvent')).toEqual({
      record: true,
      scope: 'thisEvent',
    });
  });

  it('asks about the series only for recurring events', () => {
    expect(isRecurring(event({}))).toBe(false);
    expect(isRecurring(event({ seriesId: 'weekly' as CalendarEvent['seriesId'] }))).toBe(true);
  });
});
