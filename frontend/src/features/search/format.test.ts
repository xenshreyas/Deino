import { describe, expect, test } from 'vitest';

import { asMeetingId } from '@/features/meetings';

import { activeMention, momentTime } from './format';
import { hitTarget } from './hit-target';
import type { Hit } from './types';

describe('moment times', () => {
  test.each([
    [0, '0:00'],
    [61_000, '1:01'],
    [599_999, '9:59'],
    [3_723_000, '1:02:03'],
    [-5, '0:00'],
  ])('%i ms reads as %s', (ms, expected) => {
    expect(momentTime(ms)).toBe(expected);
  });
});

describe('the @ being typed', () => {
  test('a trailing @word is a mention, and the rest is the search', () => {
    expect(activeMention('roadmap @ma')).toEqual({ prefix: 'ma', rest: 'roadmap ' });
    expect(activeMention('@')).toEqual({ prefix: '', rest: '' });
  });

  test('an @ inside a word, a finished mention or a quoted one is not', () => {
    expect(activeMention('maya@acme.com')).toBeNull();
    expect(activeMention('@maya ')).toBeNull();
    expect(activeMention('@"Maya Chen')).toBeNull();
  });
});

function hit(overrides: Partial<Hit>): Hit {
  return {
    kind: 'meeting',
    refId: 'meeting-1',
    meetingId: asMeetingId('meeting-1'),
    tsMs: null,
    endMs: null,
    label: { text: 'Roadmap', highlights: [] },
    snippet: { text: '', highlights: [] },
    date: null,
    score: 1,
    ...overrides,
  };
}

describe('where a hit leads', () => {
  test('a moment opens its meeting at its time; notes open the meeting', () => {
    expect(hitTarget(hit({ kind: 'segment', refId: 's1', tsMs: 61_000 }))).toEqual({
      kind: 'meeting',
      meetingId: 'meeting-1',
      atMs: 61_000,
    });
    expect(hitTarget(hit({ kind: 'note' }))).toEqual({
      kind: 'meeting',
      meetingId: 'meeting-1',
      atMs: null,
    });
  });

  test('people and companies become filters; standalone items go to the list', () => {
    const maya = hit({
      kind: 'person',
      refId: 'p1',
      meetingId: null,
      label: { text: 'Maya', highlights: [] },
    });
    expect(hitTarget(maya)).toEqual({
      kind: 'filter',
      chip: { kind: 'person', id: 'p1', label: 'Maya' },
    });
    expect(hitTarget(hit({ kind: 'action_item', meetingId: null }))).toEqual({
      kind: 'action-items',
    });
    expect(hitTarget(hit({ kind: 'folder', refId: 'f1', meetingId: null }))).toEqual({
      kind: 'folder',
      folderId: 'f1',
    });
  });
});
