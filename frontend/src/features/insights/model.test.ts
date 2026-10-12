import { describe, expect, it } from 'vitest';

import { toDraft, toInput } from './editor/draft';
import { columnsOf, formatMoment, formatValue, nextSort, rowsOf, sortRows } from './model';
import { asMeetingId } from '@/features/meetings';

import { asInsightId, type InsightDef, type InsightTable } from './types';

const def: InsightDef = {
  id: asInsightId('insight-1'),
  name: 'Discovery scorecard',
  description: 'Score the call.',
  fields: [
    { name: 'Budget', kind: { type: 'number' } },
    { name: 'Health', kind: { type: 'enum', options: ['Red', 'Green'] } },
  ],
  cardinality: { kind: 'fixed', count: 1 },
  applyWhen: { mode: 'automatic', conditions: [[{ kind: 'external', external: true }]] },
  revision: 2,
  createdAt: '2026-10-08T09:00:00Z',
  updatedAt: '2026-10-08T09:00:00Z',
};

function run(meeting: string, budget: number | null, revision: number) {
  return {
    meetingTitle: meeting,
    meetingStartedAt: meeting === 'Acme' ? '2026-10-08T16:00:00Z' : '2026-10-07T16:00:00Z',
    run: {
      insightId: def.id,
      meetingId: asMeetingId(meeting),
      status: 'done' as const,
      answers: [
        {
          value: {
            kind: 'fields' as const,
            fields: [
              {
                name: 'Budget',
                value: budget === null ? null : { type: 'number' as const, value: budget },
              },
            ],
          },
          speaker: 'Maya Chen',
          timestampMs: 271_000,
          segmentId: null,
        },
      ],
      revision,
      error: null,
      requestedAt: '2026-10-08T17:00:00Z',
      generatedAt: '2026-10-08T17:00:00Z',
    },
  };
}

const table: InsightTable = {
  def,
  rows: [run('Acme', 40_000, 2), run('Globex', null, 1)],
  progress: { pending: 0, done: 2, failed: 0 },
};

describe('insight values', () => {
  it('read as a person writes them', () => {
    expect(formatValue({ type: 'number', value: 40_000 })).toBe('40,000');
    expect(formatValue({ type: 'number', value: 4.5 })).toBe('4.5');
    expect(formatValue({ type: 'boolean', value: false })).toBe('No');
    expect(formatValue({ type: 'date', value: '2026-10-08' })).toBe('Oct 8, 2026');
    expect(formatValue({ type: 'choice', value: 'Green' })).toBe('Green');
    expect(formatValue(null)).toBe('');
    expect(formatMoment(271_000)).toBe('04:31');
    expect(formatMoment(3_847_000)).toBe('1:04:07');
  });
});

describe('the insight table', () => {
  it('has a column per field between the meeting and the moment', () => {
    expect(columnsOf(def).map((column) => column.label)).toEqual([
      'Meeting',
      'Date',
      'Budget',
      'Health',
      'Said',
    ]);
    expect(columnsOf({ ...def, fields: [] }).map((column) => column.key)).toContain('answer');
  });

  it('marks answers to an earlier revision as stale', () => {
    expect(rowsOf(table).map((row) => [row.meetingTitle, row.stale])).toEqual([
      ['Acme', false],
      ['Globex', true],
    ]);
  });

  it('sorts by a typed column with empty values last, either way', () => {
    const columns = columnsOf(def);
    const rows = rowsOf(table);
    const budget = columns.find((column) => column.key === 'field:Budget');
    if (budget === undefined) throw new Error('no budget column');
    const asc = nextSort({ key: 'date', direction: 'desc' }, budget);

    expect(asc).toEqual({ key: 'field:Budget', direction: 'desc' });
    expect(sortRows(rows, columns, asc).map((row) => row.meetingTitle)).toEqual(['Acme', 'Globex']);
    expect(sortRows(rows, columns, nextSort(asc, budget)).map((row) => row.meetingTitle)).toEqual([
      'Acme',
      'Globex',
    ]);
  });
});

describe('the editor draft', () => {
  it('round-trips an insight, splitting enum options and dropping blank fields', () => {
    const draft = toDraft(def);
    const health = draft.fields[1];
    if (health === undefined) throw new Error('no health field');

    expect(draft.when).toBe('matching');
    expect(health.options).toBe('Red, Green');
    const input = toInput({
      ...draft,
      fields: [...draft.fields, { key: 'blank', name: '  ', type: 'text', options: '' }],
    });
    expect(input).toEqual({
      name: def.name,
      description: def.description,
      fields: def.fields,
      cardinality: def.cardinality,
      applyWhen: def.applyWhen,
    });
    expect(toInput({ ...draft, when: 'always' }).applyWhen).toEqual({
      mode: 'automatic',
      conditions: [],
    });
    expect(toInput({ ...draft, when: 'manual', countMode: 'auto' })).toMatchObject({
      applyWhen: { mode: 'manual' },
      cardinality: { kind: 'auto' },
    });
  });
});
