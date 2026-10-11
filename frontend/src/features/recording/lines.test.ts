import { describe, expect, it } from 'vitest';

import { asMeetingId } from '@/features/meetings';

import { applyUpdate, type LiveLines, mergeLines } from './lines';
import type { Segment, SegmentId, SegmentUpdate } from './types';

const MEETING = asMeetingId('meeting-live');

function segment(id: string, start: number, text: string): Segment {
  return {
    id: id as SegmentId,
    meetingId: MEETING,
    channel: 'system',
    speakerId: null,
    start,
    end: start + 1_000,
    text,
    words: [],
    isDeleted: false,
    editedAt: null,
  };
}

function update(id: string, state: SegmentUpdate['state'], text = 'hello'): SegmentUpdate {
  return {
    meetingId: MEETING,
    segmentId: id as SegmentId,
    channel: 'mic',
    start: 500,
    end: 500,
    text,
    state,
  };
}

const NONE: LiveLines = new Map();

describe('mergeLines', () => {
  it('orders saved and live lines by when they were spoken', () => {
    const live = applyUpdate(NONE, update('b', 'open'));

    const lines = mergeLines([segment('a', 0, 'first'), segment('c', 900, 'last')], live);

    expect(lines.map((line) => [line.id, line.open])).toEqual([
      ['a', false],
      ['b', true],
      ['c', false],
    ]);
  });

  it('shows the newest text of a line that is also saved', () => {
    const live = applyUpdate(NONE, update('a', 'final', 'first words and more'));

    const [line] = mergeLines([segment('a', 0, 'first words')], live);

    expect(line).toMatchObject({ text: 'first words and more', channel: 'mic', open: false });
  });

  it('hides discarded echo even while a stale saved row still has it', () => {
    const live = applyUpdate(applyUpdate(NONE, update('a', 'open')), update('a', 'discarded'));

    expect(mergeLines([segment('a', 0, 'echo')], live)).toEqual([]);
  });

  it('skips deleted rows and lines with no words yet', () => {
    const deleted = { ...segment('a', 0, 'gone'), isDeleted: true };
    const live = applyUpdate(NONE, update('b', 'open', ' '));

    expect(mergeLines([deleted], live)).toEqual([]);
  });
});
