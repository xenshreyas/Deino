import { describe, expect, it } from 'vitest';

import { liveUpdate, segment } from '../test-data';
import { applyUpdate, type LiveLines, mergeLines } from './lines';

const NONE: LiveLines = new Map();

describe('mergeLines', () => {
  it('orders saved and live lines by when they were spoken', () => {
    const live = applyUpdate(NONE, liveUpdate('b', 'open'));

    const lines = mergeLines(
      [segment('a', 0, 'first', null), segment('c', 0.9, 'last', null)],
      live,
    );

    expect(lines.map((line) => [line.id, line.open])).toEqual([
      ['a', false],
      ['b', true],
      ['c', false],
    ]);
  });

  it('shows the newest text of a saved line and keeps its speaker, which events lack', () => {
    const live = applyUpdate(NONE, liveUpdate('a', 'final', 'first words and more'));

    const [line] = mergeLines([segment('a', 0, 'first words', 'speaker-me')], live);

    expect(line).toMatchObject({
      text: 'first words and more',
      channel: 'mic',
      open: false,
      speakerId: 'speaker-me',
    });
  });

  it('hides discarded echo even while a stale saved row still has it', () => {
    const open = applyUpdate(NONE, liveUpdate('a', 'open'));
    const live = applyUpdate(open, liveUpdate('a', 'discarded'));

    expect(mergeLines([segment('a', 0, 'echo', null)], live)).toEqual([]);
  });

  it('skips deleted rows and lines with no words yet', () => {
    const deleted = { ...segment('a', 0, 'gone', null), isDeleted: true };
    const live = applyUpdate(NONE, liveUpdate('b', 'open', ' '));

    expect(mergeLines([deleted], live)).toEqual([]);
  });

  it('keeps the saved order as served when nothing is live', () => {
    const saved = [segment('a', 0, 'one', null), segment('b', 1, 'two', null)];

    expect(mergeLines(saved, NONE).map((line) => line.text)).toEqual(['one', 'two']);
  });
});
