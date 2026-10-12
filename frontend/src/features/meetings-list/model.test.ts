import { describe, expect, test } from 'vitest';

import type { CompanyId, FolderId, PersonId, TagId } from '@/lib/ipc/generated/meetings';

import { groupByDay, rowDuration } from './day-groups';
import { activeChips, filterChoices, filterKey, isAllMeetings, without } from './filters';
import { EMPTY_SELECTION, select, selectAll } from './selection';
import { ALL_MEETINGS, type FilterOptions, type ListedMeeting, type MeetingId } from './types';

const OPTIONS: FilterOptions = {
  people: [{ id: 'p-maya' as PersonId, name: 'Maya Chen', email: null, isMe: false }],
  companies: [{ id: 'c-acme' as CompanyId, name: 'Acme' }],
  tags: [{ id: 't-customer' as TagId, name: 'Customer', color: 1 }],
  folders: [{ id: 'f-sales' as FolderId, name: 'Sales', parentId: null }],
};

const id = (raw: string) => raw as MeetingId;

function meeting(raw: string, createdAt: string): ListedMeeting {
  return {
    id: id(raw),
    title: raw,
    createdAt,
    startedAt: null,
    duration: null,
    status: 'ready',
    attendees: [],
    tags: [],
  };
}

describe('filters', () => {
  test('equal filters share a key whatever the order of their lists', () => {
    const a = { ...ALL_MEETINGS, tags: ['b', 'a'] as TagId[] };
    const b = { ...ALL_MEETINGS, tags: ['a', 'b'] as TagId[] };

    expect(filterKey(a)).toBe(filterKey(b));
    expect(isAllMeetings(ALL_MEETINGS)).toBe(true);
    expect(isAllMeetings(a)).toBe(false);
  });

  test('a list choice toggles its value and a chip names it', () => {
    const tag = filterChoices(OPTIONS, new Date(2026, 9, 12)).find(
      (c) => c.id === 'tags:t-customer',
    );

    const tagged = tag?.apply(ALL_MEETINGS) ?? ALL_MEETINGS;

    expect(tagged.tags).toEqual(['t-customer']);
    expect(activeChips(tagged, OPTIONS)).toEqual([{ field: 'tags', label: 'Customer' }]);
    expect(tag?.apply(tagged).tags).toEqual([]);
    expect(without(tagged, 'tags')).toEqual(ALL_MEETINGS);
  });

  test('date presets end today and audience choices replace each other', () => {
    const choices = filterChoices(OPTIONS, new Date(2026, 9, 12));
    const week = choices.find((c) => c.id === 'date:6')?.apply(ALL_MEETINGS);
    const external = choices.find((c) => c.id === 'external')?.apply(ALL_MEETINGS);

    expect(week?.dateRange).toEqual({ from: '2026-10-06', to: '2026-10-12' });
    expect(choices.find((c) => c.id === 'internal')?.apply(external ?? ALL_MEETINGS).audience).toBe(
      'internal',
    );
  });
});

describe('selection', () => {
  const order = ['a', 'b', 'c', 'd'].map(id);

  test('a click toggles and a Shift-click selects the range from the last click', () => {
    const one = select(EMPTY_SELECTION, { id: id('b'), range: false }, order);
    const range = select(one, { id: id('d'), range: true }, order);
    const off = select(range, { id: id('c'), range: false }, order);

    expect([...range.ids]).toEqual(['b', 'c', 'd']);
    expect([...off.ids]).toEqual(['b', 'd']);
  });

  test('select all takes every matching ID, loaded or not', () => {
    expect(selectAll(order).ids.size).toBe(4);
  });
});

describe('day groups', () => {
  test('rows group under one header per local day, today and yesterday by name', () => {
    const now = new Date(2026, 9, 12, 18);
    const rows = [
      meeting('late', new Date(2026, 9, 12, 15).toISOString()),
      meeting('early', new Date(2026, 9, 12, 9).toISOString()),
      meeting('before', new Date(2026, 9, 11, 9).toISOString()),
      meeting('last-year', new Date(2025, 2, 3, 9).toISOString()),
    ];

    const groups = groupByDay(rows, now).map((group) => [group.label, group.meetings.length]);

    expect(groups).toEqual([
      ['Today', 2],
      ['Yesterday', 1],
      ['Monday, March 3, 2025', 1],
    ]);
  });

  test('lengths read as minutes or hours and minutes', () => {
    expect(rowDuration(null)).toBe('');
    expect(rowDuration(45 * 60)).toBe('45m');
    expect(rowDuration(65 * 60)).toBe('1h 5m');
  });
});
