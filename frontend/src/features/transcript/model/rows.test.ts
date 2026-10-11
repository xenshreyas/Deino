import { describe, expect, it } from 'vitest';

import { segment, speaker } from '../test-data';
import { mergeLines } from './lines';
import { buildRows, rowAtTime } from './rows';
import { SpeakerDirectory } from './speakers';

const directory = new SpeakerDirectory([
  speaker('me', 'Me', { isMe: true }),
  speaker('priya', 'Priya'),
]);

function rows(...segments: ReturnType<typeof segment>[]) {
  return buildRows(mergeLines(segments, new Map()), directory);
}

describe('buildRows', () => {
  it('starts a turn when the speaker changes', () => {
    const built = rows(
      segment('a', 0, 'hi', 'priya'),
      segment('b', 2, 'there', 'priya'),
      segment('c', 4, 'hello', 'me'),
    );

    expect(built.map((row) => row.turnStart)).toEqual([true, false, true]);
    expect(built.map((row) => row.speaker.name)).toEqual(['Priya', 'Priya', 'Me']);
  });

  it('starts a new turn for the same speaker after a pause over a minute', () => {
    const built = rows(segment('a', 0, 'one', 'priya'), segment('b', 62, 'two', 'priya'));

    expect(built.map((row) => row.turnStart)).toEqual([true, true]);
  });

  it('keeps a turn going through pauses of up to a minute', () => {
    const built = rows(segment('a', 0, 'one', 'priya'), segment('b', 59, 'two', 'priya'));

    expect(built.map((row) => row.turnStart)).toEqual([true, false]);
  });
});

describe('rowAtTime', () => {
  const built = rows(
    segment('a', 0, 'one', 'priya'),
    segment('b', 10, 'two', 'me'),
    segment('c', 20, 'three', 'priya'),
  );

  it('finds the line being said at a moment: the last that started by then', () => {
    expect(rowAtTime(built, 0)).toBe(0);
    expect(rowAtTime(built, 15_000)).toBe(1);
    expect(rowAtTime(built, 20_000)).toBe(2);
    expect(rowAtTime(built, 9_999_999)).toBe(2);
  });

  it('answers the first line for a moment before anything was said, or none', () => {
    expect(rowAtTime(built, -5)).toBe(0);
    expect(rowAtTime([], 1_000)).toBe(0);
  });
});
