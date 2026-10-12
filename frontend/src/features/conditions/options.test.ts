import { describe, expect, it } from 'vitest';

import { asPersonId } from '@/features/entities';
import { asTagId } from '@/features/tags';

import { ALL_CONDITION_KINDS } from './condition-groups';
import { splitList } from './condition-value';
import {
  blankCondition,
  type ConditionOptions,
  pointAt,
  pointing,
  startCondition,
} from './options';

const ALICE = asPersonId('0192f3a0-6c1e-7b2a-9d4e-000000000004');
const SALES = asTagId('0192f3a0-6c1e-7b2a-9d4e-0000000000aa');
const OPTIONS: ConditionOptions = {
  templates: [],
  tags: [{ id: SALES, label: 'Sales' }],
  folders: [],
  people: [{ id: ALICE, label: 'Alice' }],
  calendars: [],
};

describe('conditions', () => {
  it('start blank when they need no option, and point at the first when they do', () => {
    expect(blankCondition('invitee_count')).toEqual({
      kind: 'invitee_count',
      comparison: 'equal',
      count: 2,
    });
    expect(startCondition('lacks_tag', OPTIONS)).toEqual({ kind: 'lacks_tag', tagId: SALES });
    expect(startCondition('has_participant', OPTIONS)).toEqual({
      kind: 'has_participant',
      personId: ALICE,
    });
    expect(startCondition('calendar_is', OPTIONS)).toBeNull();
  });

  it('every kind can start or names what it needs', () => {
    for (const kind of ALL_CONDITION_KINDS) {
      const started = startCondition(kind, OPTIONS);
      if (started !== null) expect(started.kind).toBe(kind);
    }
    expect(ALL_CONDITION_KINDS).toHaveLength(19);
  });

  it('pickers point only at options that exist', () => {
    const tagged = { kind: 'has_tag', tagId: SALES } as const;

    expect(pointing(tagged, OPTIONS)).toEqual({ choices: OPTIONS.tags, value: SALES });
    expect(pointAt(tagged, 'missing', OPTIONS)).toBe(tagged);
    expect(pointing({ kind: 'external', external: true }, OPTIONS)).toBeNull();
  });

  it('domain lists split on commas', () => {
    expect(splitList(' acme.com, ,acme.io ')).toEqual(['acme.com', 'acme.io']);
  });
});
