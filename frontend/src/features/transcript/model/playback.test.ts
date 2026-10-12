import { describe, expect, it } from 'vitest';

import { segment, speaker } from '../test-data';
import { mergeLines } from './lines';
import { rowPlayingAt, speakerBands } from './playback';
import { buildRows } from './rows';
import { SpeakerDirectory } from './speakers';

const PRIYA = speaker('speaker-priya', 'Priya Shah', { color: 3 });
const rows = buildRows(
  mergeLines(
    [
      segment('a', 0, 'first', 'speaker-priya'),
      segment('b', 2, 'second', 'speaker-priya'),
      segment('c', 10, 'third', 'speaker-priya'),
    ],
    new Map(),
  ),
  new SpeakerDirectory([PRIYA]),
);

describe('rowPlayingAt', () => {
  it('is the line being said, and nothing in a pause', () => {
    expect(rowPlayingAt(rows, 0)).toBe(0);
    expect(rowPlayingAt(rows, 2_500)).toBe(1);
    expect(rowPlayingAt(rows, 5_000)).toBe(-1);
    expect(rowPlayingAt(rows, 10_100)).toBe(2);
    expect(rowPlayingAt([], 0)).toBe(-1);
  });
});

describe('speakerBands', () => {
  it('turns saved lines into bands in their speaker colour', () => {
    expect(speakerBands(rows)[1]).toEqual({ start: 2_000, end: 2_900, color: 3 });
  });
});
