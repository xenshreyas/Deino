import { describe, expect, it } from 'vitest';

import {
  duration,
  initials,
  matchesQuery,
  paletteSlot,
  relativeDay,
  visibleRows,
  type Listable,
} from './format';

const NOW = new Date('2026-10-11T12:00:00');

function row(name: string, lastMetAt: string | null, haystack: (string | null)[] = []): Listable {
  return { name, lastMetAt, haystack };
}

describe('entity formatting', () => {
  it('takes initials from the first and last word', () => {
    expect(initials('Maya Chen')).toBe('MC');
    expect(initials('  Priya  Nair Shah ')).toBe('PS');
    expect(initials('Noor')).toBe('N');
  });

  it('gives an ID the same palette slot every time', () => {
    const slot = paletteSlot('0192f3a0-6c1e-7b2a-9d4e-1f2a3b4c5d6e');
    expect(slot).toBeGreaterThanOrEqual(1);
    expect(slot).toBeLessThanOrEqual(6);
    expect(paletteSlot('0192f3a0-6c1e-7b2a-9d4e-1f2a3b4c5d6e')).toBe(slot);
  });

  it('says how long ago a meeting was the way a person would', () => {
    expect(relativeDay('2026-10-11T09:00:00', NOW)).toBe('Today');
    expect(relativeDay('2026-10-10T09:00:00', NOW)).toBe('Yesterday');
    expect(relativeDay('2026-10-07T09:00:00', NOW)).toBe('4 days ago');
    expect(relativeDay('2026-09-28T09:00:00', NOW)).toBe('Sep 28');
    expect(relativeDay('2025-09-28T09:00:00', NOW)).toBe('Sep 28, 2025');
    expect(relativeDay('not a date', NOW)).toBe('');
  });

  it('shows durations in minutes and hours', () => {
    expect(duration(null)).toBe('');
    expect(duration(2640)).toBe('44m');
    expect(duration(3600)).toBe('1h');
    expect(duration(4980)).toBe('1h 23m');
  });
});

describe('entity lists', () => {
  it('matches every query word against the start of a name or field word', () => {
    const maya = row('Maya Chen', null, ['maya@deino.dev', 'Deino']);
    expect(matchesQuery(maya, 'ch')).toBe(true);
    expect(matchesQuery(maya, 'deino maya')).toBe(true);
    expect(matchesQuery(maya, 'hen')).toBe(false);
    expect(matchesQuery(maya, '  ')).toBe(true);
  });

  it('sorts by last met with the never-met last, or by name', () => {
    const rows = [
      row('Noor', null),
      row('Aisha', '2026-10-01T10:00:00Z'),
      row('Tom', '2026-10-08T10:00:00Z'),
    ];
    expect(visibleRows(rows, '', 'recent').map((r) => r.name)).toEqual(['Tom', 'Aisha', 'Noor']);
    expect(visibleRows(rows, '', 'name').map((r) => r.name)).toEqual(['Aisha', 'Noor', 'Tom']);
    expect(visibleRows(rows, 'to', 'recent').map((r) => r.name)).toEqual(['Tom']);
  });
});
