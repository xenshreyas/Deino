import { describe, expect, test } from 'vitest';

import { dueDay, dueGroup, dueLabel, quickDueChoices } from './dates';
import { activeChips, filterChoices, filterKey, without } from './filters';
import { groupItems } from './groups';
import { applyEdit } from './queries';
import { EMPTY_SELECTION, select, selectAll, visible } from './selection';
import {
  type ActionItemId,
  type ActionItemView,
  type FilterOptions,
  MY_OPEN_ITEMS,
  type PersonId,
} from './types';

/** Friday 9 October 2026, 09:00 local. */
const NOW = new Date(2026, 9, 9, 9, 0);
/** Noon, local, on 2026-10-`day`: how Rust stores due days. */
const noon = (day: number) => new Date(2026, 9, day, 12).toISOString();

const MAYA = 'person-maya' as PersonId;
const OPTIONS: FilterOptions = {
  people: [
    { id: 'person-me' as PersonId, name: 'Shreyas Rao', email: null, isMe: true },
    { id: MAYA, name: 'Maya Chen', email: 'maya@deino.dev', isMe: false },
  ],
  companies: [],
  tags: [],
  folders: [],
};

function item(id: string, dueAt: string | null, status: 'open' | 'done' = 'open'): ActionItemView {
  return {
    id: id as ActionItemId,
    meetingId: null,
    title: id,
    description: '',
    assigneePersonId: null,
    dueAt,
    status,
    priority: null,
    sourceSegmentId: null,
    completedAt: status === 'done' ? NOW.toISOString() : null,
    createdBy: 'ai',
    createdAt: NOW.toISOString(),
    updatedAt: NOW.toISOString(),
    meetingTitle: null,
    meetingHeldAt: null,
    assigneeName: null,
    assigneeIsMe: false,
    sourceStart: null,
  };
}

describe('due days', () => {
  test('group by the local week, which ends on Sunday', () => {
    expect(dueGroup(noon(8), NOW)).toBe('overdue');
    expect(dueGroup(noon(9), NOW)).toBe('today');
    expect(dueGroup(noon(11), NOW)).toBe('this_week');
    expect(dueGroup(noon(12), NOW)).toBe('later');
    expect(dueGroup(null, NOW)).toBe('none');
  });

  test('read as words near today and as dates further out', () => {
    expect(dueLabel(noon(9), NOW)).toBe('Today');
    expect(dueLabel(noon(10), NOW)).toBe('Tomorrow');
    expect(dueLabel(noon(8), NOW)).toBe('Yesterday');
    expect(dueLabel(noon(13), NOW)).toBe('Tue');
    expect(dueLabel(noon(23), NOW)).toBe('Oct 23');
    expect(dueLabel(new Date(2027, 0, 4, 12).toISOString(), NOW)).toBe('Jan 4, 2027');
  });

  test('quick choices are today, tomorrow, next Monday and none', () => {
    const days = quickDueChoices(NOW).map((choice) => choice.day);

    expect(days).toEqual(['2026-10-09', '2026-10-10', '2026-10-12', null]);
    expect(dueDay(noon(16))).toBe('2026-10-16');
  });
});

describe('filters', () => {
  test('the default view shows its scope and status as chips', () => {
    expect(activeChips(MY_OPEN_ITEMS, OPTIONS).map((chip) => chip.label)).toEqual(['Mine', 'Open']);
  });

  test('removing a chip widens the filter to everything on that field', () => {
    const anyone = without(MY_OPEN_ITEMS, 'assignee');
    const overdue = { ...MY_OPEN_ITEMS, due: 'overdue' as const };

    expect(anyone.assignee).toEqual({ kind: 'anyone' });
    expect(without(overdue, 'due').due).toBeNull();
    expect(filterKey(without(overdue, 'due'))).toBe(filterKey(MY_OPEN_ITEMS));
  });

  test('the menu offers people, statuses, due windows and meeting dates', () => {
    const choices = filterChoices(OPTIONS, NOW);
    const maya = choices.find((choice) => choice.label === 'Maya Chen');
    const week = choices.find((choice) => choice.id === 'held:7');

    expect(choices.some((choice) => choice.label === 'Shreyas Rao')).toBe(false);
    expect(maya?.apply(MY_OPEN_ITEMS).assignee).toEqual({ kind: 'person', personId: MAYA });
    expect(week?.apply(MY_OPEN_ITEMS).dateRange).toEqual({ from: '2026-10-03', to: null });
    expect(activeChips(maya?.apply(MY_OPEN_ITEMS) ?? MY_OPEN_ITEMS, OPTIONS)[0]?.label).toBe(
      'Assigned to Maya Chen',
    );
  });
});

describe('selection', () => {
  const order = ['a', 'b', 'c', 'd'] as ActionItemId[];
  const id = (raw: string) => raw as ActionItemId;

  test('clicks toggle and shift-clicks select the range from the last click', () => {
    const one = select(EMPTY_SELECTION, { id: id('b'), range: false }, order);
    const range = select(one, { id: id('d'), range: true }, order);
    const off = select(range, { id: id('c'), range: false }, order);

    expect([...range.ids]).toEqual(['b', 'c', 'd']);
    expect([...off.ids]).toEqual(['b', 'd']);
  });

  test('select all takes everything listed and a filter keeps only what is still listed', () => {
    expect(selectAll(order).ids.size).toBe(4);
    expect([...visible(selectAll(order), [id('a'), id('c')]).ids]).toEqual(['a', 'c']);
  });
});

describe('the list', () => {
  test('groups open items by due day and puts done ones last', () => {
    const groups = groupItems(
      [
        item('late', noon(1)),
        item('now', noon(9)),
        item('done', noon(9), 'done'),
        item('someday', null),
      ],
      NOW,
    );

    expect(groups.map((group) => [group.label, group.items.map((it) => it.title)])).toEqual([
      ['Overdue', ['late']],
      ['Today', ['now']],
      ['No date', ['someday']],
      ['Done', ['done']],
    ]);
  });

  test('a tick applies at once, stamping completion once', () => {
    const done = applyEdit(item('x', null), { field: 'status', status: 'done' }, NOW);
    const reopened = applyEdit(done, { field: 'status', status: 'open' }, NOW);

    expect([done.status, done.completedAt]).toEqual(['done', NOW.toISOString()]);
    expect([reopened.status, reopened.completedAt]).toEqual(['open', null]);
  });
});
