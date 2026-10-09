import { describe, expect, test } from 'vitest';

import { bucketMeetings, meetingTime } from './date-buckets';
import { asMeetingId, type Meeting } from './types';

/** Thursday 8 October 2026, 15:00 local time. */
const NOW = new Date(2026, 9, 8, 15, 0);

const meeting = (id: string, createdAt: Date | string): Meeting => ({
  id: asMeetingId(id),
  title: id,
  createdAt: typeof createdAt === 'string' ? createdAt : createdAt.toISOString(),
  updatedAt: typeof createdAt === 'string' ? createdAt : createdAt.toISOString(),
});

describe('meeting date buckets', () => {
  test('groups newest-first meetings under Today, Yesterday, This week and months', () => {
    const buckets = bucketMeetings(
      [
        meeting('today', new Date(2026, 9, 8, 9)),
        meeting('yesterday', new Date(2026, 9, 7, 16)),
        meeting('monday', new Date(2026, 9, 5, 10)),
        meeting('september', new Date(2026, 8, 20)),
        meeting('last-year', new Date(2025, 11, 1)),
      ],
      NOW,
    );
    expect(buckets.map(({ label, meetings }) => [label, meetings.map(({ id }) => id)])).toEqual([
      ['Today', ['today']],
      ['Yesterday', ['yesterday']],
      ['This week', ['monday']],
      ['September', ['september']],
      ['December 2025', ['last-year']],
    ]);
  });

  test('an unreadable timestamp lands in Earlier instead of breaking the list', () => {
    expect(bucketMeetings([meeting('broken', 'not a date')], NOW)[0]?.label).toBe('Earlier');
  });

  test('the trailing time gets coarser as meetings age', () => {
    expect(meetingTime(new Date(2026, 9, 8, 9, 5).toISOString(), NOW)).toBe('09:05');
    expect(meetingTime(new Date(2026, 9, 5, 10).toISOString(), NOW)).toBe('Mon 10:00');
    expect(meetingTime(new Date(2026, 8, 20).toISOString(), NOW)).toBe('Sep 20');
  });
});
