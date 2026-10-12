import { describe, expect, it } from 'vitest';

import { asTagId } from '@/features/tags';

import { blankDraft, DEFAULT_FIELDS, moved, startAction, toDraft, toInput } from './draft';
import { asAutomationId, type Automation } from './types';

const TAG = asTagId('0192f3a0-6c1e-7b2a-9d4e-0000000000aa');
const NONE = { insights: [], tags: [], folders: [] };

describe('startAction', () => {
  it('starts local steps and points named ones at the first option', () => {
    expect(startAction('notify', NONE)).toEqual({ kind: 'notify', message: '' });
    expect(startAction('run_shell', NONE)).toEqual({
      kind: 'run_shell',
      command: '',
      fields: DEFAULT_FIELDS,
    });
    expect(startAction('add_tag', { ...NONE, tags: [TAG] })).toEqual({
      kind: 'add_tag',
      tagId: TAG,
    });
  });

  it('cannot start a step with nothing to name, or a WIP provider step', () => {
    expect(startAction('add_tag', NONE)).toBeNull();
    expect(startAction('generate_insight', NONE)).toBeNull();
    expect(startAction('slack', NONE)).toBeNull();
    expect(startAction('webhook', NONE)).toBeNull();
  });

  it('keeps the transcript and audio path out of a script payload by default', () => {
    expect(DEFAULT_FIELDS.transcript).toBe(false);
    expect(DEFAULT_FIELDS.audioPath).toBe(false);
  });
});

describe('the draft', () => {
  it('round-trips an automation and drops empty condition groups', () => {
    const automation: Automation = {
      id: asAutomationId('0192f3a0-6c1e-7b2a-9d4e-000000000001'),
      name: 'Recap',
      enabled: true,
      autoRun: false,
      stopOnFailure: true,
      conditions: [[{ kind: 'external', external: true }]],
      steps: [{ kind: 'notify', message: 'Done' }],
      createdAt: '2026-10-12T09:00:00Z',
      updatedAt: '2026-10-12T09:00:00Z',
    };
    const draft = toDraft(automation);

    expect(toInput({ ...draft, conditions: [...draft.conditions, []] })).toEqual({
      name: 'Recap',
      enabled: true,
      autoRun: false,
      stopOnFailure: true,
      conditions: [[{ kind: 'external', external: true }]],
      steps: [{ kind: 'notify', message: 'Done' }],
    });
    expect(blankDraft()).toMatchObject({ enabled: true, autoRun: true, steps: [] });
  });

  it('moves steps within bounds only', () => {
    expect(moved(['a', 'b', 'c'], 1, -1)).toEqual(['b', 'a', 'c']);
    expect(moved(['a', 'b', 'c'], 1, 1)).toEqual(['a', 'c', 'b']);
    expect(moved(['a', 'b'], 0, -1)).toEqual(['a', 'b']);
  });
});
